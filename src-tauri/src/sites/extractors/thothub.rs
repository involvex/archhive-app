use crate::error::AppResult;
use crate::sites::SiteContext;
use regex::Regex;
use std::sync::OnceLock;

/// Direct MP4 extraction for Kernel Video Sharing (KVS) sites — ThotHub,
/// ThisVid. KVS players embed `video_url` / `video_alt_url*` (and
/// `license_code`) in a page script. Newer builds expose plain
/// `/get_file/...` URLs; older ones prefix obfuscated URLs with `function/0/`,
/// decoded with the same license-token swap as yt-dlp's generic KVS extractor.
/// Resolve a KVS (ThotHub, ThisVid) video page to a playable CDN URL.
///
/// KVS `get_file` links only work inside the session that loaded the page
/// (PHPSESSID / kt_* cookies): fetched by anyone else — yt-dlp's own
/// session, or the WebView — they redirect to a placeholder clip
/// (ThisVid `processing.mp4`, ThotHub `fu.mp4`). So the page is loaded in a
/// fresh cookie session (plus the user's imported cookies for the site) and
/// `get_file` is followed in that same session; the signed CDN URL it
/// redirects to plays without cookies.
pub async fn resolve_kvs_stream(
    ctx: &SiteContext,
    page_url: &str,
    site_id: &str,
    site_name: &str,
) -> AppResult<String> {
    resolve_kvs_stream_as(ctx, page_url, site_id, site_name, None).await
}

/// [`resolve_kvs_stream`] sending `user_agent` (a bot-check clearance is
/// only valid for the User-Agent it was issued to).
pub async fn resolve_kvs_stream_as(
    ctx: &SiteContext,
    page_url: &str,
    site_id: &str,
    site_name: &str,
    user_agent: Option<&str>,
) -> AppResult<String> {
    let user_agent = user_agent.unwrap_or(crate::sites::BROWSER_UA);
    use crate::error::AppError;
    use reqwest::cookie::Jar;
    use std::sync::Arc;

    let page = url::Url::parse(page_url)
        .map_err(|e| AppError::InvalidInput(format!("bad video URL: {e}")))?;
    let jar = Arc::new(Jar::default());
    if let Ok(Some(header)) = ctx._vault().cookie_header(site_id) {
        for pair in header.split(';').map(str::trim).filter(|p| p.contains('=')) {
            jar.add_cookie_str(pair, &page);
        }
    }
    // Normal clients first; on a connection failure retry with the
    // IPv4 / HTTP/1.1 fallback and keep using it for get_file.
    let make = |fallback: bool, follow: bool| {
        let builder = if fallback {
            crate::sites::fallback_client_builder()
        } else {
            reqwest::Client::builder().user_agent(user_agent)
        };
        let builder = builder
            .cookie_provider(jar.clone())
            .timeout(std::time::Duration::from_secs(30));
        if follow {
            builder.build()
        } else {
            builder.redirect(reqwest::redirect::Policy::none()).build()
        }
    };

    crate::sites::throttle::acquire(page_url).await?;
    let (resp, fallback) = match make(false, true)?.get(page_url).send().await {
        Ok(resp) => (resp, false),
        Err(e) if crate::sites::is_transport_error(&e) => {
            tracing::warn!("[{site_id}] {e}; retrying over IPv4 / HTTP/1.1");
            (make(true, true)?.get(page_url).send().await?, true)
        }
        Err(e) => return Err(e.into()),
    };
    let no_redirect = make(fallback, false)?;
    let status = resp.status();
    let html = resp.text().await?;
    if looks_like_bot_check(&html) {
        return Err(AppError::Site(format!(
            "{site_name} is showing a bot check (Cloudflare) to this device. Open Settings → \
             Cookies → {site_name} → Log in once so the check passes, then try again."
        )));
    }
    if !status.is_success() {
        return Err(AppError::Site(format!(
            "{site_name} returned HTTP {status} for the video page."
        )));
    }
    if is_private_video_page(&html) {
        return Err(AppError::Site(format!(
            "This is a private {site_name} video — only logged-in members can watch it. \
             Log in via Settings → Cookies → {site_name}, or pick a video without the 🔒."
        )));
    }
    let get_file = pick_best_stream(&html, page_url).ok_or_else(|| {
        AppError::Site(format!(
            "No player on this {site_name} page — the video may be private or need you to be \
             logged in (Settings → Cookies → {site_name})."
        ))
    })?;

    crate::sites::throttle::acquire(&get_file).await?;
    let resp = no_redirect
        .get(&get_file)
        .header(reqwest::header::REFERER, page_url)
        .send()
        .await?;
    if resp.status().is_redirection() {
        let location = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Site(format!("{site_name} sent an empty redirect")))?;
        let target = page
            .join(location)
            .map_err(|e| AppError::Site(format!("bad redirect from {site_name}: {e}")))?
            .to_string();
        if is_kvs_placeholder(&target) {
            return Err(AppError::Site(format!(
                "{site_name} served its placeholder clip instead of the video — it may be \
                 private, still processing, or need login cookies."
            )));
        }
        return Ok(target);
    }
    if resp.status().is_success() {
        return Ok(get_file);
    }
    Err(AppError::Site(format!(
        "{site_name} refused the video file (HTTP {}).",
        resp.status()
    )))
}

/// KVS "private video" pages: no player unless the viewer is a member.
fn is_private_video_page(html: &str) -> bool {
    html.contains("This video is a private video")
        || html.contains("Only active members can watch private videos")
}

/// Cloudflare / DDoS-Guard interstitials instead of the real page.
fn looks_like_bot_check(html: &str) -> bool {
    let head = &html[..html.len().min(4000)];
    head.contains("Just a moment...")
        || head.contains("cf-chl")
        || head.contains("challenge-platform")
        || head.contains("ddos-guard")
}

/// KVS placeholder clips served when `get_file` is hit outside the session.
pub(crate) fn is_kvs_placeholder(url: &str) -> bool {
    let path = url::Url::parse(url)
        .map(|u| u.path().to_ascii_lowercase())
        .unwrap_or_default();
    [
        "/processing.mp4",
        "/fu.mp4",
        "/na.mp4",
        "/removed.mp4",
        "/deleted.mp4",
        "/video_error.mp4",
    ]
    .iter()
    .any(|p| path.ends_with(p))
}

pub(crate) fn pick_best_stream(html: &str, page_url: &str) -> Option<String> {
    let license = extract_license_code(html);
    let mut candidates = collect_video_urls(html);
    if candidates.is_empty() {
        if let Some(u) = extract_json_ld_content_url(html) {
            candidates.push((0, u));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let raw = &candidates[0].1;
    let decoded = match &license {
        Some(code) => kvs_get_real_url(raw, code),
        None => raw.clone(),
    };
    Some(absolutize(page_url, &decoded))
}

fn extract_license_code(html: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"license_code\s*:\s*['"]([^'"]+)['"]"#).expect("license regex")
    });
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn collect_video_urls(html: &str) -> Vec<(i32, String)> {
    static URL_RE: OnceLock<Regex> = OnceLock::new();
    let url_re = URL_RE.get_or_init(|| {
        Regex::new(r#"(video_url|video_alt_url\d*)\s*:\s*['"]([^'"]+)['"]"#).expect("url key regex")
    });
    static TEXT_RE: OnceLock<Regex> = OnceLock::new();
    let text_re = TEXT_RE.get_or_init(|| {
        Regex::new(r#"(video_url|video_alt_url\d*)_text\s*:\s*['"]([^'"]+)['"]"#)
            .expect("text key regex")
    });

    let mut texts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for cap in text_re.captures_iter(html) {
        texts.insert(cap[1].to_string(), cap[2].to_string());
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for cap in url_re.captures_iter(html) {
        let key = cap[1].to_string();
        let url = cap[2].to_string();
        if !url.contains("/get_file/") && !url.contains("function/0/") {
            continue;
        }
        if !seen.insert(url.clone()) {
            continue;
        }
        let height = texts
            .get(&key)
            .map(|t| parse_height(t))
            .unwrap_or_else(|| parse_height(&url));
        // The unlabelled main `video_url` is the site's standard quality:
        // rank it above labelled low-res alternates (ThisVid "240p") but
        // below labelled HD ones ("720p").
        let height = if height == 0 && key == "video_url" {
            480
        } else {
            height
        };
        out.push((height, url));
    }
    out
}

fn extract_json_ld_content_url(html: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#""contentUrl"\s*:\s*"([^"]+get_file[^"]+)""#).expect("jsonld regex")
    });
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().replace("\\/", "/")))
}

fn parse_height(s: &str) -> i32 {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d{3,4})p").expect("height regex"));
    re.captures(s).and_then(|c| c[1].parse().ok()).unwrap_or(0)
}

fn absolutize(page_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if let Ok(base) = url::Url::parse(page_url) {
        if let Ok(joined) = base.join(href) {
            return joined.to_string();
        }
    }
    href.to_string()
}

/// Port of yt-dlp `GenericIE._kvs_get_real_url`.
fn kvs_get_real_url(video_url: &str, license_code: &str) -> String {
    const PREFIX: &str = "function/0/";
    if !video_url.starts_with(PREFIX) {
        return video_url.to_string();
    }
    let rest = &video_url[PREFIX.len()..];
    let Ok(parsed) = url::Url::parse(rest) else {
        return video_url.to_string();
    };
    let license_token = kvs_get_license_token(license_code);
    let path = parsed.path();
    let mut urlparts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
    const HASH_LENGTH: usize = 32;
    if urlparts.len() <= 3 || urlparts[3].len() < HASH_LENGTH {
        return rest.to_string();
    }
    let hash = urlparts[3][..HASH_LENGTH].to_string();
    let mut indices: Vec<usize> = (0..HASH_LENGTH).collect();
    let mut accum: usize = 0;
    for src in (0..HASH_LENGTH).rev() {
        accum += license_token.get(src).copied().unwrap_or(0) as usize;
        let dest = (src + accum) % HASH_LENGTH;
        indices.swap(src, dest);
    }
    let hash_chars: Vec<char> = hash.chars().collect();
    let mut new_hash = String::with_capacity(urlparts[3].len());
    for index in indices {
        new_hash.push(hash_chars.get(index).copied().unwrap_or('0'));
    }
    new_hash.push_str(&urlparts[3][HASH_LENGTH..]);
    urlparts[3] = new_hash;
    let mut out = parsed;
    out.set_path(&urlparts.join("/"));
    out.to_string()
}

fn kvs_get_license_token(license_code: &str) -> Vec<u32> {
    let license_code = license_code.replace('$', "");
    let license_values: Vec<u32> = license_code
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();
    let modlicense = license_code.replace('0', "1");
    let center = modlicense.len() / 2;
    let fronthalf: i64 = modlicense[..=center].parse().unwrap_or(0);
    let backhalf: i64 = modlicense[center..].parse().unwrap_or(0);
    let mod_str = (4 * (fronthalf - backhalf).abs()).to_string();
    let mod_trim: String = mod_str.chars().take(center + 1).collect();
    let mut token = Vec::new();
    for (index, current) in mod_trim.chars().filter_map(|c| c.to_digit(10)).enumerate() {
        for offset in 0..4 {
            let lv = license_values.get(index + offset).copied().unwrap_or(0);
            token.push((lv + current) % 10);
        }
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
<script>
var playercfg = {
  video_id: '105',
  license_code: '$496544316381867',
  video_url: 'https://www.kvs-demo.com/get_file/1/abc/0/105/105_480p.mp4/',
  video_url_text: '480p',
  video_alt_url: 'https://www.kvs-demo.com/get_file/1/def/0/105/105_720p.mp4/',
  video_alt_url_text: '720p',
};
</script>
<script type="application/ld+json">
{"contentUrl": "https://www.kvs-demo.com/get_file/1/f2aded36c9fc0a2c1f3653b6a2741361/0/105/105_720p.mp4/"}
</script>
"#;

    #[test]
    fn picks_highest_quality_get_file_url() {
        let url = pick_best_stream(FIXTURE, "https://www.kvs-demo.com/video/105/x/").unwrap();
        assert!(url.contains("105_720p.mp4"), "got {url}");
    }

    #[test]
    fn deobfuscates_function0_prefix() {
        // Path layout: /get_file/1/{32-char-hash}rest/file.mp4
        let obfuscated = "function/0/https://cdn.example.com/get_file/1/0123456789abcdef0123456789abcdefREST/file.mp4";
        let license = "$496544316381867";
        let real = kvs_get_real_url(obfuscated, license);
        assert!(!real.starts_with("function/0/"), "{real}");
        assert!(real.contains("cdn.example.com/get_file/1/"), "{real}");
        assert!(
            real.ends_with("REST/file.mp4") || real.contains("REST/file.mp4"),
            "{real}"
        );
        let parsed = url::Url::parse(&real).unwrap();
        let parts: Vec<_> = parsed.path().split('/').collect();
        assert!(parts[3].len() >= 32);
        assert_eq!(parts[3].len(), "0123456789abcdef0123456789abcdefREST".len());
    }

    #[test]
    fn main_url_beats_low_res_alternate() {
        let html = r#"video_url: 'https://thisvid.com/get_file/5/a/1/1/1.mp4/',
            video_alt_url: 'https://thisvid.com/get_file/5/b/1/1/1_240p.mp4/',
            video_alt_url_text: '240p',"#;
        assert_eq!(
            super::pick_best_stream(html, "https://thisvid.com/videos/x/").as_deref(),
            Some("https://thisvid.com/get_file/5/a/1/1/1.mp4/")
        );
        let hd = r#"video_url: 'https://x.com/get_file/1/a/1.mp4/',
            video_alt_url: 'https://x.com/get_file/1/b/1_720p.mp4/',
            video_alt_url_text: '720p',"#;
        assert!(super::pick_best_stream(hd, "https://x.com/v/")
            .unwrap()
            .contains("_720p"));
    }

    #[test]
    fn detects_placeholders_and_bot_checks() {
        assert!(super::is_kvs_placeholder(
            "http://m.thisvid.com/processing.mp4"
        ));
        assert!(super::is_kvs_placeholder("https://thothub.to/fu.mp4"));
        assert!(!super::is_kvs_placeholder(
            "https://jupiter4.thisvid.com/key=a,end=1/14830236.mp4"
        ));
        assert!(super::looks_like_bot_check(
            "<html><title>Just a moment...</title>"
        ));
        assert!(!super::looks_like_bot_check("<html><title>Video</title>"));
        assert!(super::is_private_video_page(
            "This video is a private video uploaded by x. Only active members can watch private videos."
        ));
    }

    #[test]
    fn plain_url_passthrough() {
        let u = "https://cdn.example.com/get_file/1/hash/file.mp4";
        assert_eq!(kvs_get_real_url(u, "$123"), u);
    }

    #[test]
    fn parses_real_kvs_player_snippet() {
        let html = std::fs::read_to_string("tests/fixtures/kvs_player_snippet.html")
            .expect("fixture kvs_player_snippet.html");
        let url = pick_best_stream(&html, "https://www.kvs-demo.com/video/105/x/").unwrap();
        assert!(
            url.contains("/get_file/") && url.contains("720p"),
            "expected 720p get_file url, got {url}"
        );
    }
}
