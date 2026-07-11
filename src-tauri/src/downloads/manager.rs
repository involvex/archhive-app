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
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct DownloadManager {
    db: Arc<Database>,
    app: AppHandle,
    vault: Arc<CookieVault>,
    active: Arc<Mutex<HashMap<String, bool>>>,
}

impl DownloadManager {
    pub fn new(db: Arc<Database>, app: AppHandle, vault: Arc<CookieVault>) -> Self {
        Self {
            db,
            app,
            vault,
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn queue(&self, url: &str, adapter: &str, title: Option<&str>) -> AppResult<DownloadJob> {
        let settings = self.db.get_settings()?;
        let tool = crate::downloads::image::resolve_download_tool(url, adapter);
        let plan = DownloadPlan {
            url: url.to_string(),
            output_template: settings.naming_template,
            tool,
            title: title.map(|s| s.to_string()),
            performers: vec![],
            tags: vec![],
            adapter_id: adapter.to_string(),
        };
        self.queue_plan(plan)
    }

    pub fn queue_plan(&self, plan: DownloadPlan) -> AppResult<DownloadJob> {
        let library_path = self.db.get_settings()?.library_path;
        let job = self
            .db
            .insert_download_job(&plan.url, &plan.adapter_id, plan.title.as_deref())?;
        let job_id = job.id.clone();
        let db = self.db.clone();
        let app = self.app.clone();
        let vault = self.vault.clone();
        let active = self.active.clone();
        let err_db = self.db.clone();
        let err_app = self.app.clone();

        tauri::async_runtime::spawn(async move {
            if let Err(e) = run_job_with_plan(
                db,
                app,
                vault,
                active,
                job_id.clone(),
                &plan,
                &library_path,
            )
            .await
            {
                if let Ok(Some(mut job)) = err_db.get_download_job(&job_id) {
                    job.status = DownloadStatus::Failed;
                    job.error = Some(e.to_string());
                    let _ = err_db.update_download_job(&job);
                    let _ = err_app.emit("download:progress", &job);
                }
            }
        });

        Ok(job)
    }

    pub fn cancel(&self, id: &str) -> AppResult<()> {
        if let Some(mut job) = self.db.get_download_job(id)? {
            job.status = DownloadStatus::Cancelled;
            self.db.update_download_job(&job)?;
            self.active.lock().insert(id.to_string(), false);
            let _ = self.app.emit("download:progress", &job);
        }
        Ok(())
    }
}

async fn run_job_with_plan(
    db: Arc<Database>,
    app: AppHandle,
    vault: Arc<CookieVault>,
    active: Arc<Mutex<HashMap<String, bool>>>,
    job_id: String,
    plan: &DownloadPlan,
    library_path: &str,
) -> AppResult<DownloadJob> {
    let mut job = db
        .get_download_job(&job_id)?
        .ok_or_else(|| AppError::NotFound(job_id.clone()))?;

    active.lock().insert(job_id.clone(), true);
    job.status = DownloadStatus::Active;
    db.update_download_job(&job)?;
    let _ = app.emit("download:progress", &job);

    std::fs::create_dir_all(library_path)?;

    let runner = SidecarRunner::new(app.clone());
    let db_emit = db.clone();
    let app_emit = app.clone();
    let job_id_emit = job_id.clone();
    let cookies = vault.cookie_file_for_site(&plan.adapter_id);

    let snapshot = if matches!(plan.tool, DownloadTool::GalleryDl) {
        Some(crate::downloads::gallery_dl::DirSnapshot::capture(library_path)?)
    } else {
        None
    };

    let result: AppResult<Vec<String>> = match plan.tool {
        DownloadTool::GalleryDl => {
            let parsed = runner
                .run_gallery_dl(&plan.url, library_path, |line| {
                    update_progress(&db_emit, &app_emit, &job_id_emit, line, None);
                })
                .await?;
            let paths = crate::downloads::gallery_dl::resolve_output_paths(
                &parsed,
                snapshot.as_ref().expect("gallery-dl snapshot"),
                library_path,
            )?;
            Ok(paths)
        }
        DownloadTool::DirectHttp => {
            let path = crate::downloads::image::download_direct(
                &plan.url,
                library_path,
                plan.title.as_deref(),
            )
            .await?;
            Ok(vec![path])
        }
        DownloadTool::YtDlp => {
            let path = runner
                .run_yt_dlp(
                    &plan.url,
                    library_path,
                    &plan.output_template,
                    cookies.as_deref(),
                    |line| {
                        let progress = SidecarRunner::parse_progress(line);
                        update_progress(&db_emit, &app_emit, &job_id_emit, line, progress);
                    },
                )
                .await?;
            Ok(vec![path])
        }
    };

    match result {
        Ok(output_paths) => {
            if output_paths.is_empty() {
                return Err(AppError::Download("Download produced no output files".into()));
            }
            job.status = DownloadStatus::Completed;
            job.progress = 100.0;
            job.output_path = Some(output_paths.last().cloned().unwrap_or_default());
            db.update_download_job(&job)?;
            let _ = app.emit("download:progress", &job);

            let title = plan.title.clone().unwrap_or_else(|| job.url.clone());
            for output_path in &output_paths {
                if !Path::new(output_path).exists() {
                    continue;
                }
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
            job.status = DownloadStatus::Failed;
            job.error = Some(e.to_string());
            db.update_download_job(&job)?;
            let _ = app.emit("download:progress", &job);
            return Err(e);
        }
    }

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
        if let Some(p) = progress {
            job.progress = p;
        }
        job.status = DownloadStatus::Active;
        let _ = db.update_download_job(&job);
        let _ = app.emit("download:progress", &job);
    }
}
