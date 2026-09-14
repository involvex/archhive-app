use crate::error::AppResult;
use crate::sites::SiteContext;
use regex::Regex;

/// Best-effort direct MP4 extraction for PornHub watch pages.
///
/// PornHub watch pages (`/view_video.php?viewkey=...`) embed playback URLs in
/// `flashvars` / `mediaDefinitions` as `"videoUrl":"https:\/\/..."` entries
/// (usually one per quality). Many entries are `/video/get_media?...` endpoints
/// that return a JSON array of real CDN URLs — those are followed here.
///
/// HLS playlist URLs are deliberately excluded: they are neither directly
/// playable nor downloadable via plain HTTP (see [`extract_hls_url`]).
///
/// Returns `Ok(None)` when no playable URL is found so callers can fall back
/// to yt-dlp. Network/HTTP errors are propagated as `Err`.
///
/// Used from the PornHub adapter under `#[cfg(mobile)]`.
#[cfg_attr(not(mobile), allow(dead_code))]
pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "pornhub").await?;
    let expanded = expand_candidates(ctx, &collect_candidates(&html, false)).await;
    Ok(pick_best_filtered(&expanded, false))
}

/// Best-effort HLS playlist extraction for PornHub watch pages.
///
/// Same sources as [`extract_download_url`], but returns the highest-quality
/// `.m3u8` URL instead. Meant for download via ffmpeg (`-c copy`) with
/// Referer/Cookie headers — plain HTTP fetch would only retrieve playlists.
#[cfg_attr(not(mobile), allow(dead_code))]
pub async fn extract_hls_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "pornhub").await?;
    // Collect both direct m3u8 and get_media entries, then keep playlists.
    let mut seed = collect_candidates(&html, true);
    seed.extend(
        collect_candidates(&html, false)
            .into_iter()
            .filter(|(_, u)| is_get_media(u)),
    );
    let expanded = expand_candidates(ctx, &seed).await;
    Ok(pick_best_filtered(&expanded, true))
}

fn extract_video_url(html: &str) -> Option<String> {
    pick_best(&collect_candidates(html, false))
}

fn extract_playlist_url(html: &str) -> Option<String> {
    pick_best(&collect_candidates(html, true))
}

/// Follow `/video/get_media` JSON endpoints and keep direct CDN URLs as-is.
#[cfg_attr(not(mobile), allow(dead_code))]
async fn expand_candidates(
    ctx: &SiteContext,
    candidates: &[(i32, String)],
) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = Vec::new();
    for (quality, url) in candidates {
        if is_get_media(url) {
            match expand_get_media(ctx, url).await {
                Ok(more) if !more.is_empty() => out.extend(more),
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!("pornhub get_media expand failed for {url}: {e}");
                }
            }
        } else {
            out.push((*quality, url.clone()));
        }
    }
    out
}

#[cfg_attr(not(mobile), allow(dead_code))]
async fn expand_get_media(ctx: &SiteContext, url: &str) -> AppResult<Vec<(i32, String)>> {
    let raw = ctx
        .fetch_with_headers(
            url,
            "pornhub",
            Some(&[
                ("Accept", "application/json,text/plain,*/*"),
                ("Referer", "https://www.pornhub.com/"),
                ("X-Requested-With", "XMLHttpRequest"),
            ]),
        )
        .await?;
    Ok(parse_get_media_json(&raw))
}

fn parse_get_media_json(raw: &str) -> Vec<(i32, String)> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let items = match value {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Object(map) => map
            .get("mediaDefinitions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(video_url) = obj
            .get("videoUrl")
            .or_else(|| obj.get("video_url"))
            .and_then(|v| v.as_str())
            .and_then(clean_url)
        else {
            continue;
        };
        if is_get_media(&video_url) {
            continue;
        }
        let quality = obj
            .get("quality")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .or_else(|| obj.get("height").and_then(|v| v.as_i64()))
            .unwrap_or(0) as i32;
        let score = if quality > 0 {
            quality
        } else {
            quality_score("", &video_url)
        };
        // Keep both MP4 and HLS from get_media; callers filter via collect mode.
        out.push((score, video_url));
    }
    out
}

/// Shared candidate collection. `hls` selects `.m3u8` playlist URLs,
/// otherwise direct video URLs and get_media endpoints (expanded later).
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
            // After get_media expansion, re-apply mode filter via caller context:
            // pick_best is shared; filter non-http.
            if url.starts_with("http://") || url.starts_with("https://") {
                return Some(url);
            }
        }
    }
    None
}

fn pick_best_filtered(candidates: &[(i32, String)], hls: bool) -> Option<String> {
    let filtered: Vec<(i32, String)> = candidates
        .iter()
        .filter(|(_, u)| {
            if hls {
                u.to_lowercase().contains(".m3u8")
            } else {
                looks_playable(u)
            }
        })
        .cloned()
        .collect();
    pick_best(&filtered)
}

/// Mode gate: HLS mode accepts only `.m3u8` playlists (and get_media that may
/// expand to them). MP4 mode accepts progressive URLs + get_media endpoints.
fn wanted(url: &str, hls: bool) -> bool {
    let lower = url.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    if is_get_media(url) {
        // Expand later; keep for both modes.
        return true;
    }
    if hls {
        return lower.contains(".m3u8");
    }
    looks_playable(url)
}

fn is_get_media(url: &str) -> bool {
    url.to_lowercase().contains("/video/get_media")
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
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    if lower.contains(".m3u8")
        || lower.contains("preview")
        || lower.contains("trailer")
        || lower.contains("sprite")
        || lower.contains(".jpg")
        || lower.contains(".png")
        || lower.contains(".gif")
        || is_get_media(url)
    {
        return false;
    }
    // Prefer clear progressive markers; avoid bare /video/ API paths.
    lower.contains(".mp4")
        || lower.contains("phncdn.com")
        || lower.contains("porncdn.com")
        || (lower.contains("/videos/") && lower.contains("cdn"))
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
    fn mp4_mode_keeps_get_media_for_expansion() {
        let html = r#"{"format":"mp4","quality":"720","videoUrl":"https:\/\/www.pornhub.com\/video\/get_media?s=abc"}"#;
        let cands = collect_candidates(html, false);
        assert_eq!(cands.len(), 1);
        assert!(is_get_media(&cands[0].1));
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

    #[test]
    fn parse_get_media_picks_mp4_entries() {
        let raw = r#"[
            {"quality":"480","videoUrl":"https://cdn.phncdn.com/videos/a480.mp4"},
            {"quality":"720","videoUrl":"https://cdn.phncdn.com/videos/a720.mp4"},
            {"quality":"1080","videoUrl":"https://cdn.phncdn.com/videos/master.m3u8"}
        ]"#;
        let items = parse_get_media_json(raw);
        let mp4 = pick_best_filtered(&items, false).unwrap();
        assert!(mp4.contains("a720.mp4"), "got: {mp4}");
        let hls = pick_best_filtered(&items, true).unwrap();
        assert!(hls.contains("master.m3u8"), "got: {hls}");
    }
}
