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
                    BrowseKind::Model,
                ]
            }

            fn requires_cookies(&self) -> bool {
                false
            }

            async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
                let url = tube_browse_url($id, $base, &query);
                let html = ctx.fetch_html(&url, $id).await.unwrap_or_default();
                let mut items = parse_video_links(&html, $base, $id).unwrap_or_default();
                // Drop obvious nav junk so we fall through to yt-dlp.
                items.retain(|i| !is_junk_nav_title(&i.title));
                if items.is_empty() {
                    items = ytdlp_browse_fallback(ctx, $id, &url, query.page, 24).await?;
                }
                if items.is_empty() {
                    return Err(crate::error::AppError::Site(format!(
                        "No videos found on {}. Try another query, or import cookies if the site is blocking.",
                        $name
                    )));
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
                    format!(
                        "{PH_BASE}/view_video.php?viewkey={}",
                        path_slug(&query.slug)
                    )
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

ytdlp_tube_adapter!(
    XHamsterAdapter,
    "xhamster",
    "xHamster",
    "https://xhamster.com"
);
ytdlp_tube_adapter!(
    XVideosAdapter,
    "xvideos",
    "XVIDEOS",
    "https://www.xvideos.com"
);

fn tube_browse_url(site_id: &str, base: &str, query: &BrowseQuery) -> String {
    let slug = path_slug(&query.slug);
    let q = query_slug(&query.slug);
    match site_id {
        "xvideos" => match query.kind {
            BrowseKind::Search | BrowseKind::Tag => {
                let page = if query.page > 1 {
                    format!("&p={}", query.page.saturating_sub(1))
                } else {
                    String::new()
                };
                format!("{base}/?k={q}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/models/{slug}"),
            BrowseKind::Category => format!("{base}/c/{slug}"),
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{base}/video.{slug}")
                }
            }
        },
        "xhamster" => match query.kind {
            BrowseKind::Search | BrowseKind::Tag => {
                let page = if query.page > 1 {
                    format!("?page={}", query.page)
                } else {
                    String::new()
                };
                format!("{base}/search/{slug}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/pornstars/{slug}"),
            BrowseKind::Category => format!("{base}/categories/{slug}"),
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{base}/videos/{slug}")
                }
            }
        },
        _ => format!("{base}/?k={q}"),
    }
}

fn is_junk_nav_title(title: &str) -> bool {
    let t = title.trim().to_lowercase();
    matches!(
        t.as_str(),
        "red videos"
            | "liked videos"
            | "join"
            | "login"
            | "sign in"
            | "sign up"
            | "upload"
            | "home"
            | "premium"
            | "remove ads"
            | "my favourites"
            | "my favorites"
    ) || t.len() < 3
}

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
    let orientation = query.orientation.unwrap_or(BrowseOrientation::Straight);
    let slug = path_slug(&query.slug);

    if query.slug.chars().all(|c| c.is_ascii_digit()) {
        let id = &query.slug;
        return match orientation {
            BrowseOrientation::Straight => {
                format!("{PH_BASE}/video?c={id}{}", page_amp(query.page))
            }
            BrowseOrientation::Gay => {
                format!("{PH_BASE}/gay/video?c={id}{}", page_amp(query.page))
            }
            BrowseOrientation::Lesbian => {
                format!("{PH_BASE}/lesbian/video?c={id}{}", page_amp(query.page))
            }
            BrowseOrientation::Transgender => {
                format!("{PH_BASE}/transgender/video?c={id}{}", page_amp(query.page))
            }
        };
    }

    match orientation {
        BrowseOrientation::Straight => {
            format!("{PH_BASE}/categories/{slug}{}", page_query(query.page))
        }
        // Slug-only paths under gay/lesbian/trans category hubs often have no video grid.
        // Use orientation search instead (hyphens → +).
        BrowseOrientation::Gay | BrowseOrientation::Lesbian | BrowseOrientation::Transgender => {
            let orient = match orientation {
                BrowseOrientation::Gay => "gay",
                BrowseOrientation::Lesbian => "lesbian",
                BrowseOrientation::Transgender => "transgender",
                BrowseOrientation::Straight => unreachable!(),
            };
            let search = slug.replace('-', "+");
            format!(
                "{PH_BASE}/{orient}/video/search?search={search}{}",
                page_amp(query.page)
            )
        }
    }
}

fn parse_video_links(html: &str, base: &str, site_id: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let selectors: &[&str] = match site_id {
        "pornhub" => &[
            ".ph-pornstar-videos-list .pcVideoListItem a[href*='view_video']",
            ".pcVideoListItem a[href*='view_video']",
            "a[href*='view_video.php']",
            "a[href*='view_video?']",
        ],
        "xvideos" => &[
            // Modern XVideos IDs look like /video.abc123/slug — not /videos-i-like nav links.
            "div.thumb-block a[href*='/video.']",
            "div.mozaique a[href*='/video.']",
            "a[href*='/video.']",
        ],
        "xhamster" => &[
            "a[href*='/videos/'][href*='-']",
            ".thumb-list__item a[href*='/videos/']",
            "a.video-thumb__image-container[href*='/videos/']",
        ],
        _ => &["a[href*='/video.']", "a[href*='/videos/']"],
    };
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        for el in document.select(&sel) {
            let Some(href) = el.value().attr("href") else {
                continue;
            };
            if !is_site_video_href(site_id, href) {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else if href.starts_with("//") {
                format!("https:{href}")
            } else {
                format!("{base}{href}")
            };
            if !seen.insert(url.clone()) {
                continue;
            }
            let title = el
                .value()
                .attr("title")
                .map(|s| s.to_string())
                .unwrap_or_else(|| el.text().collect::<String>().trim().to_string());
            if title.is_empty() || is_junk_nav_title(&title) {
                continue;
            }
            let img_sel = Selector::parse("img").ok();
            let thumbnail = match site_id {
                "pornhub" => el
                    .value()
                    .attr("data-mediumthumb")
                    .map(|s| s.to_string())
                    .or_else(|| {
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
                    }),
                "xvideos" | "xhamster" => img_sel.as_ref().and_then(|sel| {
                    el.select(sel).find_map(|img| {
                        img.value()
                            .attr("data-src")
                            .or_else(|| img.value().attr("data-thumb"))
                            .or_else(|| img.value().attr("src"))
                            .filter(|s| !s.starts_with("data:"))
                            .map(|s| {
                                if s.starts_with("http") || s.starts_with("//") {
                                    if s.starts_with("//") {
                                        format!("https:{s}")
                                    } else {
                                        s.to_string()
                                    }
                                } else {
                                    format!("{base}{s}")
                                }
                            })
                    })
                }),
                _ => None,
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
                description: None,
                channel: None,
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

fn is_site_video_href(site_id: &str, href: &str) -> bool {
    let lower = href.to_lowercase();
    match site_id {
        "pornhub" => is_pornhub_video_href(href),
        "xvideos" => {
            // Require /video.ID/ — excludes /videos-i-like, /account/..., etc.
            lower.contains("/video.")
                && !lower.contains("/videos-")
                && !lower.contains("/account")
                && !lower.contains("/profiles/")
        }
        "xhamster" => {
            lower.contains("/videos/")
                && !lower.contains("/videos/best")
                && !lower.contains("/videos/newest")
                && !lower.contains("/categories/")
                && !lower.contains("/users/")
                && !lower.contains("/my/")
        }
        _ => lower.contains("/video.") || lower.contains("/videos/"),
    }
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
        "a[href*='c=']",
    ];
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        for el in document.select(&sel) {
            let Some(href) = el.value().attr("href") else {
                continue;
            };
            let category_id = category_id_from_href(href);
            let slug =
                category_slug_from_href(href).or_else(|| category_id.map(|id| id.to_string()));
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
                category_id,
                video_count,
            });
        }
        if items.len() >= 40 {
            break;
        }
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn category_id_from_href(href: &str) -> Option<u32> {
    let lower = href.to_lowercase();
    for marker in ["?c=", "&c="] {
        if let Some(idx) = lower.find(marker) {
            let rest = &href[idx + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u32>() {
                return Some(id);
            }
        }
    }
    None
}

fn category_slug_from_href(href: &str) -> Option<String> {
    let lower = href.to_lowercase();
    let marker = "/categories/";
    let idx = lower.find(marker)?;
    let rest = &href[idx + marker.len()..];
    let slug = rest.split(&['/', '?', '#'][..]).next().unwrap_or("").trim();
    if slug.is_empty() || slug == "categories" || slug.chars().all(|c| c.is_ascii_digit()) {
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
    fn parses_category_id_from_href() {
        assert_eq!(category_id_from_href("/video?c=8"), Some(8));
        assert_eq!(
            category_id_from_href("/lesbian/video?c=27&page=2"),
            Some(27)
        );
        assert_eq!(category_id_from_href("/categories/big-tits"), None);
    }

    #[test]
    fn parses_count_from_text() {
        assert_eq!(parse_video_count("544,892 Videos"), Some(544892));
    }

    #[test]
    fn lesbian_numeric_category_url() {
        let url = build_pornhub_category_url(&BrowseQuery {
            kind: BrowseKind::Category,
            slug: "8".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(url, format!("{PH_BASE}/lesbian/video?c=8"));
    }

    #[test]
    fn lesbian_slug_falls_back_to_search() {
        let url = build_pornhub_category_url(&BrowseQuery {
            kind: BrowseKind::Category,
            slug: "big-tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/lesbian/video/search?search=big+tits")
        );
    }

    #[test]
    fn straight_slug_keeps_categories_path() {
        let url = build_pornhub_category_url(&BrowseQuery {
            kind: BrowseKind::Category,
            slug: "big-tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Straight),
        });
        assert_eq!(url, format!("{PH_BASE}/categories/big-tits"));
    }

    #[test]
    fn xvideos_rejects_nav_hrefs() {
        assert!(!is_site_video_href("xvideos", "/videos-i-like"));
        assert!(!is_site_video_href("xvideos", "/account"));
        assert!(is_site_video_href(
            "xvideos",
            "/video.opammko7524/3_lesbians_1_staircase"
        ));
    }

    #[test]
    fn xvideos_search_url() {
        let url = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &BrowseQuery {
                kind: BrowseKind::Search,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(url, "https://www.xvideos.com/?k=lesbian");
    }

    #[test]
    fn xhamster_search_url() {
        let url = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &BrowseQuery {
                kind: BrowseKind::Search,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(url, "https://xhamster.com/search/lesbian");
    }

    #[test]
    fn junk_nav_titles_detected() {
        assert!(is_junk_nav_title("RED videos"));
        assert!(is_junk_nav_title("Liked videos"));
        assert!(is_junk_nav_title("Join"));
        assert!(!is_junk_nav_title("Lesbian massage"));
    }
}
