use crate::error::AppResult;
use crate::models::MediaItem;
use crate::sites::yt_dlp::SidecarRunner;
use crate::sites::SiteContext;

/// yt-dlp `--flat-playlist` fallback when HTML scraping returns no items.
pub async fn ytdlp_browse_fallback(
    ctx: &SiteContext,
    site_id: &str,
    url: &str,
    page: u32,
    page_size: u32,
) -> AppResult<Vec<MediaItem>> {
    let runner = SidecarRunner::new(ctx.app().clone());
    let cookies = ctx.cookie_file_for_site(site_id);
    let entries = runner
        .list_flat_playlist(url, page, page_size, cookies.as_deref())
        .await?;
    Ok(entries
        .into_iter()
        .map(|(id, title, item_url, thumbnail)| MediaItem {
            id,
            title,
            url: item_url,
            thumbnail,
            duration: None,
            site_id: site_id.to_string(),
            performers: vec![],
            tags: vec![],
        })
        .collect())
}
