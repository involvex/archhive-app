use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
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
