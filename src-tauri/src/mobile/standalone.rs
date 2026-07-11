use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use reqwest::Client;

pub async fn resolve(url: &str) -> AppResult<MediaItem> {
    let client = Client::builder()
        .user_agent("ArcHive/0.1")
        .build()?;

    if url.contains("youtube.com") || url.contains("youtu.be") {
        return Ok(MediaItem {
            id: url.to_string(),
            title: extract_title_from_url(url),
            url: url.to_string(),
            thumbnail: None,
            duration: None,
            site_id: "youtube".to_string(),
            performers: vec![],
            tags: vec![],
        });
    }

    if is_direct_media(url) {
        let head = client.head(url).send().await;
        if head.is_ok() {
            return Ok(MediaItem {
                id: url.to_string(),
                title: extract_title_from_url(url),
                url: url.to_string(),
                thumbnail: None,
                duration: None,
                site_id: "direct".to_string(),
                performers: vec![],
                tags: vec![],
            });
        }
    }

    Err(AppError::Site(
        "Standalone mode supports YouTube and direct media URLs only. Use LAN mode for full site support.".to_string(),
    ))
}

fn is_direct_media(url: &str) -> bool {
    let lower = url.to_lowercase();
    [".mp4", ".webm", ".mkv", ".m3u8"].iter().any(|ext| lower.contains(ext))
}

fn extract_title_from_url(url: &str) -> String {
    url.split('/')
        .next_back()
        .unwrap_or("media")
        .split('?')
        .next()
        .unwrap_or("media")
        .to_string()
}
