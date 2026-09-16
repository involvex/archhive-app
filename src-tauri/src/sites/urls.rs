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
            "https://chaturbate.com/embed/{username}/?embed_video_only=1&autoplay=1"
        );
    }
    if let Some(username) = extract_username_from_url(url, "stripchat.com") {
        return format!("https://stripchat.com/embed/{username}/");
    }
    url.to_string()
}

/// Pick a playable stream URL from yt-dlp `--get-url` output.
///
/// Live cams must resolve to a **single muxed** HLS URL. When yt-dlp emits
/// separate video + audio playlists (e.g. `-f bv*+ba`), taking the first
/// `.m3u8` yields video-only media and Android greys out the volume control.
pub fn pick_stream_url_from_ytdlp(raw: &str) -> crate::error::AppResult<String> {
    use crate::error::AppError;
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && (l.starts_with("http://") || l.starts_with("https://")))
        .collect();
    if lines.is_empty() {
        return Err(AppError::Other("No stream URL resolved".to_string()));
    }

    let m3u8s: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.to_ascii_lowercase().contains(".m3u8"))
        .collect();

    if m3u8s.len() == 1 {
        return Ok(m3u8s[0].to_string());
    }

    if m3u8s.len() > 1 {
        // Prefer an explicit master / index playlist (usually includes audio).
        if let Some(master) = m3u8s.iter().find(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("master") || lower.contains("index") || lower.contains("playlist.m3u8")
        }) {
            return Ok((*master).to_string());
        }
        // Skip obvious audio-only sibling playlists from bv+ba splits.
        if let Some(av) = m3u8s.iter().find(|l| {
            let lower = l.to_ascii_lowercase();
            !lower.contains("audio")
                && !lower.contains("/a/")
                && !lower.contains("_audio")
                && !lower.contains("-audio")
        }) {
            return Ok((*av).to_string());
        }
        return Ok(m3u8s[0].to_string());
    }

    Ok(lines[0].to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_prefers_m3u8() {
        let raw = "https://cdn.example/video-only.ts\nhttps://cdn.example/master.m3u8\n";
        let url = pick_stream_url_from_ytdlp(raw).unwrap();
        assert!(url.contains("master.m3u8"));
    }

    #[test]
    fn pick_skips_audio_only_sibling() {
        let raw = "\
https://cdn.example/chunklist_v.m3u8
https://cdn.example/chunklist_audio.m3u8
";
        let url = pick_stream_url_from_ytdlp(raw).unwrap();
        assert!(url.contains("chunklist_v.m3u8"));
        assert!(!url.contains("audio"));
    }

    #[test]
    fn chaturbate_embed_allows_sound() {
        let u = derive_embed_url("https://chaturbate.com/someuser/");
        assert!(!u.contains("disable_sound"));
        assert!(u.contains("embed_video_only=1"));
    }
}
