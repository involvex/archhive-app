use crate::error::{AppError, AppResult};
use regex::Regex;
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
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
        on_line: impl Fn(&str),
    ) -> AppResult<String> {
        let output = format!("{output_dir}/{template}");
        let mut args = vec![
            url.to_string(),
            "-o".to_string(),
            output,
            "--newline".to_string(),
            "--progress".to_string(),
            "--no-overwrites".to_string(),
        ];
        if let Some(cookies) = cookies_file {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        self.spawn("yt-dlp", &args, on_line).await
    }

    pub async fn run_gallery_dl(
        &self,
        url: &str,
        output_dir: &str,
        on_line: impl Fn(&str),
    ) -> AppResult<String> {
        let args = vec![
            url.to_string(),
            "-d".to_string(),
            output_dir.to_string(),
            "--no-mtime".to_string(),
        ];
        self.spawn("gallery-dl", &args, on_line).await
    }

    pub async fn spawn_ffmpeg(
        &self,
        args: &[String],
        on_line: impl Fn(&str),
    ) -> AppResult<String> {
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
                CommandEvent::Terminated(payload)
                    if payload.code != Some(0) => {
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

    pub fn parse_progress(line: &str) -> Option<f32> {
        let re = Regex::new(r"\[download\]\s+(\d+\.?\d*)%").ok()?;
        re.captures(line)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    pub fn parse_destination(line: &str) -> Option<String> {
        let markers = ["[download] Destination: ", "Destination: "];
        for marker in markers {
            if let Some(idx) = line.find(marker) {
                let path = line[idx + marker.len()..].trim();
                if !path.is_empty() {
                    return Some(path.to_string());
                }
            }
        }
        None
    }
}
