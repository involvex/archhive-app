/// Classify URLs for bulk import (mirrors frontend `parseBulkUrls.ts`).

pub fn is_browse_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url.trim()) else {
        return false;
    };
    let path = parsed.path().to_lowercase();
    if path.contains("/search") || path.contains("/channel/") || path.contains("/channels/") {
        return true;
    }
    if parsed.query_pairs().any(|(k, _)| k == "query") {
        return true;
    }
    false
}

pub fn is_likely_video_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url.trim()) else {
        return false;
    };
    if is_browse_url(url) {
        return false;
    }
    let path = parsed.path().to_lowercase();
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    if path.contains("/watch") || path.contains("/video") {
        return true;
    }
    if host.contains("youtu.be") {
        return true;
    }
    if host.contains("youtube.com") && parsed.query_pairs().any(|(k, _)| k == "v") {
        return true;
    }
    if host.contains("redgifs.com") && path.len() > 1 {
        return true;
    }
    false
}
