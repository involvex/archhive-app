use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

const BASE: &str = "https://chaturbate.com";

pub struct ChaturbateAdapter;

#[async_trait]
impl SiteAdapter for ChaturbateAdapter {
    fn id(&self) -> &str {
        "chaturbate"
    }

    fn display_name(&self) -> &str {
        "Chaturbate"
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

    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "chaturbate/%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "chaturbate".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
        })
    }
}

impl ChaturbateAdapter {
    async fn browse_listing(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_listing_url(&query);

        // Primary: HTTP scraping with vault cookies as Cookie header.
        if let Ok(html) = ctx.fetch_html(&url, &self.id()).await {
            if let Some(rooms) = parse_listing_html(&html) {
                let items: Vec<MediaItem> = rooms.into_iter().map(|r| http_room_to_item(&r)).collect();
                let has_more = items.len() >= 30;
                return Ok(BrowsePage { items, page: query.page, has_more, total: None });
            }
        }

        // Secondary: try the internal API endpoint with cookies.
        let api_url = build_api_url(&query);
        if api_url != url {
            if let Ok(body) = ctx.fetch_html(&api_url, &self.id()).await {
                if let Some(rooms) = parse_listing_html(&body) {
                    let items: Vec<MediaItem> = rooms.into_iter().map(|r| http_room_to_item(&r)).collect();
                    let has_more = items.len() >= 30;
                    return Ok(BrowsePage { items, page: query.page, has_more, total: None });
                }
                // If the API returned JSON directly, try parsing that.
                if let Some(rooms) = parse_api_json(&body) {
                    let items: Vec<MediaItem> = rooms.into_iter().map(|r| http_room_to_item(&r)).collect();
                    let has_more = items.len() >= 30;
                    return Ok(BrowsePage { items, page: query.page, has_more, total: None });
                }
            }
        }

        // Tertiary: webview bridge (desktop only).
        #[cfg(desktop)]
        {
            let rooms =
                crate::sites::adapters::chaturbate_webview::fetch_listing(ctx.app(), ctx.vault(), &url)
                    .await?;
            if !rooms.is_empty() {
                let items: Vec<MediaItem> = rooms.into_iter().map(map_room).collect();
                let has_more = items.len() >= 30;
                return Ok(BrowsePage { items, page: query.page, has_more, total: None });
            }
        }

        // Nothing worked.
        Err(AppError::Site(
            "No rooms found from Chaturbate — import cookies in Settings → Cookies or the listing page may have changed.".into(),
        ))
    }

    /// `model` kind: return a single synthetic `MediaItem` pointing at the model's room.
    /// `resolve_livestream` (yt-dlp `--get-url`) is what actually resolves the stream URL
    /// when the user opens the player page — this listing entry is just a navigation card.
    async fn browse_model(&self, _ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let slug_raw = query.slug.trim();
        if slug_raw.is_empty() {
            return Ok(BrowsePage {
                items: vec![],
                page: query.page,
                has_more: false,
                total: None,
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
            });
        }
        Ok(BrowsePage {
            items: vec![MediaItem {
                id: Uuid::new_v4().to_string(),
                title: username.clone(),
                url: room_url,
                thumbnail: None,
                duration: None,
                site_id: "chaturbate".to_string(),
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
        })
    }
}

/// Room data parsed from server-rendered HTML or API response.
struct HttpRoom {
    username: String,
    title: String,
    thumbnail: Option<String>,
    viewers: Option<u32>,
    age: Option<u32>,
    gender: Option<String>,
}

fn http_room_to_item(room: &HttpRoom) -> MediaItem {
    let room_url = format!("{BASE}/{}/", room.username);
    let embed_url = format!("{BASE}/embed/{}/", room.username);
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: room.title.clone(),
        url: room_url,
        thumbnail: room.thumbnail.clone(),
        duration: None,
        site_id: "chaturbate".to_string(),
        performers: vec![room.username.clone()],
        tags: vec![],
        description: None,
        channel: Some(room.username.clone()),
        is_live: Some(true),
        viewers: room.viewers,
        age: room.age,
        gender: room.gender.clone(),
        stream_url: None,
        embed_url: Some(embed_url),
    }
}

fn build_api_url(query: &BrowseQuery) -> String {
    match query.kind {
        BrowseKind::Tag => format!("{BASE}/api/ts/roomlist/room-list/?tag={}", path_slug(&query.slug)),
        BrowseKind::Search => format!("{BASE}/api/ts/roomlist/room-list/?q={}", url_slug(&query.slug)),
        _ => format!("{BASE}/api/ts/roomlist/room-list/"),
    }
}

/// Try to parse the HTML response as a list of room cards.
/// Handles both server-rendered full cards and placeholder-placeholder markup.
fn parse_listing_html(html: &str) -> Option<Vec<HttpRoom>> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Try multiple selector strategies for room cards.
    let link_selectors = [
        // Direct room links: <a href="/username/"> inside room cards
        "li.roomCard a[href^='/']:not([href*='tags']):not([href*='search']):not([href*='embed'])",
        "a.roomCard__title[href^='/']",
        // Generic room links
        ".room_list_room a[href^='/']",
        "a[data-room][href^='/']",
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
            if username.len() < 2
                || username.contains('.')
                || username == "api"
                || username == "tags"
                || username == "search"
                || username == "embed"
                || username == "affiliates"
                || username == "auth"
            {
                continue;
            }

            if !seen.insert(username.to_string()) {
                continue;
            }

            let title = el
                .value()
                .attr("title")
                .map(|s| s.to_string())
                .unwrap_or_else(|| el.text().collect::<String>().trim().to_string())
                .replace('\n', " ")
                .trim()
                .to_string();

            if title.is_empty() || title.len() < 2 {
                continue;
            }

            // Extract thumbnail from parent/ancestor img elements.
            let thumbnail = find_thumbnail_in_ancestors(&document);
            let viewers = extract_viewers_from_text(&title);
            let age = extract_age_from_text(&title);

            rooms.push(HttpRoom {
                username: username.to_string(),
                title,
                thumbnail,
                viewers,
                age,
                gender: None,
            });

            if rooms.len() >= 48 {
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

fn find_thumbnail_in_ancestors(
    document: &scraper::Html,
) -> Option<String> {
    use scraper::Selector;
    let img_sel = Selector::parse(
        "img[src*='mmcdn.com'], img[data-src*='mmcdn.com'], img[src*='highwebmedia.com']",
    )
    .ok()?;
    document
        .select(&img_sel)
        .find_map(|img| {
            img.value()
                .attr("src")
                .or_else(|| img.value().attr("data-src"))
                .filter(|s| !s.starts_with("data:"))
                .map(|s| {
                    if s.starts_with("//") {
                        format!("https:{s}")
                    } else if s.starts_with('/') {
                        format!("https://static.mmcdn.com{s}")
                    } else if !s.starts_with("http") {
                        format!("https://{BASE}{s}")
                    } else {
                        s.to_string()
                    }
                })
        })
}

fn extract_viewers_from_text(text: &str) -> Option<u32> {
    // Patterns: "1.2k viewers", "500 watching", "120 viewers"
    for pattern in &["viewers", "watching"] {
        if let Some(idx) = text.to_lowercase().find(pattern) {
            let before = &text[..idx];
            let num_str: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == 'k' || *c == 'K')
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

/// Try to parse a JSON API response for room data.
fn parse_api_json(body: &str) -> Option<Vec<HttpRoom>> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let rooms_val = v
        .get("rooms")
        .or_else(|| v.get("results"))
        .or_else(|| v.get("data"))
        .or_else(|| v.as_array().and_then(|_| Some(&v)));
    let arr = rooms_val?.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let rooms: Vec<HttpRoom> = arr
        .iter()
        .filter_map(|r| {
            let username = r.get("username")?.as_str()?;
            let title = r
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or(username)
                .to_string();
            let thumbnail = r
                .get("thumbnail")
                .or_else(|| r.get("image_url"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            let viewers = r
                .get("num_users")
                .or_else(|| r.get("viewers"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let age = r.get("age").and_then(|a| a.as_u64()).map(|n| n as u32);
            let gender = r.get("gender").and_then(|g| g.as_str()).map(|s| s.to_string());
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
            })
        })
        .take(48)
        .collect();
    if rooms.is_empty() {
        None
    } else {
        Some(rooms)
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
        BrowseKind::Search => format!("{BASE}/search/?q={}", url_slug(&query.slug)),
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

fn extract_username_from_url(url: &str) -> Option<String> {
    // Accept https://chaturbate.com/<user>/ or trailing path junk; return the first
    // path segment that looks like a username.
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let path = path.split('?').next().unwrap_or(path);
    let after_host = path.split_once('/').map(|(_, rest)| rest).unwrap_or("");
    let first = after_host.split('/').find(|s| !s.is_empty())?;
    if first.is_empty() || first == "tags" || first == "search" || first == "embed" {
        return None;
    }
    Some(first.to_string())
}

fn map_room(room: crate::sites::adapters::chaturbate_webview::RawRoom) -> MediaItem {
    let username = room.username;
    let room_url = format!("{BASE}/{username}/");
    let embed_url = format!("{BASE}/embed/{username}/");
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: room.title,
        url: room_url,
        thumbnail: room.thumbnail,
        duration: None,
        site_id: "chaturbate".to_string(),
        performers: vec![username.clone()],
        tags: vec![],
        description: None,
        channel: Some(username.clone()),
        is_live: Some(true),
        viewers: room.viewers,
        age: room.age,
        gender: room.gender,
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
