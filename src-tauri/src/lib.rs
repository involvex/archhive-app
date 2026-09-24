mod commands;
mod db;
mod discovery;
mod downloads;
mod error;
mod library;
mod log_buffer;
mod media;
mod mobile;
mod models;
mod server;
mod sites;
mod state;
mod vault;

#[cfg(not(mobile))]
mod desktop;

fn init_log_subscriber() {
    let log_buffer = log_buffer::LogBuffer::instance();
    let make_writer = move || LogWriter::new(log_buffer);

    // try_init: avoid panic if a global subscriber is already registered
    let _ = tracing_subscriber::fmt()
        .with_writer(make_writer)
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

struct LogWriter {
    buffer: &'static log_buffer::LogBuffer,
}

impl LogWriter {
    fn new(buffer: &'static log_buffer::LogBuffer) -> Self {
        Self { buffer }
    }
}

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).trim_end().to_string();
        if !msg.is_empty() {
            self.buffer.push("INFO", "app", msg);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = LogWriter;
    fn make_writer(&'a self) -> Self::Writer {
        LogWriter::new(self.buffer)
    }
}

use crate::models::EngineMode;
use db::Database;
use state::AppState;
use std::sync::Arc;
use tauri::path::BaseDirectory;
use tauri::Manager;

fn resolve_lan_static_ui(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let dev = cwd.join("dist");
        if dev.join("index.html").exists() {
            return Some(dev);
        }
    }
    if let Ok(resource) = app.path().resolve("lan-ui", BaseDirectory::Resource) {
        if resource.join("index.html").exists() {
            return Some(resource);
        }
    }
    None
}

/// Mobile-only defaults (compiled into Android/iOS builds via `cfg(mobile)`).
#[cfg_attr(not(mobile), allow(dead_code))]
fn bootstrap_mobile_settings(db: &Database, data_dir: &std::path::Path) -> Result<(), String> {
    let mut settings = db.get_settings().unwrap_or_default();
    let mut changed = false;
    // Only fall back to Local when Remote LAN has no host configured.
    // Previously this forced Local on every launch and wiped intentional Remote LAN.
    let has_remote_host = settings
        .remote_host
        .as_ref()
        .is_some_and(|h| !h.trim().is_empty());
    if settings.engine_mode == EngineMode::RemoteLan && !has_remote_host {
        settings.engine_mode = EngineMode::Local;
        changed = true;
    }
    if settings.library_path.is_empty() {
        let downloads = data_dir.join("downloads");
        std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
        settings.library_path = downloads.to_string_lossy().to_string();
        changed = true;
    }
    if settings
        .remote_token
        .as_ref()
        .is_some_and(|t| t.trim().is_empty())
    {
        settings.remote_token = None;
        changed = true;
    }
    if changed {
        db.save_settings(&settings).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// On Android, initialize youtubedl-android FFmpeg so native ffmpeg/ffprobe are
/// unpacked from libffmpeg.zip.so. Also remove wrongly-installed Linux binaries
/// from the binary installer (x86_64 / glibc) that cannot run on device.
#[cfg(target_os = "android")]
fn ensure_sidecar_permissions(app: &tauri::AppHandle) {
    // Remove any wrongly-installed x86_64 / Linux binaries from the binary installer
    if let Ok(data_dir) = app.path().app_data_dir() {
        let bin_dir = data_dir.join("bin");
        for name in &["yt-dlp", "gallery-dl", "ffmpeg", "ffprobe"] {
            let path = bin_dir.join(name);
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    // FFmpeg.init unpacks ~35MB — do it off the setup thread.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        match crate::mobile::ytdlp_bridge::ensure_media_tools(&app) {
            Ok(status) => {
                eprintln!(
                    "[sidecar] media tools ok={} ffmpeg={} ffprobe={} ({})",
                    status.ok,
                    if status.ffmpeg_version.is_empty() {
                        status.ffmpeg_path.as_str()
                    } else {
                        status.ffmpeg_version.as_str()
                    },
                    if status.ffprobe_version.is_empty() {
                        status.ffprobe_path.as_str()
                    } else {
                        status.ffprobe_version.as_str()
                    },
                    status.message
                );
            }
            Err(e) => {
                eprintln!("[sidecar] ensure_media_tools failed: {e}");
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_log_subscriber();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init());

    #[cfg(not(mobile))]
    let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

    #[cfg(mobile)]
    let builder = builder.plugin(mobile::ytdlp_bridge::init());

    let builder = builder.setup(|app| {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let db = Arc::new(Database::new(data_dir.clone()).map_err(|e| e.to_string())?);

        #[cfg(mobile)]
        {
            bootstrap_mobile_settings(&db, &data_dir)?;
            ensure_sidecar_permissions(app.handle());
            // #55: Android notification channels for downloads.
            downloads::manager::register_download_channels(app.handle());
        }

        let static_ui = resolve_lan_static_ui(app.handle());
        let state = Arc::new(
            AppState::with_app(db, data_dir, app.handle().clone(), static_ui)
                .map_err(|e| e.to_string())?,
        );
        app.manage(state.clone());

        #[cfg(mobile)]
        {
            let st = state.clone();
            let mobile_port = state.get_settings().map(|s| s.lan_port).unwrap_or(8787);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = st.ensure_loopback_server(mobile_port).await {
                    tracing::warn!("Loopback server start failed: {e}");
                }
            });
        }

        // #19 watchlist auto-queue poller (desktop only).
        #[cfg(not(mobile))]
        state.spawn_watchlist_poller();

        #[cfg(not(mobile))]
        {
            app.manage(Arc::new(desktop::TrayHotkeyState::new()));
            desktop::setup(app.handle())?;
            if let Ok(settings) = state.get_settings() {
                desktop::sync_from_settings(app.handle(), &settings);
            }
        }

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
                        tracing::warn!("LAN auto-start failed: {e}");
                    }
                });
            }
        }

        Ok(())
    });

    #[cfg(not(mobile))]
    let builder = builder.on_window_event(|window, event| {
        desktop::on_window_event(window, event);
    });

    builder
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::list_sites,
            commands::browse,
            commands::put_browse_cache,
            commands::get_browse_cache,
            commands::queue_download,
            commands::queue_downloads,
            commands::list_downloads,
            commands::cancel_download,
            commands::pause_download,
            commands::resume_download,
            commands::retry_download,
            commands::delete_download,
            commands::update_network_state,
            commands::get_network_info,
            commands::queue_bulk_import,
            commands::list_scenes,
            commands::delete_scene,
            commands::ensure_performer,
            commands::list_performers,
            commands::list_tags,
            commands::get_settings,
            commands::save_settings,
            commands::scan_library,
            commands::generate_missing_thumbs,
            commands::find_duplicates,
            commands::merge_duplicates,
            commands::list_cookie_sites,
            commands::save_site_cookies,
            commands::delete_site_cookies,
            commands::export_settings_backup,
            commands::import_settings_backup,
            commands::resolve_standalone,
            commands::resolve_media_details,
            commands::resolve_stream_url,
            commands::resolve_livestream,
            commands::discover_lan_hosts,
            commands::start_lan_server,
            commands::stop_lan_server,
            commands::regenerate_lan_server,
            commands::get_scene,
            commands::update_scene,
            commands::open_scene_in_explorer,
            commands::open_scene_with_default,
            commands::batch_update_scenes,
            commands::list_pornhub_categories,
            commands::probe_scene_metadata,
            commands::probe_library_durations,
            commands::ffmpeg_status,
            commands::list_scenes_with_filter,
            commands::list_orphan_sidecars,
            commands::thumb_cache_stats,
            commands::library_storage_stats,
            commands::delete_orphan_sidecar,
            commands::clear_scene_thumb,
            commands::clear_all_thumbs,
            commands::binary_versions,
            commands::get_library_stats,
            commands::get_diagnostics,
            commands::clear_logs,
            commands::get_recent_logs,
            commands::export_performers,
            commands::set_performer_image,
            commands::record_watch_progress,
            commands::get_watch_progress,
            commands::list_watch_progress,
            commands::mark_watched,
            commands::list_watched_source_urls,
            commands::save_search,
            commands::list_saved_searches,
            commands::delete_saved_search,
            commands::check_saved_search,
            commands::update_saved_search,
            commands::poll_watchlist,
            commands::watchlist_status,
            commands::dismiss_saved_search_news,
            commands::install_yt_dlp,
            commands::install_gallery_dl,
            commands::get_installed_binaries,
            commands::uninstall_binary,
            commands::list_files,
            commands::browse_dirs,
            commands::probe_sidecar,
            commands::default_library_dir,
            commands::update_yt_dlp,
            commands::check_binary_updates,
            commands::update_binary,
            commands::rollback_binary,
            commands::load_demo_scene,
            commands::list_collections,
            commands::create_collection,
            commands::delete_collection,
            commands::update_collection,
            commands::add_scene_to_collection,
            commands::remove_scene_from_collection,
            commands::list_collection_scenes,
            commands::scene_collection_ids,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("error while running tauri application: {e}");
            tracing::error!("error while running tauri application: {e}");
            // Mobile entry uses stop_unwind → abort on panic; prefer an explicit exit
            // so the message is visible in logcat before process death.
            std::process::exit(1);
        });
}
