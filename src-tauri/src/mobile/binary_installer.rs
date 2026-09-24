use crate::error::{AppError, AppResult};
use crate::models::{BinaryLatestVersions, BinaryUpdateResult, BinaryVersions};
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

fn yt_dlp_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        // Linux desktop — never use on Android (install_yt_dlp hard-fails there)
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    }
}

fn gallery_dl_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/mikf/gallery-dl/releases/latest/download/gallery-dl.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/mikf/gallery-dl/releases/latest/download/gallery-dl_macos"
    } else {
        "https://github.com/mikf/gallery-dl/releases/latest/download/gallery-dl"
    }
}

#[derive(Debug, Clone)]
pub struct BinaryInstaller {
    client: Client,
    install_dir: PathBuf,
}

impl BinaryInstaller {
    pub fn new(_app: AppHandle, data_dir: PathBuf) -> AppResult<Self> {
        let install_dir = data_dir.join("bin");
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| AppError::Other(format!("Failed to create binary install dir: {e}")))?;
        Ok(Self {
            client: Client::builder().user_agent("ArcHive/0.1").build()?,
            install_dir,
        })
    }

    pub fn install_dir(&self) -> &Path {
        &self.install_dir
    }

    pub async fn check_installed(&self, name: &str) -> Option<PathBuf> {
        // Allowlist only known binary basenames — never join untrusted path segments.
        if !matches!(name, "yt-dlp" | "gallery-dl") {
            return None;
        }
        let path = self.install_dir.join(name);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub async fn install_yt_dlp(&self) -> AppResult<PathBuf> {
        #[cfg(target_os = "android")]
        {
            return Err(AppError::Download(
                "yt-dlp is not installed as a Linux sidecar on Android. \
                 Use the embedded youtubedl-android engine (Settings → Library → Download engine), \
                 or connect Remote LAN to a desktop host."
                    .into(),
            ));
        }
        #[cfg(not(target_os = "android"))]
        {
            let dest = self.install_dir.join("yt-dlp");
            let tmp = self.install_dir.join("yt-dlp.tmp");

            self.download_file(yt_dlp_download_url(), &tmp).await?;
            std::fs::rename(&tmp, &dest)
                .map_err(|e| AppError::Other(format!("Failed to move yt-dlp binary: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest, perms)?;
            }

            Ok(dest)
        }
    }

    pub async fn install_gallery_dl(&self) -> AppResult<PathBuf> {
        #[cfg(target_os = "android")]
        {
            return Err(AppError::Download(
                "gallery-dl is not available on Android. \
                 Use Local/Standalone for sites supported by yt-dlp, \
                 or connect Remote LAN to a desktop host."
                    .into(),
            ));
        }
        #[cfg(not(target_os = "android"))]
        {
            let dest = self.install_dir.join("gallery-dl");
            let tmp = self.install_dir.join("gallery-dl.tmp");

            self.download_file(gallery_dl_download_url(), &tmp).await?;
            std::fs::rename(&tmp, &dest)
                .map_err(|e| AppError::Other(format!("Failed to move gallery-dl binary: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest, perms)?;
            }

            Ok(dest)
        }
    }

    pub async fn get_installed_versions(&self) -> BinaryVersions {
        let mut versions = BinaryVersions::default();

        if let Some(yt_dlp) = self.check_installed("yt-dlp").await {
            if let Ok(v) = self.run_version_check(&yt_dlp, "yt-dlp").await {
                versions.ytdlp_version = Some(v);
            }
        }

        if let Some(gallery_dl) = self.check_installed("gallery-dl").await {
            if let Ok(v) = self.run_version_check(&gallery_dl, "gallery-dl").await {
                versions.gallery_dl_version = Some(v);
            }
        }

        versions
    }

    async fn download_file(&self, url: &str, dest: &Path) -> AppResult<()> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Download(format!("Failed to fetch {url}: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::Download(format!(
                "Download failed with status {} for {url}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Download(format!("Failed to read response: {e}")))?;

        std::fs::write(dest, bytes)
            .map_err(|e| AppError::Other(format!("Failed to write binary to {:?}: {e}", dest)))?;

        Ok(())
    }

    async fn run_version_check(&self, binary_path: &Path, name: &str) -> AppResult<String> {
        let output = tokio::process::Command::new(binary_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| AppError::Download(format!("Failed to run {name}: {e}")))?;

        if !output.status.success() {
            return Err(AppError::Download(format!(
                "{name} --version exited with {:?}",
                output.status.code()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            return Err(AppError::Download(format!(
                "{name} --version returned empty output"
            )));
        }

        Ok(first_line.chars().take(64).collect())
    }

    /// Query GitHub releases API for the latest version tag.
    /// Returns `None` on any error (network, parse, etc.) — caller falls back gracefully.
    pub async fn check_latest_version(&self, repo: &str) -> Option<String> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        let resp = self.client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        let parsed: GhRelease = serde_json::from_str(&body).ok()?;
        Some(parsed.tag_name.trim_start_matches('v').to_string())
    }

    /// Update a desktop binary with rollback support. On Android, returns an error.
    pub async fn update_binary(&self, name: &str) -> AppResult<BinaryUpdateResult> {
        #[cfg(target_os = "android")]
        {
            return Err(AppError::Download(format!(
                "Binary updates are not available on Android for {name}. Use the embedded \
                 youtubedl-android engine (Settings → Library → Download engine)."
            )));
        }

        #[cfg(not(target_os = "android"))]
        {
            let repo = match name {
                "yt-dlp" => "yt-dlp/yt-dlp",
                "gallery-dl" => "mikf/gallery-dl",
                _ => return Err(AppError::Other(format!("Unknown binary: {name}"))),
            };

            let latest = self.check_latest_version(repo).await.ok_or_else(|| {
                AppError::Download(format!("Failed to fetch latest version for {name}"))
            })?;

            let current = match name {
                "yt-dlp" => self.check_installed("yt-dlp").await,
                "gallery-dl" => self.check_installed("gallery-dl").await,
                _ => None,
            };

            let current_version = if let Some(path) = &current {
                self.run_version_check(path, name).await.ok()
            } else {
                None
            };

            if current_version.as_deref() == Some(&latest) {
                return Ok(BinaryUpdateResult {
                    updated: false,
                    tool: name.to_string(),
                    previous_version: current_version,
                    new_version: Some(latest.clone()),
                    backup_path: None,
                    message: format!("{} is already up to date (v{})", name, latest),
                });
            }

            let dest = self.install_dir.join(name);
            let backup = self.install_dir.join(format!("{}.bak", name));
            let tmp = self.install_dir.join(format!("{}.tmp", name));

            // Backup current binary if it exists
            if dest.exists() {
                if backup.exists() {
                    let _ = std::fs::remove_file(&backup);
                }
                std::fs::copy(&dest, &backup)
                    .map_err(|e| AppError::Other(format!("Failed to backup {name}: {e}")))?;
            }

            let download_url = match name {
                "yt-dlp" => yt_dlp_download_url(),
                "gallery-dl" => gallery_dl_download_url(),
                _ => return Err(AppError::Other(format!("Unknown binary: {name}"))),
            };

            self.download_file(download_url, &tmp).await?;
            std::fs::rename(&tmp, &dest)
                .map_err(|e| AppError::Other(format!("Failed to install {name}: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest, perms)?;
            }

            let backup_path = if backup.exists() {
                Some(backup.to_string_lossy().to_string())
            } else {
                None
            };

            Ok(BinaryUpdateResult {
                updated: true,
                tool: name.to_string(),
                previous_version: current_version.clone(),
                new_version: Some(latest.clone()),
                backup_path,
                message: format!("{} updated from {:?} to v{}", name, current_version, latest),
            })
        }
    }

    /// Rollback a binary to its `.bak` backup, if one exists.
    pub async fn rollback_binary(&self, name: &str) -> AppResult<BinaryUpdateResult> {
        #[cfg(target_os = "android")]
        {
            return Err(AppError::Download(format!(
                "Rollback is not available on Android for {name}."
            )));
        }

        #[cfg(not(target_os = "android"))]
        {
            let dest = self.install_dir.join(name);
            let backup = self.install_dir.join(format!("{}.bak", name));

            if !backup.exists() {
                return Ok(BinaryUpdateResult {
                    updated: false,
                    tool: name.to_string(),
                    previous_version: None,
                    new_version: None,
                    backup_path: None,
                    message: format!("No backup found for {}", name),
                });
            }

            let backup_version = self.run_version_check(&backup, name).await.ok();

            std::fs::copy(&backup, &dest).map_err(|e| {
                AppError::Other(format!("Failed to restore {name} from backup: {e}"))
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest, perms)?;
            }

            let _ = std::fs::remove_file(&backup);

            Ok(BinaryUpdateResult {
                updated: true,
                tool: name.to_string(),
                previous_version: None,
                new_version: backup_version,
                backup_path: None,
                message: format!("{} rolled back to previous version", name),
            })
        }
    }

    /// Fetch latest GitHub release versions for all desktop binaries.
    pub async fn check_latest_versions(&self) -> BinaryLatestVersions {
        let (yt, gd) = tokio::join!(
            self.check_latest_version("yt-dlp/yt-dlp"),
            self.check_latest_version("mikf/gallery-dl"),
        );
        BinaryLatestVersions {
            ytdlp_latest: yt,
            gallery_dl_latest: gd,
            ffmpeg_latest: None,
            ffprobe_latest: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
}
