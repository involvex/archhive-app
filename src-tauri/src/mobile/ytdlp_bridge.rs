//! Bridge to the youtubedl-android Kotlin plugin (Android only).
//!
//! The plugin handle is registered during app setup and stored in `YtDlpHandle` state.
//! Functions retrieve the handle from state and call the Kotlin plugin commands.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Serialize)]
pub struct YtDlpOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginResponse {
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<i32>,
    version: Option<String>,
    message: Option<String>,
}

/// Wrapper around the Tauri PluginHandle for the youtubedl-android plugin.
/// Stored in Tauri state during app setup.
pub struct YtDlpHandle {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle,
}

impl YtDlpHandle {
    #[cfg(target_os = "android")]
    pub fn new(handle: tauri::plugin::PluginHandle) -> Self {
        Self { handle }
    }

    #[cfg(not(target_os = "android"))]
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, args: &[String]) -> AppResult<YtDlpOutput> {
        #[cfg(target_os = "android")]
        {
            use serde_json::json;
            let payload = json!({ "args": args });
            let response: PluginResponse = self
                .handle
                .run_mobile_plugin("execute", payload)
                .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
            return Ok(YtDlpOutput {
                stdout: response.stdout.unwrap_or_default(),
                stderr: response.stderr.unwrap_or_default(),
                exit_code: response.exit_code.unwrap_or(-1),
            });
        }
        #[cfg(not(target_os = "android"))]
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }

    pub fn version(&self) -> AppResult<String> {
        #[cfg(target_os = "android")]
        {
            use serde_json::json;
            let payload = json!({});
            let response: PluginResponse = self
                .handle
                .run_mobile_plugin("version", payload)
                .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
            return Ok(response.version.unwrap_or_default());
        }
        #[cfg(not(target_os = "android"))]
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }

    pub fn update(&self) -> AppResult<String> {
        #[cfg(target_os = "android")]
        {
            use serde_json::json;
            let payload = json!({});
            let response: PluginResponse = self
                .handle
                .run_mobile_plugin("update", payload)
                .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
            return Ok(response.message.unwrap_or_default());
        }
        #[cfg(not(target_os = "android"))]
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }
}

/// Async wrapper that calls the plugin from async context.
pub async fn execute(app: &AppHandle, args: &[String]) -> AppResult<YtDlpOutput> {
    let handle = app
        .state::<Arc<YtDlpHandle>>();
    handle.execute(args)
}

pub async fn version(app: &AppHandle) -> AppResult<String> {
    let handle = app
        .state::<Arc<YtDlpHandle>>();
    handle.version()
}

pub async fn update(app: &AppHandle) -> AppResult<String> {
    let handle = app
        .state::<Arc<YtDlpHandle>>();
    handle.update()
}
