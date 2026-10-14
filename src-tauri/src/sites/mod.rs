use crate::error::{AppError, AppResult};
use crate::models::{
    BrowseKind, BrowseOptions, BrowseOrientation, BrowsePage, BrowseQuery, DownloadPlan, MediaItem,
    SiteCategory,
};
use crate::vault::CookieVault;
use async_trait::async_trait;
use std::sync::Arc;

pub mod adapters;
pub mod browse_fallback;
pub mod challenge;
pub mod clearance;
pub mod cookie_keys;
pub mod details;
pub mod extractors;
pub mod listing;
pub mod native;
pub mod registry;
pub mod throttle;
pub mod urls;
pub mod yt_dlp;

/// Browser User-Agent for site scraping.
pub(crate) const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

/// Client for retrying failed connections: IPv4 only (a bound 0.0.0.0 local
/// address) and HTTP/1.1 only. Some networks resolve a site's IPv6 address
/// but cannot reach it, or break HTTP/2, and fail with "error sending
/// request" while browsers (and yt-dlp) quietly fall back.
pub(crate) fn fallback_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(BROWSER_UA)
        .http1_only()
        .local_address(Some(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)))
        .connect_timeout(std::time::Duration::from_secs(15))
}

/// Transport-level failure worth one retry on the fallback client.
pub(crate) fn is_transport_error(e: &reqwest::Error) -> bool {
    e.is_connect() || e.is_timeout() || e.is_request()
}

#[derive(Clone)]
pub struct SiteContext {
    pub client: reqwest::Client,
    fallback: reqwest::Client,
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
            .user_agent(BROWSER_UA)
            .cookie_store(true)
            .build()?;
        let fallback = fallback_client_builder().cookie_store(true).build()?;
        Ok(Self {
            client,
            fallback,
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

    /// Whether the vault holds cookies under this key.
    pub fn has_cookies(&self, key: &str) -> bool {
        self.vault
            .cookie_header(key)
            .ok()
            .flatten()
            .is_some_and(|h| !h.is_empty())
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
        let cookie = self
            .vault
            .cookie_header(site_id)
            .ok()
            .flatten()
            .filter(|h| !h.is_empty());
        // Look like a browser navigating the site: the headers Chrome sends
        // for a page load, and the previous page on the same site as Referer.
        let referer = throttle::last_page(url);
        let has = |name: &str| {
            extra
                .unwrap_or_default()
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name))
        };
        // Callers asking for JSON etc. are XHR-style requests, not page loads.
        let xhr = extra
            .unwrap_or_default()
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Accept") && !v.contains("text/html"));
        let mut defaults: Vec<(&str, &str)> = vec![
            (
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
            ),
            ("Accept-Language", "en-US,en;q=0.9"),
            (
                "Sec-Fetch-Site",
                if referer.is_some() { "same-origin" } else { "none" },
            ),
        ];
        if xhr {
            defaults.extend([("Sec-Fetch-Dest", "empty"), ("Sec-Fetch-Mode", "cors")]);
        } else {
            defaults.extend([
                ("Upgrade-Insecure-Requests", "1"),
                ("Sec-Fetch-Dest", "document"),
                ("Sec-Fetch-Mode", "navigate"),
            ]);
        }
        if let Some(r) = referer.as_deref() {
            defaults.push(("Referer", r));
        }
        defaults.retain(|(k, _)| !has(k));
        let build = |client: &reqwest::Client| {
            let mut req = client.get(url);
            if let Some(header) = cookie.as_deref() {
                req = req.header("Cookie", header);
            }
            for (k, v) in defaults.iter().chain(extra.unwrap_or_default()) {
                req = req.header(*k, *v);
            }
            req
        };
        throttle::acquire(url).await?;
        let resp = match build(&self.client).send().await {
            Ok(resp) => resp,
            Err(e) if is_transport_error(&e) => {
                tracing::warn!("[{site_id}] {e}; retrying over IPv4 / HTTP/1.1");
                build(&self.fallback).send().await?
            }
            Err(e) => return Err(e.into()),
        };
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok());
            throttle::note_rate_limited(url, retry_after);
            return Err(AppError::Site(format!(
                "{site_id} is rate limiting requests (HTTP 429); wait a minute and retry"
            )));
        }
        if !resp.status().is_success() {
            return Err(AppError::Site(format!("HTTP {} for {url}", resp.status())));
        }
        throttle::note_ok(url);
        throttle::note_page(url);
        Ok(resp.text().await?)
    }

    /// JSON room-list style APIs (Chaturbate / Stripchat ts endpoints).
    pub async fn fetch_json_api(
        &self,
        url: &str,
        site_id: &str,
        referer: &str,
    ) -> AppResult<String> {
        tracing::info!("[{}] fetch_json_api: {}", site_id, url);
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

    /// Sort / filter options this site understands (see `BrowseQuery::filters`).
    fn browse_options(&self) -> BrowseOptions {
        BrowseOptions::default()
    }

    /// The site's category / tag index. Only called when
    /// `browse_options().has_categories` is true.
    async fn list_categories(&self, _ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
        Ok(Vec::new())
    }

    /// Drop any copy of the category / tag index the adapter keeps itself,
    /// so the next `list_categories` / `list_category_group` asks the site
    /// (the user tapped Refresh).
    fn forget_category_cache(&self) {}

    /// One section of a grouped index (`browse_options().category_groups`).
    async fn list_category_group(
        &self,
        _ctx: &SiteContext,
        _group: &str,
    ) -> AppResult<Vec<SiteCategory>> {
        Ok(Vec::new())
    }

    /// Search phrases the site suggests for `text` (sites with
    /// `browse_options().remote_suggest`).
    async fn suggest(
        &self,
        _ctx: &SiteContext,
        _text: &str,
        _orientation: Option<BrowseOrientation>,
    ) -> AppResult<Vec<String>> {
        Ok(Vec::new())
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage>;

    async fn resolve_download(
        &self,
        ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan>;

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String>;
}
