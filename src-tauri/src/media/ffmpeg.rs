use crate::error::AppResult;
use crate::sites::yt_dlp::SidecarRunner;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub struct FfmpegProcessor {
    runner: SidecarRunner,
}

impl FfmpegProcessor {
    pub fn new(app: AppHandle) -> Self {
        Self {
            runner: SidecarRunner::new(app),
        }
    }

    /// Probe the duration (in seconds) of a video file via ffprobe.
    /// Returns `None` on any failure (missing ffprobe, corrupt file, etc.).
    pub async fn probe_duration(&self, video_path: &Path) -> Option<f64> {
        let args = vec![
            "-v".to_string(),
            "error".to_string(),
            "-show_entries".to_string(),
            "format=duration".to_string(),
            "-of".to_string(),
            "default=noprint_wrappers=1:nokey=1".to_string(),
            video_path.to_string_lossy().to_string(),
        ];
        let raw = self.runner.spawn_ffprobe(&args).await.ok()?;
        raw.trim().parse::<f64>().ok()
    }

    /// Extract a single JPEG thumbnail from the video.
    ///
    /// Strategy:
    /// 1. Probe duration via ffprobe (fast, negligible cost).
    /// 2. Seek to `min(3s, duration * 0.1)` so short clips get a visible frame.
    /// 3. If the probe fails, fall back to `-ss 0` (first frame) so we never
    ///    skip a video entirely.
    /// 4. If the primary pass fails, retry with `-ss 0` as a last resort.
    pub async fn extract_thumbnail(&self, video_path: &Path) -> AppResult<PathBuf> {
        let thumb_path = video_path.with_extension("jpg");
        let video_str = video_path.to_string_lossy().to_string();

        let seek_pos = match self.probe_duration(video_path).await {
            Some(dur) if dur > 0.0 => {
                let t = dur.min(30.0) * 0.1;
                format!("{:.2}", t)
            }
            _ => "0".to_string(),
        };

        let mut args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            seek_pos,
            "-i".to_string(),
            video_str.clone(),
            "-frames:v".to_string(),
            "1".to_string(),
            "-q:v".to_string(),
            "2".to_string(),
            thumb_path.to_string_lossy().to_string(),
        ];

        if self.runner.spawn_ffmpeg(&args, |_| {}).await.is_ok() {
            return Ok(thumb_path);
        }

        // Retry with first frame if seek position was invalid.
        args[2] = "0".to_string();
        self.runner.spawn_ffmpeg(&args, |_| {}).await?;
        Ok(thumb_path)
    }

    pub async fn remux_faststart(&self, video_path: &Path) -> AppResult<PathBuf> {
        let out_path = video_path.with_extension("remux.mp4");
        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            video_path.to_string_lossy().to_string(),
            "-c".to_string(),
            "copy".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            out_path.to_string_lossy().to_string(),
        ];
        self.runner.spawn_ffmpeg(&args, |_| {}).await?;
        Ok(out_path)
    }

    /// Check whether ffmpeg/ffprobe binaries are reachable.
    /// Returns `Ok(())` if both are reachable, or an error message.
    #[allow(dead_code)]
    pub async fn check_availability(&self) -> Result<(), String> {
        // Check ffmpeg
        let ffmpeg_args = vec!["-version".to_string()];
        self.runner
            .spawn_ffmpeg(&ffmpeg_args, |_| {})
            .await
            .map_err(|e| format!("ffmpeg not found: {e}"))?;

        // Check ffprobe
        let ffprobe_args = vec!["-version".to_string()];
        self.runner
            .spawn_ffprobe(&ffprobe_args)
            .await
            .map_err(|e| format!("ffprobe not found: {e}"))?;

        Ok(())
    }
}
