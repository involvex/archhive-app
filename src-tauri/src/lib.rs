mod commands;
mod db;
mod discovery;
mod downloads;
mod error;
mod library;
mod media;
mod mobile;
mod models;
mod server;
mod sites;
mod state;
mod vault;

use db::Database;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

/// Mobile-only defaults (compiled into Android/iOS builds via `cfg(mobile)`).
#[cfg_attr(not(mobile), allow(dead_code))]
fn bootstrap_mobile_settings(db: &Database, data_dir: &std::path::Path) -> Result<(), String> {
    let mut settings = db.get_settings().unwrap_or_default();
    let mut changed = false;
    if settings.library_path.is_empty() {
        let downloads = data_dir.join("downloads");
        std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
        settings.library_path = downloads.to_string_lossy().to_string();
        changed = true;
    }
    if settings.engine_mode == crate::models::EngineMode::Local {
        settings.engine_mode = crate::models::EngineMode::RemoteLan;
        changed = true;
    }
    if settings.remote_token.as_ref().is_some_and(|t| t.trim().is_empty()) {
        settings.remote_token = None;
        changed = true;
    }
    if changed {
        db.save_settings(&settings).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let db = Arc::new(Database::new(data_dir.clone()).map_err(|e| e.to_string())?);

            #[cfg(mobile)]
            bootstrap_mobile_settings(&db, &data_dir)?;

            let static_ui = std::env::current_dir()
                .ok()
                .map(|p| p.join("dist"))
                .filter(|p| p.join("index.html").exists());
            let state = Arc::new(
                AppState::with_app(db, data_dir, app.handle().clone(), static_ui)
                    .map_err(|e| e.to_string())?,
            );
            app.manage(state.clone());

            #[cfg(not(mobile))]
            {
                let auto_lan = std::env::var("ARCHIVE_AUTO_LAN").ok().as_deref() == Some("1");
                let lan_enabled = state.get_settings().map(|s| s.lan_enabled).unwrap_or(false);
                if auto_lan || lan_enabled {
                    if auto_lan {
                        if let Ok(mut s) = state.get_settings() {
                            s.lan_token = None;
                            let _ = state.save_settings(&s);
                        }
                    }
                    let st = state.clone();
                    tauri::async_runtime::spawn(async move {
                        let port = st.get_settings().map(|s| s.lan_port).unwrap_or(8787);
                        if let Err(e) = st.ensure_lan_server(port).await {
                            eprintln!("LAN auto-start failed: {e}");
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::list_sites,
            commands::browse,
            commands::queue_download,
            commands::list_downloads,
            commands::cancel_download,
            commands::list_scenes,
            commands::list_performers,
            commands::list_tags,
            commands::get_settings,
            commands::save_settings,
            commands::scan_library,
            commands::find_duplicates,
            commands::merge_duplicates,
            commands::list_cookie_sites,
            commands::save_site_cookies,
            commands::delete_site_cookies,
            commands::resolve_standalone,
            commands::discover_lan_hosts,
            commands::start_lan_server,
            commands::stop_lan_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
