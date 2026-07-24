use crate::error::AppResult;
use crate::models::{
    AppSettings, BatchUpdateScenesRequest, BatchUpdateScenesResult, BrowseKind, BrowseOrientation,
    DownloadJob, DuplicateGroup, HealthResponse, LanHost, MediaItem, MergeDuplicatesResult,
    Performer, PornhubCategoryEntry, ScanResult, Scene, SceneSort, SiteInfo, Tag,
    UpdateSceneRequest,
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
pub fn list_sites(state: State<'_, Arc<AppState>>) -> Vec<SiteInfo> {
    tauri::async_runtime::block_on(state.list_sites())
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
pub fn ensure_performer(
    state: State<'_, Arc<AppState>>,
    name: String,
) -> CmdResult<Performer> {
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
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> CmdResult<()> {
    let prev = state.get_settings().ok();
    map_err(state.save_settings(&settings))?;
    #[cfg(not(mobile))]
    if prev
        .as_ref()
        .is_none_or(|p| crate::desktop::tray_settings_changed(p, &settings))
    {
        crate::desktop::sync_from_settings(&app, &settings);
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
            let rules = vec![r"(?<performer>[a-zA-Z0-9_]+)-\d+".to_string()];
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
) -> CmdResult<u32> {
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
