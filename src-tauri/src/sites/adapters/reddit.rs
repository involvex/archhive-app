use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

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
        let url = match query.kind {
            BrowseKind::Channel => format!("https://www.reddit.com/r/{}/", query.slug),
            BrowseKind::Search => {
                format!("https://www.reddit.com/search/?q={}&type=link", query.slug)
            }
            _ => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("https://www.reddit.com/r/{}/", query.slug)
                }
            }
        };
        let html = ctx.fetch_html(&url, "reddit").await?;
        let items = parse_reddit(&html)?;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more: false,
            total: None,
        })
    }

    async fn resolve_download(&self, _ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "reddit/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: vec![],
            tags: vec!["reddit".to_string()],
            adapter_id: "reddit".to_string(),
        })
    }
}

fn parse_reddit(html: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let sel = Selector::parse("a[data-click-id='body']").unwrap_or_else(|_| Selector::parse("a").unwrap());
    let mut items = Vec::new();
    for el in document.select(&sel) {
        let Some(href) = el.value().attr("href") else { continue };
        if !href.contains("/comments/") && !href.contains("v.redd.it") {
            continue;
        }
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.reddit.com{href}")
        };
        let title = el.text().collect::<String>().trim().to_string();
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
            break;
        }
    }
    Ok(items)
}
