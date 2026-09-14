use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, MediaItem};
use crate::vault::CookieVault;
use async_trait::async_trait;
use std::sync::Arc;

pub mod adapters;
pub mod browse_fallback;
pub mod extractors;
pub mod registry;
pub mod urls;
pub mod yt_dlp;

#[derive(Clone)]
pub struct SiteContext {
    pub client: reqwest::Client,
    vault: Arc<CookieVault>,
    app: tauri::AppHandle,
    lan_port: u16,
}

impl SiteContext {
    pub fn new(vault: Arc<CookieVault>, app: tauri::AppHandle) -> AppResult<Self> {
        Self::with_lan_port(vault, app, 8787)
    }

    pub fn with_lan_port(
        vault: Arc<CookieVault>,
        app: tauri::AppHandle,
        lan_port: u16,
    ) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .build()?;
        Ok(Self {
            client,
            vault,
            app,
            lan_port,
        })
    }

    pub fn lan_port(&self) -> u16 {
        self.lan_port
    }

    pub fn app(&self) -> &tauri::AppHandle {
        &self.app
    }

    pub fn _vault(&self) -> &CookieVault {
        &self.vault
    }

    pub fn cookie_file_for_site(&self, site_id: &str) -> Option<std::path::PathBuf> {
        self.vault.cookie_file_for_site(site_id)
    }

    pub async fn fetch_html(&self, url: &str, site_id: &str) -> AppResult<String> {
        self.fetch_with_headers(url, site_id, None).await
    }

    /// GET with vault cookies plus optional extra headers (API listings, XHR endpoints).
    pub async fn fetch_with_headers(
        &self,
        url: &str,
        site_id: &str,
        extra: Option<&[(&str, &str)]>,
    ) -> AppResult<String> {
        let mut req = self.client.get(url);
        if let Ok(Some(header)) = self.vault.cookie_header(site_id) {
            if !header.is_empty() {
                req = req.header("Cookie", header);
            }
        }
        if let Some(pairs) = extra {
            for (k, v) in pairs {
                req = req.header(*k, *v);
            }
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Site(format!("HTTP {} for {url}", resp.status())));
        }
        Ok(resp.text().await?)
    }

    /// JSON room-list style APIs (Chaturbate / Stripchat ts endpoints).
    pub async fn fetch_json_api(&self, url: &str, site_id: &str, referer: &str) -> AppResult<String> {
        let origin = referer.trim_end_matches('/');
        self.fetch_with_headers(
            url,
            site_id,
            Some(&[
                ("Accept", "*/*"),
                ("Referer", referer),
                ("Origin", origin),
                ("X-Requested-With", "XMLHttpRequest"),
            ]),
        )
        .await
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

    async fn resolve_download(
        &self,
        ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan>;

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String>;
}
