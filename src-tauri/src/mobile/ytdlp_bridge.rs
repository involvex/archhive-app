//! Tauri plugin that wraps the youtubedl-android Kotlin plugin (Android only).
//!
//! On Android, the plugin registers `YtDlpPlugin` via JNI and exposes `run_mobile_plugin`
//! calls for execute / version / update / ffmpeg+ffprobe. Registration failures are soft:
//! the app still boots and download/media commands return a clear error.
//!
//! Media tools (ffmpeg/ffprobe) come from the youtubedl-android `ffmpeg` AAR
//! (`FFmpeg.init` unpacks Android/bionic binaries). They are NOT the Linux BtbN
//! sidecars under `src-tauri/binaries/*-linux-android`.

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
    ok: Option<bool>,
    ffmpeg_path: Option<String>,
    ffprobe_path: Option<String>,
    ffmpeg_version: Option<String>,
    ffprobe_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaToolsStatus {
    pub ok: bool,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
    pub message: String,
}

const PLUGIN_UNAVAILABLE: &str =
    "yt-dlp Android plugin unavailable (YtDlpPlugin not registered). Rebuild the APK or run android:regen with overlays.";

/// Stored in Tauri managed state. Wraps the Kotlin plugin handle on Android.
pub struct YtDlpPluginState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: Option<PluginHandle<R>>,
    // Keeps the `R` parameter used on non-Android targets (avoids E0392).
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

impl<R: Runtime> YtDlpPluginState<R> {
    #[cfg(target_os = "android")]
    fn handle(&self) -> AppResult<&PluginHandle<R>> {
        self.handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))
    }

    #[cfg(target_os = "android")]
    fn execute(&self, args: &[String]) -> AppResult<YtDlpOutput> {
        use serde_json::json;
        let payload = json!({ "args": args });
        let response: PluginResponse = self
            .handle()?
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
            .handle()?
            .run_mobile_plugin("version", json!({}))
            .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
        Ok(response.version.unwrap_or_default())
    }

    #[cfg(target_os = "android")]
    fn update(&self) -> AppResult<String> {
        use serde_json::json;
        let response: PluginResponse = self
            .handle()?
            .run_mobile_plugin("update", json!({}))
            .map_err(|e| AppError::Download(format!("ytdlp plugin invoke failed: {e}")))?;
        Ok(response.message.unwrap_or_default())
    }

    #[cfg(target_os = "android")]
    fn ensure_media_tools(&self) -> AppResult<MediaToolsStatus> {
        use serde_json::json;
        let response: PluginResponse = self
            .handle()?
            .run_mobile_plugin("ensureMediaTools", json!({}))
            .map_err(|e| AppError::Download(format!("ensureMediaTools failed: {e}")))?;
        Ok(MediaToolsStatus {
            ok: response.ok.unwrap_or(false),
            ffmpeg_path: response.ffmpeg_path.unwrap_or_default(),
            ffprobe_path: response.ffprobe_path.unwrap_or_default(),
            ffmpeg_version: response.ffmpeg_version.unwrap_or_default(),
            ffprobe_version: response.ffprobe_version.unwrap_or_default(),
            message: response.message.unwrap_or_default(),
        })
    }

    #[cfg(target_os = "android")]
    fn execute_media(&self, tool: &str, args: &[String]) -> AppResult<YtDlpOutput> {
        use serde_json::json;
        let payload = json!({ "tool": tool, "args": args });
        let response: PluginResponse = self
            .handle()?
            .run_mobile_plugin("executeMedia", payload)
            .map_err(|e| AppError::Download(format!("executeMedia({tool}) failed: {e}")))?;
        let out = YtDlpOutput {
            stdout: response.stdout.unwrap_or_default(),
            stderr: response.stderr.unwrap_or_default(),
            exit_code: response.exit_code.unwrap_or(-1),
        };
        if out.exit_code != 0 {
            let detail = if out.stderr.trim().is_empty() {
                out.stdout.trim().to_string()
            } else {
                out.stderr.trim().to_string()
            };
            return Err(AppError::Download(format!(
                "{tool} exited with code {}: {detail}",
                out.exit_code
            )));
        }
        Ok(out)
    }

    /// Run media tool and return combined stdout/stderr (ffmpeg often prints to stderr).
    #[cfg(target_os = "android")]
    fn execute_media_text(&self, tool: &str, args: &[String]) -> AppResult<String> {
        let out = self.execute_media(tool, args)?;
        let text = if out.stdout.trim().is_empty() {
            out.stderr
        } else if out.stderr.trim().is_empty() {
            out.stdout
        } else {
            format!("{}\n{}", out.stdout, out.stderr)
        };
        Ok(text)
    }

    /// Share a text file (settings backup) via the Android share sheet.
    #[cfg(target_os = "android")]
    fn share_file(&self, filename: &str, text: &str, mime: &str, title: &str) -> AppResult<()> {
        use serde_json::json;
        let payload = json!({ "filename": filename, "text": text, "mime": mime, "title": title });
        let response: PluginResponse =
            self.handle()?
                .run_mobile_plugin("shareTextFile", payload)
                .map_err(|e| AppError::Download(format!("shareTextFile failed: {e}")))?;
        if response.ok.unwrap_or(false) {
            Ok(())
        } else {
            Err(AppError::Download(
                response
                    .message
                    .unwrap_or_else(|| "shareTextFile failed".into()),
            ))
        }
    }
}

/// Build the Tauri plugin. Pass to `Builder::plugin(...)`.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("ytdlp")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                match api.register_android_plugin("com.archhive.app", "YtDlpPlugin") {
                    Ok(handle) => {
                        app.manage(YtDlpPluginState {
                            handle: Some(handle),
                        });
                    }
                    Err(e) => {
                        eprintln!(
                            "[ytdlp] Failed to register YtDlpPlugin (continuing without it): {e}"
                        );
                        tracing::error!(
                            "Failed to register YtDlpPlugin (app will boot; downloads unavailable): {e}"
                        );
                        app.manage(YtDlpPluginState::<R> { handle: None });
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

#[cfg(target_os = "android")]
fn with_plugin_state<T>(
    app: &AppHandle,
    f: impl FnOnce(&YtDlpPluginState<tauri::Wry>) -> AppResult<T>,
) -> AppResult<T> {
    let state = app
        .try_state::<YtDlpPluginState<tauri::Wry>>()
        .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
    f(&state)
}

/// Execute yt-dlp via the Kotlin plugin.
pub fn execute(app: &AppHandle, args: &[String]) -> AppResult<YtDlpOutput> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.execute(args))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, args);
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }
}

/// Query the embedded yt-dlp version.
pub fn version(app: &AppHandle) -> AppResult<String> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.version())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }
}

/// Trigger yt-dlp update via the Kotlin plugin.
pub fn update(app: &AppHandle) -> AppResult<String> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.update())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(AppError::Download(
            "youtubedl-android is only available on Android".into(),
        ))
    }
}

/// Share a text file (settings backup) via the Android share sheet.
pub fn share_text_file(
    app: &AppHandle,
    filename: &str,
    text: &str,
    mime: &str,
    title: &str,
) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.share_file(filename, text, mime, title))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, filename, text, mime, title);
        Err(AppError::Download(
            "File sharing is only available on Android".into(),
        ))
    }
}

/// Unpack/locate Android-native ffmpeg + ffprobe via youtubedl-android FFmpeg.init.
pub fn ensure_media_tools(app: &AppHandle) -> AppResult<MediaToolsStatus> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.ensure_media_tools())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(AppError::Download(
            "Android media tools are only available on Android".into(),
        ))
    }
}

/// Open a local file in another Android app (system chooser) through the
/// FileProvider; `file://` URIs are rejected across apps on Android 7+.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn open_file(app: &AppHandle, path: &str) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        with_plugin_state(app, |state| {
            let response: PluginResponse = state
                .handle()?
                .run_mobile_plugin("openFile", json!({ "path": path, "mime": "" }))
                .map_err(|e| AppError::Other(format!("openFile failed: {e}")))?;
            if response.ok.unwrap_or(false) {
                Ok(())
            } else {
                Err(AppError::Other(
                    response
                        .message
                        .unwrap_or_else(|| "no app can open this file".into()),
                ))
            }
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, path);
        Err(AppError::Other("open_file is Android-only".into()))
    }
}

/// Run ffmpeg or ffprobe through the Kotlin plugin (Android-native binaries).
/// Returns combined stdout/stderr text on success.
pub fn execute_media(app: &AppHandle, tool: &str, args: &[String]) -> AppResult<String> {
    #[cfg(target_os = "android")]
    {
        with_plugin_state(app, |state| state.execute_media_text(tool, args))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, tool, args);
        Err(AppError::Download(
            "Android media tools are only available on Android".into(),
        ))
    }
}

/// Best-effort Android keep-alive / WorkManager helpers (no-op off Android).
pub fn start_keep_alive(app: &AppHandle, title: &str, text: &str, progress: i32) {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        let _ = with_plugin_state(app, |state| {
            let _ = state.handle()?.run_mobile_plugin::<PluginResponse>(
                "startKeepAlive",
                json!({ "title": title, "text": text, "progress": progress }),
            );
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, title, text, progress);
    }
}

pub fn update_keep_alive(app: &AppHandle, title: &str, text: &str, progress: i32) {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        let _ = with_plugin_state(app, |state| {
            let _ = state.handle()?.run_mobile_plugin::<PluginResponse>(
                "updateKeepAlive",
                json!({ "title": title, "text": text, "progress": progress }),
            );
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, title, text, progress);
    }
}

pub fn stop_keep_alive(app: &AppHandle) {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        let _ = with_plugin_state(app, |state| {
            let _ = state
                .handle()?
                .run_mobile_plugin::<PluginResponse>("stopKeepAlive", json!({}));
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
    }
}

pub fn schedule_pending_resume(app: &AppHandle, require_unmetered: bool) {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        let _ = with_plugin_state(app, |state| {
            let _ = state.handle()?.run_mobile_plugin::<PluginResponse>(
                "schedulePendingResume",
                json!({ "requireUnmetered": require_unmetered }),
            );
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, require_unmetered);
    }
}

pub fn cancel_pending_resume(app: &AppHandle) {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        let _ = with_plugin_state(app, |state| {
            let _ = state
                .handle()?
                .run_mobile_plugin::<PluginResponse>("cancelPendingResume", json!({}));
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
    }
}

/// Immersive video fullscreen on Android: hide the system bars and (for
/// landscape video) lock to sensor-landscape. No-op elsewhere — desktop and
/// browsers use the HTML Fullscreen API.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn set_immersive(app: &AppHandle, enabled: bool, landscape: bool) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        with_plugin_state(app, |state| {
            let response: PluginResponse = state
                .handle()?
                .run_mobile_plugin(
                    "setImmersive",
                    json!({ "enabled": enabled, "landscape": landscape }),
                )
                .map_err(|e| AppError::Other(format!("setImmersive failed: {e}")))?;
            if response.ok.unwrap_or(false) {
                Ok(())
            } else {
                Err(AppError::Other(
                    response
                        .message
                        .unwrap_or_else(|| "setImmersive failed".into()),
                ))
            }
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, enabled, landscape);
        Ok(())
    }
}

/// Android: may ArcHive manage all files in shared storage (needed to delete
/// library files in e.g. Download/ArcHive that this install didn't create)?
/// Other platforms: always true.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn storage_access(app: &AppHandle) -> AppResult<bool> {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        with_plugin_state(app, |state| {
            let response: PluginResponse = state
                .handle()?
                .run_mobile_plugin("storageAccess", json!({}))
                .map_err(|e| AppError::Other(format!("storageAccess failed: {e}")))?;
            Ok(response.ok.unwrap_or(false))
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(true)
    }
}

/// Android: open the system "All files access" screen for ArcHive.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn open_storage_access_settings(app: &AppHandle) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        use serde_json::json;
        with_plugin_state(app, |state| {
            let response: PluginResponse = state
                .handle()?
                .run_mobile_plugin("openStorageAccessSettings", json!({}))
                .map_err(|e| AppError::Other(format!("openStorageAccessSettings failed: {e}")))?;
            if response.ok.unwrap_or(false) {
                Ok(())
            } else {
                Err(AppError::Other(response.message.unwrap_or_else(|| {
                    "could not open the settings screen".into()
                })))
            }
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(())
    }
}
