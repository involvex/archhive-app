use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::library::import::import_download;
use crate::library::LibraryScanner;
use crate::models::{DownloadJob, DownloadPlan, DownloadStatus, DownloadTool};
use crate::sites::yt_dlp::SidecarRunner;
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

        tauri::async_runtime::spawn(worker_loop(
            queue_rx,
            worker_db,
            worker_app,
            worker_vault,
            worker_flags,
        ));

        let manager = Self {
            db: db.clone(),
            app: app.clone(),
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
        };
        self.queue_plan(plan)
    }

    pub fn queue_plan(&self, plan: DownloadPlan) -> AppResult<DownloadJob> {
        let job =
            self.db
                .insert_download_job(&plan.url, &plan.adapter_id, plan.title.as_deref())?;
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
    let tool = crate::downloads::image::resolve_download_tool(&job.url, &job.adapter);
    Ok(DownloadPlan {
        url: job.url.clone(),
        output_template: crate::downloads::naming::to_ytdlp_output_template(
            &settings.naming_template,
        ),
        tool,
        title: job.title.clone(),
        performers: vec![],
        tags: vec![],
        adapter_id: job.adapter.clone(),
    })
}

fn mark_job_failed(db: &Database, app: &AppHandle, job_id: &str, error: &str) {
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
}

async fn worker_loop(
    mut queue_rx: mpsc::UnboundedReceiver<String>,
    db: Arc<Database>,
    app: AppHandle,
    vault: Arc<CookieVault>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
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
                    mark_job_failed(&db, &app, &job_id, &e.to_string());
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
                mark_job_failed(&db, &app, &job_id, &e.to_string());
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
            let path = crate::downloads::image::download_direct(
                &plan.url,
                library_path,
                plan.title.as_deref(),
            )
            .await?;
            Ok(vec![path])
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

            let title = plan.title.clone().unwrap_or_else(|| job.url.clone());
            for output_path in &existing {
                let file_title = Path::new(output_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&title);
                let scene_id = import_download(
                    &db,
                    file_title,
                    Some(output_path),
                    Some(&job.url),
                    &plan.performers,
                    &plan.tags,
                    None,
                    None,
                    None,
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
