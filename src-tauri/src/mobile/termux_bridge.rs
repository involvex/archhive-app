//! Termux bridge (RUN_COMMAND-Intent) for the user's custom Termux fork.
//!
//! The Kotlin `TermuxBridgePlugin` (installed by `scripts/patch-android-cookie.ps1`)
//! forwards explicit intents to the fork's `RunCommandService` so ArcHive can hand the
//! decrypted Netscape cookie jar to Termux's home for the user's manual `yt-dlp`
//! workflow. The fork's constants are hardcoded in the Kotlin overlay (runtime package
//! `com.involvex.termux_app`, Java namespace `com.invapp`, action prefix
//! `com.involvex.termux_app.RUN_COMMAND*`).
//!
//! Android only: on desktop both functions return a clear error / empty status.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

const PLUGIN_UNAVAILABLE: &str = "Termux bridge unavailable (TermuxBridgePlugin not registered). Re-run bun run android:patches and rebuild the APK.";

/// Runtime package of the user's custom Termux fork (mirrors the Kotlin overlay).
pub const TERMUX_PACKAGE: &str = "com.involvex.termux_app";

/// Home dir of the fork, used as the default working directory.
pub const TERMUX_HOME: &str = "/data/data/com.involvex.termux_app/files/home";

/// Termux `sh` executable (prefix lives in the fork's private data dir).
pub const TERMUX_SH: &str = "/data/data/com.involvex.termux_app/files/usr/bin/sh";

pub struct TermuxBridgeState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: Option<PluginHandle<R>>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

/// Availability of the Termux bridge.
#[derive(Debug, Clone, Serialize)]
pub struct TermuxStatus {
    pub installed: bool,
    pub permission_granted: bool,
    pub available: bool,
    pub message: String,
}

/// A command to run inside Termux via the RUN_COMMAND intent.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // fields are read by the Android branch only
pub struct RunCommand {
    pub executable: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TermuxStatusResponse {
    installed: Option<bool>,
    permission_granted: Option<bool>,
    available: Option<bool>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TermuxOkResponse {
    ok: Option<bool>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("termuxBridge")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                match api.register_android_plugin("com.archhive.app", "TermuxBridgePlugin") {
                    Ok(handle) => {
                        app.manage(TermuxBridgeState {
                            handle: Some(handle),
                        });
                    }
                    Err(e) => {
                        eprintln!("[termuxBridge] Failed to register TermuxBridgePlugin: {e}");
                        tracing::warn!("Termux bridge registration failed: {e}");
                        app.manage(TermuxBridgeState::<R> { handle: None });
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

/// Query whether the Termux fork is installed and ArcHive may run commands in it.
pub fn status(app: &AppHandle) -> AppResult<TermuxStatus> {
    #[cfg(target_os = "android")]
    {
        let state = app
            .try_state::<TermuxBridgeState<tauri::Wry>>()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let handle = state
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let response: TermuxStatusResponse = handle
            .run_mobile_plugin("status", serde_json::json!({}))
            .map_err(|e| AppError::Download(format!("termux status failed: {e}")))?;
        Ok(TermuxStatus {
            installed: response.installed.unwrap_or(false),
            permission_granted: response.permission_granted.unwrap_or(false),
            available: response.available.unwrap_or(false),
            message: response.message.unwrap_or_default(),
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(TermuxStatus {
            installed: false,
            permission_granted: false,
            available: false,
            message: "Termux bridge is Android-only.".into(),
        })
    }
}

/// Run one command in Termux via RUN_COMMAND (app-shell runner, headless).
pub fn send_run_command(app: &AppHandle, cmd: &RunCommand) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        let state = app
            .try_state::<TermuxBridgeState<tauri::Wry>>()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let handle = state
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let response: TermuxOkResponse = handle
            .run_mobile_plugin(
                "sendRunCommand",
                serde_json::json!({
                    "executable": cmd.executable,
                    "arguments": cmd.arguments,
                    "stdin": cmd.stdin,
                    "workdir": cmd.workdir,
                    "label": cmd.label,
                }),
            )
            .map_err(|e| AppError::Download(format!("termux RUN_COMMAND failed: {e}")))?;
        if response.ok.unwrap_or(false) {
            Ok(())
        } else {
            Err(AppError::Download("termux RUN_COMMAND rejected".into()))
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, cmd);
        Err(AppError::Download("Termux bridge is Android-only".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termux_constants_match_the_custom_fork() {
        assert_eq!(TERMUX_PACKAGE, "com.involvex.termux_app");
        assert!(TERMUX_SH.starts_with("/data/data/com.involvex.termux_app/"));
        assert!(TERMUX_HOME.starts_with("/data/data/com.involvex.termux_app/"));
    }

    #[test]
    fn run_command_deserializes_from_camel_case_payload() {
        let cmd: RunCommand = serde_json::from_value(serde_json::json!({
            "executable": TERMUX_SH,
            "arguments": ["-c", "echo hi"],
            "stdin": "cookie-line",
            "workdir": TERMUX_HOME,
            "label": "ArcHive"
        }))
        .expect("payload should deserialize");
        assert_eq!(cmd.arguments.len(), 2);
        assert_eq!(cmd.stdin.as_deref(), Some("cookie-line"));
    }

    #[test]
    fn run_command_defaults_optional_fields() {
        let cmd: RunCommand = serde_json::from_value(serde_json::json!({
            "executable": TERMUX_SH
        }))
        .expect("minimal payload should deserialize");
        assert!(cmd.arguments.is_empty());
        assert!(cmd.stdin.is_none());
        assert!(cmd.workdir.is_none());
    }
}
