use crate::db::Database;
use crate::downloads::DownloadManager;
use crate::error::AppResult;
use crate::models::{
    AppSettings, BrowseKind, BrowseOrientation, BrowseQuery, DownloadJob, DuplicateGroup,
    HealthResponse, MediaItem, Performer, ScanResult, Scene, SiteInfo, Tag,
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
    library_root_cache: Arc<Mutex<Option<PathBuf>>>,
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
        let site_ctx = Arc::new(SiteContext::new(vault.clone(), app.clone())?);
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
            library_root_cache: Arc::new(Mutex::new(None)),
        })
    }

    pub fn cached_library_root(&self) -> AppResult<PathBuf> {
        if let Some(root) = self.library_root_cache.lock().clone() {
            return Ok(root);
        }
        let settings = self.get_settings()?;
        let path = Self::validate_library_path(&settings.library_path, &self.data_dir)?;
        // Always store canonical path so Range media checks match Windows \\?\ prefixes.
        let root = PathBuf::from(&path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(&path));
        *self.library_root_cache.lock() = Some(root.clone());
        Ok(root)
    }

    fn invalidate_library_cache(&self) {
        *self.library_root_cache.lock() = None;
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
        orientation: Option<BrowseOrientation>,
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
                    orientation,
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
            description: None,
            channel: None,
            };
            let plan = site_adapter.resolve_download(&self.site_ctx, &item).await?;
            return self.downloads.queue_plan(plan);
        }

        self.downloads.queue(url, &adapter_id, None)
    }

    pub async fn queue_downloads(&self, urls: &[String]) -> AppResult<Vec<DownloadJob>> {
        let mut jobs = Vec::with_capacity(urls.len());
        for url in urls {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                continue;
            }
            match self.queue_download(trimmed, None).await {
                Ok(job) => jobs.push(job),
                Err(e) => eprintln!("[queue] skipped {trimmed}: {e}"),
            }
        }
        Ok(jobs)
    }

    pub fn list_downloads(&self) -> AppResult<Vec<DownloadJob>> {
        self.db.list_download_jobs()
    }

    pub fn cancel_download(&self, id: &str) -> AppResult<()> {
        self.downloads.cancel(id)
    }

    pub fn pause_download(&self, id: &str) -> AppResult<()> {
        self.downloads.pause(id)
    }

    pub fn resume_download(&self, id: &str) -> AppResult<()> {
        self.downloads.resume(id)
    }

    pub fn delete_download(&self, id: &str) -> AppResult<()> {
        self.downloads.delete(id)
    }

    pub async fn queue_bulk_import(
        &self,
        urls: &[String],
        expand_browse: bool,
        import_all: bool,
    ) -> AppResult<crate::models::BulkImportResult> {
        use crate::downloads::bulk::{is_browse_url, is_likely_video_url};
        use crate::sites::yt_dlp::SidecarRunner;

        const MAX_PER_BROWSE: u32 = 100;
        let runner = SidecarRunner::new(self.site_ctx.app().clone());
        let mut queued = 0u32;
        let mut expanded = 0u32;
        let mut skipped = 0u32;

        for raw in urls {
            let url = raw.trim();
            if url.is_empty() {
                continue;
            }

            if (expand_browse || import_all) && is_browse_url(url) {
                let adapter = self
                    .sites
                    .detect(url)
                    .unwrap_or_else(|| "generic_ytdlp".to_string());
                let cookies = self.vault.cookie_file_for_site(&adapter);
                match runner
                    .list_flat_playlist_all(url, MAX_PER_BROWSE, cookies.as_deref())
                    .await
                {
                    Ok(entries) => {
                        expanded += 1;
                        for (_, _, video_url, _) in entries {
                            if is_likely_video_url(&video_url) || import_all {
                                if self.queue_download(&video_url, None).await.is_ok() {
                                    queued += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[bulk] browse expand failed {url}: {e}");
                        skipped += 1;
                    }
                }
                continue;
            }

            if is_likely_video_url(url) || import_all {
                if self.queue_download(url, None).await.is_ok() {
                    queued += 1;
                }
            } else {
                skipped += 1;
            }
        }

        Ok(crate::models::BulkImportResult {
            queued,
            expanded,
            skipped,
        })
    }

    pub fn list_scenes(
        &self,
        query: Option<&str>,
        sort: crate::models::SceneSort,
    ) -> AppResult<Vec<Scene>> {
        self.db.list_scenes(query, sort)
    }

    pub fn delete_scene(&self, id: &str, delete_files: bool) -> AppResult<()> {
        self.db.delete_scene(id, delete_files)
    }

    pub fn ensure_performer(&self, name: &str) -> AppResult<Performer> {
        let id = self.db.upsert_performer(name)?;
        Ok(Performer {
            id,
            name: name.to_string(),
            aliases: vec![],
            image: None,
            favorite: false,
            scene_count: 0,
        })
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
        let prev_path = self.get_settings().ok().map(|s| s.library_path);
        self.db.save_settings(settings)?;
        if prev_path.as_deref() != Some(settings.library_path.as_str()) {
            self.invalidate_library_cache();
        }
        Ok(())
    }

    pub fn scan_library(&self) -> AppResult<ScanResult> {
        let settings = self.db.get_settings()?;
        let path = Self::validate_library_path(&settings.library_path, &self.data_dir)?;
        let rules = vec![r"(?<performer>[a-zA-Z0-9_]+)-\d+".to_string()];
        crate::library::LibraryScanner::scan(&self.db, &path, &rules, None)
    }

    pub fn validate_library_path(library_path: &str, _data_dir: &Path) -> AppResult<String> {
        let trimmed = library_path.trim();
        if trimmed.is_empty() {
            return Err(crate::error::AppError::InvalidInput(
                "Library path is not configured. Set it in Settings → Library.".into(),
            ));
        }
        let path = std::path::Path::new(trimmed);
        if !path.is_absolute() {
            return Err(crate::error::AppError::InvalidInput(
                "Library path must be absolute (e.g. I:\\videos or C:\\Users\\you\\Videos).".into(),
            ));
        }
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Ok(canonical.to_string_lossy().to_string())
    }

    pub fn stop_lan_server(&self) -> AppResult<()> {
        if let Some(mut server) = self.lan_server.lock().take() {
            server.stop();
        }
        let mut settings = self.get_settings()?;
        settings.lan_enabled = false;
        self.save_settings(&settings)?;
        Ok(())
    }

    pub async fn ensure_lan_server(self: &Arc<Self>, port: u16) -> AppResult<String> {
        let open_dev = std::env::var("ARCHIVE_AUTO_LAN").ok().as_deref() == Some("1");

        if self.lan_server.lock().is_some() {
            let current = self.get_settings()?.lan_token.unwrap_or_default();
            if open_dev && !current.is_empty() {
                self.stop_lan_server()?;
            } else {
                return Ok(current);
            }
        }

        let mut settings = self.get_settings()?;
        let token = if open_dev {
            String::new()
        } else {
            match settings.lan_token.clone() {
                Some(t) if !t.is_empty() => t,
                _ => crate::server::generate_token(),
            }
        };
        let static_dir = self.static_ui_path();
        let server =
            crate::server::LanServer::start(self.clone(), port, token.clone(), static_dir).await?;
        *self.lan_server.lock() = Some(server);
        settings.lan_enabled = true;
        settings.lan_port = port;
        settings.lan_token = if token.is_empty() {
            None
        } else {
            Some(token.clone())
        };
        self.save_settings(&settings)?;
        eprintln!(
            "[lan] server started on port {port} auth_required={}",
            !token.is_empty()
        );
        Ok(token)
    }

    pub async fn regenerate_lan_server(self: &Arc<Self>, port: u16) -> AppResult<String> {
        self.stop_lan_server()?;
        let mut settings = self.get_settings()?;
        settings.lan_token = None;
        self.save_settings(&settings)?;
        self.ensure_lan_server(port).await
    }

    pub fn get_scene(&self, id: &str) -> AppResult<Scene> {
        self.db.get_scene(id)
    }

    pub fn update_scene(
        &self,
        id: &str,
        title: Option<&str>,
        performers: Option<&[String]>,
        tags: Option<&[String]>,
        rename_file: bool,
    ) -> AppResult<Scene> {
        self.db
            .update_scene(id, title, performers, tags, rename_file)
    }

    pub fn batch_update_scenes(
        &self,
        ids: &[String],
        performers_add: Option<&[String]>,
        tags_add: Option<&[String]>,
    ) -> AppResult<crate::models::BatchUpdateScenesResult> {
        let updated = self.db.batch_update_scenes(ids, performers_add, tags_add)?;
        Ok(crate::models::BatchUpdateScenesResult { updated })
    }

    pub async fn list_pornhub_categories(
        &self,
        orientation: crate::models::BrowseOrientation,
    ) -> AppResult<Vec<crate::models::PornhubCategoryEntry>> {
        let url = crate::sites::adapters::pornhub::categories_page_url(orientation);
        let html = self.site_ctx.fetch_html(&url, "pornhub").await?;
        Ok(crate::sites::adapters::pornhub::parse_pornhub_categories(
            &html,
            orientation,
        ))
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
        let removed = self
            .db
            .merge_duplicates(keep_id, remove_ids, delete_files)?;
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

    pub async fn resolve_media_details(&self, url: &str) -> AppResult<MediaItem> {
        let runner = crate::sites::yt_dlp::SidecarRunner::new(self.site_ctx.app().clone());
        let site_id = self
            .sites
            .detect(url)
            .unwrap_or_else(|| "custom".to_string());
        let cookies = self.site_ctx.cookie_file_for_site(&site_id);
        let json = runner
            .resolve_media_json(url, cookies.as_deref())
            .await?;

        let title = json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(url)
            .to_string();
        let description = json
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let channel = json
            .get("uploader")
            .or_else(|| json.get("channel"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let thumbnail = json
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                json.get("thumbnails")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|t| t.get("url"))
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string())
            });
        let duration = json
            .get("duration")
            .and_then(|v| v.as_f64())
            .map(|d| d as u32);
        let tags = json
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut performers = Vec::new();
        if let Some(ch) = channel.as_ref() {
            performers.push(ch.clone());
        }

        Ok(MediaItem {
            id: json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(url)
                .to_string(),
            title,
            url: url.to_string(),
            thumbnail,
            duration,
            site_id,
            performers,
            tags,
            description,
            channel,
        })
    }

    pub async fn generate_missing_thumbs(&self) -> AppResult<u32> {
        crate::library::LibraryScanner::generate_missing_thumbs(
            self.db.clone(),
            self.site_ctx.app().clone(),
            2,
        )
        .await
    }

    pub fn static_ui_path(&self) -> Option<PathBuf> {
        self.static_ui_dir.lock().clone()
    }
}
