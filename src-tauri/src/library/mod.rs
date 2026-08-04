use crate::db::Database;
use crate::error::AppResult;
use crate::library::hashing::{compute_oshash, compute_phash_from_image};
use crate::media::FfmpegProcessor;
use crate::models::ScanProgress;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Semaphore;

pub mod auto_tag;
pub mod duplicates;
pub mod hashing;
pub mod import;
pub mod thumbnail;

pub struct LibraryScanner;

type ProgressCb = Box<dyn Fn(ScanProgress) + Send>;

impl LibraryScanner {
    pub fn scan(
        db: &Database,
        library_path: &str,
        rules: &[String],
        on_progress: Option<ProgressCb>,
    ) -> AppResult<crate::models::ScanResult> {
        use crate::library::auto_tag::apply_filename_rules;
        use walkdir::WalkDir;

        let path = Path::new(library_path);
        if !path.exists() {
            std::fs::create_dir_all(path)?;
            return Ok(crate::models::ScanResult {
                added: 0,
                updated: 0,
            });
        }

        // Batch-load all known paths for O(1) lookups instead of per-file SQL queries.
        let mut known_paths = db.all_scene_paths()?;

        let mut added = 0u32;
        let mut updated = 0u32;
        let mut scanned = 0u32;
        let extensions = ["mp4", "mkv", "webm", "mov", "avi", "m4v"];

        let emit = |scanned: u32, added: u32, updated: u32| {
            if let Some(ref cb) = on_progress {
                cb(ScanProgress {
                    scanned,
                    added,
                    updated,
                });
            }
        };

        for entry in WalkDir::new(path)
            .max_depth(5)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !extensions.contains(&ext.as_str()) {
                continue;
            }

            scanned += 1;
            let path_str = file_path.to_string_lossy().to_string();
            let title = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();

            if known_paths.contains(&path_str) {
                updated += 1;
            } else {
                let (performers, tags) = apply_filename_rules(&title, rules);
                match db.insert_scene(
                    &title,
                    Some(&path_str),
                    None,
                    &performers,
                    &tags,
                    None,
                    None,
                    None,
                    None,
                ) {
                    Ok(_) => {
                        added += 1;
                        // Add to set so duplicate files in the same scan are caught.
                        known_paths.insert(path_str);
                    }
                    Err(e) => eprintln!("scan skip {}: {e}", path_str),
                }
            }

            if scanned.is_multiple_of(10) {
                emit(scanned, added, updated);
            }
        }

        emit(scanned, added, updated);
        Ok(crate::models::ScanResult { added, updated })
    }

    /// Generate JPEG sidecars for library scenes missing thumbs.
    /// Also probes and writes duration for scenes that don't have one.
    /// Caps concurrency to avoid USB thrash.
    pub async fn generate_missing_thumbs(
        db: Arc<Database>,
        app: AppHandle,
        concurrency: usize,
    ) -> AppResult<crate::models::ThumbGenResult> {
        let missing = db.list_scenes_missing_thumbs()?;
        if missing.is_empty() {
            return Ok(crate::models::ThumbGenResult {
                generated: 0,
                errors: 0,
            });
        }

        let limit = concurrency.max(1);
        let sem = Arc::new(Semaphore::new(limit));
        let mut handles = Vec::new();
        let generated = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let errors = Arc::new(std::sync::atomic::AtomicU32::new(0));

        for (scene_id, path_str) in missing {
            let permit = sem
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| crate::error::AppError::Other(format!("thumb semaphore: {e}")))?;
            let db = db.clone();
            let app = app.clone();
            let generated = generated.clone();
            let errors = errors.clone();
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let video_path = PathBuf::from(&path_str);
                if !video_path.is_file() {
                    errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return;
                }

                let ffmpeg = FfmpegProcessor::new(app);

                // Always re-extract thumbnail (don't rely on potentially stale sidecar).
                match ffmpeg.extract_thumbnail(&video_path).await {
                    Ok(thumb) => {
                        let thumb_str = thumb.to_string_lossy().to_string();
                        if db.set_scene_thumb(&scene_id, &thumb_str).is_ok() {
                            // Offload blocking hash computations to the blocking thread pool.
                            let phash = {
                                let t = thumb.clone();
                                tokio::task::spawn_blocking(move || compute_phash_from_image(&t).ok())
                                    .await
                                    .ok()
                                    .flatten()
                            };
                            let oshash = {
                                let p = video_path.clone();
                                tokio::task::spawn_blocking(move || compute_oshash(&p).ok())
                                    .await
                                    .ok()
                                    .flatten()
                            };
                            let _ = db.update_scene_hashes(
                                &scene_id,
                                phash.as_deref(),
                                oshash.as_deref(),
                                Some(&thumb_str),
                            );
                            generated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        eprintln!("[thumb] failed for {path_str}: {e}");
                        errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }

                // Probe duration if not already set.
                if let Some(dur) = ffmpeg.probe_duration(&video_path).await {
                    if dur > 0.0 {
                        let _ = db.update_scene_duration(&scene_id, dur as u32);
                    }
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
        Ok(crate::models::ThumbGenResult {
            generated: generated.load(std::sync::atomic::Ordering::Relaxed),
            errors: errors.load(std::sync::atomic::Ordering::Relaxed),
        })
    }

    /// Probe durations for all scenes that don't have one set.
    /// Returns the count of durations successfully probed.
    pub async fn probe_library_durations(
        db: Arc<Database>,
        app: AppHandle,
        concurrency: usize,
    ) -> AppResult<crate::models::DurationProbeResult> {
        let missing = db.list_scenes_missing_durations()?;
        if missing.is_empty() {
            return Ok(crate::models::DurationProbeResult {
                probed: 0,
                errors: 0,
            });
        }

        let limit = concurrency.max(1);
        let sem = Arc::new(Semaphore::new(limit));
        let mut handles = Vec::new();
        let probed = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let errors = Arc::new(std::sync::atomic::AtomicU32::new(0));

        for (scene_id, path_str) in missing {
            let permit = sem
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| crate::error::AppError::Other(format!("probe semaphore: {e}")))?;
            let db = db.clone();
            let app = app.clone();
            let probed = probed.clone();
            let errors = errors.clone();
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let video_path = PathBuf::from(&path_str);
                if !video_path.is_file() {
                    errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return;
                }

                let ffmpeg = FfmpegProcessor::new(app);
                match ffmpeg.probe_duration(&video_path).await {
                    Some(dur) if dur > 0.0 => {
                        if db.update_scene_duration(&scene_id, dur as u32).is_ok() {
                            probed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    _ => {
                        errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
        Ok(crate::models::DurationProbeResult {
            probed: probed.load(std::sync::atomic::Ordering::Relaxed),
            errors: errors.load(std::sync::atomic::Ordering::Relaxed),
        })
    }

    pub async fn post_process_file(
        db: &Database,
        app: AppHandle,
        scene_id: &str,
        video_path: &Path,
    ) -> AppResult<(Option<String>, Option<String>, Option<String>)> {
        let ffmpeg = FfmpegProcessor::new(app.clone());
        let thumb = ffmpeg.extract_thumbnail(video_path).await.ok();
        let remuxed = if video_path.extension().and_then(|e| e.to_str()) == Some("mp4") {
            ffmpeg.remux_faststart(video_path).await.ok()
        } else {
            None
        };

        let final_path: &Path = remuxed.as_deref().unwrap_or(video_path);
        let oshash = compute_oshash(final_path).ok();
        let phash = thumb
            .as_ref()
            .and_then(|t| compute_phash_from_image(t).ok());

        let thumb_str = thumb.as_ref().map(|p| p.to_string_lossy().to_string());
        let path_str = final_path.to_string_lossy().to_string();

        db.update_scene_path(scene_id, &path_str, thumb_str.as_deref())?;
        db.update_scene_hashes(
            scene_id,
            phash.as_deref(),
            oshash.as_deref(),
            thumb_str.as_deref(),
        )?;

        Ok((phash, oshash, thumb_str))
    }
}
