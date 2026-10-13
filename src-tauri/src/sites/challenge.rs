//! Fallback for sites that show a Cloudflare / bot check to ArcHive's
//! embedded engines (yt-dlp, the Rust HTTP client) — e.g. KVS tubes like
//! CrazyPorn / Love4Porn / 4KPorn and SpankBang reached through xPomelo.
//!
//! On Android the video page is opened in the in-app browser (a real
//! WebView), where the check passes on its own or after one tap. The
//! rendered HTML comes back here, the video URL is picked from it (KVS
//! player config, SpankBang `stream_data`, `<video>` / `<source>`, og:video,
//! JSON-LD), and session-bound KVS `get_file` links are followed inside the
//! same browser session to their CDN URL.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

/// yt-dlp / adapter error text that means "the site refused the embedded
/// client", not "the video is gone".
pub fn is_bot_block(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    [
        "cloudflare",
        "anti-bot",
        "bot check",
        "impersonat",
        "just a moment",
        "http error 403",
        "403 forbidden",
        "http 403",
    ]
    .iter()
    .any(|m| e.contains(m))
}

/// Page title of a Cloudflare / DDoS-Guard interstitial (desktop window).
#[cfg_attr(not(desktop), allow(dead_code))]
pub fn is_challenge_title(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    [
        "just a moment",
        "attention required",
        "checking your browser",
        "ddos-guard",
        "security verification",
        "verify you are human",
    ]
    .iter()
    .any(|m| t.contains(m))
}

/// Sites whose own adapters resolve streams (a 403 there means something
/// else — geo block, removed video) don't get the in-app browser fallback.
pub fn webview_fallback_applies(url: &str) -> bool {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_default();
    ![
        "pornhub.com",
        "xvideos.com",
        "xnxx.com",
        "xhamster.com",
        "youporn.com",
        "eporner.com",
        "youtube.com",
        "youtu.be",
        "tiktok.com",
        "instagram.com",
        "twitter.com",
        "x.com",
        "reddit.com",
        "redgifs.com",
        "chaturbate.com",
        "stripchat.com",
    ]
    .iter()
    .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Best playable URL on a rendered video page.
pub fn extract_media_url(html: &str, page_url: &str) -> Option<String> {
    crate::sites::extractors::thothub::pick_best_stream(html, page_url)
        .or_else(|| spankbang_stream(html))
        .or_else(|| video_tag_source(html, page_url))
        .or_else(|| meta_video(html, page_url))
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

/// SpankBang: `var stream_data = {'240p': ['https://…'], …, 'm3u8': [...]}`.
fn spankbang_stream(html: &str) -> Option<String> {
    use std::sync::LazyLock;
    static BLOCK: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?s)stream_data\s*=\s*\{(.*?)\};").expect("stream_data")
    });
    static ENTRY: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"['"](\d{3,4}p|4k|m3u8)['"]\s*:\s*\[\s*['"](https?://[^'"]+)['"]"#)
            .expect("stream entry")
    });
    let block = BLOCK.captures(html)?.get(1)?.as_str();
    ENTRY
        .captures_iter(block)
        .map(|c| {
            let rank = match &c[1] {
                "4k" => 2160,
                "m3u8" => 1,
                q => q.trim_end_matches('p').parse().unwrap_or(0),
            };
            (rank, c[2].replace("\\/", "/"))
        })
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, u)| u)
}

/// `<video src>` or `<source src>` (mp4 / HLS) once the site's player ran.
fn video_tag_source(html: &str, page_url: &str) -> Option<String> {
    let doc = scraper::Html::parse_document(html);
    let sel = scraper::Selector::parse("video[src], video source[src], source[type^='video'][src]")
        .expect("video selector");
    doc.select(&sel)
        .filter_map(|e| e.value().attr("src"))
        .map(|s| crate::sites::urls::html_unescape(s.trim()))
        .filter(|s| !s.is_empty() && !s.starts_with("blob:") && !s.starts_with("data:"))
        .filter_map(|s| url::Url::parse(page_url).ok()?.join(&s).ok())
        .map(|u| u.to_string())
        .find(|u| !looks_like_ad(u))
}

fn meta_video(html: &str, page_url: &str) -> Option<String> {
    let doc = scraper::Html::parse_document(html);
    let sel = scraper::Selector::parse(
        r#"meta[property="og:video:secure_url"], meta[property="og:video:url"], meta[property="og:video"], meta[name="twitter:player:stream"]"#,
    )
    .expect("meta selector");
    let base = url::Url::parse(page_url).ok()?;
    doc.select(&sel)
        .filter_map(|m| m.value().attr("content"))
        .filter_map(|c| base.join(c.trim()).ok())
        .map(|u| u.to_string())
        // og:video is often an embed page, not a media file.
        .find(|u| is_media_url(u))
        .or_else(|| {
            static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
                regex::Regex::new(r#""contentUrl"\s*:\s*"([^"]+)""#).expect("contentUrl")
            });
            RE.captures_iter(html)
                .map(|c| c[1].replace("\\/", "/"))
                .find(|u| is_media_url(u))
        })
}

fn is_media_url(u: &str) -> bool {
    let path = u
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    [".mp4", ".m3u8", ".webm", ".mov", ".m4v"]
        .iter()
        .any(|e| path.ends_with(e) || path.contains(&format!("{e}/")))
}

fn looks_like_ad(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    [
        "preroll",
        "/ads/",
        "vast",
        "exoclick",
        "trafficjunky",
        "magsrv",
    ]
    .iter()
    .any(|m| l.contains(m))
}

fn same_host(a: &str, b: &str) -> bool {
    let host = |u: &str| {
        url::Url::parse(u).ok().and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
    };
    host(a).is_some() && host(a) == host(b)
}

/// A resolved stream plus what the CDN needs to serve it.
#[derive(Debug, Clone)]
pub struct ResolvedMedia {
    pub url: String,
    pub referer: String,
    /// Browser cookies, when the URL is still on the page's own host.
    pub cookie: Option<String>,
    pub user_agent: String,
}

/// Open `page_url` in the in-app browser and resolve its video.
#[cfg(target_os = "android")]
pub async fn resolve_via_webview(
    app: &tauri::AppHandle,
    page_url: &str,
) -> crate::error::AppResult<ResolvedMedia> {
    use crate::error::AppError;
    // One in-app browser at a time (the plugin shows a single dialog).
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let host = url::Url::parse(page_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| "the site".into());
    let page = crate::cookie_login::open_page(app, page_url, &format!("Opening {host}")).await?;
    let result = async {
        let referer = if page.url.is_empty() {
            page_url.to_string()
        } else {
            page.url.clone()
        };
        let media = extract_media_url(&page.html, &referer).ok_or_else(|| {
            AppError::Site(format!(
                "Opened the page on {host} but found no video player on it — the video may \
                 have been removed or need a login."
            ))
        })?;
        // KVS get_file (and other same-host links) are tied to the browser
        // session: let the browser follow them to the CDN.
        let (url, cookie) = if same_host(&media, &referer) {
            let followed = crate::cookie_login::follow_in_page(app, &media, &referer).await?;
            let cookie = same_host(&followed.url, &referer)
                .then(|| {
                    if followed.cookies.is_empty() {
                        page.cookies.clone()
                    } else {
                        followed.cookies.clone()
                    }
                })
                .filter(|c| !c.is_empty());
            (followed.url, cookie)
        } else {
            (media, None)
        };
        if crate::sites::extractors::thothub::is_kvs_placeholder(&url) {
            return Err(AppError::Site(format!(
                "{host} served its placeholder clip instead of the video."
            )));
        }
        Ok(ResolvedMedia {
            url,
            referer,
            cookie,
            user_agent: page.user_agent.clone(),
        })
    }
    .await;
    crate::cookie_login::close_page(app).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_block_errors() {
        assert!(is_bot_block(
            "ERROR: [generic] Got HTTP Error 403 caused by Cloudflare anti-bot challenge"
        ));
        assert!(is_bot_block(
            "ERROR: [SpankBang] 8va69: Unable to download webpage: HTTP Error 403: Forbidden"
        ));
        assert!(is_bot_block("ThotHub is showing a bot check (Cloudflare)"));
        assert!(!is_bot_block("HTTP Error 404: Not Found"));
        assert!(webview_fallback_applies(
            "https://crazyporn.xxx/videos/1/x/"
        ));
        assert!(webview_fallback_applies(
            "https://www.spankbang.com/8va69/video/x"
        ));
        assert!(!webview_fallback_applies(
            "https://de.xhamster.com/videos/x"
        ));
    }

    #[test]
    fn spankbang_picks_highest_mp4() {
        let html = r#"<script>var stream_data = {'240p': ['https://vdownload-1.sb-cd.com/a-240p.mp4?t=1'],
            '720p': ['https://vdownload-1.sb-cd.com/a-720p.mp4?t=1'], '1080p': [],
            'm3u8': ['https://hls.sb-cd.com/a/master.m3u8']};</script>"#;
        assert_eq!(
            extract_media_url(html, "https://spankbang.com/8va69/video/x").as_deref(),
            Some("https://vdownload-1.sb-cd.com/a-720p.mp4?t=1")
        );
    }

    #[test]
    fn kvs_config_then_video_tag_then_meta() {
        let kvs = r#"<script>var flashvars = { license_code: '$496544316381867',
            video_url: 'https://www.kvs-demo.com/get_file/1/abc/0/105/105.mp4/', video_url_text: '480p' };</script>"#;
        assert_eq!(
            extract_media_url(kvs, "https://www.kvs-demo.com/videos/105/x/").as_deref(),
            Some("https://www.kvs-demo.com/get_file/1/abc/0/105/105.mp4/")
        );
        let tag = r#"<video class="vjs" src="/media/v.mp4?x=1&amp;y=2"></video>"#;
        assert_eq!(
            extract_media_url(tag, "https://site.test/videos/1/").as_deref(),
            Some("https://site.test/media/v.mp4?x=1&y=2")
        );
        let meta = r#"<meta property="og:video" content="https://site.test/embed/1">
            <meta property="og:video:url" content="https://cdn.site.test/v/1.mp4">"#;
        assert_eq!(
            extract_media_url(meta, "https://site.test/videos/1/").as_deref(),
            Some("https://cdn.site.test/v/1.mp4")
        );
        assert_eq!(
            extract_media_url("<p>nothing</p>", "https://site.test/"),
            None
        );
    }

    #[test]
    fn hosts() {
        assert!(same_host(
            "https://www.crazyporn.xxx/get_file/1/x/",
            "https://crazyporn.xxx/videos/1/x/"
        ));
        assert!(!same_host(
            "https://cdn.crazyporn.xxx/x.mp4",
            "https://crazyporn.xxx/v/"
        ));
    }
}
