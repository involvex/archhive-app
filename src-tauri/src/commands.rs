use crate::error::AppResult;
use crate::models::{
    AppSettings, BatchUpdateScenesRequest, BatchUpdateScenesResult, BrowseKind, BrowseOrientation,
    CheckSavedSearchResult, Collection, CreateCollectionRequest, DownloadJob, DuplicateGroup,
    ExportCollectionRequest, ExportCollectionResult, FfmpegStatus, HealthResponse, LanHost,
    LibraryStats, MarkWatchedRequest, MarkWatchedResult, MediaItem, MergeDuplicatesResult,
    OrphanSidecar, Performer, PornhubCategoryEntry, SaveSearchRequest, SavedSearch, ScanResult,
    Scene, SceneFilter, SceneSort, SiteInfo, Tag, UpdateCollectionRequest,
    UpdateSavedSearchRequest, UpdateSceneRequest, WatchProgress, WatchlistPollResult,
    WatchlistStatus,
};
use crate::state::AppState;
use crate::vault::CookieSiteInfo;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

fn map_err<T>(result: AppResult<T>) -> CmdResult<T> {
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn health() -> HealthResponse {
    AppState::health()
}

#[tauri::command]
pub async fn list_sites(state: State<'_, Arc<AppState>>) -> Result<Vec<SiteInfo>, String> {
    Ok(state.list_sites().await)
}

#[tauri::command]
pub async fn browse(
    state: State<'_, Arc<AppState>>,
    site_id: String,
    kind: BrowseKind,
    slug: String,
    page: Option<u32>,
    orientation: Option<BrowseOrientation>,
) -> CmdResult<crate::models::BrowsePage> {
    map_err(
        state
            .browse(&site_id, kind, &slug, page.unwrap_or(1), orientation)
            .await,
    )
}

#[tauri::command]
pub fn put_browse_cache(
    state: State<'_, Arc<AppState>>,
    site_id: String,
    kind: String,
    slug: String,
    page: u32,
    orientation: Option<String>,
    payload: crate::models::BrowsePage,
    ttl_secs: Option<i64>,
) -> CmdResult<()> {
    map_err(state.db.put_browse_cache(
        &site_id,
        &kind,
        &slug,
        page,
        orientation.as_deref(),
        &payload,
        ttl_secs.unwrap_or(86_400),
    ))
}

#[tauri::command]
pub fn get_browse_cache(
    state: State<'_, Arc<AppState>>,
    site_id: String,
    kind: String,
    slug: String,
    page: u32,
    orientation: Option<String>,
    allow_stale: Option<bool>,
) -> CmdResult<Option<crate::models::BrowsePage>> {
    map_err(state.db.get_browse_cache(
        &site_id,
        &kind,
        &slug,
        page,
        orientation.as_deref(),
        allow_stale.unwrap_or(true),
    ))
}

#[tauri::command]
pub async fn queue_download(
    state: State<'_, Arc<AppState>>,
    url: String,
    adapter: Option<String>,
    title: Option<String>,
) -> CmdResult<DownloadJob> {
    map_err(
        state
            .queue_download(&url, adapter.as_deref(), title.as_deref())
            .await,
    )
}

#[tauri::command]
pub async fn queue_downloads(
    state: State<'_, Arc<AppState>>,
    urls: Vec<String>,
) -> CmdResult<Vec<DownloadJob>> {
    map_err(state.queue_downloads(&urls).await)
}

#[tauri::command]
pub fn list_downloads(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<DownloadJob>> {
    map_err(state.list_downloads())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.cancel_download(&id))
}

#[tauri::command]
pub fn pause_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.pause_download(&id))
}

#[tauri::command]
pub fn resume_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.resume_download(&id))
}

#[tauri::command]
pub fn retry_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.retry_download(&id))
}

#[tauri::command]
pub fn delete_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.delete_download(&id))
}

#[tauri::command]
pub fn update_network_state(
    state: State<'_, Arc<AppState>>,
    connection_type: String,
    metered: bool,
) -> CmdResult<()> {
    state.network_monitor.set_connection_type(&connection_type);
    state.network_monitor.set_metered(metered);
    // Immediately check and update downloads
    let monitor = state.network_monitor.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = monitor.check_and_update_downloads().await {
            tracing::warn!(
                "[network] failed to update downloads after network change: {}",
                e
            );
        }
    });
    Ok(())
}

#[derive(serde::Serialize)]
pub struct NetworkInfo {
    connection_type: String,
    metered: bool,
}

#[tauri::command]
pub fn get_network_info(state: State<'_, Arc<AppState>>) -> CmdResult<NetworkInfo> {
    Ok(NetworkInfo {
        connection_type: state.network_monitor.connection_type(),
        metered: state.network_monitor.is_metered(),
    })
}

#[tauri::command]
pub async fn queue_bulk_import(
    state: State<'_, Arc<AppState>>,
    urls: Vec<String>,
    expand_browse: bool,
    import_all: bool,
) -> CmdResult<crate::models::BulkImportResult> {
    map_err(
        state
            .queue_bulk_import(&urls, expand_browse, import_all)
            .await,
    )
}

#[tauri::command]
pub fn list_scenes(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
    sort: Option<SceneSort>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> CmdResult<Vec<Scene>> {
    map_err(state.list_scenes(query.as_deref(), sort.unwrap_or_default(), limit, offset))
}

#[tauri::command]
pub fn delete_scene(
    state: State<'_, Arc<AppState>>,
    id: String,
    delete_files: Option<bool>,
) -> CmdResult<()> {
    map_err(state.delete_scene(&id, delete_files.unwrap_or(false)))
}

#[tauri::command]
pub fn ensure_performer(state: State<'_, Arc<AppState>>, name: String) -> CmdResult<Performer> {
    map_err(state.ensure_performer(&name))
}

#[tauri::command]
pub fn list_performers(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
) -> CmdResult<Vec<Performer>> {
    map_err(state.list_performers(query.as_deref()))
}

#[tauri::command]
pub fn list_tags(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<Tag>> {
    map_err(state.list_tags())
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> CmdResult<AppSettings> {
    map_err(state.get_settings())
}

#[tauri::command]
pub fn save_settings(
    _app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> CmdResult<()> {
    let _prev = state.get_settings().ok();
    map_err(state.save_settings(&settings))?;
    #[cfg(not(mobile))]
    if _prev
        .as_ref()
        .is_none_or(|p| crate::desktop::tray_settings_changed(p, &settings))
    {
        crate::desktop::sync_from_settings(&_app, &settings);
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_library(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> CmdResult<ScanResult> {
    let app_state = state.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let settings = app_state.db.get_settings().map_err(|e| e.to_string())?;
            let path = AppState::validate_library_path(&settings.library_path, &app_state.data_dir)
                .map_err(|e| e.to_string())?;
            let rules = settings.auto_tag_rules.clone();
            let app_for_progress = app.clone();
            crate::library::LibraryScanner::scan(
                &app_state.db,
                &path,
                &rules,
                Some(Box::new(move |progress| {
                    let _ = app_for_progress.emit("library:scan-progress", progress);
                })),
            )
            .map_err(|e| e.to_string())
        }))
        .map_err(|_| "Library scan panicked — skipped unreadable files.".to_string())?
    })
    .await
    .map_err(|e| e.to_string())??;

    // Best-effort thumb generation after scan (does not fail the scan).
    let thumbs_state = state.inner().clone();
    let _ = thumbs_state.generate_missing_thumbs().await;

    Ok(result)
}

#[tauri::command]
pub async fn generate_missing_thumbs(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::ThumbGenResult> {
    map_err(state.generate_missing_thumbs().await)
}

#[tauri::command]
pub async fn resolve_media_details(
    state: State<'_, Arc<AppState>>,
    url: String,
) -> CmdResult<MediaItem> {
    map_err(state.resolve_media_details(&url).await)
}

#[tauri::command]
pub async fn resolve_stream_url(state: State<'_, Arc<AppState>>, url: String) -> CmdResult<String> {
    map_err(state.resolve_stream_url(&url).await)
}

#[tauri::command]
pub async fn resolve_livestream(
    state: State<'_, Arc<AppState>>,
    url: String,
) -> CmdResult<serde_json::Value> {
    let embed_url = crate::sites::urls::derive_embed_url(&url);
    // Try yt-dlp stream URL resolution first. If it fails, fall back to the
    // embed iframe (e.g. Chaturbate affiliate embed with embed_video_only=1).
    let stream_url = match state.resolve_stream_url(&url).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(
                "resolve_stream_url failed for live stream, falling back to embed iframe: {}",
                e
            );
            String::new()
        }
    };
    Ok(serde_json::json!({
        "stream_url": stream_url,
        "embed_url": embed_url,
    }))
}

#[tauri::command]
pub fn find_duplicates(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<DuplicateGroup>> {
    map_err(state.find_duplicates())
}

#[tauri::command]
pub fn merge_duplicates(
    state: State<'_, Arc<AppState>>,
    keep_id: String,
    remove_ids: Vec<String>,
    delete_files: Option<bool>,
) -> CmdResult<MergeDuplicatesResult> {
    map_err(state.merge_duplicates(&keep_id, &remove_ids, delete_files.unwrap_or(false)))
}

#[tauri::command]
pub fn list_cookie_sites(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<CookieSiteInfo>> {
    map_err(state.list_cookie_sites())
}

#[tauri::command]
pub fn save_site_cookies(
    state: State<'_, Arc<AppState>>,
    site_id: String,
    cookies: String,
) -> CmdResult<()> {
    map_err(state.save_site_cookies(&site_id, &cookies))
}

#[tauri::command]
pub fn delete_site_cookies(state: State<'_, Arc<AppState>>, site_id: String) -> CmdResult<()> {
    map_err(state.delete_site_cookies(&site_id))
}

#[tauri::command]
pub fn export_settings_backup(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::SettingsBackup> {
    map_err(state.export_settings_backup())
}

#[tauri::command]
pub fn import_settings_backup(
    _app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    backup: crate::models::SettingsBackup,
    options: Option<crate::models::SettingsBackupImportOptions>,
) -> CmdResult<crate::models::SettingsBackupImportResult> {
    let opts = options.unwrap_or(crate::models::SettingsBackupImportOptions {
        include_remote_credentials: false,
    });
    let result = map_err(state.import_settings_backup(&backup, &opts))?;
    #[cfg(not(mobile))]
    if let Ok(settings) = state.get_settings() {
        crate::desktop::sync_from_settings(&_app, &settings);
    }
    Ok(result)
}

#[tauri::command]
pub async fn resolve_standalone(
    state: State<'_, Arc<AppState>>,
    url: String,
) -> CmdResult<MediaItem> {
    map_err(state.resolve_standalone(&url).await)
}

#[tauri::command]
pub async fn discover_lan_hosts(timeout_ms: Option<u64>) -> CmdResult<Vec<LanHost>> {
    let timeout = timeout_ms.unwrap_or(4000);
    tauri::async_runtime::spawn_blocking(move || crate::discovery::discover_lan_hosts(timeout))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_lan_server(
    state: State<'_, Arc<AppState>>,
    port: u16,
) -> CmdResult<serde_json::Value> {
    let token = map_err(state.ensure_lan_server(port).await)?;
    Ok(serde_json::json!({ "token": token, "port": port, "auth_required": !token.is_empty() }))
}

#[tauri::command]
pub fn stop_lan_server(state: State<'_, Arc<AppState>>) -> CmdResult<()> {
    map_err(state.stop_lan_server())
}

#[tauri::command]
pub async fn regenerate_lan_server(
    state: State<'_, Arc<AppState>>,
    port: u16,
) -> CmdResult<serde_json::Value> {
    let token = map_err(state.regenerate_lan_server(port).await)?;
    Ok(serde_json::json!({ "token": token, "port": port, "auth_required": !token.is_empty() }))
}

#[tauri::command]
pub fn get_scene(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<Scene> {
    map_err(state.get_scene(&id))
}

#[tauri::command]
pub fn update_scene(
    state: State<'_, Arc<AppState>>,
    id: String,
    body: UpdateSceneRequest,
) -> CmdResult<Scene> {
    map_err(state.update_scene(
        &id,
        body.title.as_deref(),
        body.performers.as_deref(),
        body.tags.as_deref(),
        body.rename_file.unwrap_or(false),
        body.notes.as_deref(),
        body.rating,
    ))
}

#[tauri::command]
pub async fn open_scene_in_explorer(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let scene = map_err(state.get_scene(&id))?;
    let path = scene
        .path
        .ok_or_else(|| "Scene has no file on disk".to_string())?;
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_scene_with_default(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let scene = map_err(state.get_scene(&id))?;
    let path = scene
        .path
        .ok_or_else(|| "Scene has no file on disk".to_string())?;
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_update_scenes(
    state: State<'_, Arc<AppState>>,
    body: BatchUpdateScenesRequest,
) -> CmdResult<BatchUpdateScenesResult> {
    map_err(state.batch_update_scenes(
        &body.scene_ids,
        body.performers_add.as_deref(),
        body.tags_add.as_deref(),
    ))
}

#[tauri::command]
pub async fn list_pornhub_categories(
    state: State<'_, Arc<AppState>>,
    orientation: BrowseOrientation,
) -> CmdResult<Vec<PornhubCategoryEntry>> {
    map_err(state.list_pornhub_categories(orientation).await)
}

#[tauri::command]
pub async fn probe_scene_metadata(
    state: State<'_, Arc<AppState>>,
    scene_id: String,
) -> CmdResult<Scene> {
    map_err(state.probe_scene_metadata(&scene_id).await)
}

#[tauri::command]
pub async fn probe_library_durations(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    concurrency: Option<u32>,
) -> CmdResult<crate::models::DurationProbeResult> {
    map_err(
        state
            .probe_library_durations(app, concurrency.unwrap_or(2) as usize)
            .await,
    )
}

#[tauri::command]
pub async fn ffmpeg_status(state: State<'_, Arc<AppState>>) -> CmdResult<FfmpegStatus> {
    map_err(state.ffmpeg_status().await)
}

#[tauri::command]
pub fn list_scenes_with_filter(
    state: State<'_, Arc<AppState>>,
    filter: SceneFilter,
    limit: Option<i64>,
    offset: Option<i64>,
) -> CmdResult<Vec<Scene>> {
    map_err(state.list_scenes_with_filter(&filter, limit, offset))
}

#[tauri::command]
pub fn list_orphan_sidecars(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<OrphanSidecar>> {
    map_err(state.list_orphan_sidecars())
}

/// Q43: thumbnail sidecar cache totals for the Settings storage row.
#[tauri::command]
pub fn thumb_cache_stats(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::ThumbCacheStats> {
    map_err(state.thumb_cache_stats())
}

/// Single-pass orphan + thumb-cache scan for Settings → Library.
#[tauri::command]
pub fn library_storage_stats(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::LibraryStorageStats> {
    map_err(state.library_storage_stats())
}

#[tauri::command]
pub fn delete_orphan_sidecar(state: State<'_, Arc<AppState>>, path: String) -> CmdResult<()> {
    map_err(state.delete_orphan_sidecar(&path))
}

#[tauri::command]
pub fn clear_scene_thumb(state: State<'_, Arc<AppState>>, scene_id: String) -> CmdResult<()> {
    map_err(state.clear_scene_thumb(&scene_id))
}

#[tauri::command]
pub fn clear_all_thumbs(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::ClearThumbsResult> {
    map_err(state.clear_all_thumbs())
}

#[tauri::command]
pub async fn binary_versions(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::BinaryVersions> {
    map_err(state.binary_versions().await)
}

#[tauri::command]
pub async fn get_library_stats(state: State<'_, Arc<AppState>>) -> CmdResult<LibraryStats> {
    let s = Arc::clone(&state);
    let result = tauri::async_runtime::spawn_blocking(move || s.get_library_stats())
        .await
        .map_err(|e| e.to_string())?;
    map_err(result)
}

#[tauri::command]
pub async fn get_diagnostics(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::DiagnosticsData> {
    map_err(state.get_diagnostics().await)
}

#[tauri::command]
pub fn clear_logs() -> CmdResult<()> {
    crate::log_buffer::LogBuffer::instance().clear();
    Ok(())
}

#[tauri::command]
pub fn get_recent_logs(limit: Option<usize>) -> CmdResult<Vec<crate::models::LogEntry>> {
    Ok(crate::log_buffer::LogBuffer::instance().get_recent(limit.unwrap_or(200)))
}

#[tauri::command]
pub fn record_watch_progress(
    state: State<'_, Arc<AppState>>,
    scene_id: String,
    position_secs: f64,
    duration_secs: f64,
) -> CmdResult<WatchProgress> {
    map_err(state.record_watch_progress(&scene_id, position_secs, duration_secs))
}

#[tauri::command]
pub fn get_watch_progress(
    state: State<'_, Arc<AppState>>,
    scene_id: String,
) -> CmdResult<Option<WatchProgress>> {
    map_err(state.get_watch_progress(&scene_id))
}

#[tauri::command]
pub fn list_watch_progress(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<WatchProgress>> {
    map_err(state.list_watch_progress())
}

#[tauri::command]
pub fn mark_watched(
    state: State<'_, Arc<AppState>>,
    body: MarkWatchedRequest,
) -> CmdResult<MarkWatchedResult> {
    map_err(state.mark_watched(&body.scene_ids, body.watched))
}

#[tauri::command]
pub fn list_watched_source_urls(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<String>> {
    map_err(state.list_watched_source_urls())
}

#[tauri::command]
pub fn save_search(
    state: State<'_, Arc<AppState>>,
    body: SaveSearchRequest,
) -> CmdResult<SavedSearch> {
    map_err(state.create_saved_search(&body))
}

#[tauri::command]
pub fn list_saved_searches(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<SavedSearch>> {
    map_err(state.list_saved_searches())
}

#[tauri::command]
pub fn delete_saved_search(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<bool> {
    map_err(state.delete_saved_search(&id))
}

#[tauri::command]
pub async fn check_saved_search(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> CmdResult<CheckSavedSearchResult> {
    map_err(state.check_saved_search(&id).await)
}

#[tauri::command]
pub fn update_saved_search(
    state: State<'_, Arc<AppState>>,
    id: String,
    body: UpdateSavedSearchRequest,
) -> CmdResult<bool> {
    map_err(state.set_saved_search_auto_queue(&id, body.auto_queue))
}

#[tauri::command]
pub async fn poll_watchlist(state: State<'_, Arc<AppState>>) -> CmdResult<WatchlistPollResult> {
    map_err(state.poll_watchlist_once().await)
}

#[tauri::command]
pub fn watchlist_status(state: State<'_, Arc<AppState>>) -> CmdResult<WatchlistStatus> {
    map_err(state.watchlist_status())
}

#[tauri::command]
pub fn dismiss_saved_search_news(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<bool> {
    map_err(state.dismiss_saved_search_news(&id))
}

#[tauri::command]
pub fn export_performers(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<Performer>> {
    map_err(state.list_performers(None))
}

#[tauri::command]
pub fn set_performer_image(
    state: State<'_, Arc<AppState>>,
    id: String,
    image: Option<String>,
) -> CmdResult<()> {
    map_err(state.set_performer_image(&id, image.as_deref()))
}

#[tauri::command]
pub async fn install_yt_dlp(state: State<'_, Arc<AppState>>) -> CmdResult<serde_json::Value> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    let path = installer
        .install_yt_dlp()
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "installed": true,
        "path": path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn install_gallery_dl(state: State<'_, Arc<AppState>>) -> CmdResult<serde_json::Value> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    let path = installer
        .install_gallery_dl()
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "installed": true,
        "path": path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn get_installed_binaries(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::BinaryVersions> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    Ok(installer.get_installed_versions().await)
}

#[tauri::command]
pub async fn uninstall_binary(state: State<'_, Arc<AppState>>, name: String) -> CmdResult<bool> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;

    let path = installer.install_dir().join(&name);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn list_files(
    state: State<'_, Arc<AppState>>,
    path: Option<String>,
) -> CmdResult<crate::models::FilesListResponse> {
    map_err(state.list_files(path.as_deref().unwrap_or("")))
}

/// Diagnose why an ffmpeg/ffprobe sidecar reports "not found": distinguishes
/// "not bundled in this build" from "bundled but failed to execute".
#[tauri::command]
pub async fn probe_sidecar(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> CmdResult<crate::models::SidecarProbe> {
    if name != "ffmpeg" && name != "ffprobe" {
        return Err("Only ffmpeg and ffprobe can be probed.".to_string());
    }

    #[cfg(target_os = "android")]
    {
        let app = state.app_handle().clone();
        let tool = name.clone();
        let status = tokio::task::spawn_blocking(move || {
            let status = crate::mobile::ytdlp_bridge::ensure_media_tools(&app)?;
            let version = if tool == "ffmpeg" {
                status.ffmpeg_version.clone()
            } else {
                status.ffprobe_version.clone()
            };
            let path = if tool == "ffmpeg" {
                status.ffmpeg_path.clone()
            } else {
                status.ffprobe_path.clone()
            };
            if version.is_empty() && path.is_empty() {
                return Ok(crate::models::SidecarProbe {
                    name: tool,
                    bundled: false,
                    detail: format!("not found after FFmpeg.init — {}", status.message),
                });
            }
            Ok(crate::models::SidecarProbe {
                name: tool,
                bundled: true,
                detail: if version.is_empty() { path } else { version },
            })
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e: crate::error::AppError| e.to_string())?;
        return Ok(status);
    }

    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_shell::ShellExt;
        let app = state.app_handle().clone();
        let bin = format!("binaries/{name}");
        if let Err(e) = app.shell().sidecar(&bin) {
            return Ok(crate::models::SidecarProbe {
                name: name.clone(),
                bundled: false,
                detail: format!("sidecar not bundled in this build: {e}"),
            });
        }
        let runner = crate::sites::yt_dlp::SidecarRunner::new(app);
        match runner.probe_version(&name).await {
            Ok(detail) => Ok(crate::models::SidecarProbe {
                name,
                bundled: true,
                detail,
            }),
            Err(e) => Ok(crate::models::SidecarProbe {
                name,
                bundled: true,
                detail: format!(
                    "bundled but failed to execute — {e}. \
                     If the message mentions a missing shared library, \
                     re-run bun run setup:binaries for a static desktop build."
                ),
            }),
        }
    }
}

/// List subdirectories of an absolute filesystem path for the in-app folder
/// picker (used on mobile, where the native dialog cannot pick directories).
#[tauri::command]
pub fn browse_dirs(
    state: State<'_, Arc<AppState>>,
    path: Option<String>,
) -> CmdResult<crate::models::DirBrowseResponse> {
    map_err(state.browse_dirs(path.as_deref()))
}

/// Resolve the default on-device download folder (`<app-data>/downloads`,
/// creating it if needed). Used by the Settings folder picker "Reset" action.
#[tauri::command]
pub fn default_library_dir(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    let dir = state.data_dir.join("downloads");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

/// Update the embedded yt-dlp engine on Android (youtubedl-android downloads
/// the latest build into app storage). Desktop bundles yt-dlp as a sidecar.
#[tauri::command]
pub async fn update_yt_dlp(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    #[cfg(target_os = "android")]
    {
        let app = state.app_handle().clone();
        // The Kotlin bridge downloads tens of MB synchronously over JNI —
        // keep it off the async runtime so other commands keep flowing.
        let task = tokio::task::spawn_blocking(move || crate::mobile::ytdlp_bridge::update(&app));
        match tokio::time::timeout(std::time::Duration::from_secs(600), task).await {
            Ok(Ok(Ok(msg))) => Ok(msg),
            Ok(Ok(Err(e))) => Err(e.to_string()),
            Ok(Err(join_err)) => Err(format!("yt-dlp update task failed: {join_err}")),
            Err(_) => Err(
                "yt-dlp update timed out after 10 minutes. Check connectivity and retry."
                    .to_string(),
            ),
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        Err(
            "yt-dlp is bundled with the desktop app — update the app to get a newer engine."
                .to_string(),
        )
    }
}

/// Check GitHub for the latest release versions of yt-dlp and gallery-dl. (#69)
#[tauri::command]
pub async fn check_binary_updates(
    state: State<'_, Arc<AppState>>,
) -> CmdResult<crate::models::BinaryLatestVersions> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;
    Ok(installer.check_latest_versions().await)
}

/// Update a desktop binary (yt-dlp or gallery-dl) with rollback support. (#69)
#[tauri::command]
pub async fn update_binary(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> CmdResult<crate::models::BinaryUpdateResult> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;
    installer
        .update_binary(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Rollback a binary to its `.bak` backup, if one exists. (#69)
#[tauri::command]
pub async fn rollback_binary(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> CmdResult<crate::models::BinaryUpdateResult> {
    let installer = crate::mobile::binary_installer::BinaryInstaller::new(
        state.app_handle().clone(),
        state.data_dir.clone(),
    )
    .map_err(|e| e.to_string())?;
    installer
        .rollback_binary(&name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> CmdResult<Vec<Collection>> {
    state.list_collections().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_collection(
    state: State<'_, AppState>,
    req: CreateCollectionRequest,
) -> CmdResult<String> {
    state.create_collection(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.delete_collection(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    req: UpdateCollectionRequest,
) -> CmdResult<()> {
    state.update_collection(&id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_scene_to_collection(
    state: State<'_, AppState>,
    scene_id: String,
    collection_id: String,
) -> CmdResult<()> {
    state
        .add_scene_to_collection(&scene_id, &collection_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_scene_from_collection(
    state: State<'_, AppState>,
    scene_id: String,
    collection_id: String,
) -> CmdResult<()> {
    state
        .remove_scene_from_collection(&scene_id, &collection_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_collection_scenes(
    state: State<'_, AppState>,
    collection_id: String,
) -> CmdResult<Vec<Scene>> {
    state
        .list_collection_scenes(&collection_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scene_collection_ids(
    state: State<'_, AppState>,
    scene_id: String,
) -> CmdResult<Vec<String>> {
    state
        .scene_collection_ids(&scene_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_collection(
    state: State<'_, AppState>,
    collection_id: String,
    req: ExportCollectionRequest,
) -> CmdResult<ExportCollectionResult> {
    let scenes = state
        .list_collection_scenes(&collection_id)
        .map_err(|e| e.to_string())?;
    let collection = state
        .list_collections()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|c| c.id == collection_id)
        .ok_or("Collection not found")?;

    let format = req.format.to_lowercase();
    if format != "m3u" {
        return Err("Unsupported format. Only 'm3u' is supported.".into());
    }

    let mut m3u = String::new();
    m3u.push_str("#EXTM3U\n");

    for scene in scenes {
        let title = scene.title.replace('\n', " ").replace('\r', "");
        let duration = scene.duration.unwrap_or(0);
        let path = scene.path.unwrap_or_default();
        m3u.push_str(&format!("#EXTINF:{},{}", duration, title));
        m3u.push('\n');
        m3u.push_str(&path);
        m3u.push('\n');
    }

    let filename = format!(
        "{}.m3u",
        collection.name.replace('/', "_").replace('\\', "_")
    );

    Ok(ExportCollectionResult {
        content: m3u,
        filename,
    })
}

/// #73: Load a demo scene for empty-state testing in the first-run wizard.
/// Downloads a small, known-good test video and imports it via the normal pipeline.
#[tauri::command]
pub async fn load_demo_scene(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    // Use a small, reliable public-domain test video (Big Buck Bunny, 720p).
    // This exercises the full download → import → thumbnail pipeline.
    const DEMO_URL: &str =
        "https://download.blender.org/mirror/BigBuckBunny_Bbik-sequence_720p_24fps_1537KB.mp4";
    const DEMO_TITLE: &str = "Big Buck Bunny (Demo Scene)";

    let job = state
        .queue_download(DEMO_URL, Some("generic_ytdlp"), Some(DEMO_TITLE))
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Demo scene queued (job {}). It will download and appear in your library shortly.",
        job.id
    ))
}
