/// Path segment: spaces to hyphens, lowercased (pornstar/model URLs).
pub fn path_slug(slug: &str) -> String {
    slug.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Query string value (search terms with spaces).
pub fn query_slug(slug: &str) -> String {
    url::form_urlencoded::byte_serialize(slug.trim().as_bytes()).collect()
}

/// Derive an embed URL from a live cam room URL.
/// For Chaturbate, appends `embed_video_only=1` so the iframe plays video
/// directly (no chat) as a fallback when yt-dlp stream resolution fails.
/// Returns the original URL if no known pattern matches.
pub fn derive_embed_url(url: &str) -> String {
    if let Some(username) = extract_username_from_url(url, "chaturbate.com") {
        return format!(
            "https://chaturbate.com/embed/{username}/?embed_video_only=1&disable_sound=1&autoplay=1"
        );
    }
    if let Some(username) = extract_username_from_url(url, "stripchat.com") {
        return format!("https://stripchat.com/embed/{username}/");
    }
    url.to_string()
}

fn extract_username_from_url(url: &str, domain: &str) -> Option<String> {
    if !url.contains(domain) {
        return None;
    }
    let username = url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or("");
    if username.is_empty() || username.contains('?') {
        return None;
    }
    Some(username.to_string())
}
