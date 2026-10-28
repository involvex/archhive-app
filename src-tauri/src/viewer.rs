//! In-app viewer: secondary native webview windows.
//!
//! Opens external sites in a top-level webview instead of an `<iframe>`.
//! Top-level loads ignore `X-Frame-Options` / CSP `frame-ancestors`, so sites
//! that block embedding (CamsFinder, and most cam sites) render with their own
//! cookies, age gates and player — the "iframe of the website" users expect,
//! implemented as a real window.
//!
//! Android requires the Activity Embedding setup applied by
//! `scripts/patch-android-multiwindow.ps1` (ViewerActivity + split rules).
//! Desktop gets a regular secondary window.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Open `url` in a new native webview window. Re-tapping a URL that already
/// has a viewer focuses the existing window instead of failing on the
/// duplicate label. Returns an error message when the window cannot be created
/// (e.g. Android < 12L without multi-window support) so the frontend can fall
/// back to the external browser.
pub fn open_viewer(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid url '{url}': {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("unsupported url scheme '{}'", parsed.scheme()));
    }

    // Stable label per URL: dedupes re-taps, unique per room otherwise.
    let mut hasher = DefaultHasher::new();
    parsed.as_str().hash(&mut hasher);
    let label = format!("viewer-{:016x}", hasher.finish());

    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    let host = parsed.host_str().unwrap_or("page").to_string();
    // Use the main window as the manager so Tauri derives the window
    // relationship (Android activity stack) automatically.
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let builder = WebviewWindowBuilder::new(&main, label, WebviewUrl::External(parsed))
        .title(format!("ArcHive — {host}"));
    #[cfg(target_os = "android")]
    let builder = builder.activity_name("ViewerActivity");

    builder
        .build()
        .map_err(|e| format!("failed to open in-app viewer: {e}"))?;
    Ok(())
}
