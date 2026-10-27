use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use regex::Regex;
use tracing::info;
use uuid::Uuid;

const BASE: &str = "https://stripchat.com";

const MAX_ROOMS: usize = 48;

const RESERVED_USERNAMES: &[&str] = &[
    "api", "tags", "search", "embed", "auth", "login", "register", "about", "terms", "dmca", "faq",
    "pricing", "blog", "press",
];

fn is_valid_username(username: &str) -> bool {
    if username.len() < 2 || username.contains('.') {
        return false;
    }
    !RESERVED_USERNAMES.contains(&username)
}

pub struct StripchatAdapter;

#[async_trait]
impl SiteAdapter for StripchatAdapter {
    fn id(&self) -> &str {
        "stripchat"
    }

    fn display_name(&self) -> &str {
        "Stripchat"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Livestream,
            BrowseKind::Tag,
            BrowseKind::Search,
            BrowseKind::Model,
        ]
    }

    fn requires_cookies(&self) -> bool {
        true
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        match query.kind {
            BrowseKind::Model => self.browse_model(ctx, query).await,
            BrowseKind::Livestream | BrowseKind::Tag | BrowseKind::Search => {
                self.browse_listing(ctx, query).await
            }
            _ => self.browse_listing(ctx, query).await,
        }
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        // Try native parsing of __PRELOADED_STATE__ first (avoids yt-dlp bot-detection
        // issues on some clients/networks).
        if let Ok(native) = self.resolve_stream_url_native(ctx, url).await {
            return Ok(native);
        }
        match crate::sites::urls::resolve_cam_stream_url(ctx, self.id(), url).await {
            Ok(url) => Ok(url),
            Err(_e) => {
                // Fall back to the embed iframe URL — the player loads it in
                // an <iframe> for cam sites yt-dlp can't resolve directly.
                Ok(crate::sites::urls::derive_embed_url(url))
            }
        }
    }

    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "stripchat/%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "stripchat".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
            referer: None,
            initial_status: None,
        })
    }
}

#[derive(Debug)]
struct PreloadedState {
    model_id: String,
    hosts: Vec<String>,
}

fn parse_preloaded_state_html(html: &str, username: &str) -> AppResult<PreloadedState> {
    // Try to find __PRELOADED_STATE__
    let re = Regex::new(r"(?s)<script\b[^>]*>\s*window\.__PRELOADED_STATE__\s*=(.+?)</script>")
        .map_err(|e| AppError::Other(format!("regex error: {e}")))?;
    let cap = re
        .captures(html)
        .and_then(|c| c.get(1))
        .ok_or_else(|| AppError::Other("Unable to extract data".to_string()))?;
    let mut json_str = cap.as_str().trim().to_string();
    if json_str.ends_with(';') {
        json_str.pop();
    }
    // Try to parse directly first
    let data: serde_json::Value = serde_json::from_str(&json_str)
        .or_else(|_| {
            let unescaped = crate::sites::urls::html_unescape(&json_str);
            serde_json::from_str(&unescaped)
        })
        .map_err(|_| AppError::Other("Unable to extract data".to_string()))?;

    // Check for private show
    if let Some(show) = data.get("viewCam").and_then(|v| v.get("show")) {
        if show.is_object() && !show.is_null() {
            return Err(AppError::Other("Model is in a private show".to_string()));
        }
    }
    // Check if live
    let is_live = data
        .get("viewCam")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.get("isLive"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_live {
        return Err(AppError::Other(format!("{} is offline", username)));
    }

    let model_id = data
        .get("viewCam")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Other("Unable to extract model id".to_string()))?
        .to_string();

    // Look for hls hosts
    let mut hosts: Vec<String> = Vec::new();
    if let Some(h) = data
        .pointer("/config/data/features/featuresV2/hlsFallback/fallbackDomains")
        .and_then(|v| v.as_array())
    {
        for item in h {
            if let Some(s) = item.as_str() {
                hosts.push(s.to_string());
            }
        }
    }
    if let Some(h) = data
        .pointer("/configV3/static/features/hlsFallback/fallbackDomains")
        .and_then(|v| v.as_array())
    {
        for item in h {
            if let Some(s) = item.as_str() {
                if !hosts.contains(&s.to_string()) {
                    hosts.push(s.to_string());
                }
            }
        }
    }
    if let Some(h) = data
        .pointer("/config/data/hlsStreamHost")
        .and_then(|v| v.as_str())
    {
        hosts.push(h.to_string());
    }
    if let Some(h) = data
        .pointer("/configV3/static/hlsStreamHost")
        .and_then(|v| v.as_str())
    {
        hosts.push(h.to_string());
    }

    if hosts.is_empty() {
        return Err(AppError::Other("Unable to extract stream host".to_string()));
    }

    Ok(PreloadedState { model_id, hosts })
}

/// Native (no yt-dlp) resolution of a Stripchat room HLS URL by parsing the
/// room page's `window.__PRELOADED_STATE__` blob.
///
/// Used both by the adapter's `resolve_stream_url` and by
/// `AppState::resolve_livestream_urls` (the live-player path), which would
/// otherwise go straight to yt-dlp - whose Stripchat extractor frequently
/// fails with "Unable to extract data".
pub(crate) async fn resolve_stream_url_native(ctx: &SiteContext, url: &str) -> AppResult<String> {
    let username = url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or("");
    if username.is_empty() || username.contains('?') || !is_valid_username(username) {
        return Err(AppError::Other("invalid stripchat username".to_string()));
    }

    let resp = ctx
        .client
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| AppError::Other(format!("stripchat fetch failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "stripchat fetch failed: {}",
            resp.status()
        )));
    }
    let html = resp
        .text()
        .await
        .map_err(|e| AppError::Other(format!("stripchat read failed: {e}")))?;

    let state = parse_preloaded_state_html(&html, username).map_err(|e| {
        info!("[stripchat] native resolve failed for {username}: {e}");
        e
    })?;

    // Try hosts in order
    for host in &state.hosts {
        let m3u8 = format!(
            "https://edge-hls.{host}/hls/{}/master/{}_auto.m3u8",
            state.model_id, state.model_id
        );
        // Quick check if it's accessible
        if let Ok(r) = ctx.client.head(&m3u8).send().await {
            if r.status().is_success() || r.status().is_redirection() {
                info!("[stripchat] native resolve ok for {username} via {host}");
                return Ok(m3u8);
            }
        }
    }
    // If head checks fail, still return first one - hls.js might handle it
    info!(
        "[stripchat] native resolve ok for {username} (no HEAD check passed, using {})",
        state.hosts[0]
    );
    Ok(format!(
        "https://edge-hls.{}/hls/{}/master/{}_auto.m3u8",
        state.hosts[0], state.model_id, state.model_id
    ))
}

impl StripchatAdapter {
    async fn resolve_stream_url_native(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        resolve_stream_url_native(ctx, url).await
    }

    async fn browse_listing(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_listing_url(&query);
        let api_url = build_api_url(&query);

        info!(
            "[stripchat] browse_listing: kind={:?}, slug={}, api_url={}",
            query.kind, query.slug, api_url
        );

        if let Ok(body) = ctx
            .fetch_json_api(&api_url, self.id(), &format!("{BASE}/"))
            .await
        {
            if let Some(rooms) = parse_api_json(&body) {
                info!("[stripchat] API returned {} rooms", rooms.len());
                return Ok(build_browse_page(rooms, query.page));
            }
        }

        if let Ok(html) = ctx.fetch_html(&url, self.id()).await {
            if let Some(rooms) = parse_listing_html(&html) {
                info!("[stripchat] HTML returned {} rooms", rooms.len());
                return Ok(build_browse_page(rooms, query.page));
            }
        }

        // Tertiary: webview bridge (desktop only).
        #[cfg(desktop)]
        {
            let rooms = crate::sites::adapters::stripchat_webview::_fetch_listing(
                ctx.app(),
                ctx._vault(),
                &url,
            )
            .await?;
            if !rooms.is_empty() {
                return Ok(build_webview_browse_page(rooms, query.page));
            }
        }

        // Nothing worked.
        Err(AppError::Site(
            "No rooms found from Stripchat — import cookies in Settings → Cookies or the listing page may have changed.".into(),
        ))
    }

    /// `model` kind: return a single synthetic `MediaItem` pointing at the model's room.
    async fn browse_model(&self, _ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let slug_raw = query.slug.trim();
        if slug_raw.is_empty() {
            return Ok(BrowsePage {
                items: vec![],
                page: query.page,
                has_more: false,
                total: None,
                ..Default::default()
            });
        }
        let (room_url, username) = if slug_raw.starts_with("http") {
            let u = extract_username_from_url(slug_raw).unwrap_or_default();
            (slug_raw.to_string(), u)
        } else {
            let u = path_slug(slug_raw);
            (format!("{BASE}/{u}/"), u)
        };
        if username.is_empty() {
            return Ok(BrowsePage {
                items: vec![],
                page: query.page,
                has_more: false,
                total: None,
                ..Default::default()
            });
        }
        Ok(BrowsePage {
            items: vec![MediaItem {
                id: Uuid::new_v4().to_string(),
                title: username.clone(),
                url: room_url,
                thumbnail: None,
                duration: None,
                site_id: "stripchat".to_string(),
                performers: vec![username.clone()],
                tags: vec![],
                description: None,
                channel: Some(username.clone()),
                is_live: Some(true),
                viewers: None,
                age: None,
                gender: None,
                stream_url: None,
                embed_url: Some(format!("{BASE}/embed/{username}/")),
            }],
            page: query.page,
            has_more: false,
            total: Some(1),
            ..Default::default()
        })
    }
}

/// Room data parsed from API response or HTML.
struct HttpRoom {
    username: String,
    title: String,
    thumbnail: Option<String>,
    viewers: Option<u32>,
    age: Option<u32>,
    gender: Option<String>,
    stream_url: Option<String>,
}

fn room_to_item(room: &HttpRoom) -> MediaItem {
    let room_url = format!("{BASE}/{}/", room.username);
    let embed_url = format!("{BASE}/embed/{}/", room.username);
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: room.title.clone(),
        url: room_url,
        thumbnail: room.thumbnail.clone(),
        duration: None,
        site_id: "stripchat".to_string(),
        performers: vec![room.username.clone()],
        tags: vec![],
        description: None,
        channel: Some(room.username.clone()),
        is_live: Some(true),
        viewers: room.viewers,
        age: room.age,
        gender: room.gender.clone(),
        stream_url: room.stream_url.clone(),
        embed_url: Some(embed_url),
    }
}

fn build_browse_page(rooms: Vec<HttpRoom>, page: u32) -> BrowsePage {
    let items: Vec<MediaItem> = rooms.into_iter().map(|r| room_to_item(&r)).collect();
    let has_more = items.len() >= 30;
    BrowsePage {
        items,
        page,
        has_more,
        total: None,
        ..Default::default()
    }
}

#[cfg(desktop)]
fn build_webview_browse_page(
    rooms: Vec<crate::sites::adapters::stripchat_webview::_RawRoom>,
    page: u32,
) -> BrowsePage {
    let items: Vec<MediaItem> = rooms.iter().map(map_room).collect();
    let has_more = items.len() >= 30;
    BrowsePage {
        items,
        page,
        has_more,
        total: None,
        ..Default::default()
    }
}

fn build_api_url(query: &BrowseQuery) -> String {
    let offset = (query.page.saturating_sub(1)) * 48;
    match query.kind {
        BrowseKind::Tag => {
            let tag = path_slug(&query.slug);
            format!(
                "{BASE}/api/front/models?improveTs=false&removeShows=false&limit={MAX_ROOMS}&offset={offset}&primaryTag=girls&sortBy=stripRanking&filterGroupTags=%5B%5B%22{tag}%22%5D%5D"
            )
        }
        BrowseKind::Search => {
            let q = url_slug(&query.slug);
            format!(
                "{BASE}/api/front/v4/models/search/group/username?query={q}&limit={MAX_ROOMS}&offset={offset}&primaryTag=girls"
            )
        }
        _ => format!(
            "{BASE}/api/front/models?improveTs=false&removeShows=false&limit={MAX_ROOMS}&offset={offset}&primaryTag=girls&sortBy=stripRanking"
        ),
    }
}

fn build_listing_url(query: &BrowseQuery) -> String {
    match query.kind {
        BrowseKind::Livestream => {
            if query.slug.starts_with("http") {
                query.slug.clone()
            } else if query.slug.is_empty() {
                format!("{BASE}/")
            } else {
                format!("{}/{}/", BASE, path_slug(&query.slug))
            }
        }
        BrowseKind::Tag => format!("{BASE}/tags/{}/", path_slug(&query.slug)),
        BrowseKind::Search => format!("{BASE}/search?q={}", url_slug(&query.slug)),
        _ => {
            if query.slug.starts_with("http") {
                query.slug.clone()
            } else if query.slug.is_empty() {
                format!("{BASE}/")
            } else {
                format!("{}/{}/", BASE, path_slug(&query.slug))
            }
        }
    }
}

/// Try to parse a JSON API response for room data.
fn parse_api_json(body: &str) -> Option<Vec<HttpRoom>> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let rooms_val = v
        .get("rooms")
        .or_else(|| v.get("results"))
        .or_else(|| v.get("data"))
        .or_else(|| v.get("models"))
        .or_else(|| if v.is_array() { Some(&v) } else { None });
    let arr = rooms_val?.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let rooms: Vec<HttpRoom> = arr
        .iter()
        .filter_map(|r| {
            let username = r.get("username")?.as_str()?;
            let title = crate::sites::urls::html_unescape(
                r.get("subject")
                    .or_else(|| r.get("title"))
                    .or_else(|| r.get("display_name"))
                    .or_else(|| r.get("room_subject"))
                    .and_then(|t| t.as_str())
                    .unwrap_or(username),
            )
            .trim()
            .to_string();
            let thumbnail = r
                .get("previewUrlThumbSmall")
                .or_else(|| r.get("previewUrlThumbBig"))
                .or_else(|| r.get("previewUrl"))
                .or_else(|| r.get("avatarUrl"))
                .or_else(|| r.get("img"))
                .or_else(|| r.get("thumbnailUrl"))
                .or_else(|| r.get("thumbnail"))
                .or_else(|| r.get("image_url"))
                .or_else(|| r.get("preview"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            let viewers = r
                .get("viewersCount")
                .or_else(|| r.get("num_users"))
                .or_else(|| r.get("viewers"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let age = r.get("age").and_then(|a| a.as_u64()).map(|n| n as u32);
            let gender = r
                .get("gender")
                .or_else(|| r.get("genderGroup"))
                .and_then(|g| g.as_str())
                .map(|s| s.to_string());
            let stream_url = r
                .get("hlsPlaylist")
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            Some(HttpRoom {
                username: username.to_string(),
                title: if title.is_empty() {
                    username.to_string()
                } else {
                    title
                },
                thumbnail,
                viewers,
                age,
                gender,
                stream_url,
            })
        })
        .take(MAX_ROOMS)
        .collect();
    if rooms.is_empty() {
        None
    } else {
        Some(rooms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_api_json_reads_stripchat_models() {
        let body = r#"{
            "models": [{
                "username": "demo_model",
                "previewUrlThumbSmall": "https://example.com/thumb.jpg",
                "viewersCount": 42,
                "genderGroup": "F",
                "hlsPlaylist": "https://edge-hls.example/hls/1/master/1_240p.m3u8",
                "isLive": true
            }],
            "totalCount": 1
        }"#;
        let rooms = parse_api_json(body).expect("rooms");
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].username, "demo_model");
        assert_eq!(rooms[0].viewers, Some(42));
        assert!(rooms[0].stream_url.as_deref().unwrap().contains("m3u8"));
    }

    #[test]
    fn parse_preloaded_state_html_extracts_model_and_hosts() {
        // Try to load fixture if present
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("stripchat_preloaded.html");
        if fixture_path.exists() {
            let html = std::fs::read_to_string(&fixture_path).expect("fixture");
            let state = parse_preloaded_state_html(&html, "madelinexx").expect("state");
            assert!(!state.model_id.is_empty());
            assert!(!state.hosts.is_empty());
            return;
        }
        // Fallback to synthetic minimal fixture
        let html = r#"<!doctype html><html><body><script>window.__PRELOADED_STATE__={"viewCam":{"model":{"id":"123456","isLive":true},"show":null},"config":{"data":{"features":{"featuresV2":{"hlsFallback":{"fallbackDomains":["doppiocdn.net","doppiocdn.com"]}}},"hlsStreamHost":"doppiocdn.net"}}};</script></body></html>"#;
        let state = parse_preloaded_state_html(html, "demo").expect("state");
        assert_eq!(state.model_id, "123456");
        assert!(state.hosts.contains(&"doppiocdn.net".to_string()));
    }

    #[test]
    fn parse_preloaded_state_html_rejects_offline() {
        let html = r#"<!doctype html><html><body><script>window.__PRELOADED_STATE__={"viewCam":{"model":{"id":"123456","isLive":false},"show":null},"config":{"data":{"features":{"featuresV2":{"hlsFallback":{"fallbackDomains":["doppiocdn.net"]}}}}}};</script></body></html>"#;
        let err = parse_preloaded_state_html(html, "demo").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("offline"));
    }

    #[test]
    fn parse_preloaded_state_html_rejects_private_show() {
        let html = r#"<!doctype html><html><body><script>window.__PRELOADED_STATE__={"viewCam":{"model":{"id":"123456","isLive":true},"show":{"type":"private"}},"config":{"data":{"features":{"featuresV2":{"hlsFallback":{"fallbackDomains":["doppiocdn.net"]}}}}}};</script></body></html>"#;
        let err = parse_preloaded_state_html(html, "demo").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("private"));
    }
}

/// Try to parse the HTML response as a list of room cards.
fn parse_listing_html(html: &str) -> Option<Vec<HttpRoom>> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Multiple selector strategies for room cards.
    let link_selectors = [
        "a[href^='/']:not([href*='tags']):not([href*='search']):not([href*='embed']):not([href*='api']):not([href*='auth']):not([href*='login']):not([href*='register']):not([href*='about']):not([href*='terms']):not([href*='dmca']):not([href*='faq']):not([href*='pricing']):not([href*='blog']):not([href*='press'])",
        ".room-card a[href^='/']",
        "a.model-card[href^='/']",
        ".models-list a[href^='/']",
    ];

    let mut rooms: Vec<HttpRoom> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in &link_selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        for el in document.select(&sel) {
            let href = el.value().attr("href").unwrap_or("");
            let username = href
                .trim_matches('/')
                .split('/')
                .next()
                .filter(|s| !s.is_empty() && !s.contains('?') && !s.contains('#'))?;

            // Filter out non-username paths.
            if !is_valid_username(username) {
                continue;
            }

            if !seen.insert(username.to_string()) {
                continue;
            }

            let title = crate::sites::urls::html_unescape(
                &el.value()
                    .attr("title")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| el.text().collect::<String>().trim().to_string()),
            )
            .replace('\n', " ")
            .trim()
            .to_string();

            if title.is_empty() || title.len() < 2 {
                continue;
            }

            // Extract thumbnail from nearby img elements.
            let thumbnail = find_thumbnail_near_element(&el);
            let viewers = extract_viewers_from_text(&title);
            let age = extract_age_from_text(&title);

            rooms.push(HttpRoom {
                username: username.to_string(),
                title,
                thumbnail,
                viewers,
                age,
                gender: None,
                stream_url: None,
            });

            if rooms.len() >= MAX_ROOMS {
                break;
            }
        }
        if !rooms.is_empty() {
            break;
        }
    }

    if rooms.is_empty() {
        None
    } else {
        Some(rooms)
    }
}

fn find_thumbnail_near_element(element: &scraper::ElementRef<'_>) -> Option<String> {
    use scraper::Selector;

    // Try multiple thumbnail CDN domains used by Stripchat
    let img_selectors = [
        "img[src*='static.stripchat.com']",
        "img[data-src*='static.stripchat.com']",
        "img[src*='thumbs-strip']",
    ];

    for sel_str in &img_selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        if let Some(img) = element.select(&sel).next() {
            let src = img
                .value()
                .attr("src")
                .or_else(|| img.value().attr("data-src"))
                .filter(|s| !s.starts_with("data:"));
            if let Some(s) = src {
                return Some(normalize_thumb_url(s));
            }
        }
    }

    // Fallback: find any img that looks like a thumbnail within this element
    let any_img = Selector::parse("img").ok()?;
    element.select(&any_img).find_map(|img| {
        let src = img
            .value()
            .attr("src")
            .or_else(|| img.value().attr("data-src"))
            .filter(|s| !s.starts_with("data:") && s.len() > 10)?;
        Some(normalize_thumb_url(src))
    })
}

fn normalize_thumb_url(s: &str) -> String {
    if s.starts_with("//") {
        format!("https:{s}")
    } else if s.starts_with('/') {
        format!("https://static.stripchat.com{s}")
    } else if !s.starts_with("http") {
        format!("https://{BASE}{s}")
    } else {
        s.to_string()
    }
}

fn extract_viewers_from_text(text: &str) -> Option<u32> {
    for pattern in &["viewers", "watching", "people"] {
        if let Some(idx) = text.to_lowercase().find(pattern) {
            let before = &text[..idx];
            let num_str: String = before
                .chars()
                .rev()
                .take_while(|c| {
                    c.is_ascii_digit() || *c == '.' || *c == ',' || *c == 'k' || *c == 'K'
                })
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if !num_str.is_empty() {
                if let Some(num) = parse_viewer_count(&num_str) {
                    return Some(num);
                }
            }
        }
    }
    None
}

fn parse_viewer_count(raw: &str) -> Option<u32> {
    let lower = raw.trim().to_lowercase();
    if let Some(num_str) = lower.strip_suffix('k') {
        if let Ok(num) = num_str.parse::<f64>() {
            return Some((num * 1000.0) as u32);
        }
    }
    lower.replace(',', "").parse::<u32>().ok()
}

fn extract_age_from_text(text: &str) -> Option<u32> {
    for word in text.split_whitespace() {
        if let Ok(age) = word.parse::<u32>() {
            if (18..=99).contains(&age) {
                return Some(age);
            }
        }
    }
    None
}

fn extract_username_from_url(url: &str) -> Option<String> {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let path = path.split('?').next().unwrap_or(path);
    let after_host = path.split_once('/').map(|(_, rest)| rest).unwrap_or("");
    let first = after_host.split('/').find(|s| !s.is_empty())?;
    if first.is_empty() || !is_valid_username(first) {
        return None;
    }
    Some(first.to_string())
}

#[cfg(desktop)]
fn map_room(room: &crate::sites::adapters::stripchat_webview::_RawRoom) -> MediaItem {
    let username = &room.username;
    let room_url = format!("{BASE}/{username}/");
    let embed_url = format!("{BASE}/embed/{username}/");
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: room.title.clone(),
        url: room_url,
        thumbnail: room.thumbnail.clone(),
        duration: None,
        site_id: "stripchat".to_string(),
        performers: vec![username.clone()],
        tags: vec![],
        description: None,
        channel: Some(username.clone()),
        is_live: Some(true),
        viewers: room.viewers,
        age: room.age,
        gender: room.gender.clone(),
        stream_url: None,
        embed_url: Some(embed_url),
    }
}

fn path_slug(slug: &str) -> String {
    slug.trim_start_matches('/')
        .trim_end_matches('/')
        .replace(' ', "_")
}

fn url_slug(slug: &str) -> String {
    slug.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            b' ' => "+".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
