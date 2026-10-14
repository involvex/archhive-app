use crate::models::SiteInfo;
use crate::sites::adapters::{
    chaturbate::ChaturbateAdapter, custom::CustomUrlAdapter, eporner::EpornerAdapter,
    generic_ytdlp::GenericYtDlpAdapter, pornwitch::PornwitchAdapter, reddit::RedditAdapter,
    redgifs::RedgifsAdapter, stripchat::StripchatAdapter, thothub::ThotHubAdapter,
    xpomelo::XpomeloAdapter, PornhubAdapter, XHamsterAdapter, XVideosAdapter, XnxxAdapter,
    YouPornAdapter,
};
use crate::sites::SiteAdapter;
use std::sync::Arc;

pub struct SiteRegistry {
    adapters: Vec<Arc<dyn SiteAdapter>>,
}

impl SiteRegistry {
    pub fn new() -> Self {
        let adapters: Vec<Arc<dyn SiteAdapter>> = vec![
            Arc::new(ChaturbateAdapter),
            Arc::new(StripchatAdapter),
            Arc::new(ThotHubAdapter),
            Arc::new(EpornerAdapter),
            Arc::new(PornhubAdapter),
            Arc::new(YouPornAdapter),
            Arc::new(XnxxAdapter),
            Arc::new(XHamsterAdapter),
            Arc::new(XVideosAdapter),
            Arc::new(XpomeloAdapter),
            Arc::new(PornwitchAdapter),
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
                browse_options: a.browse_options(),
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn SiteAdapter>> {
        self.adapters.iter().find(|a| a.id() == id).cloned()
    }

    /// The adapter for `url`, matched on its host (subdomains included).
    /// Substring matching misfired: `anysex.com/` contains `x.com/`, and a
    /// video slug mentioning "reddit" isn't a Reddit link.
    pub fn detect(&self, url: &str) -> Option<String> {
        let host = url::Url::parse(url.trim())
            .ok()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
            .unwrap_or_default();
        let on = |domain: &str| host == domain || host.ends_with(&format!(".{domain}"));
        const KNOWN: &[(&str, &[&str])] = &[
            ("youporn", &["youporn.com"]),
            ("xnxx", &["xnxx.com"]),
            ("instagram", &["instagram.com"]),
            ("thothub", &["thethothub.com", "thothub.to"]),
            ("youtube", &["youtube.com", "youtu.be"]),
            ("tiktok", &["tiktok.com"]),
            ("twitter", &["twitter.com", "x.com"]),
            ("reddit", &["reddit.com", "redd.it"]),
            ("xvideos", &["xvideos.com"]),
            (
                "xhamster",
                &["xhamster.com", "xhamster.desi", "xhamster.one"],
            ),
            ("eporner", &["eporner.com"]),
            ("pornhub", &["pornhub.com"]),
            ("chaturbate", &["chaturbate.com"]),
            ("stripchat", &["stripchat.com"]),
        ];
        if !host.is_empty() {
            for (id, domains) in KNOWN {
                if domains.iter().any(|d| on(d)) {
                    return Some((*id).to_string());
                }
            }
            for adapter in &self.adapters {
                let base = url::Url::parse(adapter.base_url()).ok().and_then(|u| {
                    u.host_str()
                        .map(|h| h.trim_start_matches("www.").to_string())
                });
                if base.is_some_and(|b| on(&b)) {
                    return Some(adapter.id().to_string());
                }
            }
        }
        Some("generic_ytdlp".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_matches_hosts_not_substrings() {
        let r = SiteRegistry::new();
        let d = |u: &str| r.detect(u).unwrap();
        assert_eq!(d("https://anysex.com/video/1/x/"), "generic_ytdlp");
        assert_eq!(
            d("https://www.fapcat.com/videos/1/reddit-girl/"),
            "generic_ytdlp"
        );
        assert_eq!(d("https://x.com/user/status/1"), "twitter");
        assert_eq!(d("https://mobile.twitter.com/u"), "twitter");
        assert_eq!(d("https://de.xhamster.com/videos/x"), "xhamster");
        assert_eq!(d("https://i.redd.it/abc.jpg"), "reddit");
        assert_eq!(d("https://thisvid.com/videos/x/"), "thisvid");
        assert_eq!(d("https://www.redgifs.com/watch/x"), "redgifs");
        assert_eq!(d("https://xpomelo.com/category/x"), "xpomelo");
        assert_eq!(d("https://pornwitch.com/search/hottest"), "pornwitch");
        assert_eq!(
            d("https://www.pornhub.com/view_video.php?viewkey=1"),
            "pornhub"
        );
        assert_eq!(d("not a url"), "generic_ytdlp");
    }
}
