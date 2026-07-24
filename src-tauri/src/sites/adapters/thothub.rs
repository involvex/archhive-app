use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
use crate::sites::urls::path_slug;
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use scraper::{Html, Selector};
use uuid::Uuid;

pub struct ThotHubAdapter;

const BASE: &str = "https://thethothub.com";

#[async_trait]
impl SiteAdapter for ThotHubAdapter {
    fn id(&self) -> &str {
        "thothub"
    }

    fn display_name(&self) -> &str {
        "ThotHub"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Tag,
            BrowseKind::Model,
            BrowseKind::Search,
            BrowseKind::Video,
        ]
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_browse_url(&query)?;
        let mut items = match ctx.fetch_html(&url, self.id()).await {
            Ok(html) => parse_listing(&html, self.id()),
            Err(_) => Vec::new(),
        };
        if items.is_empty() {
            items = ytdlp_browse_fallback(ctx, self.id(), &url, query.page, 24).await?;
        }
        let has_more = items.len() >= 24;
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

fn build_browse_url(query: &BrowseQuery) -> AppResult<String> {
    let path = match query.kind {
        BrowseKind::Tag | BrowseKind::Model => format!("/tags/{}/", path_slug(&query.slug)),
        BrowseKind::Search => {
            // KVS search: /search/{query}/ (not ?search=)
            let q = path_slug(&query.slug);
            let page_suffix = if query.page > 1 {
                format!("?from={}", (query.page.saturating_sub(1)) * 24)
            } else {
                String::new()
            };
            return Ok(format!("{BASE}/search/{q}/{page_suffix}"));
        }
        BrowseKind::Video => {
            if query.slug.starts_with("http") {
                return Ok(query.slug.clone());
            }
            if query.slug.chars().all(|c| c.is_ascii_digit()) {
                format!("/videos/{}/", query.slug)
            } else {
                format!("/videos/{}/", path_slug(&query.slug))
            }
        }
        BrowseKind::Channel => format!("/channels/{}/", path_slug(&query.slug)),
        BrowseKind::Category => format!("/tags/{}/", path_slug(&query.slug)),
    };

    let page_suffix = if query.page > 1 {
        format!("?page={}", query.page)
    } else {
        String::new()
    };
    Ok(format!("{BASE}{path}{page_suffix}"))
}

fn is_video_href(href: &str) -> bool {
    href.contains("/videos/")
        || href.contains("/video/")
        || href.contains("/watch/")
        || href.contains("/v/")
        || href
            .trim_matches('/')
            .rsplit('/')
            .next()
            .is_some_and(|seg| seg.chars().all(|c| c.is_ascii_digit()) && !seg.is_empty())
}

fn abs_url(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else {
        format!("{BASE}/{}", href.trim_start_matches('/'))
    }
}

fn parse_listing(html: &str, site_id: &str) -> Vec<MediaItem> {
    let document = Html::parse_document(html);
    let container_selectors = [
        ".list-videos .item",
        ".list-videos",
        ".video-item",
        ".thumb-block",
        ".thumb",
        "article.post-item",
        ".post-item",
        ".card",
        ".videos-list .item",
    ];
    let link_sel = Selector::parse("a[href]").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in container_selectors {
        let Ok(container_sel) = Selector::parse(sel_str) else {
            continue;
        };
        for el in document.select(&container_sel) {
            let candidates: Vec<_> = el.select(&link_sel).collect();
            for a in candidates {
                let Some(href) = a.value().attr("href") else {
                    continue;
                };
                if !is_video_href(href) {
                    continue;
                }

                let url = abs_url(href);
                if !seen.insert(url.clone()) {
                    continue;
                }

                let title = a
                    .value()
                    .attr("title")
                    .map(|s| s.to_string())
                    .or_else(|| {
                        let text = a.text().collect::<String>();
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .unwrap_or_else(|| "Untitled".to_string());

                let thumbnail = el
                    .select(&img_sel)
                    .find_map(|img| {
                        img.value()
                            .attr("data-original")
                            .or_else(|| img.value().attr("data-src"))
                            .or_else(|| img.value().attr("src"))
                            .filter(|s| !s.starts_with("data:"))
                            .map(abs_url)
                    })
                    .or_else(|| {
                        a.select(&img_sel).find_map(|img| {
                            img.value()
                                .attr("data-original")
                                .or_else(|| img.value().attr("data-src"))
                                .or_else(|| img.value().attr("src"))
                                .filter(|s| !s.starts_with("data:"))
                                .map(abs_url)
                        })
                    });

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

                if items.len() >= 48 {
                    return items;
                }
            }
        }
        if items.len() >= 12 {
            break;
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn build_model_url() {
        let q = BrowseQuery {
            kind: BrowseKind::Model,
            slug: "sweetie fox".to_string(),
            page: 1,
            orientation: None,
        };
        assert!(build_browse_url(&q).unwrap().contains("/tags/sweetie-fox/"));
        assert!(build_browse_url(&q).unwrap().contains("thethothub.com"));
    }

    #[test]
    fn build_search_url_uses_query_param() {
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "sweetie fox".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_browse_url(&q).unwrap();
        assert!(
            url.contains("/search/sweetie-fox/"),
            "unexpected search url: {url}"
        );
    }

    #[test]
    fn build_video_url_numeric() {
        let q = BrowseQuery {
            kind: BrowseKind::Video,
            slug: "286350".to_string(),
            page: 1,
            orientation: None,
        };
        assert!(build_browse_url(&q).unwrap().contains("/videos/286350/"));
    }

    #[test]
    fn parse_fixture_search_page() {
        let html = fs::read_to_string("tests/fixtures/thothub_search.html")
            .expect("fixture thothub_search.html");
        let items = parse_listing(&html, "thothub");
        assert!(!items.is_empty(), "expected videos from fixture");
        assert!(items.iter().all(|i| i.url.contains("thothub")));
    }
}
