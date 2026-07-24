use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;

pub struct CustomUrlAdapter;

fn normalize_url(slug: &str) -> AppResult<String> {
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("URL is required.".into()));
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{trimmed}"))
}

#[async_trait]
impl SiteAdapter for CustomUrlAdapter {
    fn id(&self) -> &str {
        "custom"
    }

    fn display_name(&self) -> &str {
        "Custom URL"
    }

    fn base_url(&self) -> &str {
        ""
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Video]
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = normalize_url(&query.slug)?;
        match ytdlp_browse_fallback(ctx, self.id(), &url, query.page, 24).await {
            Ok(items) if items.len() > 1 => {
                let has_more = items.len() >= 24;
                Ok(BrowsePage {
                    items,
                    page: query.page,
                    has_more,
                    total: None,
                })
            }
            Ok(items) if !items.is_empty() => Ok(BrowsePage {
                items,
                page: query.page,
                has_more: false,
                total: Some(1),
            }),
            _ => Ok(BrowsePage {
                items: vec![MediaItem {
                    id: url.clone(),
                    title: url.clone(),
                    url,
                    thumbnail: None,
                    duration: None,
                    site_id: self.id().to_string(),
                    performers: vec![],
                    tags: vec![],
                description: None,
                channel: None,
                }],
                page: query.page,
                has_more: false,
                total: Some(1),
            }),
        }
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
