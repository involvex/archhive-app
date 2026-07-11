use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
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
                true
            }

            async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
                let url = match query.kind {
                    BrowseKind::Tag => format!("{}/tags/{}", $base, query.slug),
                    BrowseKind::Search => format!("{}/search/{}", $base, query.slug),
                    BrowseKind::Channel => format!("{}/channels/{}", $base, query.slug),
                    BrowseKind::Video => {
                        if query.slug.starts_with("http") {
                            query.slug.clone()
                        } else {
                            format!("{}/videos/{}", $base, query.slug)
                        }
                    }
                    BrowseKind::Model => format!("{}/pornstar/{}", $base, query.slug),
                };
                let html = ctx.fetch_html(&url, $id).await?;
                let items = parse_video_links(&html, $base, $id)?;
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

ytdlp_tube_adapter!(PornhubAdapter, "pornhub", "PornHub", "https://www.pornhub.com");
ytdlp_tube_adapter!(XHamsterAdapter, "xhamster", "xHamster", "https://xhamster.com");
ytdlp_tube_adapter!(XVideosAdapter, "xvideos", "XVIDEOS", "https://www.xvideos.com");

fn parse_video_links(html: &str, base: &str, site_id: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let sel = Selector::parse("a[href*='video'], a[href*='watch']").unwrap();
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for el in document.select(&sel) {
        let Some(href) = el.value().attr("href") else { continue };
        let url = if href.starts_with("http") {
            href.to_string()
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
        if title.is_empty() {
            continue;
        }
        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title,
            url,
            thumbnail: None,
            duration: None,
            site_id: site_id.to_string(),
            performers: vec![],
            tags: vec![],
        });
        if items.len() >= 40 {
            break;
        }
    }
    Ok(items)
}
