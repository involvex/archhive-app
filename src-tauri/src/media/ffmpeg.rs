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

    pub async fn extract_thumbnail(&self, video_path: &Path) -> AppResult<PathBuf> {
        let thumb_path = video_path.with_extension("jpg");
        let args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            "00:00:03".to_string(),
            "-i".to_string(),
            video_path.to_string_lossy().to_string(),
            "-frames:v".to_string(),
            "1".to_string(),
            "-q:v".to_string(),
            "2".to_string(),
            thumb_path.to_string_lossy().to_string(),
        ];
        self.runner
            .spawn_ffmpeg(&args, |_| {})
            .await?;
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
}
