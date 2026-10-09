/// Path segment: lowercased, whitespace to hyphens, restricted to
/// `[a-z0-9-_]` (RFC 3986 unreserved + hyphen).
///
/// Everything else (`/ ? # & . %` …) is dropped so a user-supplied slug can
/// never escape the path segment it is interpolated into (no `../` traversal,
/// no query/hash smuggling). Runs of hyphens collapse to one and leading /
/// trailing hyphens are trimmed.
///
/// Underscore is kept deliberately: Reddit subreddit names (`r/learn_programming`)
/// and ThotHub tags legally contain `_`, and `_` cannot break out of a path
/// segment.
pub fn path_slug(slug: &str) -> String {
    let hyphenated: String = slug
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    // Collapse `--` runs and trim edge hyphens left behind by stripped chars.
    let mut out = String::with_capacity(hyphenated.len());
    let mut prev_dash = false;
    for c in hyphenated.chars() {
        if c == '-' {
            if prev_dash {
                continue;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
        out.push(c);
    }
    out.trim_matches('-').to_string()
}

/// Query string value (search terms with spaces).
pub fn query_slug(slug: &str) -> String {
    url::form_urlencoded::byte_serialize(slug.trim().as_bytes()).collect()
}

/// Decode HTML entities (`&#39;`, `&amp;`, `&quot;`, …) from scraped text.
pub fn html_unescape(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}

/// Strip a leading `MM:SS` or `HH:MM:SS` duration pattern from text.
/// Used as a last-resort fallback when YouPorn titles are scraped as just
/// the duration string (e.g. `05:47`).
pub fn strip_duration_from_title(text: &str) -> String {
    let re = regex::Regex::new(r"^\d{1,2}:\d{2}(?::\d{2})?\s*[-:]?\s*").unwrap();
    re.replace(text, "").trim().to_string()
}

/// Derive an embed URL from a live cam room URL.
/// For Chaturbate, appends `embed_video_only=1` so the iframe plays video
/// directly (no chat) as a fallback when yt-dlp stream resolution fails.
/// Returns the original URL if no known pattern matches.
///
/// Note: Chaturbate embeds may have audio muted by default due to autoplay
/// policies. The `autoplay=1` parameter helps, but users may need to interact
/// with the player to enable audio on some browsers.
pub fn derive_embed_url(url: &str) -> String {
    if let Some(username) = extract_username_from_url(url, "chaturbate.com") {
        return format!("https://chaturbate.com/embed/{username}/?embed_video_only=1&autoplay=1");
    }
    if let Some(username) = extract_username_from_url(url, "stripchat.com") {
        return format!("https://stripchat.com/embed/{username}/");
    }
    url.to_string()
}

/// Resolve a cam HLS URL via yt-dlp without a brittle `-f best` selector.
///
/// Chaturbate often has no literal `best` format id (especially on Android
/// youtubedl-android), so `-f best/b` fails with "Requested format is not
/// available". Default format selection + audio-preferring retry.
///
/// For live streams, we prioritize audio-included formats because many cams
/// produce video-only streams, which Android/iOS will play muted.
pub async fn resolve_cam_stream_url(
    ctx: &crate::sites::SiteContext,
    site_id: &str,
    url: &str,
) -> crate::error::AppResult<String> {
    let (video, audio) = resolve_cam_stream_urls(ctx, site_id, url).await?;
    if audio.is_none() {
        return Ok(video);
    }
    // Separate audio+video chunklists can't play in one `<video>` tag.
    // Probe `-J` metadata for a single muxed/master manifest first.
    let runner = crate::sites::yt_dlp::SidecarRunner::new(ctx.app().clone());
    let cookies = ctx.cookie_file_for_site(site_id);
    if let Ok(meta) = runner.resolve_media_json(url, cookies.as_deref()).await {
        if let Some(master) = pick_master_manifest_from_ytdlp_json(&meta) {
            return Ok(master);
        }
    }
    Err(crate::error::AppError::Other(
        "Separate audio+video manifests - using embed fallback".to_string(),
    ))
}

/// Probe yt-dlp `-J` metadata for a multivariant (master) HLS manifest.
///
/// Chaturbate live exposes separate video-only and audio-only chunklists, so
/// `--get-url` yields two unusable-alone URLs. The `-J` dump usually still
/// lists a master/playlist manifest (or a format carrying both codecs) that
/// hls.js can play with its audio-track selection. Returns `None` when no
/// such manifest exists, in which case callers fall back to the embed iframe.
pub fn pick_master_manifest_from_ytdlp_json(v: &serde_json::Value) -> Option<String> {
    let formats = v.get("formats")?.as_array()?;
    // 1) Explicit master / index / playlist manifests first.
    for f in formats {
        let url = f.get("url")?.as_str()?;
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        let lower = url.to_ascii_lowercase();
        if !lower.contains(".m3u8") {
            continue;
        }
        if lower.contains("master") || lower.contains("index") || lower.contains("playlist.m3u8") {
            return Some(url.to_string());
        }
    }
    // 2) Any HLS format advertising both video and audio codecs.
    for f in formats {
        let url = f.get("url")?.as_str()?;
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        if !url.to_ascii_lowercase().contains(".m3u8") {
            continue;
        }
        let vcodec = f.get("vcodec").and_then(|c| c.as_str()).unwrap_or("none");
        let acodec = f.get("acodec").and_then(|c| c.as_str()).unwrap_or("none");
        if vcodec != "none" && acodec != "none" {
            return Some(url.to_string());
        }
    }
    None
}

/// Split yt-dlp `--get-url` output into a playable video URL plus an optional
/// separate audio URL.
///
/// Chaturbate/Stripchat live emits one video-only and one audio-only chunklist.
/// Browsers can't mux two HLS URLs in a single `<video>` tag, so live playback
/// drives a second hidden `<audio>` element from the audio URL when present
/// (see `HlsVideoPlayer audioSrc`). Single-URL and master-playlist outputs
/// keep `audio` as `None`.
pub fn pick_av_urls_from_ytdlp(raw: &str) -> crate::error::AppResult<(String, Option<String>)> {
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
        return Ok((m3u8s[0].to_string(), None));
    }

    if m3u8s.len() > 1 {
        // Prefer an explicit master / index playlist (usually includes audio).
        if let Some(master) = m3u8s.iter().find(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("master") || lower.contains("index") || lower.contains("playlist.m3u8")
        }) {
            return Ok(((*master).to_string(), None));
        }

        let is_audio_like = |l: &str| {
            let lower = l.to_ascii_lowercase();
            lower.contains("audio")
                || lower.contains("/a/")
                || lower.contains("_audio")
                || lower.contains("-audio")
        };
        let audio = m3u8s.iter().find(|l| is_audio_like(l));
        let video = m3u8s.iter().find(|l| !is_audio_like(l));
        match (video, audio) {
            (Some(v), Some(a)) => return Ok((v.to_string(), Some(a.to_string()))),
            (Some(v), None) => return Ok((v.to_string(), None)),
            _ => return Ok((m3u8s[0].to_string(), None)),
        }
    }

    Ok((lines[0].to_string(), None))
}

/// Resolve cam HLS URLs via yt-dlp, keeping a separate audio URL when the cam
/// serves split audio/video chunklists (Chaturbate/Stripchat live).
pub async fn resolve_cam_stream_urls(
    ctx: &crate::sites::SiteContext,
    site_id: &str,
    url: &str,
) -> crate::error::AppResult<(String, Option<String>)> {
    let runner = crate::sites::yt_dlp::SidecarRunner::new(ctx.app().clone());
    let cookies = ctx.cookie_file_for_site(site_id);

    let format_attempts: &[Option<&str>] = &[
        None,                             // Try default first - yt-dlp prefers formats with audio
        Some("bestaudio+bestvideo/best"), // Merge separate audio+video if needed
    ];

    let mut last_err = None;
    for fmt in format_attempts {
        let mut args = vec![
            url.to_string(),
            "--get-url".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
        ];
        if let Some(f) = fmt {
            args.push("-f".to_string());
            args.push(f.to_string());
        }
        if let Some(cookies) = cookies.as_ref() {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        match runner.run_capture_for_stream_url("yt-dlp", &args).await {
            Ok(raw) => match pick_av_urls_from_ytdlp(&raw) {
                Ok(pair) => return Ok(pair),
                Err(e) => last_err = Some(e),
            },
            Err(e) => {
                let msg = e.to_string();
                // Format selector rejected — try next attempt.
                if msg.contains("Requested format is not available")
                    || msg.contains("format is not available")
                {
                    last_err = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    // Last-ditch: retry yt-dlp on the embed-page variant for cams that fail
    // on the room URL ("Unable to extract data"). The embed endpoint serves
    // a different page structure that yt-dlp's extractor can often parse.
    let embed_url = derive_embed_url(url);
    if embed_url != url {
        let mut embed_args = vec![
            embed_url.clone(),
            "--get-url".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "--no-check-certificates".to_string(),
        ];
        if let Some(cookies) = cookies.as_ref() {
            embed_args.push("--cookies".to_string());
            embed_args.push(cookies.to_string_lossy().to_string());
        }
        if let Ok(raw) = runner
            .run_capture_for_stream_url("yt-dlp", &embed_args)
            .await
        {
            if let Ok(pair) = pick_av_urls_from_ytdlp(&raw) {
                return Ok(pair);
            }
        }
    }

    // When `--get-url` produced nothing usable, probe `-J` metadata for a
    // master manifest that hls.js can play with audio before giving up.
    if let Ok(meta) = runner.resolve_media_json(url, cookies.as_deref()).await {
        if let Some(master) = pick_master_manifest_from_ytdlp_json(&meta) {
            return Ok((master, None));
        }
    }

    Err(last_err
        .unwrap_or_else(|| crate::error::AppError::Other("No stream URL resolved".to_string())))
}
/// Pick a playable stream URL from yt-dlp `--get-url` output.
///
/// Live cams must resolve to a **single muxed** HLS URL. When yt-dlp emits
/// separate video + audio playlists (e.g. `-f bv*+ba`), taking the first
/// `.m3u8` yields video-only media and Android/iOS will play muted, so this
/// returns an error on that shape. Live callers should use
/// `pick_av_urls_from_ytdlp` and play the audio URL in a second element.
pub fn pick_stream_url_from_ytdlp(raw: &str) -> crate::error::AppResult<String> {
    let (video, audio) = pick_av_urls_from_ytdlp(raw)?;
    if audio.is_some() {
        return Err(crate::error::AppError::Other(
            "Separate audio+video manifests - using embed fallback".to_string(),
        ));
    }
    Ok(video)
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
    fn path_slug_hardening() {
        // Basic behaviour preserved.
        assert_eq!(path_slug("Big Tits"), "big-tits");
        assert_eq!(path_slug("  Amateur  "), "amateur");
        // Path-breaking characters are stripped (no traversal / query smuggling).
        assert_eq!(path_slug("../../etc"), "etc");
        assert_eq!(path_slug("foo?page=99"), "foopage99");
        assert_eq!(path_slug("a#b&c.d%e"), "abcde");
        // Runs collapse, edge hyphens left by stripped chars are trimmed.
        assert_eq!(path_slug("a -- b"), "a-b");
        assert_eq!(path_slug("  --Foo__Bar-- "), "foo__bar");
        // Underscores survive (legal in subreddit / tag names, path-safe).
        assert_eq!(path_slug("learn_programming"), "learn_programming");
        // Digits survive (numeric category ids pass through elsewhere).
        assert_eq!(path_slug("27"), "27");
    }

    #[test]
    fn query_slug_encodes_specials() {
        // `&` must not be able to inject a second query param.
        assert_eq!(query_slug("big & busty"), "big+%26+busty");
        // Unicode is percent-encoded, not passed through raw.
        let encoded = query_slug("amateur €");
        assert!(encoded.starts_with("amateur+"), "got {encoded}");
        assert!(!encoded.contains('€'), "got {encoded}");
        assert!(!encoded.contains('&'), "got {encoded}");
    }

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
        // The pair picker splits video + audio; the single-URL wrapper errors
        // on this shape so live callers use dual-element playback instead.
        let (video, audio) = pick_av_urls_from_ytdlp(raw).unwrap();
        assert!(video.contains("chunklist_v.m3u8"));
        assert!(!video.contains("audio"));
        assert!(audio.unwrap().contains("chunklist_audio.m3u8"));
        assert!(pick_stream_url_from_ytdlp(raw).is_err());
    }

    #[test]
    fn chaturbate_embed_allows_sound() {
        let u = derive_embed_url("https://chaturbate.com/someuser/");
        assert!(!u.contains("disable_sound"));
        assert!(u.contains("embed_video_only=1"));
        assert!(u.contains("autoplay=1"));
    }

    #[test]
    fn pick_prefers_muxed_over_video_only() {
        // When there's a master playlist with audio+video, prefer it
        let raw = "\
https://cdn.example/video.m3u8
https://cdn.example/master.m3u8
https://cdn.example/audio.m3u8
";
        let url = pick_stream_url_from_ytdlp(raw).unwrap();
        assert!(url.contains("master.m3u8"));
    }

    #[test]
    fn pick_prefers_chunklist_without_audio_suffix() {
        // Chaturbate uses chunklist manifests for separate video/audio streams.
        // When both audio and video are present in the URLs, we should return an error
        // to signal fallback to the embed URL.
        let raw = "\
https://cdn.example/chunklist_5_audio_123.m3u8
https://cdn.example/chunklist_3_video_456.m3u8
";
        let result = pick_stream_url_from_ytdlp(raw);
        // Should return an error because we have separate audio+video manifests
        assert!(result.is_err());
    }

    #[test]
    fn pick_handles_separate_audio_video() {
        // Test that we return video URL when there's only video (no audio in path)
        let raw = "\
https://cdn.example/chunklist_3_video.m3u8
";
        let url = pick_stream_url_from_ytdlp(raw).unwrap();
        assert!(url.contains("chunklist_3_video"));
    }

    #[test]
    fn pick_master_manifest_from_json_prefers_master_url() {
        let v: serde_json::Value = serde_json::json!({
            "formats": [
                {"url": "https://edge.example/chunklist_3_video_1_llhls.m3u8", "vcodec": "h264", "acodec": "none"},
                {"url": "https://edge.example/chunklist_5_audio_1_llhls.m3u8", "vcodec": "none", "acodec": "mp4a"},
                {"url": "https://edge.example/master.m3u8?session=abc", "vcodec": "h264", "acodec": "mp4a"},
            ]
        });
        let url = pick_master_manifest_from_ytdlp_json(&v).unwrap();
        assert!(url.contains("master.m3u8"));
    }

    #[test]
    fn pick_master_manifest_from_json_falls_back_to_muxed_format() {
        let v: serde_json::Value = serde_json::json!({
            "formats": [
                {"url": "https://edge.example/chunklist_3_video_1_llhls.m3u8", "vcodec": "h264", "acodec": "none"},
                {"url": "https://edge.example/muxed_720p.m3u8", "vcodec": "h264", "acodec": "mp4a"},
            ]
        });
        let url = pick_master_manifest_from_ytdlp_json(&v).unwrap();
        assert!(url.contains("muxed_720p.m3u8"));
    }

    #[test]
    fn pick_master_manifest_from_json_returns_none_without_master() {
        let v: serde_json::Value = serde_json::json!({
            "formats": [
                {"url": "https://edge.example/chunklist_3_video_1_llhls.m3u8", "vcodec": "h264", "acodec": "none"},
                {"url": "https://edge.example/chunklist_5_audio_1_llhls.m3u8", "vcodec": "none", "acodec": "mp4a"},
            ]
        });
        assert!(pick_master_manifest_from_ytdlp_json(&v).is_none());
    }

    #[test]
    fn html_unescape_decodes_common_entities() {
        assert_eq!(html_unescape("i&#x27;m nika"), "i'm nika");
        assert_eq!(html_unescape("test &amp; stuff"), "test & stuff");
        assert_eq!(html_unescape("&quot;hello&quot;"), "\"hello\"");
        assert_eq!(html_unescape("no entities here"), "no entities here");
        assert_eq!(html_unescape("a &lt; b &gt; c"), "a < b > c");
    }

    #[test]
    fn strip_duration_from_title_removes_leading_duration() {
        assert_eq!(strip_duration_from_title("05:47 Some Title"), "Some Title");
        assert_eq!(
            strip_duration_from_title("1:23:45 Long Title"),
            "Long Title"
        );
        assert_eq!(strip_duration_from_title("12:34"), "");
        assert_eq!(
            strip_duration_from_title("Real Title Here"),
            "Real Title Here"
        );
    }
}
