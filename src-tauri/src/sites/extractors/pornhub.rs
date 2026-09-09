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
/// Returns `Ok(None)` when no playable URL is found so callers can fall back
/// to yt-dlp. Network/HTTP errors are propagated as `Err`.
pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "pornhub").await?;
    Ok(extract_video_url(&html))
}

fn extract_video_url(html: &str) -> Option<String> {
    let mut candidates: Vec<(i32, String)> = Vec::new();

    // 1. flashvars mediaDefinitions: "videoUrl":"https:\/\/...mp4..."
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
                if looks_playable(&cleaned) {
                    candidates.push((quality, cleaned));
                }
            }
        }
    }

    // 2. Plain <source src="...mp4"> tags.
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

    // 3. og:video meta fallback.
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

    // Dedupe, then pick highest quality score (stable: first wins ties).
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen = std::collections::HashSet::new();
    for (_, url) in candidates {
        if seen.insert(url.clone()) {
            return Some(url);
        }
    }
    None
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
}
