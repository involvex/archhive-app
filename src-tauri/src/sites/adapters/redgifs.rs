use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;

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
        vec![
            BrowseKind::Search,
            BrowseKind::Tag,
            BrowseKind::Model,
            BrowseKind::Video,
        ]
    }

    fn browse_options(&self) -> crate::models::BrowseOptions {
        use crate::models::FilterOption as O;
        crate::models::BrowseOptions {
            sorts: vec![
                O::new("trending", "Trending"),
                O::new("latest", "Newest"),
                O::new("top", "Top"),
            ],
            filter_kinds: vec![BrowseKind::Search, BrowseKind::Tag],
            ..Default::default()
        }
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        use crate::sites::extractors::redgifs;
        const PAGE_SIZE: u32 = 40;
        let slug = query.slug.trim();
        let (items, pages) = match query.kind {
            BrowseKind::Video | BrowseKind::Livestream => {
                let item = redgifs::extract_info(ctx, slug).await?;
                (vec![item], Some(1))
            }
            BrowseKind::Model | BrowseKind::Channel => {
                let user = slug
                    .trim_end_matches('/')
                    .rsplit("/users/")
                    .next()
                    .unwrap_or(slug);
                redgifs::user_gifs(ctx, user, query.page, PAGE_SIZE).await?
            }
            _ => {
                // Empty query → trending feed; tag slugs use hyphens.
                let text = slug.replace('-', " ");
                let order = query
                    .filters
                    .sort
                    .as_deref()
                    .filter(|s| self.browse_options().sorts.iter().any(|o| o.id == *s))
                    .unwrap_or("trending");
                redgifs::search(ctx, &text, order, query.page, PAGE_SIZE).await?
            }
        };
        if items.is_empty() {
            return Err(AppError::Site("No RedGifs results for this query.".into()));
        }
        let has_more = pages.map_or(items.len() as u32 >= PAGE_SIZE, |p| query.page < p);
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
            ..Default::default()
        })
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        if let Ok(Some(direct_url)) =
            crate::sites::extractors::redgifs::extract_download_url(ctx, url).await
        {
            return Ok(direct_url);
        }

        let runner = crate::sites::yt_dlp::SidecarRunner::new(ctx.app().clone());
        let cookies = ctx.cookie_file_for_site(self.id());
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
        if let Ok(Some(direct_url)) =
            crate::sites::extractors::redgifs::extract_download_url(ctx, &item.url).await
        {
            let enriched = crate::sites::extractors::redgifs::extract_info(ctx, &item.url).await;
            let (performers, channel, tags, thumbnail, duration) = enriched
                .as_ref()
                .ok()
                .map(|m| {
                    (
                        m.performers.clone(),
                        m.channel.clone(),
                        m.tags.clone(),
                        m.thumbnail.clone(),
                        m.duration,
                    )
                })
                .unwrap_or_default();
            return Ok(DownloadPlan {
                url: direct_url,
                output_template: "redgifs/%(title)s.%(ext)s".to_string(),
                tool: DownloadTool::DirectHttp,
                title: Some(item.title.clone()),
                performers,
                tags,
                adapter_id: "redgifs".to_string(),
                thumbnail_url: thumbnail.or(item.thumbnail.clone()),
                duration: duration.or(item.duration),
                channel,
                referer: None,
                initial_status: None,
            });
        }

        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "redgifs/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::GalleryDl,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "redgifs".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
            referer: None,
            initial_status: None,
        })
    }
}
