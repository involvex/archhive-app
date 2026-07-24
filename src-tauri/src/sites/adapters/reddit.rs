use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, MediaItem};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
use crate::sites::urls::{path_slug, query_slug};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

const OLD_BASE: &str = "https://old.reddit.com";

pub struct RedditAdapter;

#[async_trait]
impl SiteAdapter for RedditAdapter {
    fn id(&self) -> &str {
        "reddit"
    }

    fn display_name(&self) -> &str {
        "Reddit"
    }

    fn base_url(&self) -> &str {
        "https://www.reddit.com"
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Channel, BrowseKind::Search, BrowseKind::Video]
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_browse_url(&query)?;
        let html = ctx.fetch_html(&url, "reddit").await?;
        let mut items = parse_reddit(&html);
        if items.is_empty() {
            items = ytdlp_browse_fallback(ctx, self.id(), &url, query.page, 30).await?;
        }
        let has_more = items.len() >= 25;
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
        let tool = crate::downloads::image::resolve_download_tool(&item.url, "reddit");
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "reddit/%(title)s.%(ext)s".to_string(),
            tool,
            title: Some(item.title.clone()),
            performers: vec![],
            tags: vec!["reddit".to_string()],
            adapter_id: "reddit".to_string(),
        })
    }
}

fn build_browse_url(query: &BrowseQuery) -> AppResult<String> {
    Ok(match query.kind {
        BrowseKind::Channel => format!("{OLD_BASE}/r/{}/", path_slug(&query.slug)),
        BrowseKind::Search => {
            format!(
                "{OLD_BASE}/search?q={}&type=link&sort=relevance",
                query_slug(&query.slug)
            )
        }
        BrowseKind::Video => {
            if query.slug.starts_with("http") {
                query.slug.clone()
            } else {
                format!("{OLD_BASE}/r/{}/", path_slug(&query.slug))
            }
        }
        _ => format!("{OLD_BASE}/r/{}/", path_slug(&query.slug)),
    })
}

fn parse_reddit(html: &str) -> Vec<MediaItem> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let selectors = [
        "a.search-title",
        ".thing .title a.title",
        ".search-result a.title",
        "a[data-click-id='body']",
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
            if !href.contains("/comments/") && !href.contains("v.redd.it") {
                continue;
            }
            if href.contains("/user/") || href.contains("/wiki/") {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://www.reddit.com{href}")
            };
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
            items.push(MediaItem {
                id: Uuid::new_v4().to_string(),
                title,
                url,
                thumbnail: None,
                duration: None,
                site_id: "reddit".to_string(),
                performers: vec![],
                tags: vec![],
            });
            if items.len() >= 30 {
                return items;
            }
        }
        if !items.is_empty() {
            break;
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_url_encodes_spaces() {
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "sweetie fox".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_browse_url(&q).unwrap();
        assert!(url.starts_with(OLD_BASE));
        assert!(url.contains("q=sweetie"));
    }

    #[test]
    fn channel_uses_old_reddit() {
        let q = BrowseQuery {
            kind: BrowseKind::Channel,
            slug: "nsfw".to_string(),
            page: 1,
            orientation: None,
        };
        assert_eq!(
            build_browse_url(&q).unwrap(),
            "https://old.reddit.com/r/nsfw/"
        );
    }
}
