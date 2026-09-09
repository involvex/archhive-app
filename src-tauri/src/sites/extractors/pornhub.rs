use crate::error::AppResult;
use crate::sites::SiteContext;
use regex::Regex;

/// Best-effort direct MP4 extraction for PornHub watch pages.
///
/// PornHub watch pages (`/view_video.php?viewkey=...`) embed playback URLs in
/// `flashvars` / `mediaDefinitions` as `"videoUrl":"https:\/\/..."` entries
/// (usually one per quality). This extractor collects those candidates plus
/// `<source src="...mp4">` and `og:video` fallbacks, unescapes them, and
/// returns the highest-quality MP4 URL.
///
/// HLS playlist URLs are deliberately excluded: they are neither directly
/// playable nor downloadable via plain HTTP (see [`extract_hls_url`]).
///
/// Returns `Ok(None)` when no playable URL is found so callers can fall back
/// to yt-dlp. Network/HTTP errors are propagated as `Err`.
pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "pornhub").await?;
    Ok(extract_video_url(&html))
}

/// Best-effort HLS playlist extraction for PornHub watch pages.
///
/// Same sources as [`extract_download_url`], but returns the highest-quality
/// `.m3u8` URL instead. Meant for download via ffmpeg (`-c copy`) with
/// Referer/Cookie headers — plain HTTP fetch would only retrieve playlists.
pub async fn extract_hls_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "pornhub").await?;
    Ok(extract_playlist_url(&html))
}

fn extract_video_url(html: &str) -> Option<String> {
    pick_best(&collect_candidates(html, false))
}

fn extract_playlist_url(html: &str) -> Option<String> {
    pick_best(&collect_candidates(html, true))
}

/// Shared candidate collection. `hls` selects `.m3u8` playlist URLs,
/// otherwise direct MP4 URLs.
fn collect_candidates(html: &str, hls: bool) -> Vec<(i32, String)> {
    let mut candidates: Vec<(i32, String)> = Vec::new();

    // 1. flashvars mediaDefinitions: "videoUrl":"https:\/\/..."
    // Capture with surrounding context so we can rank by nearby quality label.
    if let Ok(re) = Regex::new(r#""videoUrl"\s*:\s*"((?:\\.|[^"\\])+)"#) {
        for caps in re.captures_iter(html) {
            let raw = match caps.get(1) {
                Some(m) => m.as_str(),
                None => continue,
            };
            let start = caps.get(0).map(|m| m.start()).unwrap_or(0);
            let context_start = start.saturating_sub(300);
            let quality = quality_score(&html[context_start..start], raw);
            if let Some(cleaned) = clean_url(raw) {
                if wanted(&cleaned, hls) {
                    candidates.push((quality, cleaned));
                }
            }
        }
    }

    // 2. Plain <source src="..."> tags (mp4 mode only; playlists are
    // collected from videoUrl entries above).
    if !hls {
        if let Ok(re) = Regex::new(r#"(?i)<source[^>]+src\s*=\s*"([^"]+\.mp4[^"]*)""#) {
            for caps in re.captures_iter(html) {
                if let Some(m) = caps.get(1) {
                    if let Some(cleaned) = clean_url(m.as_str()) {
                        if looks_playable(&cleaned) {
                            candidates.push((quality_score(m.as_str(), m.as_str()), cleaned));
                        }
                    }
                }
            }
        }
    }

    // 3. og:video meta fallback (mp4 mode only).
    if !hls {
        if let Ok(re) =
            Regex::new(r#"(?i)<meta[^>]+property\s*=\s*"og:video"[^>]+content\s*=\s*"([^"]+)""#)
        {
            if let Some(caps) = re.captures(html) {
                if let Some(m) = caps.get(1) {
                    if let Some(cleaned) = clean_url(m.as_str()) {
                        if looks_playable(&cleaned) {
                            candidates.push((0, cleaned));
                        }
                    }
                }
            }
        }
    }

    candidates
}

/// Dedupe candidates, then pick the highest quality score.
/// Stable: first wins ties.
fn pick_best(candidates: &[(i32, String)]) -> Option<String> {
    let mut ranked = candidates.to_vec();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen = std::collections::HashSet::new();
    for (_, url) in ranked {
        if seen.insert(url.clone()) {
            return Some(url);
        }
    }
    None
}

/// Mode gate: HLS mode accepts only `.m3u8` playlists, MP4 mode accepts
/// direct video URLs and rejects playlists (a playlist fetched over plain
/// HTTP is useless to both the player and the direct downloader).
fn wanted(url: &str, hls: bool) -> bool {
    let lower = url.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    if hls {
        return lower.contains(".m3u8");
    }
    looks_playable(url)
}

fn quality_score(context: &str, url: &str) -> i32 {
    let hay = format!("{context} {url}");
    for (label, score) in [
        ("2160", 60),
        ("1440", 50),
        ("1080", 40),
        ("720", 30),
        ("480", 20),
        ("360", 10),
        ("240", 5),
    ] {
        if hay.contains(label) {
            return score;
        }
    }
    0
}

fn looks_playable(url: &str) -> bool {
    let lower = url.to_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && (lower.contains(".mp4") || lower.contains("videoUrl") || lower.contains("/videos/"))
        && !lower.contains(".m3u8")
        && !lower.contains("preview")
        && !lower.contains("trailer")
        && !lower.contains("sprite")
        && !lower.contains(".jpg")
        && !lower.contains(".png")
        && !lower.contains(".gif")
}

fn clean_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Prefer proper JSON unescaping (handles \/ \u0026 etc.).
    if let Ok(unescaped) = serde_json::from_str::<String>(&format!("\"{trimmed}\"")) {
        let fixed = unescaped.replace("&amp;", "&");
        if !fixed.trim().is_empty() {
            return Some(fixed);
        }
    }
    let fixed = trimmed
        .replace("\\/", "/")
        .replace("\\u0026", "&")
        .replace("\\u0026amp;", "&")
        .replace("&amp;", "&")
        .replace("\\\\", "\\");
    if fixed.trim().is_empty() {
        return None;
    }
    Some(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_highest_quality_video_url() {
        let html = r#"
            {"format":"mp4","quality":"480","videoUrl":"https:\/\/cdn.phncdn.com\/videos\/a480.mp4?x=1\u0026y=2"}
            {"format":"mp4","quality":"720","videoUrl":"https:\/\/cdn.phncdn.com\/videos\/a720.mp4?x=1\u0026y=2"}
        "#;
        let got = extract_video_url(html).unwrap();
        assert!(got.contains("a720.mp4"), "got: {got}");
        assert!(got.contains("?x=1&y=2"), "got: {got}");
        assert!(!got.contains("\\/"), "got: {got}");
    }

    #[test]
    fn falls_back_to_source_tag() {
        let html =
            r#"<video><source src="https://cdn.example.com/v.mp4?token=abc&amp;x=1"></video>"#;
        let got = extract_video_url(html).unwrap();
        assert_eq!(got, "https://cdn.example.com/v.mp4?token=abc&x=1");
    }

    #[test]
    fn falls_back_to_og_video() {
        let html = r#"<meta property="og:video" content="https://cdn.example.com/og.mp4">"#;
        let got = extract_video_url(html).unwrap();
        assert_eq!(got, "https://cdn.example.com/og.mp4");
    }

    #[test]
    fn ignores_preview_and_images() {
        let html = r#"
            {"videoUrl":"https://cdn.example.com/preview.mp4"}
            {"videoUrl":"https://cdn.example.com/sprite.jpg"}
        "#;
        assert!(extract_video_url(html).is_none());
    }

    #[test]
    fn returns_none_when_nothing_found() {
        assert!(extract_video_url("<html><body>no video here</body></html>").is_none());
    }

    #[test]
    fn mp4_mode_rejects_playlists() {
        let html = r#"{"format":"hls","quality":"1080","videoUrl":"https:\/\/cdn.phncdn.com\/videos\/master.m3u8?h=1"}"#;
        assert!(extract_video_url(html).is_none());
    }

    #[test]
    fn hls_mode_picks_playlist_and_ignores_mp4() {
        let html = r#"
            {"format":"mp4","quality":"720","videoUrl":"https:\/\/cdn.phncdn.com\/videos\/a720.mp4"}
            {"format":"hls","quality":"1080","videoUrl":"https:\/\/cdn.phncdn.com\/videos\/master.m3u8?h=1"}
        "#;
        let got = extract_playlist_url(html).unwrap();
        assert!(got.contains("master.m3u8"), "got: {got}");
    }

    #[test]
    fn hls_mode_returns_none_without_playlists() {
        let html =
            r#"{"format":"mp4","quality":"720","videoUrl":"https://cdn.example.com/a720.mp4"}"#;
        assert!(extract_playlist_url(html).is_none());
    }
}
