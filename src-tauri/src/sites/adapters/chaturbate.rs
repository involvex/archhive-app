use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
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
        let url = match query.kind {
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
            BrowseKind::Search => {
                format!("{BASE}/search/?q={}", url_slug(&query.slug))
            }
            BrowseKind::Model => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}/", BASE, path_slug(&query.slug))
                }
            }
            _ => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}/", BASE, path_slug(&query.slug))
                }
            }
        };

        let html = ctx.fetch_html(&url, "chaturbate").await?;
        let items = parse_room_list(&html);

        if items.is_empty() {
            let fallback =
                ytdlp_browse_fallback(ctx, "chaturbate", &url, query.page, 48).await?;
            return Ok(BrowsePage {
                items: fallback,
                page: query.page,
                has_more: false,
                total: None,
            });
        }

        let has_more = items.len() >= 48;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
        })
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

fn parse_room_list(html: &str) -> Vec<MediaItem> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Primary selector: room list items on the main page
    let room_sel = Selector::parse("ul#room_list li, div.room-list-tile, div.model-link-div").unwrap();
    let link_sel = Selector::parse("a[href]").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    let mut items = Vec::new();

    for room_el in document.select(&room_sel) {
        // Find the first link that looks like a room link
        let Some(link_el) = room_el
            .select(&link_sel)
            .find(|a| {
                let href = a.value().attr("href").unwrap_or("");
                href.starts_with('/') && !href.contains("/tags/") && !href.contains("/search/")
            })
        else {
            continue;
        };

        let href = link_el.value().attr("href").unwrap_or("/");
        let username = href.trim_start_matches('/').trim_end_matches('/');

        if username.is_empty() || username.contains('/') {
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
                    .filter(|s| !s.starts_with("data:") && !s.is_empty())
                    .map(|s| {
                        if s.starts_with("http") {
                            s.to_string()
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
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
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
