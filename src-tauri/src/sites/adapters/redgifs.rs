use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

pub struct RedgifsAdapter;

#[async_trait]
impl SiteAdapter for RedgifsAdapter {
    fn id(&self) -> &str {
        "redgifs"
    }

    fn display_name(&self) -> &str {
        "RedGifs"
    }

    fn base_url(&self) -> &str {
        "https://www.redgifs.com"
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Tag, BrowseKind::Search, BrowseKind::Video]
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = match query.kind {
            BrowseKind::Tag => format!("https://www.redgifs.com/tags/{}", query.slug),
            BrowseKind::Search => format!("https://www.redgifs.com/search/{}", query.slug),
            _ => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("https://www.redgifs.com/watch/{}", query.slug)
                }
            }
        };
        let html = ctx.fetch_html(&url, "redgifs").await?;
        let items = parse_redgifs(&html)?;
        let has_more = items.len() >= 20;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
        })
    }

    async fn resolve_download(&self, _ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "redgifs/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::GalleryDl,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "redgifs".to_string(),
        })
    }
}

fn parse_redgifs(html: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let sel = Selector::parse("a[href*='/watch/']").unwrap();
    let mut items = Vec::new();
    for el in document.select(&sel) {
        let Some(href) = el.value().attr("href") else { continue };
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.redgifs.com{href}")
        };
        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title: url.split('/').last().unwrap_or("redgif").to_string(),
            url,
            thumbnail: None,
            duration: None,
            site_id: "redgifs".to_string(),
            performers: vec![],
            tags: vec![],
        });
        if items.len() >= 30 {
            break;
        }
    }
    Ok(items)
}
