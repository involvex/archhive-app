//! Tauri plugin that wraps the youtubedl-android Kotlin plugin (Android only).
//!
//! On Android, the plugin registers `YtDlpPlugin` via JNI and exposes `run_mobile_plugin`
//! calls for execute / version / update. On non-Android platforms the managed state is
//! a stub that always returns an error.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Serialize)]
pub struct YtDlpOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // fields are populated via JNI deserialization on Android only
struct PluginResponse {
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<i32>,
    version: Option<String>,
    message: Option<String>,
}

/// Stored in Tauri managed state. Wraps the Kotlin plugin handle on Android.
pub struct YtDlpPluginState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    // Keeps the `R` parameter used on non-Android targets (avoids E0392).
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

impl<R: Runtime> YtDlpPluginState<R> {
    #[cfg(target_os = "android")]
    fn execute(&self, args: &[String]) -> AppResult<YtDlpOutput> {
        use serde_json::json;
        let payload = json!({ "args": args });
        let response: PluginResponse = self
            .handle
            .run_mobile_plugin("execute", payload)
            .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
        Ok(YtDlpOutput {
            stdout: response.stdout.unwrap_or_default(),
            stderr: response.stderr.unwrap_or_default(),
            exit_code: response.exit_code.unwrap_or(-1),
        })
    }

    #[cfg(target_os = "android")]
    fn version(&self) -> AppResult<String> {
        use serde_json::json;
        let response: PluginResponse = self
            .handle
            .run_mobile_plugin("version", json!({}))
            .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
        Ok(response.version.unwrap_or_default())
    }

    #[cfg(target_os = "android")]
    fn update(&self) -> AppResult<String> {
        use serde_json::json;
        let response: PluginResponse = self
            .handle
            .run_mobile_plugin("update", json!({}))
            .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
        Ok(response.message.unwrap_or_default())
    }
}

/// Build the Tauri plugin. Pass to `Builder::plugin(...)`.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("ytdlp")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api
                    .register_android_plugin("com.archhive.app", "YtDlpPlugin")
                    .map_err(|e| format!("Failed to register YtDlpPlugin: {e}"))?;
                app.manage(YtDlpPluginState { handle });
            }
            Ok(())
        })
        .build()
}

/// Execute yt-dlp via the Kotlin plugin.
pub fn execute(app: &AppHandle, args: &[String]) -> AppResult<YtDlpOutput> {
    #[cfg(target_os = "android")]
    {
        let state = app.state::<YtDlpPluginState<tauri::Wry>>();
        state.execute(args)
    }
    #[cfg(not(target_os = "android"))]
    Err(AppError::Download(
        "youtubedl-android is only available on Android".into(),
    ))
}

/// Query the embedded yt-dlp version.
pub fn version(app: &AppHandle) -> AppResult<String> {
    #[cfg(target_os = "android")]
    {
        let state = app.state::<YtDlpPluginState<tauri::Wry>>();
        state.version()
    }
    #[cfg(not(target_os = "android"))]
    Err(AppError::Download(
        "youtubedl-android is only available on Android".into(),
    ))
}

/// Trigger yt-dlp update via the Kotlin plugin.
pub fn update(app: &AppHandle) -> AppResult<String> {
    #[cfg(target_os = "android")]
    {
        let state = app.state::<YtDlpPluginState<tauri::Wry>>();
        state.update()
    }
    #[cfg(not(target_os = "android"))]
    Err(AppError::Download(
        "youtubedl-android is only available on Android".into(),
    ))
}
