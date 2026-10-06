//! Share-target bridge for Android (ACTION_SEND / ACTION_SEND_MULTIPLE).
//!
//! The Kotlin `ShareIntentPlugin` (installed by `scripts/patch-android-share.ps1`)
//! stashes shared text/plain in a static slot. The frontend pulls it via the
//! `get_pending_share` command (take + clear), so cold-start shares that arrive
//! before the webview is ready are not lost.
//!
//! Pure pull model: no JS event emission from Kotlin. `AppShell` polls on boot,
//! on window focus, and on an interval while on mobile.

use crate::error::{AppError, AppResult};
use serde::Deserialize;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

const PLUGIN_UNAVAILABLE: &str =
    "Share plugin unavailable (ShareIntentPlugin not registered). Re-run bun run android:patches and rebuild the APK.";

pub struct SharePluginState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: Option<PluginHandle<R>>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShareResponse {
    text: Option<String>,
    #[allow(dead_code)]
    ok: Option<bool>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("share")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                match api.register_android_plugin("com.archhive.app", "ShareIntentPlugin") {
                    Ok(handle) => {
                        app.manage(SharePluginState {
                            handle: Some(handle),
                        });
                    }
                    Err(e) => {
                        eprintln!("[share] Failed to register ShareIntentPlugin: {e}");
                        tracing::warn!(
                            "Share plugin registration failed (shares will be ignored): {e}"
                        );
                        app.manage(SharePluginState::<R> { handle: None });
                    }
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}

/// Take + clear the pending shared text. `Ok(None)` when nothing was shared.
pub fn take_pending(app: &AppHandle) -> AppResult<Option<String>> {
    #[cfg(target_os = "android")]
    {
        let state = app
            .try_state::<SharePluginState<tauri::Wry>>()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let handle = state
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let response: ShareResponse = handle
            .run_mobile_plugin("getSharedText", serde_json::json!({}))
            .map_err(|e| AppError::Download(format!("share plugin invoke failed: {e}")))?;
        let text = response.text.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_unavailable_message_mentions_patch_step() {
        assert!(PLUGIN_UNAVAILABLE.contains("android:patches"));
    }
}
