use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use reqwest::Client;

/// Browser UA so oEmbed / HEAD probes aren't rejected by bot filters.
/// The old "ArcHive/0.1" token got blocked by several hosts.
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36";

/// Subset of the YouTube oEmbed response (no API key needed).
#[derive(Debug, serde::Deserialize)]
struct OEmbed {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author_name: Option<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
}

pub async fn resolve(url: &str) -> AppResult<MediaItem> {
    let client = Client::builder().user_agent(USER_AGENT).build()?;

    if url.contains("youtube.com") || url.contains("youtu.be") {
        let (title, channel, thumbnail) = fetch_youtube_oembed(&client, url).await;
        return Ok(MediaItem {
            id: url.to_string(),
            title: title.unwrap_or_else(|| extract_title_from_url(url)),
            url: url.to_string(),
            thumbnail,
            duration: None,
            site_id: "youtube".to_string(),
            performers: vec![],
            tags: vec![],
            description: None,
            channel,
            is_live: None,
            viewers: None,
            age: None,
            gender: None,
            stream_url: None,
            embed_url: None,
        });
    }

    if is_direct_media(url) {
        let head = client.head(url).send().await;
        if head.is_ok() {
            return Ok(MediaItem {
                id: url.to_string(),
                title: extract_title_from_url(url),
                url: url.to_string(),
                thumbnail: None,
                duration: None,
                site_id: "direct".to_string(),
                performers: vec![],
                tags: vec![],
                description: None,
                channel: None,
                is_live: None,
                viewers: None,
                age: None,
                gender: None,
                // Direct URLs play in the in-app player without resolving.
                stream_url: Some(url.to_string()),
                embed_url: None,
            });
        }
    }

    Err(AppError::Site(
        "Standalone mode supports YouTube and direct media URLs only. Use LAN mode for full site support.".to_string(),
    ))
}

/// YouTube oEmbed (https://www.youtube.com/oembed) returns title, author and
/// thumbnail without an API key. Best-effort: any failure yields Nones and the
/// caller falls back to URL-derived placeholders.
async fn fetch_youtube_oembed(
    client: &Client,
    url: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    // oEmbed expects the *watch* URL; youtu.be shorts still resolve.
    let endpoint = format!(
        "https://www.youtube.com/oembed?url={}&format=json",
        urlencoding(url)
    );
    let resp = client.get(&endpoint).send().await;
    let Ok(resp) = resp else {
        return (None, None, None);
    };
    if !resp.status().is_success() {
        return (None, None, None);
    }
    let parsed: Result<OEmbed, _> = resp.json().await;
    match parsed {
        Ok(o) => (o.title, o.author_name, o.thumbnail_url),
        Err(_) => (None, None, None),
    }
}

/// Minimal percent-encoding for the oEmbed `url` query value.
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn is_direct_media(url: &str) -> bool {
    let lower = url.to_lowercase();
    [".mp4", ".webm", ".mkv", ".m3u8"]
        .iter()
        .any(|ext| lower.contains(ext))
}

fn extract_title_from_url(url: &str) -> String {
    // For watch URLs the last path segment is "watch" — prefer the `v` param.
    if let Some(q) = url.split('?').nth(1) {
        for pair in q.split('&') {
            if let Some(v) = pair.strip_prefix("v=") {
                if !v.is_empty() {
                    return format!("YouTube video {v}");
                }
            }
        }
    }
    url.split('/')
        .next_back()
        .unwrap_or("media")
        .split('?')
        .next()
        .unwrap_or("media")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_url_title_prefers_video_id() {
        assert_eq!(
            extract_title_from_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            "YouTube video dQw4w9WgXcQ"
        );
    }

    #[test]
    fn direct_url_title_uses_last_segment() {
        assert_eq!(
            extract_title_from_url("https://cdn.example.com/clips/ep12.mp4?token=abc"),
            "ep12.mp4"
        );
    }

    #[test]
    fn urlencoding_leaves_unreserved_chars() {
        assert_eq!(
            urlencoding("https://www.youtube.com/watch?v=abc"),
            "https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3Dabc"
        );
    }
}
