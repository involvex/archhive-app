use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::library::auto_tag::apply_filename_rules;
use crate::library::import::import_download;
use crate::library::thumbnail::download_remote_thumbnail;
use crate::library::LibraryScanner;
use crate::media::FfmpegProcessor;
use crate::models::{DownloadJob, DownloadPlan, DownloadStatus, DownloadTool};
use crate::sites::yt_dlp::{enrich_metadata_from_ytdlp_json, SidecarRunner};
use crate::vault::CookieVault;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Semaphore};

const MAX_CONCURRENT_DOWNLOADS: usize = 2;

pub struct DownloadManager {
    db: Arc<Database>,
    app: AppHandle,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    queue_tx: mpsc::UnboundedSender<String>,
}

impl DownloadManager {
    pub fn new(db: Arc<Database>, app: AppHandle, vault: Arc<CookieVault>) -> Self {
        let cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (queue_tx, queue_rx) = mpsc::unbounded_channel();

        let worker_db = db.clone();
        let worker_app = app.clone();
        let worker_vault = vault.clone();
        let worker_flags = cancel_flags.clone();
        let worker_queue_tx = queue_tx.clone();

        tauri::async_runtime::spawn(worker_loop(
            queue_rx,
            worker_db,
            worker_app,
            worker_vault,
            worker_flags,
            worker_queue_tx,
        ));

        let manager = Self {
            db: db.clone(),
            app,
            cancel_flags,
            queue_tx,
        };

        if let Ok(jobs) = db.list_download_jobs() {
            for job in jobs {
                if matches!(job.status, DownloadStatus::Pending) {
                    let _ = manager.queue_tx.send(job.id);
                }
            }
        }

        manager
    }

    fn register_cancel(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(true));
        self.cancel_flags
            .lock()
            .insert(job_id.to_string(), flag.clone());
        flag
    }

    fn clear_cancel(&self, job_id: &str) {
        self.cancel_flags.lock().remove(job_id);
    }

    fn stop_flag(&self, job_id: &str) {
        if let Some(flag) = self.cancel_flags.lock().get(job_id) {
            flag.store(false, Ordering::Relaxed);
        }
    }

    fn enqueue(&self, job_id: &str) -> AppResult<()> {
        self.queue_tx
            .send(job_id.to_string())
            .map_err(|e| AppError::Other(format!("download queue: {e}")))?;
        Ok(())
    }

    pub fn queue(&self, url: &str, adapter: &str, title: Option<&str>) -> AppResult<DownloadJob> {
        let settings = self.db.get_settings()?;
        let tool = crate::downloads::image::resolve_download_tool(url, adapter);
        let plan = DownloadPlan {
            url: url.to_string(),
            output_template: crate::downloads::naming::to_ytdlp_output_template(
                &settings.naming_template,
            ),
            tool,
            title: title.map(|s| s.to_string()),
            performers: vec![],
            tags: vec![],
            adapter_id: adapter.to_string(),
            thumbnail_url: None,
            duration: None,
            channel: None,
            referer: None,
        };
        self.queue_plan(plan)
    }

    pub fn queue_plan(&self, plan: DownloadPlan) -> AppResult<DownloadJob> {
        let job = self.db.insert_download_job(
            &plan.url,
            &plan.adapter_id,
            plan.title.as_deref(),
            None,
        )?;
        self.db.store_download_job_metadata(
            &job.id,
            &plan.performers,
            &plan.tags,
            plan.thumbnail_url.as_deref(),
            plan.duration,
            plan.channel.as_deref(),
            plan.referer.as_deref(),
            &plan.tool,
        )?;
        self.register_cancel(&job.id);
        self.enqueue(&job.id)?;
        Ok(job)
    }

    pub fn pause(&self, id: &str) -> AppResult<()> {
        let Some(mut job) = self.db.get_download_job(id)? else {
            return Ok(());
        };
        if !matches!(job.status, DownloadStatus::Active | DownloadStatus::Pending) {
            return Ok(());
        }
        self.stop_flag(id);
        job.status = DownloadStatus::Paused;
        self.db.update_download_job(&job)?;
        let _ = self.app.emit("download:progress", &job);
        Ok(())
    }

    pub fn resume(&self, id: &str) -> AppResult<()> {
        let Some(mut job) = self.db.get_download_job(id)? else {
            return Ok(());
        };
        if job.status != DownloadStatus::Paused {
            return Ok(());
        }
        self.register_cancel(id);
        job.status = DownloadStatus::Pending;
        job.error = None;
        self.db.update_download_job(&job)?;
        let _ = self.app.emit("download:progress", &job);
        self.enqueue(id)?;
        Ok(())
    }

    pub fn retry(&self, id: &str) -> AppResult<()> {
        let Some(mut job) = self.db.get_download_job(id)? else {
            return Ok(());
        };
        if !matches!(
            job.status,
            DownloadStatus::Failed | DownloadStatus::Cancelled | DownloadStatus::Completed
        ) {
            return Ok(());
        }
        self.register_cancel(id);
        job.status = DownloadStatus::Pending;
        job.progress = 0.0;
        job.error = None;
        job.output_path = None;
        job.retry_count = 0;
        job.last_retry_at = None;
        self.db.update_download_job(&job)?;
        let _ = self.app.emit("download:progress", &job);
        self.enqueue(id)?;
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> AppResult<()> {
        let Some(mut job) = self.db.get_download_job(id)? else {
            return Ok(());
        };
        if matches!(
            job.status,
            DownloadStatus::Completed | DownloadStatus::Cancelled
        ) {
            return Ok(());
        }
        self.stop_flag(id);
        job.status = DownloadStatus::Cancelled;
        self.db.update_download_job(&job)?;
        let _ = self.app.emit("download:progress", &job);
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        if let Some(job) = self.db.get_download_job(id)? {
            if matches!(job.status, DownloadStatus::Active | DownloadStatus::Pending) {
                self.stop_flag(id);
            }
        }
        self.clear_cancel(id);
        self.db.delete_download_job(id)?;
        let _ = self.app.emit("download:deleted", id);
        Ok(())
    }
}

fn plan_from_job(db: &Database, job: &DownloadJob) -> AppResult<DownloadPlan> {
    let settings = db.get_settings()?;
    let (performers, tags, thumbnail_url, duration, channel, referer, persisted_tool) =
        deserialize_job_metadata(db, &job.id).unwrap_or_default();
    // Prefer the tool stored at queue time; fall back to URL-based detection
    // for jobs queued before tool persistence existed.
    let tool = persisted_tool
        .unwrap_or_else(|| crate::downloads::image::resolve_download_tool(&job.url, &job.adapter));
    Ok(DownloadPlan {
        url: job.url.clone(),
        output_template: crate::downloads::naming::to_ytdlp_output_template(
            &settings.naming_template,
        ),
        tool,
        title: job.title.clone(),
        performers,
        tags,
        adapter_id: job.adapter.clone(),
        thumbnail_url,
        duration,
        channel,
        referer,
    })
}

fn deserialize_job_metadata(
    db: &Database,
    job_id: &str,
) -> AppResult<(
    Vec<String>,
    Vec<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
    Option<String>,
    Option<crate::models::DownloadTool>,
)> {
    db.get_download_job_metadata(job_id)
}

pub(crate) fn mark_job_failed(
    db: Arc<Database>,
    app: &AppHandle,
    queue_tx: &mpsc::UnboundedSender<String>,
    job_id: &str,
    error: &str,
) {
    let Ok(Some(mut job)) = db.get_download_job(job_id) else {
        return;
    };
    if matches!(
        job.status,
        DownloadStatus::Paused
            | DownloadStatus::Cancelled
            | DownloadStatus::Completed
            | DownloadStatus::Failed
    ) {
        return;
    }
    job.status = DownloadStatus::Failed;
    job.error = Some(error.to_string());
    let _ = db.update_download_job(&job);
    let _ = app.emit("download:progress", &job);

    let settings = db.get_settings().ok();
    let max_retries = settings
        .as_ref()
        .map(|s| s.download_max_retries)
        .unwrap_or(2);
    let delay_secs = settings
        .as_ref()
        .map(|s| s.download_retry_delay_seconds)
        .unwrap_or(30);

    if job.retry_count >= max_retries {
        return;
    }

    let db = db.clone();
    let app = app.clone();
    let queue_tx = queue_tx.clone();
    let job_id = job_id.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs as u64)).await;
        let Ok(Some(mut job)) = db.get_download_job(&job_id) else {
            return;
        };
        if job.status != DownloadStatus::Failed {
            return;
        }
        job.retry_count += 1;
        job.last_retry_at = Some(chrono::Utc::now().to_rfc3339());
        job.status = DownloadStatus::Pending;
        job.progress = 0.0;
        job.error = None;
        job.output_path = None;
        let _ = db.update_download_job(&job);
        let _ = app.emit("download:progress", &job);
        let _ = queue_tx.send(job_id);
    });
}

async fn worker_loop(
    mut queue_rx: mpsc::UnboundedReceiver<String>,
    db: Arc<Database>,
    app: AppHandle,
    vault: Arc<CookieVault>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    queue_tx: mpsc::UnboundedSender<String>,
) {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    while let Some(job_id) = queue_rx.recv().await {
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };
        let db = db.clone();
        let app = app.clone();
        let vault = vault.clone();
        let cancel_flags = cancel_flags.clone();
        let queue_tx = queue_tx.clone();
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            let cancel = cancel_flags
                .lock()
                .get(&job_id)
                .cloned()
                .unwrap_or_else(|| Arc::new(AtomicBool::new(true)));

            let Ok(Some(job)) = db.get_download_job(&job_id) else {
                return;
            };
            if matches!(
                job.status,
                DownloadStatus::Paused | DownloadStatus::Cancelled | DownloadStatus::Completed
            ) {
                return;
            }
            let plan = match plan_from_job(&db, &job) {
                Ok(p) => p,
                Err(e) => {
                    mark_job_failed(db.clone(), &app, &queue_tx, &job_id, &e.to_string());
                    return;
                }
            };
            let library_path = db
                .get_settings()
                .map(|s| s.library_path)
                .unwrap_or_default();
            if let Err(e) = run_job_with_plan(
                db.clone(),
                app.clone(),
                vault,
                cancel,
                job_id.clone(),
                &plan,
                &library_path,
            )
            .await
            {
                // run_job_with_plan marks Failed for tool errors; catch early ? failures too
                mark_job_failed(db.clone(), &app, &queue_tx, &job_id, &e.to_string());
            }
        });
    }
}

async fn run_job_with_plan(
    db: Arc<Database>,
    app: AppHandle,
    vault: Arc<CookieVault>,
    cancel: Arc<AtomicBool>,
    job_id: String,
    plan: &DownloadPlan,
    library_path: &str,
) -> AppResult<DownloadJob> {
    let mut job = db
        .get_download_job(&job_id)?
        .ok_or_else(|| AppError::NotFound(job_id.clone()))?;

    if job.status == DownloadStatus::Paused || job.status == DownloadStatus::Cancelled {
        return Ok(job);
    }

    if !cancel.load(Ordering::Relaxed) {
        return Ok(job);
    }

    job.status = DownloadStatus::Active;
    job.error = None;
    db.update_download_job(&job)?;
    let _ = app.emit("download:progress", &job);

    std::fs::create_dir_all(library_path)?;

    let runner = SidecarRunner::new(app.clone());
    let db_emit = db.clone();
    let app_emit = app.clone();
    let job_id_emit = job_id.clone();
    let cookies = vault.cookie_file_for_site(&plan.adapter_id);

    let result: AppResult<Vec<String>> = match plan.tool {
        DownloadTool::GalleryDl => {
            if !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            // Per-job subdir so DirSnapshot stays cheap (no full-library walk).
            let job_dir = Path::new(library_path).join("_dl").join(&job_id);
            std::fs::create_dir_all(&job_dir)?;
            let job_dir_str = job_dir.to_string_lossy().to_string();
            let snapshot = crate::downloads::gallery_dl::DirSnapshot::capture(&job_dir_str)?;
            let parsed = runner
                .run_gallery_dl(&plan.url, &job_dir_str, |line| {
                    update_progress(&db_emit, &app_emit, &job_id_emit, line, None);
                })
                .await;
            if parsed.is_err() && !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            let parsed = parsed?;
            let paths = crate::downloads::gallery_dl::resolve_output_paths(
                &parsed,
                &snapshot,
                &job_dir_str,
            )?;
            Ok(paths)
        }
        DownloadTool::DirectHttp => {
            if !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            let cookie_header = vault.cookie_header(&plan.adapter_id).ok().flatten();
            let mut last_reported = 0u64;
            let path = crate::downloads::image::download_direct(
                &plan.url,
                library_path,
                plan.title.as_deref(),
                plan.referer.as_deref(),
                cookie_header.as_deref(),
                Some(|downloaded: u64, total: Option<u64>| {
                    // Throttle progress events to ~1MB steps plus completion.
                    let done = total.is_some_and(|t| downloaded >= t);
                    if downloaded >= last_reported + 1024 * 1024 || done {
                        last_reported = downloaded;
                        let progress = total.and_then(|t| {
                            if t > 0 {
                                Some(downloaded as f32 / t as f32)
                            } else {
                                None
                            }
                        });
                        update_progress(&db_emit, &app_emit, &job_id_emit, "", progress);
                    }
                }),
            )
            .await;
            if path.is_err() && !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            Ok(vec![path?])
        }
        DownloadTool::FfmpegHls => {
            if !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            let cookie_header = vault.cookie_header(&plan.adapter_id).ok().flatten();
            std::fs::create_dir_all(library_path)?;
            let base = plan
                .title
                .clone()
                .map(|t| crate::downloads::image::sanitize_filename(&t))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "video".to_string());
            let filename = if base.contains('.') {
                base
            } else {
                format!("{base}.mp4")
            };
            let out_path =
                crate::downloads::image::unique_path(Path::new(library_path).join(&filename));
            let processor = crate::media::FfmpegProcessor::new(app.clone());
            let result = processor
                .download_hls(
                    &plan.url,
                    &out_path,
                    plan.referer.as_deref(),
                    cookie_header.as_deref(),
                    plan.duration,
                    Some(|fraction: Option<f32>| {
                        update_progress(&db_emit, &app_emit, &job_id_emit, "", fraction);
                    }),
                )
                .await;
            if result.is_err() {
                let _ = std::fs::remove_file(&out_path);
                if !cancel.load(Ordering::Relaxed) {
                    return handle_stopped(&db, &app, &job_id);
                }
            }
            result?;
            Ok(vec![out_path.to_string_lossy().to_string()])
        }
        DownloadTool::YtDlp => {
            let cancel_clone = cancel.clone();
            let settings = db.get_settings().unwrap_or_default();
            let format_args = SidecarRunner::format_selection_args(
                settings.download_quality,
                settings.prefer_mp4,
            );
            let path_result = runner
                .run_yt_dlp(
                    &plan.url,
                    library_path,
                    &plan.output_template,
                    cookies.as_deref(),
                    cancel_clone,
                    |line| {
                        let progress = SidecarRunner::parse_progress(line);
                        update_progress(&db_emit, &app_emit, &job_id_emit, line, progress);
                    },
                    &format_args,
                )
                .await;
            if path_result.is_err() && !cancel.load(Ordering::Relaxed) {
                return handle_stopped(&db, &app, &job_id);
            }
            Ok(vec![path_result?])
        }
    };

    if !cancel.load(Ordering::Relaxed) {
        return handle_stopped(&db, &app, &job_id);
    }

    match result {
        Ok(output_paths) => {
            let existing: Vec<String> = output_paths
                .into_iter()
                .filter(|p| !p.trim().is_empty() && Path::new(p).exists())
                .collect();
            if existing.is_empty() {
                let err = AppError::Download("Download produced no output files".into());
                job.status = DownloadStatus::Failed;
                job.error = Some(err.to_string());
                db.update_download_job(&job)?;
                let _ = app.emit("download:progress", &job);
                return Err(err);
            }
            job.status = DownloadStatus::Completed;
            job.progress = 100.0;
            job.error = None;
            job.output_path = Some(existing.last().cloned().unwrap_or_default());
            db.update_download_job(&job)?;
            let _ = app.emit("download:progress", &job);

            let mut enriched_performers = plan.performers.clone();
            let mut enriched_tags = plan.tags.clone();
            let mut enriched_channel = plan.channel.clone();
            let mut enriched_duration = plan.duration;
            let mut enriched_thumb_url = plan.thumbnail_url.clone();

            if plan.tool == DownloadTool::YtDlp
                && enriched_performers.is_empty()
                && enriched_tags.is_empty()
            {
                let cookies = vault.cookie_file_for_site(&plan.adapter_id);
                if let Ok(json) = runner
                    .resolve_media_json(&plan.url, cookies.as_deref())
                    .await
                {
                    let (yt_performers, yt_tags, yt_channel, yt_dur, yt_thumb) =
                        enrich_metadata_from_ytdlp_json(&json);
                    if enriched_performers.is_empty() && !yt_performers.is_empty() {
                        enriched_performers = yt_performers;
                    }
                    if enriched_tags.is_empty() && !yt_tags.is_empty() {
                        enriched_tags = yt_tags;
                    }
                    if enriched_channel.is_none() {
                        enriched_channel = yt_channel;
                    }
                    if enriched_duration.is_none() {
                        enriched_duration = yt_dur;
                    }
                    if enriched_thumb_url.is_none() {
                        enriched_thumb_url = yt_thumb;
                    }
                    db.store_download_job_metadata(
                        &job.id,
                        &enriched_performers,
                        &enriched_tags,
                        enriched_thumb_url.as_deref(),
                        enriched_duration,
                        enriched_channel.as_deref(),
                        plan.referer.as_deref(),
                        &plan.tool,
                    )?;
                }
            }

            let plan_title = plan.title.clone().unwrap_or_else(|| job.url.clone());
            for output_path in &existing {
                let file_title = Path::new(output_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&plan_title);
                let downloaded_thumb = if let Some(ref thumb_url) = enriched_thumb_url {
                    download_remote_thumbnail(thumb_url, Path::new(output_path)).await
                } else {
                    None
                };
                // Fallback: if remote thumbnail failed, try ffprobe extraction.
                let thumb_path = match downloaded_thumb {
                    Some(p) => Some(p.to_string_lossy().to_string()),
                    None => {
                        let ffmpeg = FfmpegProcessor::new(app.clone());
                        match ffmpeg.extract_thumbnail(Path::new(output_path)).await {
                            Ok(p) => Some(p.to_string_lossy().to_string()),
                            Err(_) => None,
                        }
                    }
                };
                let thumb_str = thumb_path.as_deref();

                let settings = db.get_settings().unwrap_or_default();
                let rules = &settings.auto_tag_rules;
                let stem = Path::new(output_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let (fn_performers, fn_tags) = apply_filename_rules(stem, rules);

                let mut merged_performers = enriched_performers.clone();
                for p in fn_performers {
                    if !merged_performers.iter().any(|x| x.eq_ignore_ascii_case(&p)) {
                        merged_performers.push(p);
                    }
                }
                let mut merged_tags = enriched_tags.clone();
                for t in fn_tags {
                    if !merged_tags.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
                        merged_tags.push(t);
                    }
                }

                let scene_id = import_download(
                    &db,
                    file_title,
                    Some(output_path),
                    Some(&job.url),
                    &merged_performers,
                    &merged_tags,
                    thumb_str,
                    None,
                    None,
                    enriched_duration,
                    enriched_channel.as_deref(),
                )?;
                let _ = LibraryScanner::post_process_file(
                    &db,
                    app.clone(),
                    &scene_id,
                    Path::new(output_path),
                )
                .await;
            }
        }
        Err(e) => {
            if !cancel.load(Ordering::Relaxed) || e.to_string().contains("stopped") {
                return handle_stopped(&db, &app, &job_id);
            }
            job.status = DownloadStatus::Failed;
            job.error = Some(e.to_string());
            db.update_download_job(&job)?;
            let _ = app.emit("download:progress", &job);
            return Err(e);
        }
    }

    Ok(job)
}

fn handle_stopped(db: &Database, app: &AppHandle, job_id: &str) -> AppResult<DownloadJob> {
    let Some(mut job) = db.get_download_job(job_id)? else {
        return Err(AppError::NotFound(job_id.to_string()));
    };
    if matches!(
        job.status,
        DownloadStatus::Paused | DownloadStatus::Cancelled | DownloadStatus::Completed
    ) {
        return Ok(job);
    }
    job.status = DownloadStatus::Paused;
    db.update_download_job(&job)?;
    let _ = app.emit("download:progress", &job);
    Ok(job)
}

fn update_progress(
    db: &Database,
    app: &AppHandle,
    job_id: &str,
    _line: &str,
    progress: Option<f32>,
) {
    if let Ok(Some(mut job)) = db.get_download_job(job_id) {
        if matches!(
            job.status,
            DownloadStatus::Paused | DownloadStatus::Cancelled
        ) {
            return;
        }
        if let Some(p) = progress {
            job.progress = p;
        }
        job.status = DownloadStatus::Active;
        let _ = db.update_download_job(&job);
        let _ = app.emit("download:progress", &job);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppSettings;
    use tempfile::tempdir;

    #[test]
    fn retry_fields_persist_in_db() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().to_path_buf()).unwrap();
        let job = db
            .insert_download_job("https://example.com/video.mp4", "generic_ytdlp", None, None)
            .unwrap();

        let loaded = db.get_download_job(&job.id).unwrap().unwrap();
        assert_eq!(loaded.retry_count, 0);
        assert_eq!(loaded.last_retry_at, None);

        let mut updated = loaded;
        updated.retry_count = 3;
        updated.last_retry_at = Some("2024-01-01T00:00:00Z".to_string());
        db.update_download_job(&updated).unwrap();

        let reloaded = db.get_download_job(&job.id).unwrap().unwrap();
        assert_eq!(reloaded.retry_count, 3);
        assert_eq!(
            reloaded.last_retry_at,
            Some("2024-01-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn default_retry_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.download_max_retries, 2);
        assert_eq!(settings.download_retry_delay_seconds, 30);
    }

    #[test]
    fn manual_retry_resets_count() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().to_path_buf()).unwrap();
        let mut job = db
            .insert_download_job("https://example.com/video.mp4", "generic_ytdlp", None, None)
            .unwrap();
        job.status = DownloadStatus::Failed;
        job.retry_count = 2;
        job.error = Some("timeout".to_string());
        db.update_download_job(&job).unwrap();

        let loaded = db.get_download_job(&job.id).unwrap().unwrap();
        assert_eq!(loaded.retry_count, 2);

        let mut reset = loaded;
        reset.retry_count = 0;
        reset.last_retry_at = None;
        reset.status = DownloadStatus::Pending;
        reset.error = None;
        db.update_download_job(&reset).unwrap();

        let final_job = db.get_download_job(&job.id).unwrap().unwrap();
        assert_eq!(final_job.retry_count, 0);
        assert_eq!(final_job.status, DownloadStatus::Pending);
        assert_eq!(final_job.error, None);
    }
}
