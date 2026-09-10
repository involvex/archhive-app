use crate::error::AppResult;
use crate::models::{
    AppSettings, BatchUpdateScenesRequest, BatchUpdateScenesResult, BrowseKind, BrowseOrientation,
    CheckSavedSearchResult, DownloadJob, DuplicateGroup, FfmpegStatus, HealthResponse, LanHost,
    LibraryStats, MarkWatchedRequest, MarkWatchedResult, MediaItem, MergeDuplicatesResult,
    OrphanSidecar, Performer, PornhubCategoryEntry, SaveSearchRequest, SavedSearch, ScanResult,
    Scene, SceneFilter, SceneSort, SiteInfo, Tag, UpdateSavedSearchRequest, UpdateSceneRequest,
    WatchProgress, WatchlistPollResult, WatchlistStatus,
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
pub async fn queue_download(
    state: State<'_, Arc<AppState>>,
    url: String,
    adapter: Option<String>,
) -> CmdResult<DownloadJob> {
    map_err(state.queue_download(&url, adapter.as_deref()).await)
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
) -> CmdResult<Vec<Scene>> {
    map_err(state.list_scenes(query.as_deref(), sort.unwrap_or_default()))
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
    let stream_url = state
        .resolve_stream_url(&url)
        .await
        .map_err(|e| e.to_string())?;
    let embed_url = crate::sites::urls::derive_embed_url(&url);
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
) -> CmdResult<Vec<Scene>> {
    map_err(state.list_scenes_with_filter(&filter))
}

#[tauri::command]
pub fn list_orphan_sidecars(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<OrphanSidecar>> {
    map_err(state.list_orphan_sidecars())
}

#[tauri::command]
pub fn delete_orphan_sidecar(path: String) -> CmdResult<()> {
    let p = std::path::Path::new(&path);
    if !p.is_file() {
        return Err(format!("File not found: {path}"));
    }
    std::fs::remove_file(p).map_err(|e| e.to_string())
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
    use tauri_plugin_shell::ShellExt;
    if name != "ffmpeg" && name != "ffprobe" {
        return Err("Only ffmpeg and ffprobe can be probed.".to_string());
    }
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
    match runner.tool_version(&name).await {
        Some(detail) => Ok(crate::models::SidecarProbe {
            name,
            bundled: true,
            detail,
        }),
        None => {
            let detail =
                format!("bundled but failed to execute — check logcat for 'sidecar {name}' errors");
            Ok(crate::models::SidecarProbe {
                name,
                bundled: true,
                detail,
            })
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
