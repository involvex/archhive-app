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

    /// Probe the video resolution (width, height) via ffprobe.
    /// Returns `None` on any failure.
    pub async fn probe_resolution(&self, video_path: &Path) -> Option<(u32, u32)> {
        let args = vec![
            "-v".to_string(),
            "error".to_string(),
            "-select_streams".to_string(),
            "v:0".to_string(),
            "-show_entries".to_string(),
            "stream=width,height".to_string(),
            "-of".to_string(),
            "csv=p=0".to_string(),
            video_path.to_string_lossy().to_string(),
        ];
        let raw = self.runner.spawn_ffprobe(&args).await.ok()?;
        let mut parts = raw.trim().split(',');
        let width: u32 = parts.next()?.trim().parse().ok()?;
        let height: u32 = parts.next()?.trim().parse().ok()?;
        if width == 0 || height == 0 {
            return None;
        }
        Some((width, height))
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

    /// Download an HLS (`.m3u8`) stream to `out_path` via stream copy.
    ///
    /// Sends `Referer` (+ optional `Cookie`) headers, required by gated CDNs
    /// such as PornHub's phncdn that refuse bare `<video>` / plain-HTTP
    /// fetches. `duration_secs` enables fractional progress parsed from
    /// ffmpeg's `time=` output; without it the callback only marks activity
    /// (throttled to one call per 2s).
    pub async fn download_hls(
        &self,
        url: &str,
        out_path: &Path,
        referer: Option<&str>,
        cookie_header: Option<&str>,
        duration_secs: Option<u32>,
        mut on_progress: Option<impl FnMut(Option<f32>)>,
    ) -> AppResult<()> {
        let mut headers = String::new();
        if let Some(r) = referer.filter(|s| !s.is_empty()) {
            headers.push_str(&format!("Referer: {r}\r\n"));
        }
        if let Some(c) = cookie_header.filter(|s| !s.is_empty()) {
            headers.push_str(&format!("Cookie: {c}\r\n"));
        }
        let mut args = vec![
            "-y".to_string(),
            "-hide_banner".to_string(),
            "-loglevel".to_string(),
            "info".to_string(),
        ];
        if !headers.is_empty() {
            args.push("-headers".to_string());
            args.push(headers);
        }
        args.extend([
            "-i".to_string(),
            url.to_string(),
            "-c".to_string(),
            "copy".to_string(),
            out_path.to_string_lossy().to_string(),
        ]);

        let mut last_fraction = 0.0f32;
        let mut last_emit = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(10))
            .unwrap_or_else(std::time::Instant::now);
        self.runner
            .spawn_ffmpeg(&args, |line| {
                let Some(t) = parse_ffmpeg_time(line) else {
                    return;
                };
                let fraction = duration_secs
                    .filter(|d| *d > 0)
                    .map(|d| (t / f64::from(d)).clamp(0.0, 1.0) as f32);
                let advanced = fraction.is_some_and(|f| f >= last_fraction + 0.02);
                if advanced || last_emit.elapsed() >= std::time::Duration::from_secs(2) {
                    if let Some(f) = fraction {
                        last_fraction = f;
                    }
                    last_emit = std::time::Instant::now();
                    if let Some(cb) = on_progress.as_mut() {
                        cb(fraction);
                    }
                }
            })
            .await?;
        Ok(())
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

/// Parse ffmpeg's `time=HH:MM:SS.xx` progress stamp into seconds.
/// Returns `None` for lines without a stamp (e.g. `time=N/A`).
fn parse_ffmpeg_time(line: &str) -> Option<f64> {
    let idx = line.find("time=")?;
    let rest = &line[idx + "time=".len()..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ':'))?;
    let mut parts = rest[..end].split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

#[cfg(test)]
mod tests {
    use super::parse_ffmpeg_time;

    #[test]
    fn parses_progress_timestamp() {
        let line = "frame= 1234 fps=60 q=-1.0 size=    8192kB time=00:01:23.45 bitrate= 803.1kbits/s speed=1.01x";
        assert!((parse_ffmpeg_time(line).unwrap() - 83.45).abs() < 0.01);
    }

    #[test]
    fn rejects_missing_timestamp() {
        assert!(parse_ffmpeg_time("frame= 1 fps=0 q=-1.0 size= 0kB time=N/A speed=N/A").is_none());
        assert!(parse_ffmpeg_time("video:1234kB audio:0kB").is_none());
    }
}
