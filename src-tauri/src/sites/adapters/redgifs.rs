use crate::error::{AppError, AppResult};
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
            BrowseKind::Livestream => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("https://www.redgifs.com/watch/{}", query.slug)
                }
            }
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
        })
    }
}

fn parse_redgifs(html: &str) -> AppResult<Vec<MediaItem>> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let sel = Selector::parse("a[href*='/watch/']").unwrap();
    let mut items = Vec::new();
    for el in document.select(&sel) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.redgifs.com{href}")
        };
        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title: url.split('/').next_back().unwrap_or("redgif").to_string(),
            url,
            thumbnail: None,
            duration: None,
            site_id: "redgifs".to_string(),
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
        });
        if items.len() >= 30 {
            break;
        }
    }
    Ok(items)
}
