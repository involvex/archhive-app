use crate::models::SiteInfo;
use crate::sites::adapters::{
    custom::CustomUrlAdapter, generic_ytdlp::GenericYtDlpAdapter, reddit::RedditAdapter,
    redgifs::RedgifsAdapter, thothub::ThotHubAdapter, PornhubAdapter, XHamsterAdapter,
    XVideosAdapter, XnxxAdapter, YouPornAdapter,
};
use crate::sites::SiteAdapter;
use std::sync::Arc;

pub struct SiteRegistry {
    adapters: Vec<Arc<dyn SiteAdapter>>,
}

impl SiteRegistry {
    pub fn new() -> Self {
        let adapters: Vec<Arc<dyn SiteAdapter>> = vec![
            Arc::new(ThotHubAdapter),
            Arc::new(PornhubAdapter),
            Arc::new(YouPornAdapter),
            Arc::new(XnxxAdapter),
            Arc::new(XHamsterAdapter),
            Arc::new(XVideosAdapter),
            Arc::new(RedditAdapter),
            Arc::new(RedgifsAdapter),
            Arc::new(CustomUrlAdapter),
            Arc::new(GenericYtDlpAdapter::youtube()),
            Arc::new(GenericYtDlpAdapter::tiktok()),
            Arc::new(GenericYtDlpAdapter::instagram()),
            Arc::new(GenericYtDlpAdapter::twitter()),
            Arc::new(GenericYtDlpAdapter::thisvid()),
        ];
        Self { adapters }
    }

    pub fn list(&self) -> Vec<SiteInfo> {
        self.adapters
            .iter()
            .map(|a| SiteInfo {
                id: a.id().to_string(),
                display_name: a.display_name().to_string(),
                base_url: a.base_url().to_string(),
                supported_kinds: a.supported_kinds(),
                requires_cookies: a.requires_cookies(),
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn SiteAdapter>> {
        self.adapters.iter().find(|a| a.id() == id).cloned()
    }

    pub fn detect(&self, url: &str) -> Option<String> {
        let lower = url.to_lowercase();
        if lower.contains("youporn.com") {
            return Some("youporn".to_string());
        }
        if lower.contains("xnxx.com") {
            return Some("xnxx".to_string());
        }
        if lower.contains("instagram.com") {
            return Some("instagram".to_string());
        }
        if lower.contains("thethothub.com") || lower.contains("thothub.to") {
            return Some("thothub".to_string());
        }
        if lower.contains("youtube.com") || lower.contains("youtu.be") {
            return Some("youtube".to_string());
        }
        if lower.contains("tiktok.com") {
            return Some("tiktok".to_string());
        }
        if lower.contains("twitter.com") || lower.contains("x.com/") || lower.contains("//x.com") {
            return Some("twitter".to_string());
        }
        if lower.contains("reddit.com") || lower.contains("i.redd.it") || lower.contains("redd.it")
        {
            return Some("reddit".to_string());
        }
        if lower.contains("xvideos.com") {
            return Some("xvideos".to_string());
        }
        if lower.contains("xhamster.com") {
            return Some("xhamster".to_string());
        }
        if lower.contains("pornhub.com") {
            return Some("pornhub".to_string());
        }
        for adapter in &self.adapters {
            if lower.contains(&adapter.id().to_lowercase()) {
                return Some(adapter.id().to_string());
            }
        }
        Some("generic_ytdlp".to_string())
    }
}
