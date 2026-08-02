use crate::error::AppResult;
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
        // yt-dlp's Chaturbate extractor only handles individual room URLs, NOT listings.
        // Passing /tags/<x>/ or /search/?q=... to yt-dlp fails with
        // "ERROR: [Chaturbate] <slug>: Room is currently offline". So we NEVER use the
        // yt-dlp browse fallback here. Instead we scrape the server-rendered HTML for
        // every listing kind, and return an empty page (with has_more=false) if no
        // rooms are found. The frontend surfaces a "no rooms found" message.

        match query.kind {
            BrowseKind::Model => self.browse_model(ctx, query).await,
            BrowseKind::Livestream => self.browse_listing(ctx, query).await,
            BrowseKind::Tag => self.browse_listing(ctx, query).await,
            BrowseKind::Search => self.browse_listing(ctx, query).await,
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
        })
    }
}

impl ChaturbateAdapter {
    /// `livestream` / `tag` / `search` / generic: render an HTML listing page and scrape it.
    /// Never falls back to yt-dlp — it cannot list Chaturbate rooms.
    async fn browse_listing(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_listing_url(&query);
        let html = ctx.fetch_html(&url, "chaturbate").await?;
        let items = parse_room_list(&html);
        let has_more = items.len() >= 48;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
        })
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
    let path = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let path = path.split('?').next().unwrap_or(path);
    let after_host = path.split_once('/').map(|(_, rest)| rest).unwrap_or("");
    let first = after_host.split('/').find(|s| !s.is_empty())?;
    if first.is_empty() || first == "tags" || first == "search" || first == "embed" {
        return None;
    }
    Some(first.to_string())
}

fn parse_room_list(html: &str) -> Vec<MediaItem> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Primary selector: room list items on the main page and tag/search pages.
    let room_sel =
        Selector::parse("ul#room_list li, div.room-list-tile, div.model-link-div, li.room-list-tile, div.room_list > div").unwrap();
    let link_sel = Selector::parse("a[href]").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    let mut items = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for room_el in document.select(&room_sel) {
        // Find the first link that looks like a room link (path-only username).
        let Some(link_el) = room_el
            .select(&link_sel)
            .find(|a| {
                let href = a.value().attr("href").unwrap_or("");
                href.starts_with('/')
                    && !href.contains("/tags/")
                    && !href.contains("/search/")
                    && !href.contains("/embed/")
            })
        else {
            continue;
        };

        let href = link_el.value().attr("href").unwrap_or("/");
        let username_raw = href.trim_start_matches('/').trim_end_matches('/');
        // strip any trailing query/fragment
        let username = username_raw.split(['?', '#']).next().unwrap_or(username_raw);

        if username.is_empty() || username.contains('/') {
            continue;
        }

        // dedupe by username (room list sometimes repeats)
        if !seen.insert(username.to_string()) {
            continue;
        }

        let room_url = format!("{BASE}/{username}/");
        let embed_url = format!("{BASE}/embed/{username}/");

        // Extract title / room title
        let title = room_el
            .text()
            .map(|t| t.trim().to_string())
            .find(|t| !t.is_empty())
            .unwrap_or_else(|| username.to_string());

        // Extract thumbnail
        let thumbnail = room_el
            .select(&img_sel)
            .find_map(|img| {
                img.value()
                    .attr("src")
                    .or_else(|| img.value().attr("data-src"))
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .filter(|s| !s.starts_with("data:") && !s.is_empty())
                    .map(|s| {
                        if s.starts_with("http") {
                            s.to_string()
                        } else if s.starts_with("//") {
                            format!("https:{s}")
                        } else {
                            format!("https:{s}")
                        }
                    })
            });

        // Try to extract viewer count from text content
        let viewers = extract_viewers_from_text(&room_el.text().collect::<Vec<_>>().join(" "));

        // Try to extract gender from room title or classes
        let gender = detect_gender(&room_el.html());

        // Try to extract age from room title
        let age = extract_age_from_text(&title);

        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title,
            url: room_url,
            thumbnail,
            duration: None,
            site_id: "chaturbate".to_string(),
            performers: vec![username.to_string()],
            tags: vec![],
            description: None,
            channel: Some(username.to_string()),
            is_live: Some(true),
            viewers,
            age,
            gender,
            stream_url: None,
            embed_url: Some(embed_url),
        });

        if items.len() >= 48 {
            break;
        }
    }

    items
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

fn extract_viewers_from_text(text: &str) -> Option<u32> {
    // Look for patterns like "123 viewers", "123 people watching", etc.
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    // Heuristic: if there's a number followed by "viewers" or similar
    let lower = text.to_lowercase();
    if lower.contains("viewer") || lower.contains("watching") || lower.contains("people") {
        digits.parse().ok()
    } else {
        None
    }
}

fn detect_gender(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    if lower.contains("gender-female") || lower.contains("females") || lower.contains("girl") {
        Some("female".to_string())
    } else if lower.contains("gender-male") || lower.contains("males") || lower.contains("guy") {
        Some("male".to_string())
    } else if lower.contains("gender-couple") || lower.contains("couples") {
        Some("couple".to_string())
    } else if lower.contains("gender-trans") || lower.contains("trans") {
        Some("trans".to_string())
    } else {
        None
    }
}

fn extract_age_from_text(text: &str) -> Option<u32> {
    // Look for common patterns like "19", "22f", "21F" in room titles
    for word in text.split_whitespace() {
        let cleaned: String = word.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Ok(age) = cleaned.parse::<u32>() {
            if (18..=99).contains(&age) {
                return Some(age);
            }
        }
    }
    None
}
