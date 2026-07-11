use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, MediaItem};
use crate::vault::CookieVault;
use async_trait::async_trait;
use std::sync::Arc;

pub mod adapters;
pub mod browse_fallback;
pub mod registry;
pub mod urls;
pub mod yt_dlp;

#[derive(Clone)]
pub struct SiteContext {
    pub client: reqwest::Client,
    vault: Arc<CookieVault>,
    app: tauri::AppHandle,
}

impl SiteContext {
    pub fn new(vault: Arc<CookieVault>, app: tauri::AppHandle) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .build()?;
        Ok(Self { client, vault, app })
    }

    pub fn app(&self) -> &tauri::AppHandle {
        &self.app
    }

    pub fn cookie_file_for_site(&self, site_id: &str) -> Option<std::path::PathBuf> {
        self.vault.cookie_file_for_site(site_id)
    }

    pub async fn fetch_html(&self, url: &str, site_id: &str) -> AppResult<String> {
        let mut req = self.client.get(url);
        if let Ok(Some(header)) = self.vault.cookie_header(site_id) {
            if !header.is_empty() {
                req = req.header("Cookie", header);
            }
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Site(format!("HTTP {} for {url}", resp.status())));
        }
        Ok(resp.text().await?)
    }
}

#[async_trait]
pub trait SiteAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn base_url(&self) -> &str;
    fn supported_kinds(&self) -> Vec<BrowseKind>;
    fn requires_cookies(&self) -> bool {
        false
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage>;

    async fn resolve_download(&self, ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan>;
}
