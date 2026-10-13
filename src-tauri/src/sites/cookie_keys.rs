//! Which cookie-vault entry a URL uses.
//!
//! Sites with their own adapter keep their adapter id (`pornhub`, …). Any
//! other site — xPomelo sources such as SpankBang or FapCat, pasted URLs —
//! gets a per-domain entry, `host-<domain>` (`host-spankbang-com`), filled
//! by Settings → Cookies (login browser / import) or by a bot-check
//! clearance on desktop. yt-dlp (desktop and the Android engine), the KVS
//! resolver and direct downloads all read cookies through this.

use crate::sites::registry::SiteRegistry;

/// Adapter ids that don't own a site's cookies (they front many sites).
const SHARED_IDS: &[&str] = &["generic_ytdlp", "custom", "xpomelo"];

/// `https://www.SpankBang.com/x` or `spankbang.com` → `host-spankbang-com`.
pub fn host_key(url_or_domain: &str) -> Option<String> {
    let host = if url_or_domain.contains("://") {
        url::Url::parse(url_or_domain).ok()?.host_str()?.to_string()
    } else {
        url_or_domain
            .trim()
            .trim_end_matches('/')
            .split('/')
            .next()?
            .to_string()
    };
    let host = host.to_ascii_lowercase();
    let host = host.trim_start_matches("www.");
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    let slug: String = host
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    Some(format!("host-{slug}"))
}

/// True for a key made by [`host_key`].
pub fn is_host_key(key: &str) -> bool {
    key.starts_with("host-")
}

/// The vault key for `url`, given the adapter id that handles it.
pub fn key_for(adapter_id: &str, url: &str) -> String {
    if SHARED_IDS.contains(&adapter_id) {
        host_key(url).unwrap_or_else(|| adapter_id.to_string())
    } else {
        adapter_id.to_string()
    }
}

/// The vault key for `url` (adapter detected from the URL).
pub fn key_for_url(sites: &SiteRegistry, url: &str) -> String {
    let id = sites
        .detect(url)
        .unwrap_or_else(|| "generic_ytdlp".to_string());
    let known = sites.get(&id).is_some() && !SHARED_IDS.contains(&id.as_str());
    if known {
        id
    } else {
        host_key(url).unwrap_or(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(
            host_key("https://www.SpankBang.com/8va69/video/x").as_deref(),
            Some("host-spankbang-com")
        );
        assert_eq!(host_key("4kporn.xxx").as_deref(), Some("host-4kporn-xxx"));
        assert_eq!(host_key("hoes.tube/").as_deref(), Some("host-hoes-tube"));
        assert_eq!(host_key("localhost"), None);
        assert_eq!(
            key_for("xpomelo", "https://fapcat.com/v/1"),
            "host-fapcat-com"
        );
        assert_eq!(
            key_for("pornhub", "https://www.pornhub.com/view"),
            "pornhub"
        );
        let sites = SiteRegistry::new();
        assert_eq!(
            key_for_url(&sites, "https://www.pornhub.com/view_video.php?viewkey=1"),
            "pornhub"
        );
        assert_eq!(
            key_for_url(&sites, "https://crazyporn.xxx/videos/1/x/"),
            "host-crazyporn-xxx"
        );
        assert!(is_host_key("host-crazyporn-xxx"));
    }
}
