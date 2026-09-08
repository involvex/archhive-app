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
use tauri_plugin_shell::ShellExt;

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
    app: tauri::AppHandle,
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
        let downloads = Arc::new(DownloadManager::new(db.clone(), app.clone(), vault.clone()));
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
            app,
        })
    }

    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.app
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
                is_live: None,
                viewers: None,
                age: None,
                gender: None,
                stream_url: None,
                embed_url: None,
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

    pub fn retry_download(&self, id: &str) -> AppResult<()> {
        self.downloads.retry(id)
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

    pub fn set_performer_image(&self, id: &str, image: Option<&str>) -> AppResult<()> {
        self.db.update_performer_image(id, image)
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
        let rules = settings.auto_tag_rules.clone();
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
        let token = if open_dev || !settings.lan_auth_enabled {
            String::new()
        } else {
            match settings.lan_token.clone() {
                Some(t) if !t.is_empty() => t,
                _ => crate::server::generate_token(),
            }
        };
        let static_dir = self.static_ui_path();
        let server = crate::server::LanServer::start(
            self.clone(),
            port,
            token.clone(),
            settings.lan_auth_enabled,
            static_dir,
        )
        .await?;
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
        notes: Option<&str>,
    ) -> AppResult<Scene> {
        self.db
            .update_scene(id, title, performers, tags, rename_file, notes)
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
        let json = runner.resolve_media_json(url, cookies.as_deref()).await?;

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
            is_live: None,
            viewers: None,
            age: None,
            gender: None,
            stream_url: None,
            embed_url: None,
        })
    }

    /// Resolve a direct streamable URL for a media item using `yt-dlp --get-url`.
    /// Used by the in-app Watch button to preview a remote video without downloading.
    pub async fn resolve_stream_url(&self, url: &str) -> AppResult<String> {
        let site_id = self
            .sites
            .detect(url)
            .unwrap_or_else(|| "custom".to_string());

        if let Some(adapter) = self.sites.get(&site_id) {
            if let Ok(stream_url) = adapter.resolve_stream_url(&self.site_ctx, url).await {
                return Ok(stream_url);
            }
        }

        let runner = crate::sites::yt_dlp::SidecarRunner::new(self.site_ctx.app().clone());
        let cookies = self.site_ctx.cookie_file_for_site(&site_id);
        let mut args = vec![
            url.to_string(),
            "--get-url".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
        ];
        if let Some(cookies) = cookies.as_ref() {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = runner.run_capture_for_stream_url("yt-dlp", &args).await?;
        let stream_url = raw
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .ok_or_else(|| crate::error::AppError::Other("No stream URL resolved".to_string()))?
            .to_string();
        Ok(stream_url)
    }

    pub async fn generate_missing_thumbs(&self) -> AppResult<crate::models::ThumbGenResult> {
        crate::library::LibraryScanner::generate_missing_thumbs(
            self.db.clone(),
            self.site_ctx.app().clone(),
            2,
        )
        .await
    }

    pub async fn probe_scene_metadata(&self, scene_id: &str) -> AppResult<crate::models::Scene> {
        let scene = self.db.get_scene(scene_id)?;
        let video_path = scene
            .path
            .as_deref()
            .ok_or_else(|| crate::error::AppError::NotFound("scene has no path".into()))?;
        let path = std::path::Path::new(video_path);
        if !path.is_file() {
            return Err(crate::error::AppError::NotFound(format!(
                "file not found: {video_path}"
            )));
        }
        let ffmpeg = crate::media::FfmpegProcessor::new(self.site_ctx.app().clone());

        // Probe duration and write to DB.
        if let Some(dur) = ffmpeg.probe_duration(path).await {
            if dur > 0.0 {
                let _ = self.db.update_scene_duration(scene_id, dur as u32);
            }
        }

        // Probe resolution and write to DB.
        if let Some((width, height)) = ffmpeg.probe_resolution(path).await {
            let _ = self.db.update_scene_resolution(scene_id, width, height);
        }

        // Extract thumbnail if missing.
        if scene
            .thumb
            .as_deref()
            .is_none_or(|t| t.is_empty() || !std::path::Path::new(t).is_file())
        {
            if let Ok(thumb) = ffmpeg.extract_thumbnail(path).await {
                let thumb_str = thumb.to_string_lossy().to_string();
                let _ = self.db.set_scene_thumb(scene_id, &thumb_str);
            }
        }

        self.db.get_scene(scene_id)
    }

    pub async fn probe_library_durations(
        &self,
        app: tauri::AppHandle,
        concurrency: usize,
    ) -> AppResult<crate::models::DurationProbeResult> {
        crate::library::LibraryScanner::probe_library_durations(self.db.clone(), app, concurrency)
            .await
    }

    pub async fn ffmpeg_status(&self) -> AppResult<crate::models::FfmpegStatus> {
        let ffmpeg = crate::media::FfmpegProcessor::new(self.site_ctx.app().clone());
        let available = ffmpeg.check_availability().await.is_ok();
        // If the combined check passes, both are available.
        // If it fails, try each individually to give granular info.
        if available {
            return Ok(crate::models::FfmpegStatus {
                ffmpeg_available: true,
                ffprobe_available: true,
            });
        }
        // Try ffmpeg alone
        let ffmpeg_ok = {
            let args = vec!["-version".to_string()];
            self.site_ctx
                .app()
                .shell()
                .sidecar("binaries/ffmpeg")
                .and_then(|cmd| cmd.args(&args).spawn())
                .is_ok()
        };
        let ffprobe_ok = {
            let args = vec!["-version".to_string()];
            self.site_ctx
                .app()
                .shell()
                .sidecar("binaries/ffprobe")
                .and_then(|cmd| cmd.args(&args).spawn())
                .is_ok()
        };
        Ok(crate::models::FfmpegStatus {
            ffmpeg_available: ffmpeg_ok,
            ffprobe_available: ffprobe_ok,
        })
    }

    pub fn list_scenes_with_filter(
        &self,
        filter: &crate::models::SceneFilter,
    ) -> AppResult<Vec<crate::models::Scene>> {
        self.db.list_scenes_with_filter(filter)
    }

    pub fn list_orphan_sidecars(&self) -> AppResult<Vec<crate::models::OrphanSidecar>> {
        let settings = self.get_settings()?;
        let path = Self::validate_library_path(&settings.library_path, &self.data_dir)?;
        self.db.list_orphan_sidecars(&path)
    }

    pub fn clear_scene_thumb(&self, scene_id: &str) -> AppResult<()> {
        self.db.clear_scene_thumb(scene_id)
    }

    pub fn clear_all_thumbs(&self) -> AppResult<crate::models::ClearThumbsResult> {
        let cleared = self.db.clear_all_thumbs()?;
        Ok(crate::models::ClearThumbsResult { cleared })
    }

    fn watched_threshold(&self) -> f64 {
        self.get_settings()
            .map(|s| s.watched_threshold as f64)
            .unwrap_or(0.9)
            .clamp(0.5, 1.0)
    }

    /// Watchlist poller tuning (review: named constants, single due-check).
    const POLL_TICK_SECS: u64 = 60;
    /// Cap per search per pass so a runaway listing can't flood the queue.
    const MAX_QUEUE_PER_SEARCH: usize = 25;
    /// Cap stored snapshot keys so the row stays small.
    const SNAPSHOT_KEY_CAP: usize = 300;

    /// A saved search is due when never checked or older than the interval.
    /// Unparseable timestamps are treated as due (fail-open toward checking).
    fn is_search_due(
        last_checked_at: &Option<String>,
        interval: chrono::Duration,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> bool {
        match last_checked_at {
            None => true,
            Some(ts) => chrono::DateTime::parse_from_rfc3339(ts)
                .map(|t| now.signed_duration_since(t.with_timezone(&chrono::Utc)) >= interval)
                .unwrap_or(true),
        }
    }

    /// Record playback position. `watched` latches: once watched, a scene
    /// stays watched until explicitly unmarked via `mark_watched`.
    pub fn record_watch_progress(
        &self,
        scene_id: &str,
        position_secs: f64,
        duration_secs: f64,
    ) -> AppResult<crate::models::WatchProgress> {
        // Scene must exist (cheap EXISTS check, not a full row hydration).
        if !self.db.scene_exists(scene_id)? {
            return Err(crate::error::AppError::NotFound(format!(
                "scene {scene_id}"
            )));
        }
        let position = position_secs.max(0.0);
        let duration = duration_secs.max(0.0);
        let threshold = self.watched_threshold();
        let auto_watched = duration > 0.0 && position >= threshold * duration;
        let existing = self.db.get_watch_progress(scene_id)?;
        let watched = existing.map(|e| e.watched).unwrap_or(false) || auto_watched;
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.db
            .record_watch_progress(scene_id, position, duration, watched, &updated_at)?;
        Ok(crate::models::WatchProgress {
            scene_id: scene_id.to_string(),
            position_secs: position,
            duration_secs: duration,
            watched,
            updated_at,
        })
    }

    pub fn get_watch_progress(
        &self,
        scene_id: &str,
    ) -> AppResult<Option<crate::models::WatchProgress>> {
        self.db.get_watch_progress(scene_id)
    }

    pub fn list_watch_progress(&self) -> AppResult<Vec<crate::models::WatchProgress>> {
        self.db.list_watch_progress()
    }

    pub fn list_watched_source_urls(&self) -> AppResult<Vec<String>> {
        self.db.list_watched_source_urls()
    }

    pub fn create_saved_search(
        &self,
        req: &crate::models::SaveSearchRequest,
    ) -> AppResult<crate::models::SavedSearch> {
        // Validate the site exists before storing.
        if self.sites.get(&req.site_id).is_none() {
            return Err(crate::error::AppError::NotFound(format!(
                "site {}",
                req.site_id
            )));
        }
        self.db.create_saved_search(req)
    }

    pub fn list_saved_searches(&self) -> AppResult<Vec<crate::models::SavedSearch>> {
        self.db.list_saved_searches()
    }

    pub fn delete_saved_search(&self, id: &str) -> AppResult<bool> {
        self.db.delete_saved_search(id)
    }

    pub fn set_saved_search_auto_queue(&self, id: &str, auto_queue: bool) -> AppResult<bool> {
        // Validate the id so typos don't silently no-op.
        if self.db.get_saved_search(id)?.is_none() {
            return Err(crate::error::AppError::NotFound(format!(
                "saved search {id}"
            )));
        }
        self.db.set_saved_search_auto_queue(id, auto_queue)
    }

    fn watch_poll_interval(&self) -> chrono::Duration {
        let mins = self
            .get_settings()
            .map(|s| s.watch_poll_interval_mins)
            .unwrap_or(60)
            .clamp(5, 1440);
        chrono::Duration::minutes(mins as i64)
    }

    /// One poller pass (#19 auto-queue pairing): check every due auto-queue
    /// saved search and queue new matches as downloads (capped per search).
    /// A search is due when never checked or older than the poll interval.
    pub async fn poll_watchlist_once(&self) -> AppResult<crate::models::WatchlistPollResult> {
        let interval = self.watch_poll_interval();
        let now = chrono::Utc::now();
        let mut checked = 0u32;
        let mut queued = 0u32;
        let mut errors = 0u32;
        let searches = self.db.list_saved_searches()?;
        for search in searches.iter().filter(|s| s.auto_queue) {
            if !Self::is_search_due(&search.last_checked_at, interval, &now) {
                continue;
            }
            match self.check_saved_search(&search.id).await {
                Ok(result) => {
                    checked += 1;
                    let urls: Vec<String> = result
                        .new_items
                        .iter()
                        .take(Self::MAX_QUEUE_PER_SEARCH)
                        .map(|i| i.url.clone())
                        .filter(|u| !u.trim().is_empty())
                        .collect();
                    if !urls.is_empty() {
                        match self.queue_downloads(&urls).await {
                            Ok(jobs) => queued += jobs.len() as u32,
                            Err(e) => {
                                eprintln!("[watchlist] queue failed for {}: {e}", search.name);
                                errors += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[watchlist] check failed for {}: {e}", search.name);
                    errors += 1;
                }
            }
        }
        let finished_at = chrono::Utc::now().to_rfc3339();
        let _ = self
            .db
            .record_poll_run(&finished_at, checked, queued, errors);
        Ok(crate::models::WatchlistPollResult {
            checked,
            queued,
            errors,
        })
    }

    pub fn watchlist_status(&self) -> AppResult<crate::models::WatchlistStatus> {
        let interval = self.watch_poll_interval();
        let now = chrono::Utc::now();
        let searches = self.db.list_saved_searches()?;
        let mut auto_queue_count = 0u32;
        let mut due_count = 0u32;
        for search in searches.iter().filter(|s| s.auto_queue) {
            auto_queue_count += 1;
            if Self::is_search_due(&search.last_checked_at, interval, &now) {
                due_count += 1;
            }
        }
        Ok(crate::models::WatchlistStatus {
            last_run: self.db.last_poll_run()?,
            auto_queue_count,
            due_count,
        })
    }

    pub fn dismiss_saved_search_news(&self, id: &str) -> AppResult<bool> {
        if self.db.get_saved_search(id)?.is_none() {
            return Err(crate::error::AppError::NotFound(format!(
                "saved search {id}"
            )));
        }
        self.db.dismiss_saved_search_news(id)
    }

    /// Background loop for the watchlist poller. Ticks every minute; each
    /// pass only touches searches past their poll interval.
    pub fn spawn_watchlist_poller(self: &std::sync::Arc<Self>) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut tick =
                tokio::time::interval(tokio::time::Duration::from_secs(Self::POLL_TICK_SECS));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                match state.poll_watchlist_once().await {
                    Ok(r) if r.checked > 0 => {
                        eprintln!(
                            "[watchlist] poll: checked {}, queued {}, errors {}",
                            r.checked, r.queued, r.errors
                        );
                    }
                    Err(e) => eprintln!("[watchlist] poll failed: {e}"),
                    _ => {}
                }
            }
        });
    }

    /// Re-run a saved search (page 1) and diff against the stored snapshot.
    /// First check after saving seeds the baseline and reports 0 new items.
    pub async fn check_saved_search(
        &self,
        id: &str,
    ) -> AppResult<crate::models::CheckSavedSearchResult> {
        let search = self
            .db
            .get_saved_search(id)?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("saved search {id}")))?;
        let page = self
            .browse(
                &search.site_id,
                search.kind,
                &search.slug,
                1,
                search.orientation,
            )
            .await?;
        let previous = self.db.saved_search_keys(id)?;
        let baseline = previous.is_empty() && search.last_checked_at.is_none();
        let seen: std::collections::HashSet<&str> = previous.iter().map(|s| s.as_str()).collect();
        let mut keys = Vec::with_capacity(page.items.len());
        let mut fresh = Vec::new();
        for item in &page.items {
            let key = if item.url.trim().is_empty() {
                item.id.clone()
            } else {
                item.url.clone()
            };
            keys.push(key.clone());
            if !baseline && !seen.contains(key.as_str()) {
                fresh.push(item.clone());
            }
        }
        keys.truncate(Self::SNAPSHOT_KEY_CAP);
        let keys_json = serde_json::to_string(&keys)
            .map_err(|e| crate::error::AppError::Other(format!("snapshot serialize: {e}")))?;
        let checked_at = chrono::Utc::now().to_rfc3339();
        let new_count = fresh.len() as u32;
        self.db
            .update_saved_search_check(id, &keys_json, new_count, &checked_at)?;
        Ok(crate::models::CheckSavedSearchResult {
            search_id: id.to_string(),
            new_items: fresh,
            new_count,
            total: page.items.len(),
            checked_at,
        })
    }

    pub fn mark_watched(
        &self,
        ids: &[String],
        watched: bool,
    ) -> AppResult<crate::models::MarkWatchedResult> {
        // Validate ids against the library so typos don't create orphan rows.
        let valid = self.db.existing_scene_ids(ids)?;
        let updated_at = chrono::Utc::now().to_rfc3339();
        let updated = self.db.mark_watched(&valid, watched, &updated_at)?;
        Ok(crate::models::MarkWatchedResult { updated })
    }

    pub async fn binary_versions(&self) -> AppResult<crate::models::BinaryVersions> {
        use crate::sites::yt_dlp::SidecarRunner;
        let runner = SidecarRunner::new(self.site_ctx.app().clone());
        let (ffmpeg, ffprobe, ytdlp, gallery_dl) = tokio::join!(
            runner.tool_version("ffmpeg"),
            runner.tool_version("ffprobe"),
            runner.tool_version("yt-dlp"),
            runner.tool_version("gallery-dl"),
        );
        Ok(crate::models::BinaryVersions {
            ffmpeg_version: ffmpeg,
            ffprobe_version: ffprobe,
            ytdlp_version: ytdlp,
            gallery_dl_version: gallery_dl,
        })
    }

    pub fn get_library_stats(&self) -> AppResult<crate::models::LibraryStats> {
        let scenes = self
            .db
            .list_scenes(None, crate::models::SceneSort::Newest)?;
        let performers = self.db.list_performers(None)?;
        let tags = self.db.list_tags()?;

        let settings = self.get_settings()?;
        let library_path = Self::validate_library_path(&settings.library_path, &self.data_dir)?;
        let library_dir = std::path::Path::new(&library_path);

        let mut total_size: u64 = 0;
        if library_dir.exists() {
            for entry in walkdir::WalkDir::new(library_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }

        Ok(crate::models::LibraryStats {
            scene_count: scenes.len() as u64,
            performer_count: performers.len() as u64,
            tag_count: tags.len() as u64,
            total_size_bytes: total_size,
            free_space_bytes: 0,
        })
    }

    pub fn static_ui_path(&self) -> Option<PathBuf> {
        self.static_ui_dir.lock().clone()
    }

    pub fn list_files(&self, path: &str) -> AppResult<crate::models::FilesListResponse> {
        let root = self.cached_library_root()?;
        let rel = path.trim_start_matches('/');
        let dir = if rel.is_empty() {
            root.clone()
        } else {
            root.join(rel)
        };
        if !dir.is_dir() {
            return Err(crate::error::AppError::NotFound(format!(
                "Directory not found: {}",
                dir.display()
            )));
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let meta = entry.metadata()?;
            let abs = entry.path();
            let rel_path = relative_path(&root, &abs);
            let mime = if meta.is_dir() {
                None
            } else {
                Some(mime_from_path(&abs).to_string())
            };
            entries.push(crate::models::FileEntry {
                name,
                path: rel_path,
                is_dir: meta.is_dir(),
                size: if meta.is_file() { Some(meta.len()) } else { None },
                mime,
            });
        }
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        Ok(crate::models::FilesListResponse {
            path: rel.to_string(),
            entries,
        })
    }
}

fn relative_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .to_string()
        .replace('\\', "/")
}

fn mime_from_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        Some(e) if e == "mp4" => "video/mp4",
        Some(e) if e == "webm" => "video/webm",
        Some(e) if e == "mkv" => "video/x-matroska",
        Some(e) if e == "avi" => "video/x-msvideo",
        Some(e) if e == "mov" => "video/quicktime",
        Some(e) if e == "wmv" => "video/x-ms-wmv",
        Some(e) if e == "flv" => "video/x-flv",
        Some(e) if e == "m4v" => "video/x-m4v",
        Some(e) if e == "jpg" || e == "jpeg" => "image/jpeg",
        Some(e) if e == "png" => "image/png",
        Some(e) if e == "gif" => "image/gif",
        Some(e) if e == "webp" => "image/webp",
        Some(e) if e == "mp3" => "audio/mpeg",
        Some(e) if e == "m4a" => "audio/mp4",
        Some(e) if e == "opus" => "audio/opus",
        Some(e) if e == "wav" => "audio/wav",
        Some(e) if e == "txt" => "text/plain",
        Some(e) if e == "srt" => "text/plain",
        Some(e) if e == "vtt" => "text/vtt",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn search_due_logic() {
        let interval = chrono::Duration::minutes(60);
        let now = chrono::Utc::now();
        // Never checked → due.
        assert!(AppState::is_search_due(&None, interval, &now));
        // Recent check → not due.
        let recent = Some(now.to_rfc3339());
        assert!(!AppState::is_search_due(&recent, interval, &now));
        // Old check → due.
        let old = Some((now - chrono::Duration::minutes(61)).to_rfc3339());
        assert!(AppState::is_search_due(&old, interval, &now));
        // Corrupt timestamp → due (fail-open).
        assert!(AppState::is_search_due(
            &Some("not-a-date".to_string()),
            interval,
            &now
        ));
    }
}
