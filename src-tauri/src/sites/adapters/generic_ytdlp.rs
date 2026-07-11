use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::yt_dlp::SidecarRunner;
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
            kinds: vec![
                crate::models::BrowseKind::Channel,
                crate::models::BrowseKind::Video,
            ],
        }
    }

    pub fn twitter() -> Self {
        Self {
            site_id: "twitter",
            name: "Twitter / X",
            base: "https://x.com",
            kinds: vec![
                crate::models::BrowseKind::Channel,
                crate::models::BrowseKind::Video,
            ],
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

fn profile_url(base: &str, site_id: &str, slug: &str) -> String {
    let trimmed = slug.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    let clean = trimmed.trim_start_matches('@');
    match site_id {
        "tiktok" | "youtube" => format!("{base}/@{clean}"),
        _ => format!("{base}/{clean}"),
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
        ctx: &SiteContext,
        query: BrowseQuery,
    ) -> AppResult<crate::models::BrowsePage> {
        let url = match query.kind {
            crate::models::BrowseKind::Video | crate::models::BrowseKind::Search => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}", self.base, query.slug.trim_start_matches('/'))
                }
            }
            crate::models::BrowseKind::Channel | crate::models::BrowseKind::Model => {
                profile_url(self.base, self.site_id, &query.slug)
            }
            crate::models::BrowseKind::Tag | crate::models::BrowseKind::Category => {
                format!("{}/tags/{}", self.base, query.slug)
            }
        };

        if matches!(
            query.kind,
            BrowseKind::Channel | BrowseKind::Model | BrowseKind::Search
        ) {
            let runner = SidecarRunner::new(ctx.app().clone());
            let cookies = ctx.cookie_file_for_site(self.site_id);
            let entries = runner
                .list_flat_playlist(&url, query.page, 24, cookies.as_deref())
                .await?;
            let items = entries
                .into_iter()
                .map(|(id, title, item_url, thumbnail)| MediaItem {
                    id,
                    title,
                    url: item_url,
                    thumbnail,
                    duration: None,
                    site_id: self.site_id.to_string(),
                    performers: vec![],
                    tags: vec![],
                })
                .collect::<Vec<_>>();
            let has_more = items.len() >= 24;
            return Ok(BrowsePage {
                items,
                page: query.page,
                has_more,
                total: None,
            });
        }

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
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.site_id.to_string(),
        })
    }
}
