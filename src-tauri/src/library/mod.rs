use crate::db::Database;
use crate::error::AppResult;
use crate::library::hashing::{compute_oshash, compute_phash_from_image};
use crate::media::FfmpegProcessor;
use crate::models::ScanProgress;
use std::path::Path;
use tauri::AppHandle;

pub mod auto_tag;
pub mod duplicates;
pub mod hashing;
pub mod import;

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

        for entry in WalkDir::new(path).max_depth(5).into_iter().filter_map(|e| e.ok()) {
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

            if db.scene_exists_by_path(&path_str)? {
                updated += 1;
                if scanned.is_multiple_of(10) {
                    emit(scanned, added, updated);
                }
                continue;
            }

            let (performers, tags) = apply_filename_rules(&title, rules);
            db.insert_scene(
                &title,
                Some(&path_str),
                None,
                &performers,
                &tags,
                None,
                None,
                None,
            )?;
            added += 1;
            if scanned.is_multiple_of(10) {
                emit(scanned, added, updated);
            }
        }

        emit(scanned, added, updated);
        Ok(crate::models::ScanResult { added, updated })
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
