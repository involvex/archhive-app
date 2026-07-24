use crate::error::{AppError, AppResult};
use regex::Regex;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

pub struct SidecarRunner {
    app: AppHandle,
}

impl SidecarRunner {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub async fn run_yt_dlp(
        &self,
        url: &str,
        output_dir: &str,
        template: &str,
        cookies_file: Option<&Path>,
        cancel: Arc<AtomicBool>,
        on_line: impl Fn(&str),
        format_args: &[String],
    ) -> AppResult<String> {
        let output = format!("{output_dir}/{template}");
        let mut args = vec![
            url.to_string(),
            "-o".to_string(),
            output,
            "--newline".to_string(),
            "--progress".to_string(),
            "--no-overwrites".to_string(),
            // Emit final path explicitly (newer yt-dlp); Destination: is not always printed.
            "--print".to_string(),
            "after_move:filepath".to_string(),
            "--print".to_string(),
            "filepath".to_string(),
        ];
        args.extend(format_args.iter().cloned());
        if let Some(cookies) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        self.spawn_cancellable("yt-dlp", &args, cancel, on_line, Some(output_dir))
            .await
    }

    /// Build yt-dlp `-f` / `-S` args from download quality prefs.
    pub fn format_selection_args(
        quality: crate::models::DownloadQuality,
        prefer_mp4: bool,
    ) -> Vec<String> {
        use crate::models::DownloadQuality;
        let height = match quality {
            DownloadQuality::Best => None,
            DownloadQuality::Height1080 => Some(1080u32),
            DownloadQuality::Height720 => Some(720),
            DownloadQuality::Height480 => Some(480),
        };

        let mut args = Vec::new();
        if prefer_mp4 {
            let format = match height {
                Some(h) => format!(
                    "bv*[height<={h}][ext=mp4]+ba[ext=m4a]/b[height<={h}][ext=mp4]/bv*[height<={h}]+ba/b[height<={h}]/b"
                ),
                None => {
                    "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b".to_string()
                }
            };
            args.push("-f".to_string());
            args.push(format);
            args.push("-S".to_string());
            args.push("res,ext:mp4:m4a".to_string());
        } else if let Some(h) = height {
            args.push("-f".to_string());
            args.push(format!("bv*[height<={h}]+ba/b[height<={h}]/b"));
        }
        args
    }

    /// Resolve metadata for a single URL via yt-dlp `-J` (no download).
    pub async fn resolve_media_json(
        &self,
        url: &str,
        cookies_file: Option<&Path>,
    ) -> AppResult<serde_json::Value> {
        let mut args = vec![
            url.to_string(),
            "-J".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
        ];
        if let Some(cookies) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = self.run_capture("yt-dlp", &args).await?;
        serde_json::from_str(&raw)
            .map_err(|e| AppError::Download(format!("yt-dlp JSON parse: {e}")))
    }

    pub async fn run_gallery_dl(
        &self,
        url: &str,
        output_dir: &str,
        on_line: impl Fn(&str),
    ) -> AppResult<Vec<String>> {
        let args = vec![
            url.to_string(),
            "-d".to_string(),
            output_dir.to_string(),
            "--no-mtime".to_string(),
        ];
        self.spawn_gallery_dl("gallery-dl", &args, on_line).await
    }

    async fn spawn_gallery_dl(
        &self,
        name: &str,
        args: &[String],
        on_line: impl Fn(&str),
    ) -> AppResult<Vec<String>> {
        let sidecar_result = self
            .app
            .shell()
            .sidecar(format!("binaries/{name}"))
            .map(|cmd| cmd.args(args).spawn());

        match sidecar_result {
            Ok(Ok((rx, _child))) => self.consume_gallery_dl(rx, name, on_line).await,
            _ => {
                let (rx, _child) = self
                    .app
                    .shell()
                    .command(name)
                    .args(args)
                    .spawn()
                    .map_err(|e| AppError::Download(format!("spawn {name}: {e}")))?;
                self.consume_gallery_dl(rx, name, on_line).await
            }
        }
    }

    pub async fn list_flat_playlist(
        &self,
        url: &str,
        page: u32,
        page_size: u32,
        cookies_file: Option<&Path>,
    ) -> AppResult<Vec<(String, String, String, Option<String>)>> {
        let start = (page.saturating_sub(1)) * page_size + 1;
        let end = page * page_size;
        let mut args = vec![
            url.to_string(),
            "--flat-playlist".to_string(),
            "-J".to_string(),
            "--no-warnings".to_string(),
            format!("--playlist-start={start}"),
            format!("--playlist-end={end}"),
        ];
        if let Some(cookies) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = self.run_capture("yt-dlp", &args).await?;
        parse_flat_playlist_json(&raw)
    }

    /// Expand channel/search/playlist URLs up to `max_entries` videos.
    pub async fn list_flat_playlist_all(
        &self,
        url: &str,
        max_entries: u32,
        cookies_file: Option<&Path>,
    ) -> AppResult<Vec<(String, String, String, Option<String>)>> {
        let mut args = vec![
            url.to_string(),
            "--flat-playlist".to_string(),
            "-J".to_string(),
            "--no-warnings".to_string(),
            format!("--playlist-end={max_entries}"),
        ];
        if let Some(cookies) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = self.run_capture("yt-dlp", &args).await?;
        parse_flat_playlist_json(&raw)
    }

    async fn spawn_cancellable(
        &self,
        name: &str,
        args: &[String],
        cancel: Arc<AtomicBool>,
        on_line: impl Fn(&str),
        output_dir: Option<&str>,
    ) -> AppResult<String> {
        let sidecar_result = self
            .app
            .shell()
            .sidecar(format!("binaries/{name}"))
            .map(|cmd| cmd.args(args).spawn());

        match sidecar_result {
            Ok(Ok((rx, child))) => {
                self.consume_cancellable(rx, child, name, cancel, on_line, output_dir)
                    .await
            }
            _ => {
                let (rx, child) = self
                    .app
                    .shell()
                    .command(name)
                    .args(args)
                    .spawn()
                    .map_err(|e| AppError::Download(format!("spawn {name}: {e}")))?;
                self.consume_cancellable(rx, child, name, cancel, on_line, output_dir)
                    .await
            }
        }
    }

    async fn consume_cancellable(
        &self,
        mut rx: tauri::async_runtime::Receiver<CommandEvent>,
        child: CommandChild,
        name: &str,
        cancel: Arc<AtomicBool>,
        on_line: impl Fn(&str),
        output_dir: Option<&str>,
    ) -> AppResult<String> {
        let mut destination = String::new();
        let mut line_buf = String::new();
        loop {
            if !cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                return Err(AppError::Download("stopped".into()));
            }
            let event = tokio::select! {
                biased;
                _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                    continue;
                }
                event = rx.recv() => event,
            };
            let Some(event) = event else {
                break;
            };
            match event {
                CommandEvent::Stdout(chunk) | CommandEvent::Stderr(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    feed_lines(&mut line_buf, &text, |line| {
                        on_line(line);
                        if let Some(path) = Self::parse_destination(line) {
                            destination = path;
                        }
                    });
                }
                CommandEvent::Terminated(payload) => {
                    if !cancel.load(Ordering::Relaxed) {
                        return Err(AppError::Download("stopped".into()));
                    }
                    if payload.code != Some(0) {
                        return Err(AppError::Download(format!(
                            "{name} exited with code {:?}",
                            payload.code
                        )));
                    }
                    break;
                }
                _ => {}
            }
        }
        feed_lines_flush(&mut line_buf, |line| {
            on_line(line);
            if let Some(path) = Self::parse_destination(line) {
                destination = path;
            }
        });

        if destination.trim().is_empty() {
            if let Some(dir) = output_dir {
                if let Some(found) = resolve_newest_media(Path::new(dir)) {
                    return Ok(found);
                }
            }
            return Err(AppError::Download(format!(
                "{name} finished but no output destination was reported"
            )));
        }
        Ok(destination)
    }

    async fn run_capture(&self, name: &str, args: &[String]) -> AppResult<String> {
        let sidecar_result = self
            .app
            .shell()
            .sidecar(format!("binaries/{name}"))
            .map(|cmd| cmd.args(args).spawn());

        let mut rx = match sidecar_result {
            Ok(Ok((rx, _child))) => rx,
            _ => {
                let (rx, _child) = self
                    .app
                    .shell()
                    .command(name)
                    .args(args)
                    .spawn()
                    .map_err(|e| AppError::Download(format!("spawn {name}: {e}")))?;
                rx
            }
        };

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut code = None;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    stdout.push_str(&String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    stderr.push_str(&String::from_utf8_lossy(&line));
                }
                CommandEvent::Terminated(payload) => {
                    code = payload.code;
                }
                _ => {}
            }
        }
        if code != Some(0) {
            let detail = if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            };
            return Err(AppError::Download(format!(
                "{name} exited with code {:?}{}",
                code,
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            )));
        }
        Ok(stdout)
    }

    pub async fn spawn_ffmpeg(&self, args: &[String], on_line: impl Fn(&str)) -> AppResult<String> {
        self.spawn("ffmpeg", args, on_line).await
    }

    async fn spawn(
        &self,
        name: &str,
        args: &[String],
        on_line: impl Fn(&str),
    ) -> AppResult<String> {
        let sidecar_result = self
            .app
            .shell()
            .sidecar(format!("binaries/{name}"))
            .map(|cmd| cmd.args(args).spawn());

        match sidecar_result {
            Ok(Ok((rx, _child))) => self.consume(rx, name, on_line).await,
            _ => {
                let (rx, _child) = self
                    .app
                    .shell()
                    .command(name)
                    .args(args)
                    .spawn()
                    .map_err(|e| AppError::Download(format!("spawn {name}: {e}")))?;
                self.consume(rx, name, on_line).await
            }
        }
    }

    async fn consume(
        &self,
        mut rx: tauri::async_runtime::Receiver<CommandEvent>,
        name: &str,
        on_line: impl Fn(&str),
    ) -> AppResult<String> {
        let mut destination = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    on_line(&text);
                    if let Some(path) = Self::parse_destination(&text) {
                        destination = path;
                    }
                }
                CommandEvent::Terminated(payload) if payload.code != Some(0) => {
                    return Err(AppError::Download(format!(
                        "{name} exited with code {:?}",
                        payload.code
                    )));
                }
                _ => {}
            }
        }
        Ok(destination)
    }

    async fn consume_gallery_dl(
        &self,
        mut rx: tauri::async_runtime::Receiver<CommandEvent>,
        name: &str,
        on_line: impl Fn(&str),
    ) -> AppResult<Vec<String>> {
        let mut destinations = Vec::new();
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    on_line(&text);
                    if let Some(path) = crate::downloads::gallery_dl::parse_gallery_dl_path(&text) {
                        destinations.push(path);
                    }
                    if let Some(path) = Self::parse_destination(&text) {
                        destinations.push(path);
                    }
                }
                CommandEvent::Terminated(payload) if payload.code != Some(0) => {
                    return Err(AppError::Download(format!(
                        "{name} exited with code {:?}",
                        payload.code
                    )));
                }
                _ => {}
            }
        }
        Ok(destinations)
    }

    pub fn parse_progress(line: &str) -> Option<f32> {
        let re = Regex::new(r"\[download\]\s+(\d+\.?\d*)%").ok()?;
        re.captures(line)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    pub fn parse_destination(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let markers = ["[download] Destination: ", "Destination: "];
        for marker in markers {
            if let Some(idx) = trimmed.find(marker) {
                let path = trimmed[idx + marker.len()..].trim();
                if !path.is_empty() {
                    return Some(path.to_string());
                }
            }
        }

        // [Merger] Merging formats into "path"  or  into path
        if let Some(idx) = trimmed.find("[Merger] Merging formats into ") {
            let rest = trimmed[idx + "[Merger] Merging formats into ".len()..].trim();
            let path = rest.trim_matches('"').trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }

        // [download] path has already been downloaded
        if let Some(idx) = trimmed.find(" has already been downloaded") {
            let prefix = &trimmed[..idx];
            let path = prefix.strip_prefix("[download] ").unwrap_or(prefix).trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }

        // Bare filepath from --print (no leading [tag])
        if !trimmed.starts_with('[') && looks_like_media_path(trimmed) {
            return Some(trimmed.to_string());
        }

        None
    }
}

const MEDIA_EXTS: &[&str] = &[
    ".mp4", ".m4v", ".webm", ".mkv", ".avi", ".mov", ".wmv", ".flv", ".opus", ".m4a", ".mp3",
];

fn looks_like_media_path(s: &str) -> bool {
    let lower = s.to_lowercase();
    MEDIA_EXTS.iter().any(|ext| lower.ends_with(ext))
}

fn feed_lines(buf: &mut String, chunk: &str, mut on_line: impl FnMut(&str)) {
    buf.push_str(chunk);
    while let Some(idx) = buf.find('\n') {
        let line = buf[..idx].trim_end_matches('\r').to_string();
        buf.drain(..=idx);
        if !line.is_empty() {
            on_line(&line);
        }
    }
}

fn feed_lines_flush(buf: &mut String, mut on_line: impl FnMut(&str)) {
    let rest = buf.trim().to_string();
    buf.clear();
    if !rest.is_empty() {
        on_line(&rest);
    }
}

fn resolve_newest_media(root: &Path) -> Option<String> {
    if !root.exists() {
        return None;
    }
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if !looks_like_media_path(&path.to_string_lossy()) {
            continue;
        }
        let modified = entry.metadata().ok()?.modified().ok()?;
        match &best {
            None => best = Some((modified, path.to_path_buf())),
            Some((prev, _)) if modified > *prev => best = Some((modified, path.to_path_buf())),
            _ => {}
        }
    }
    best.map(|(_, p)| p.to_string_lossy().to_string())
}

fn parse_flat_playlist_json(raw: &str) -> AppResult<Vec<(String, String, String, Option<String>)>> {
    let value: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|e| AppError::Download(format!("yt-dlp JSON: {e}")))?;

    let entries = value
        .get("entries")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_else(|| vec![value.clone()]);

    let mut out = Vec::new();
    for entry in entries {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let url = obj
            .get("url")
            .or_else(|| obj.get("webpage_url"))
            .or_else(|| obj.get("original_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let Some(url) = url else { continue };
        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&url)
            .to_string();
        let thumbnail = thumbnail_from_entry(obj);
        out.push((id, title, url, thumbnail));
    }

    if out.is_empty() {
        return Err(AppError::Site(
            "No videos found for this profile or URL.".to_string(),
        ));
    }

    Ok(out)
}

fn thumbnail_from_entry(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    if let Some(thumb) = obj.get("thumbnail").and_then(|v| v.as_str()) {
        if !thumb.is_empty() {
            return Some(thumb.to_string());
        }
    }
    obj.get("thumbnails")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.last())
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_destination_marker() {
        let p = SidecarRunner::parse_destination("[download] Destination: C:\\lib\\foo\\bar.mp4")
            .unwrap();
        assert!(p.ends_with("bar.mp4"));
    }

    #[test]
    fn parses_merger_line() {
        let p = SidecarRunner::parse_destination(
            "[Merger] Merging formats into \"/tmp/out/video.mp4\"",
        )
        .unwrap();
        assert_eq!(p, "/tmp/out/video.mp4");
    }

    #[test]
    fn parses_already_downloaded() {
        let p = SidecarRunner::parse_destination(
            "[download] D:\\lib\\clip.webm has already been downloaded",
        )
        .unwrap();
        assert_eq!(p, "D:\\lib\\clip.webm");
    }

    #[test]
    fn parses_bare_print_filepath() {
        let p = SidecarRunner::parse_destination("/home/user/vid.mkv").unwrap();
        assert_eq!(p, "/home/user/vid.mkv");
    }

    #[test]
    fn format_selection_prefers_mp4_with_height_cap() {
        use crate::models::DownloadQuality;
        let args = SidecarRunner::format_selection_args(DownloadQuality::Height720, true);
        assert!(args.windows(2).any(|w| w[0] == "-f" && w[1].contains("height<=720")));
        assert!(args.windows(2).any(|w| w[0] == "-S" && w[1].contains("ext:mp4")));
    }

    #[test]
    fn format_selection_best_without_mp4_is_empty() {
        use crate::models::DownloadQuality;
        let args = SidecarRunner::format_selection_args(DownloadQuality::Best, false);
        assert!(args.is_empty());
    }

    #[test]
    fn ignores_progress_lines() {
        assert!(SidecarRunner::parse_destination("[download]  45.2% of 10.00MiB").is_none());
    }

    #[test]
    fn feed_lines_handles_split_chunks() {
        let mut buf = String::new();
        let mut lines = Vec::new();
        feed_lines(&mut buf, "[download] Destina", |l| {
            lines.push(l.to_string())
        });
        feed_lines(&mut buf, "tion: /a/b.mp4\n", |l| lines.push(l.to_string()));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Destination:"));
    }
}
