use crate::error::{AppError, AppResult};
use crate::models::BinaryVersions;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

fn yt_dlp_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        // Linux / Android — generic Linux binary
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
        let path = self.install_dir.join(name);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub async fn install_yt_dlp(&self) -> AppResult<PathBuf> {
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

    pub async fn install_gallery_dl(&self) -> AppResult<PathBuf> {
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
}
