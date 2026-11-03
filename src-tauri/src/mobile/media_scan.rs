//! Android MediaStore bridge.
//!
//! Vaulting *moves* a file out of the shared volume. Android's `MediaStore`
//! keeps the old row until something tells it to rescan that path, which is why
//! moved files can linger in gallery apps. The Kotlin `MediaScanPlugin`
//! (installed by `scripts/patch-android-vault.ps1`) calls
//! `MediaScannerConnection.scanFile`, which drops a stale row for a missing
//! path and indexes a new one.
//!
//! `delete_paths` asks the plugin to remove leftovers via ContentResolver and,
//! on API 30+, `MediaStore.createDeleteRequest` (user consent dialog).
//!
//! No-op on every other platform.

use serde::{Deserialize, Serialize};
#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;
use tauri::plugin::{Builder, TauriPlugin};
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{AppHandle, Runtime};

pub struct MediaScanState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: Option<PluginHandle<R>>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanPayload {
    paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeletePayload {
    paths: Vec<String>,
}

/// Result of a MediaStore / filesystem delete attempt on Android.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub deleted: bool,
    pub pending_user_consent: bool,
    #[serde(default)]
    pub leftover_paths: Vec<String>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("media-scan")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                match api.register_android_plugin("com.archhive.app", "MediaScanPlugin") {
                    Ok(handle) => {
                        app.manage(MediaScanState {
                            handle: Some(handle),
                        });
                    }
                    Err(e) => {
                        tracing::warn!("[media-scan] registration failed: {e}");
                        app.manage(MediaScanState::<R> { handle: None });
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

/// Ask MediaStore to forget (`removed = true`) or index these absolute paths.
///
/// Best effort: a failure only means a gallery may keep a stale thumbnail.
pub fn rescan_paths<R: Runtime>(app: &AppHandle<R>, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    #[cfg(target_os = "android")]
    {
        let Some(state) = app.try_state::<MediaScanState<R>>() else {
            return;
        };
        let Some(handle) = state.handle.as_ref() else {
            return;
        };
        let payload = ScanPayload {
            paths: paths.to_vec(),
        };
        if let Err(e) = handle.run_mobile_plugin::<serde_json::Value>("scanFile", payload) {
            tracing::warn!("[media-scan] rescan failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, paths);
    }
}

/// Try to delete files via Android MediaStore as a fallback when direct deletion fails.
///
/// Returns typed status so callers can surface “approve system delete” when
/// `pending_user_consent` is true.
pub fn delete_paths<R: Runtime>(app: &AppHandle<R>, paths: &[String]) -> DeleteResult {
    if paths.is_empty() {
        return DeleteResult {
            deleted: true,
            pending_user_consent: false,
            leftover_paths: Vec::new(),
        };
    }
    #[cfg(target_os = "android")]
    {
        let Some(state) = app.try_state::<MediaScanState<R>>() else {
            return DeleteResult {
                deleted: false,
                pending_user_consent: false,
                leftover_paths: paths.to_vec(),
            };
        };
        let Some(handle) = state.handle.as_ref() else {
            return DeleteResult {
                deleted: false,
                pending_user_consent: false,
                leftover_paths: paths.to_vec(),
            };
        };
        let payload = DeletePayload {
            paths: paths.to_vec(),
        };
        match handle.run_mobile_plugin::<DeleteResult>("deleteFile", payload) {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!("[media-scan] delete failed: {e}");
                DeleteResult {
                    deleted: false,
                    pending_user_consent: false,
                    leftover_paths: paths.to_vec(),
                }
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, paths);
        DeleteResult {
            deleted: true,
            pending_user_consent: false,
            leftover_paths: Vec::new(),
        }
    }
}
