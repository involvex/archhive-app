use crate::db::Database;
use crate::error::AppResult;
use crate::models::DownloadStatus;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct NetworkMonitor {
    db: Arc<Database>,
    app: AppHandle,
    is_wifi: Arc<RwLock<bool>>,
    is_metered: Arc<RwLock<bool>>,
}

impl NetworkMonitor {
    pub fn new(db: Arc<Database>, app: AppHandle) -> Self {
        Self {
            db,
            app,
            is_wifi: Arc::new(RwLock::new(true)), // Default to true (allow downloads)
            is_metered: Arc::new(RwLock::new(false)),
        }
    }
    
    pub fn set_connection_type(&self, connection_type: &str) {
        let is_wifi = connection_type == "wifi" || connection_type == "ethernet";
        *self.is_wifi.write() = is_wifi;
    }
    
    pub fn set_metered(&self, metered: bool) {
        *self.is_metered.write() = metered;
    }

    pub async fn check_and_update_downloads(&self) -> AppResult<()> {
        let settings = self.db.get_settings()?;
        
        let is_wifi = *self.is_wifi.read();
        let _is_metered = *self.is_metered.read();
        
        let jobs = self.db.list_download_jobs()?;
        
        for mut job in jobs {
            let mut should_update = false;
            
            // Check if we should pause due to Wi-Fi only setting
            if settings.download_on_wifi_only && !is_wifi && matches!(job.status, DownloadStatus::Pending | DownloadStatus::Active) {
                job.status = DownloadStatus::WaitingForWifi;
                job.error = Some("Waiting for Wi-Fi connection".to_string());
                should_update = true;
            }
            
            // Check if we should resume from waiting_for_wifi
            if settings.download_on_wifi_only && is_wifi && job.status == DownloadStatus::WaitingForWifi {
                job.status = DownloadStatus::Pending;
                job.error = None;
                should_update = true;
            }
            
            if should_update {
                self.db.update_download_job(&job)?;
                let _ = self.app.emit("download:progress", &job);
                
                // If status changed to Pending, re-enqueue
                if job.status == DownloadStatus::Pending {
                    let _ = self.app.emit("download:enqueue", job.id);
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
        
        if settings.download_on_wifi_only && !is_wifi {
            return Ok(false);
        }
        
        Ok(true)
    }
}
