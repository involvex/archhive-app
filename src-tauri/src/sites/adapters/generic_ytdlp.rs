use crate::error::AppResult;
use crate::models::{DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;

pub struct GenericYtDlpAdapter {
    pub site_id: &'static str,
    pub name: &'static str,
    pub base: &'static str,
    pub kinds: Vec<crate::models::BrowseKind>,
}

impl GenericYtDlpAdapter {
    pub fn youtube() -> Self {
        Self {
            site_id: "youtube",
            name: "YouTube",
            base: "https://www.youtube.com",
            kinds: vec![
                crate::models::BrowseKind::Channel,
                crate::models::BrowseKind::Search,
                crate::models::BrowseKind::Video,
            ],
        }
    }

    pub fn tiktok() -> Self {
        Self {
            site_id: "tiktok",
            name: "TikTok",
            base: "https://www.tiktok.com",
            kinds: vec![crate::models::BrowseKind::Channel, crate::models::BrowseKind::Video],
        }
    }

    pub fn twitter() -> Self {
        Self {
            site_id: "twitter",
            name: "Twitter / X",
            base: "https://x.com",
            kinds: vec![crate::models::BrowseKind::Channel, crate::models::BrowseKind::Video],
        }
    }

    pub fn thisvid() -> Self {
        Self {
            site_id: "thisvid",
            name: "ThisVid",
            base: "https://thisvid.com",
            kinds: vec![
                crate::models::BrowseKind::Tag,
                crate::models::BrowseKind::Search,
                crate::models::BrowseKind::Video,
            ],
        }
    }
}

#[async_trait]
impl SiteAdapter for GenericYtDlpAdapter {
    fn id(&self) -> &str {
        self.site_id
    }

    fn display_name(&self) -> &str {
        self.name
    }

    fn base_url(&self) -> &str {
        self.base
    }

    fn supported_kinds(&self) -> Vec<crate::models::BrowseKind> {
        self.kinds.clone()
    }

    async fn browse(
        &self,
        _ctx: &SiteContext,
        query: crate::models::BrowseQuery,
    ) -> AppResult<crate::models::BrowsePage> {
        let url = match query.kind {
            crate::models::BrowseKind::Video | crate::models::BrowseKind::Search => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}", self.base, query.slug)
                }
            }
            crate::models::BrowseKind::Channel => format!("{}/@{}", self.base, query.slug),
            crate::models::BrowseKind::Tag => format!("{}/tags/{}", self.base, query.slug),
            crate::models::BrowseKind::Model => format!("{}/models/{}", self.base, query.slug),
        };

        Ok(crate::models::BrowsePage {
            items: vec![MediaItem {
                id: url.clone(),
                title: query.slug.clone(),
                url,
                thumbnail: None,
                duration: None,
                site_id: self.site_id.to_string(),
                performers: vec![],
                tags: vec![],
            }],
            page: query.page,
            has_more: false,
            total: Some(1),
        })
    }

    async fn resolve_download(&self, _ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.site_id.to_string(),
        })
    }
}
