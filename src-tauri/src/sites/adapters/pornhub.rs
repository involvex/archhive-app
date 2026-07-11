use crate::error::AppResult;
use crate::models::{
    BrowseKind, BrowseOrientation, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem,
};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
use crate::sites::urls::{path_slug, query_slug};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

macro_rules! ytdlp_tube_adapter {
    ($struct_name:ident, $id:expr, $name:expr, $base:expr) => {
        pub struct $struct_name;

        #[async_trait]
        impl SiteAdapter for $struct_name {
            fn id(&self) -> &str {
                $id
            }

            fn display_name(&self) -> &str {
                $name
            }

            fn base_url(&self) -> &str {
                $base
            }

            fn supported_kinds(&self) -> Vec<BrowseKind> {
                vec![
                    BrowseKind::Tag,
                    BrowseKind::Search,
                    BrowseKind::Channel,
                    BrowseKind::Video,
                ]
            }

            fn requires_cookies(&self) -> bool {
                false
            }

            async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
                let url = match query.kind {
                    BrowseKind::Tag => format!("{}/tags/{}", $base, path_slug(&query.slug)),
                    BrowseKind::Search => format!("{}/?k={}", $base, query_slug(&query.slug)),
                    BrowseKind::Channel => format!("{}/channels/{}", $base, path_slug(&query.slug)),
                    BrowseKind::Video => {
                        if query.slug.starts_with("http") {
                            query.slug.clone()
                        } else {
                            format!("{}/videos/{}", $base, path_slug(&query.slug))
                        }
                    }
                    BrowseKind::Model => format!("{}/pornstar/{}", $base, path_slug(&query.slug)),
                    BrowseKind::Category => format!("{}/categories/{}", $base, path_slug(&query.slug)),
                };
                let html = ctx.fetch_html(&url, $id).await?;
                let mut items = parse_video_links(&html, $base, $id)?;
                if items.is_empty() {
                    items = ytdlp_browse_fallback(ctx, $id, &url, query.page, 24).await?;
                }
                let has_more = items.len() >= 20;
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
                    output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
                    tool: DownloadTool::YtDlp,
                    title: Some(item.title.clone()),
                    performers: item.performers.clone(),
                    tags: item.tags.clone(),
                    adapter_id: $id.to_string(),
                })
            }
        }
    };
}

pub struct PornhubAdapter;

const PH_BASE: &str = "https://www.pornhub.com";

#[async_trait]
impl SiteAdapter for PornhubAdapter {
    fn id(&self) -> &str {
        "pornhub"
    }

    fn display_name(&self) -> &str {
        "PornHub"
    }

    fn base_url(&self) -> &str {
        PH_BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Category,
            BrowseKind::Search,
            BrowseKind::Model,
            BrowseKind::Channel,
            BrowseKind::Tag,
            BrowseKind::Video,
        ]
    }

    fn requires_cookies(&self) -> bool {
        true
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = match query.kind {
            BrowseKind::Category => build_pornhub_category_url(&query),
            BrowseKind::Search | BrowseKind::Tag => {
                format!("{PH_BASE}/video/search?search={}", query_slug(&query.slug))
            }
            BrowseKind::Model => format!("{PH_BASE}/pornstar/{}", path_slug(&query.slug)),
            BrowseKind::Channel => format!("{PH_BASE}/channels/{}", path_slug(&query.slug)),
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{PH_BASE}/view_video.php?viewkey={}", path_slug(&query.slug))
                }
            }
        };
        let html = ctx.fetch_html(&url, self.id()).await?;
        let mut items = parse_video_links(&html, PH_BASE, self.id())?;
        if items.is_empty() {
            items = ytdlp_browse_fallback(ctx, self.id(), &url, query.page, 24).await?;
        }
        if items.is_empty() {
            return Err(crate::error::AppError::Site(
                "No videos found. Import PornHub cookies in Settings → Cookies.".into(),
            ));
        }
        let has_more = items.len() >= 20;
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
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.id().to_string(),
        })
    }
}

ytdlp_tube_adapter!(XHamsterAdapter, "xhamster", "xHamster", "https://xhamster.com");
ytdlp_tube_adapter!(XVideosAdapter, "xvideos", "XVIDEOS", "https://www.xvideos.com");

fn page_query(page: u32) -> String {
    if page > 1 {
        format!("?page={page}")
    } else {
        String::new()
    }
}

fn page_amp(page: u32) -> String {
    if page > 1 {
        format!("&page={page}")
    } else {
        String::new()
    }
}

fn build_pornhub_category_url(query: &BrowseQuery) -> String {
    let orientation = query
        .orientation
        .unwrap_or(BrowseOrientation::Straight);
    let slug = path_slug(&query.slug);

    if query.slug.chars().all(|c| c.is_ascii_digit()) {
        let id = &query.slug;
        return match orientation {
            BrowseOrientation::Gay => {
                format!("{PH_BASE}/gay/video?c={id}{}", page_amp(query.page))
            }
            BrowseOrientation::Transgender => {
                format!("{PH_BASE}/transgender/video?c={id}{}", page_amp(query.page))
            }
            _ => format!("{PH_BASE}/video?c={id}{}", page_amp(query.page)),
        };
    }

    match orientation {
        BrowseOrientation::Straight => {
            format!("{PH_BASE}/categories/{slug}{}", page_query(query.page))
        }
        BrowseOrientation::Gay => {
            format!("{PH_BASE}/gay/categories/{slug}{}", page_query(query.page))
        }
        BrowseOrientation::Lesbian => {
            format!("{PH_BASE}/lesbian/categories/{slug}{}", page_query(query.page))
        }
        BrowseOrientation::Transgender => {
            format!("{PH_BASE}/transgender/categories/{slug}{}", page_query(query.page))
        }
    }
}

fn parse_video_links(html: &str, base: &str, site_id: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let selectors: &[&str] = if site_id == "pornhub" {
        &[
            ".ph-pornstar-videos-list .pcVideoListItem a[href*='view_video']",
            ".pcVideoListItem a[href*='view_video']",
            "a[href*='view_video.php']",
            "a[href*='view_video?']",
        ]
    } else {
        &[
            "a[href*='view_video']",
            "a[href*='/video']",
            "a[href*='watch']",
            ".pcVideoListItem a",
            ".ph-video-item a",
            ".videoBox a",
        ]
    };
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else { continue };
        for el in document.select(&sel) {
            let Some(href) = el.value().attr("href") else {
                continue;
            };
            if site_id == "pornhub" && !is_pornhub_video_href(href) {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{base}{href}")
            };
            if site_id == "pornhub" && !url.contains("view_video") {
                continue;
            }
            if site_id != "pornhub"
                && !url.contains("video")
                && !url.contains("view_video")
                && !url.contains("watch")
            {
                continue;
            }
            if !seen.insert(url.clone()) {
                continue;
            }
            let title = el
                .value()
                .attr("title")
                .map(|s| s.to_string())
                .unwrap_or_else(|| el.text().collect::<String>().trim().to_string());
            if title.is_empty() {
                continue;
            }
            let img_sel = Selector::parse("img").ok();
            let thumbnail = if site_id == "pornhub" {
                el.value().attr("data-mediumthumb").map(|s| s.to_string()).or_else(|| {
                    img_sel.as_ref().and_then(|sel| {
                        el.select(sel).find_map(|img| {
                            img.value()
                                .attr("data-src")
                                .or_else(|| img.value().attr("src"))
                                .map(|s| {
                                    if s.starts_with("http") {
                                        s.to_string()
                                    } else {
                                        format!("{base}{s}")
                                    }
                                })
                        })
                    })
                })
            } else {
                None
            };
            items.push(MediaItem {
                id: Uuid::new_v4().to_string(),
                title,
                url,
                thumbnail,
                duration: None,
                site_id: site_id.to_string(),
                performers: vec![],
                tags: vec![],
            });
            if items.len() >= 40 {
                break;
            }
        }
        if items.len() >= 20 {
            break;
        }
    }

    Ok(items)
}

fn is_pornhub_video_href(href: &str) -> bool {
    let lower = href.to_lowercase();
    if !lower.contains("view_video") {
        return false;
    }
    let skip = [
        "/channel/",
        "/channels/",
        "/categories/",
        "/video/search",
        "/users/",
        "/model/",
        "/pornstar/",
    ];
    !skip.iter().any(|s| lower.contains(s))
}

pub fn categories_page_url(orientation: BrowseOrientation) -> String {
    match orientation {
        BrowseOrientation::Straight => format!("{PH_BASE}/categories"),
        BrowseOrientation::Gay => format!("{PH_BASE}/gay/categories"),
        BrowseOrientation::Lesbian => format!("{PH_BASE}/lesbian/categories"),
        BrowseOrientation::Transgender => format!("{PH_BASE}/transgender/categories"),
    }
}

pub fn parse_pornhub_categories(
    html: &str,
    orientation: BrowseOrientation,
) -> Vec<crate::models::PornhubCategoryEntry> {
    use crate::models::PornhubCategoryEntry;
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let selectors = [
        "li.categoryWrapper a",
        ".categoriesList li a",
        ".categoryList li a",
        "a[href*='/categories/']",
    ];
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else { continue };
        for el in document.select(&sel) {
            let Some(href) = el.value().attr("href") else { continue };
            let slug = category_slug_from_href(href);
            let Some(slug) = slug else { continue };
            let key = format!("{orientation:?}:{slug}");
            if !seen.insert(key) {
                continue;
            }
            let name = category_name_from_element(&el);
            if name.is_empty() {
                continue;
            }
            let video_count = category_count_from_element(&el);
            items.push(PornhubCategoryEntry {
                name,
                slug,
                orientation,
                category_id: None,
                video_count,
            });
        }
        if items.len() >= 20 {
            break;
        }
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn category_slug_from_href(href: &str) -> Option<String> {
    let lower = href.to_lowercase();
    let marker = "/categories/";
    let idx = lower.find(marker)?;
    let rest = &href[idx + marker.len()..];
    let slug = rest
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or("")
        .trim();
    if slug.is_empty() || slug == "categories" {
        return None;
    }
    Some(slug.to_string())
}

fn category_name_from_element(el: &scraper::element_ref::ElementRef<'_>) -> String {
    use scraper::Selector;
    if let Ok(sel) = Selector::parse(".categoryTitle, .title, span") {
        if let Some(span) = el.select(&sel).next() {
            let text = span.text().collect::<String>().trim().to_string();
            if !text.is_empty() && !text.to_lowercase().contains("video") {
                return text;
            }
        }
    }
    el.text()
        .collect::<String>()
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn category_count_from_element(el: &scraper::element_ref::ElementRef<'_>) -> Option<u32> {
    use scraper::Selector;
    let text = if let Ok(sel) = Selector::parse(".videosNumber, .categoryCount, span") {
        el.select(&sel)
            .map(|s| s.text().collect::<String>())
            .collect::<String>()
    } else {
        el.text().collect::<String>()
    };
    parse_video_count(&text)
}

fn parse_video_count(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.replace(',', "").parse().ok()
}

#[cfg(test)]
mod category_tests {
    use super::*;

    #[test]
    fn parses_slug_from_href() {
        assert_eq!(
            category_slug_from_href("/categories/big-tits?page=2"),
            Some("big-tits".to_string())
        );
    }

    #[test]
    fn parses_count_from_text() {
        assert_eq!(parse_video_count("544,892 Videos"), Some(544892));
    }
}
