use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::urls::query_slug;
use crate::sites::yt_dlp::SidecarRunner;
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;

pub struct GenericYtDlpAdapter {
    pub site_id: &'static str,
    pub name: &'static str,
    pub base: &'static str,
    pub kinds: Vec<crate::models::BrowseKind>,
    pub cookies: bool,
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
            cookies: false,
        }
    }

    pub fn tiktok() -> Self {
        Self {
            site_id: "tiktok",
            name: "TikTok",
            base: "https://www.tiktok.com",
            kinds: vec![
                crate::models::BrowseKind::Channel,
                crate::models::BrowseKind::Search,
                crate::models::BrowseKind::Video,
            ],
            cookies: false,
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
            cookies: false,
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
            cookies: true,
        }
    }

    pub fn instagram() -> Self {
        Self {
            site_id: "instagram",
            name: "Instagram",
            base: "https://www.instagram.com",
            kinds: vec![
                crate::models::BrowseKind::Channel,
                crate::models::BrowseKind::Video,
            ],
            cookies: true,
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
        "instagram" => format!("{base}/{clean}/"),
        _ => format!("{base}/{clean}"),
    }
}

fn search_url(base: &str, site_id: &str, slug: &str) -> String {
    let trimmed = slug.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    let q = query_slug(trimmed);
    match site_id {
        "tiktok" => format!("{base}/search/video?q={q}"),
        "youtube" => format!("{base}/results?search_query={q}"),
        "thisvid" => format!("{base}/search/?q={q}"),
        _ => format!("{base}/search?q={q}"),
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

    fn requires_cookies(&self) -> bool {
        self.cookies
    }

    async fn browse(
        &self,
        ctx: &SiteContext,
        query: BrowseQuery,
    ) -> AppResult<crate::models::BrowsePage> {
        let url = match query.kind {
            BrowseKind::Search => search_url(self.base, self.site_id, &query.slug),
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}", self.base, query.slug.trim_start_matches('/'))
                }
            }
            BrowseKind::Channel | BrowseKind::Model => {
                profile_url(self.base, self.site_id, &query.slug)
            }
            BrowseKind::Tag | BrowseKind::Category => {
                format!("{}/tags/{}", self.base, query.slug)
            }
            BrowseKind::Livestream => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{}/{}", self.base, query.slug.trim_start_matches('/'))
                }
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
                .await
                .map_err(|e| {
                    crate::error::AppError::Site(format!(
                        "{} browse failed: {e}. Import cookies in Settings if the site blocks anonymous access.",
                        self.name
                    ))
                })?;
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
                    description: None,
                    channel: None,
                    is_live: None,
                    viewers: None,
                    age: None,
                    gender: None,
                    stream_url: None,
                    embed_url: None,
                })
                .collect::<Vec<_>>();
            if items.is_empty() {
                return Err(crate::error::AppError::Site(format!(
                    "No results from {}. Check the query or try again later.",
                    self.name
                )));
            }
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
                description: None,
                channel: None,
                is_live: None,
                viewers: None,
                age: None,
                gender: None,
                stream_url: None,
                embed_url: None,
            }],
            page: query.page,
            has_more: false,
            total: Some(1),
        })
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        let direct = match self.site_id {
            "youtube" => crate::sites::extractors::youtube::extract_download_url(ctx, url).await?,
            "tiktok" => crate::sites::extractors::tiktok::extract_download_url(ctx, url).await?,
            "twitter" => crate::sites::extractors::twitter::extract_download_url(ctx, url).await?,
            _ => None,
        };
        if let Some(stream_url) = direct {
            return Ok(stream_url);
        }

        let runner = SidecarRunner::new(ctx.app().clone());
        let cookies = ctx.cookie_file_for_site(self.site_id);
        let mut args = vec![
            url.to_string(),
            "--get-url".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
        ];
        if let Some(cookies) = cookies.as_ref() {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = runner.run_capture_for_stream_url("yt-dlp", &args).await?;
        let stream_url = raw
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .ok_or_else(|| AppError::Other("No stream URL resolved".to_string()))?
            .to_string();
        Ok(stream_url)
    }

    async fn resolve_download(
        &self,
        ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        let tool = match self.site_id {
            "youtube" => {
                match crate::sites::extractors::youtube::extract_download_url(ctx, &item.url).await
                {
                    Ok(Some(direct_url)) => {
                        let enriched =
                            crate::sites::extractors::youtube::extract_info(ctx, &item.url).await;
                        let (performers, tags, channel, duration, thumbnail) = enriched
                            .as_ref()
                            .ok()
                            .map(|m| {
                                (
                                    m.performers.clone(),
                                    m.tags.clone(),
                                    m.channel.clone(),
                                    m.duration,
                                    m.thumbnail.clone(),
                                )
                            })
                            .unwrap_or_default();
                        return Ok(DownloadPlan {
                            url: direct_url,
                            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
                            tool: DownloadTool::DirectHttp,
                            title: Some(item.title.clone()),
                            performers,
                            tags,
                            adapter_id: self.site_id.to_string(),
                            thumbnail_url: thumbnail.or(item.thumbnail.clone()),
                            duration: duration.or(item.duration),
                            channel,
                        });
                    }
                    _ => DownloadTool::YtDlp,
                }
            }
            "tiktok" => {
                match crate::sites::extractors::tiktok::extract_download_url(ctx, &item.url).await {
                    Ok(Some(direct_url)) => {
                        let enriched =
                            crate::sites::extractors::tiktok::extract_info(ctx, &item.url).await;
                        let (performers, tags, channel, thumbnail) = enriched
                            .as_ref()
                            .ok()
                            .map(|m| {
                                (
                                    m.performers.clone(),
                                    m.tags.clone(),
                                    m.channel.clone(),
                                    m.thumbnail.clone(),
                                )
                            })
                            .unwrap_or_default();
                        return Ok(DownloadPlan {
                            url: direct_url,
                            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
                            tool: DownloadTool::DirectHttp,
                            title: Some(item.title.clone()),
                            performers,
                            tags,
                            adapter_id: self.site_id.to_string(),
                            thumbnail_url: thumbnail.or(item.thumbnail.clone()),
                            duration: item.duration,
                            channel,
                        });
                    }
                    _ => DownloadTool::YtDlp,
                }
            }
            "twitter" => {
                match crate::sites::extractors::twitter::extract_download_url(ctx, &item.url).await
                {
                    Ok(Some(direct_url)) => {
                        let enriched =
                            crate::sites::extractors::twitter::extract_info(ctx, &item.url).await;
                        let (performers, channel) = enriched
                            .as_ref()
                            .ok()
                            .map(|m| (m.performers.clone(), m.channel.clone()))
                            .unwrap_or_default();
                        return Ok(DownloadPlan {
                            url: direct_url,
                            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
                            tool: DownloadTool::DirectHttp,
                            title: Some(item.title.clone()),
                            performers,
                            tags: Vec::new(),
                            adapter_id: self.site_id.to_string(),
                            thumbnail_url: enriched.as_ref().ok().and_then(|m| m.thumbnail.clone()),
                            duration: item.duration,
                            channel,
                        });
                    }
                    _ => DownloadTool::YtDlp,
                }
            }
            _ => DownloadTool::YtDlp,
        };

        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.site_id.to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
        })
    }
}
