use crate::db::Database;
use crate::downloads::DownloadManager;
use crate::error::AppResult;
use crate::models::{AppSettings, DataSaverMode, DownloadQuality, DownloadStatus};
use parking_lot::RwLock;
use std::sync::{Arc, Weak};
use tauri::{AppHandle, Emitter};

pub struct NetworkMonitor {
    db: Arc<Database>,
    app: AppHandle,
    is_wifi: Arc<RwLock<bool>>,
    is_metered: Arc<RwLock<bool>>,
    connection_type: Arc<RwLock<String>>,
    downloads: RwLock<Weak<DownloadManager>>,
}

impl NetworkMonitor {
    pub fn new(db: Arc<Database>, app: AppHandle) -> Self {
        Self {
            db,
            app,
            is_wifi: Arc::new(RwLock::new(true)),
            is_metered: Arc::new(RwLock::new(false)),
            connection_type: Arc::new(RwLock::new("unknown".to_string())),
            downloads: RwLock::new(Weak::new()),
        }
    }

    pub fn bind_downloads(&self, downloads: Arc<DownloadManager>) {
        *self.downloads.write() = Arc::downgrade(&downloads);
    }

    pub fn set_connection_type(&self, connection_type: &str) {
        let is_wifi = connection_type == "wifi" || connection_type == "ethernet";
        *self.is_wifi.write() = is_wifi;
        *self.connection_type.write() = connection_type.to_string();
    }

    pub fn set_metered(&self, metered: bool) {
        *self.is_metered.write() = metered;
    }

    #[allow(dead_code)]
    pub fn is_wifi(&self) -> bool {
        *self.is_wifi.read()
    }

    pub fn is_metered(&self) -> bool {
        *self.is_metered.read()
    }

    pub fn connection_type(&self) -> String {
        self.connection_type.read().clone()
    }

    pub async fn check_and_update_downloads(&self) -> AppResult<()> {
        let settings = self.db.get_settings()?;
        let is_wifi = *self.is_wifi.read();
        let jobs = self.db.list_download_jobs()?;

        for mut job in jobs {
            let mut should_update = false;

            if settings.download_on_wifi_only
                && !is_wifi
                && matches!(
                    job.status,
                    DownloadStatus::Pending | DownloadStatus::Active
                )
            {
                job.status = DownloadStatus::WaitingForWifi;
                job.error = Some("Waiting for Wi-Fi connection".to_string());
                should_update = true;
            }

            if settings.download_on_wifi_only
                && is_wifi
                && job.status == DownloadStatus::WaitingForWifi
            {
                job.status = DownloadStatus::Pending;
                job.error = None;
                should_update = true;
            }

            if should_update {
                self.db.update_download_job(&job)?;
                let _ = self.app.emit("download:progress", &job);

                if job.status == DownloadStatus::Pending {
                    if let Some(dm) = self.downloads.read().upgrade() {
                        let _ = dm.enqueue_job_id(&job.id);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn can_start_download(&self) -> AppResult<bool> {
        let settings = self.db.get_settings()?;
        if !settings.download_on_wifi_only {
            return Ok(true);
        }
        let is_wifi = *self.is_wifi.read();
        Ok(!(settings.download_on_wifi_only && !is_wifi))
    }
}

/// Cap download quality when data saver applies.
pub fn effective_download_quality(settings: &AppSettings, is_metered: bool) -> DownloadQuality {
    let apply_cap = match settings.data_saver {
        DataSaverMode::Off => false,
        DataSaverMode::OnMetered => is_metered,
        DataSaverMode::Always => true,
    };
    if apply_cap {
        settings
            .download_quality
            .min_quality(DownloadQuality::Height480)
    } else {
        settings.download_quality
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_saver_off_keeps_quality() {
        let mut s = AppSettings::default();
        s.download_quality = DownloadQuality::Best;
        s.data_saver = DataSaverMode::Off;
        assert_eq!(
            effective_download_quality(&s, true),
            DownloadQuality::Best
        );
    }

    #[test]
    fn data_saver_on_metered_caps_when_metered() {
        let mut s = AppSettings::default();
        s.download_quality = DownloadQuality::Best;
        s.data_saver = DataSaverMode::OnMetered;
        assert_eq!(
            effective_download_quality(&s, true),
            DownloadQuality::Height480
        );
        assert_eq!(
            effective_download_quality(&s, false),
            DownloadQuality::Best
        );
    }

    #[test]
    fn data_saver_always_caps() {
        let mut s = AppSettings::default();
        s.download_quality = DownloadQuality::Height1080;
        s.data_saver = DataSaverMode::Always;
        assert_eq!(
            effective_download_quality(&s, false),
            DownloadQuality::Height480
        );
    }

    #[test]
    fn quality_min_respects_lower_user_setting() {
        let mut s = AppSettings::default();
        s.download_quality = DownloadQuality::Height480;
        s.data_saver = DataSaverMode::Always;
        assert_eq!(
            effective_download_quality(&s, true),
            DownloadQuality::Height480
        );
    }
}
