use crate::db::Database;
use crate::downloads::DownloadManager;
use crate::error::AppResult;
use crate::models::{
    AppSettings, BrowseKind, BrowseQuery, DownloadJob, DuplicateGroup, HealthResponse, MediaItem,
    Performer, Scene, ScanResult, SiteInfo, Tag,
};
use crate::server::LanServer;
use crate::sites::registry::SiteRegistry;
use crate::sites::SiteContext;
use crate::vault::{CookieSiteInfo, CookieVault};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct AppState {
    pub db: Arc<Database>,
    pub data_dir: PathBuf,
    pub sites: Arc<SiteRegistry>,
    pub site_ctx: Arc<SiteContext>,
    pub downloads: Arc<DownloadManager>,
    pub vault: Arc<CookieVault>,
    pub lan_server: Arc<Mutex<Option<LanServer>>>,
    pub static_ui_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl AppState {
    pub fn with_app(
        db: Arc<Database>,
        data_dir: PathBuf,
        app: tauri::AppHandle,
        static_ui_dir: Option<PathBuf>,
    ) -> AppResult<Self> {
        let vault = Arc::new(CookieVault::new(data_dir.clone(), db.connection())?);
        let sites = Arc::new(SiteRegistry::new());
        let site_ctx = Arc::new(SiteContext::new(vault.clone())?);
        let downloads = Arc::new(DownloadManager::new(db.clone(), app, vault.clone()));
        Ok(Self {
            db,
            data_dir,
            sites,
            site_ctx,
            downloads,
            vault,
            lan_server: Arc::new(Mutex::new(None)),
            static_ui_dir: Arc::new(Mutex::new(static_ui_dir)),
        })
    }

    pub fn health() -> HealthResponse {
        HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub async fn list_sites(&self) -> Vec<SiteInfo> {
        self.sites.list()
    }

    pub async fn browse(
        &self,
        site_id: &str,
        kind: BrowseKind,
        slug: &str,
        page: u32,
    ) -> AppResult<crate::models::BrowsePage> {
        let adapter = self
            .sites
            .get(site_id)
            .ok_or_else(|| crate::error::AppError::NotFound(format!("site {site_id}")))?;
        adapter
            .browse(
                &self.site_ctx,
                BrowseQuery {
                    kind,
                    slug: slug.to_string(),
                    page,
                },
            )
            .await
    }

    pub async fn queue_download(&self, url: &str, adapter: Option<&str>) -> AppResult<DownloadJob> {
        let adapter_id = adapter
            .map(|s| s.to_string())
            .or_else(|| self.sites.detect(url))
            .unwrap_or_else(|| "youtube".to_string());

        if let Some(site_adapter) = self.sites.get(&adapter_id) {
            let item = MediaItem {
                id: uuid::Uuid::new_v4().to_string(),
                title: url.to_string(),
                url: url.to_string(),
                thumbnail: None,
                duration: None,
                site_id: adapter_id.clone(),
                performers: vec![],
                tags: vec![],
            };
            let plan = site_adapter
                .resolve_download(&self.site_ctx, &item)
                .await?;
            return self.downloads.queue_plan(plan);
        }

        self.downloads.queue(url, &adapter_id, None)
    }

    pub fn list_downloads(&self) -> AppResult<Vec<DownloadJob>> {
        self.db.list_download_jobs()
    }

    pub fn cancel_download(&self, id: &str) -> AppResult<()> {
        self.downloads.cancel(id)
    }

    pub fn list_scenes(&self, query: Option<&str>) -> AppResult<Vec<Scene>> {
        self.db.list_scenes(query)
    }

    pub fn list_performers(&self, query: Option<&str>) -> AppResult<Vec<Performer>> {
        self.db.list_performers(query)
    }

    pub fn list_tags(&self) -> AppResult<Vec<Tag>> {
        self.db.list_tags()
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        self.db.get_settings()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        self.db.save_settings(settings)
    }

    pub fn scan_library(&self) -> AppResult<ScanResult> {
        let settings = self.db.get_settings()?;
        let path = Self::validate_library_path(&settings.library_path, &self.data_dir)?;
        let rules = vec![r"(?<performer>[a-zA-Z0-9_]+)-\d+".to_string()];
        crate::library::LibraryScanner::scan(&self.db, &path, &rules, None)
    }

    pub fn validate_library_path(library_path: &str, data_dir: &Path) -> AppResult<String> {
        let trimmed = library_path.trim();
        if trimmed.is_empty() {
            return Err(crate::error::AppError::InvalidInput(
                "Library path is not configured. Set it in Settings → Library.".into(),
            ));
        }
        let path = std::path::Path::new(trimmed);
        if !path.is_absolute() && trimmed == "." {
            return Err(crate::error::AppError::InvalidInput(
                "Invalid library path.".into(),
            ));
        }
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            std::fs::create_dir_all(path)?;
            path.canonicalize()?
        };
        let data_canonical = data_dir.canonicalize().unwrap_or_else(|_| data_dir.to_path_buf());
        let videos = dirs::video_dir().and_then(|p| p.canonicalize().ok());
        let home = dirs::home_dir().and_then(|p| p.canonicalize().ok());
        let allowed_roots = [Some(data_canonical), videos, home]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let allowed = allowed_roots.iter().any(|root| canonical.starts_with(root))
            || canonical.components().count() >= 2;
        if !allowed {
            return Err(crate::error::AppError::InvalidInput(format!(
                "Library path must be under your home, Videos, or app data directory: {}",
                canonical.display()
            )));
        }
        Ok(canonical.to_string_lossy().to_string())
    }

    pub fn find_duplicates(&self) -> AppResult<Vec<DuplicateGroup>> {
        let settings = self.db.get_settings()?;
        self.db.find_duplicate_groups(settings.phash_threshold)
    }

    pub fn merge_duplicates(
        &self,
        keep_id: &str,
        remove_ids: &[String],
        delete_files: bool,
    ) -> AppResult<crate::models::MergeDuplicatesResult> {
        let removed = self.db.merge_duplicates(keep_id, remove_ids, delete_files)?;
        Ok(crate::models::MergeDuplicatesResult { removed })
    }

    pub fn list_cookie_sites(&self) -> AppResult<Vec<CookieSiteInfo>> {
        self.vault.list_sites()
    }

    pub fn save_site_cookies(&self, site_id: &str, cookies: &str) -> AppResult<()> {
        self.vault.save_cookies(site_id, cookies)
    }

    pub fn delete_site_cookies(&self, site_id: &str) -> AppResult<()> {
        self.vault.delete_cookies(site_id)
    }

    pub async fn resolve_standalone(&self, url: &str) -> AppResult<MediaItem> {
        crate::mobile::standalone::resolve(url).await
    }

    pub fn static_ui_path(&self) -> Option<PathBuf> {
        self.static_ui_dir.lock().clone()
    }
}
