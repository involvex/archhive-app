//! Android foreground keep-alive while LAN sharing (0.0.0.0) is on.
//! Separate from the download keep-alive so stopping share does not kill
//! an active download notification (and vice versa).

use tauri::AppHandle;

/// Best-effort Android keep-alive for LAN hosting (no-op off Android).
pub fn start_keep_alive(app: &AppHandle, title: &str, text: &str) {
    crate::mobile::ytdlp_bridge::start_lan_share_keep_alive(app, title, text);
}

pub fn stop_keep_alive(app: &AppHandle) {
    crate::mobile::ytdlp_bridge::stop_lan_share_keep_alive(app);
}
