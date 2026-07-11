use crate::error::AppResult;
use crate::models::{
    AppSettings, BrowseKind, DownloadJob, DuplicateGroup, HealthResponse, MediaItem,
    MergeDuplicatesResult, Performer, Scene, ScanResult, SiteInfo, Tag,
};
use crate::server::{generate_token, LanServer};
use crate::state::AppState;
use crate::vault::CookieSiteInfo;
use std::sync::Arc;
use tauri::State;

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
) -> CmdResult<crate::models::BrowsePage> {
    map_err(
        state
            .browse(&site_id, kind, &slug, page.unwrap_or(1))
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
pub fn list_downloads(state: State<'_, Arc<AppState>>) -> CmdResult<Vec<DownloadJob>> {
    map_err(state.list_downloads())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, Arc<AppState>>, id: String) -> CmdResult<()> {
    map_err(state.cancel_download(&id))
}

#[tauri::command]
pub fn list_scenes(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
) -> CmdResult<Vec<Scene>> {
    map_err(state.list_scenes(query.as_deref()))
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
pub fn save_settings(state: State<'_, Arc<AppState>>, settings: AppSettings) -> CmdResult<()> {
    map_err(state.save_settings(&settings))
}

#[tauri::command]
pub fn scan_library(state: State<'_, Arc<AppState>>) -> CmdResult<ScanResult> {
    map_err(state.scan_library())
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
pub async fn start_lan_server(
    state: State<'_, Arc<AppState>>,
    port: u16,
) -> CmdResult<serde_json::Value> {
    if state.lan_server.lock().is_some() {
        let token = state
            .get_settings()
            .ok()
            .and_then(|s| s.lan_token)
            .unwrap_or_default();
        return Ok(serde_json::json!({ "token": token }));
    }
    let token = generate_token();
    let app_state = state.inner().clone();
    let static_dir = app_state.static_ui_path();
    let server = map_err(LanServer::start(app_state, port, token.clone(), static_dir).await)?;
    *state.lan_server.lock() = Some(server);
    let mut settings = map_err(state.get_settings())?;
    settings.lan_enabled = true;
    settings.lan_port = port;
    settings.lan_token = Some(token.clone());
    map_err(state.save_settings(&settings))?;
    Ok(serde_json::json!({ "token": token, "port": port }))
}

#[tauri::command]
pub fn stop_lan_server(state: State<'_, Arc<AppState>>) -> CmdResult<()> {
    let mut guard = state.lan_server.lock();
    if let Some(mut server) = guard.take() {
        server.stop();
    }
    let mut settings = map_err(state.get_settings())?;
    settings.lan_enabled = false;
    map_err(state.save_settings(&settings))
}
