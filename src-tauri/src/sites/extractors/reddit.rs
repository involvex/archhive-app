use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use crate::sites::SiteContext;
use serde_json::Value;

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub async fn extract_info(ctx: &SiteContext, url: &str) -> AppResult<MediaItem> {
    let json_url = format!("{}.json?raw_json=1", url);
    let mut req = ctx.client.get(&json_url);
    req = req.header("User-Agent", USER_AGENT);
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(AppError::Site(format!("HTTP {} for Reddit", resp.status())));
    }

    let json: Value = resp.json().await?;
    let listing = json
        .get(0)
        .and_then(|v| v.get("data"))
        .and_then(|v| v.get("children"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("data"));

    let data = match listing {
        Some(d) => d,
        None => return Err(AppError::Site("Invalid Reddit JSON response".into())),
    };

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Reddit Post")
        .to_string();

    let author = data
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let post_url = data
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| url)
        .to_string();

    let selftext = data
        .get("selftext")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .filter(|s| s.starts_with("http"))
        .map(|s| s.to_string());

    let is_video = data
        .get("is_video")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let media_url = if is_video {
        data.get("media")
            .and_then(|m| m.get("reddit_video"))
            .and_then(|v| v.get("fallback_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    let final_url = media_url.unwrap_or(post_url);

    Ok(MediaItem {
        id: data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| url)
            .to_string(),
        title,
        url: final_url.clone(),
        thumbnail,
        duration: None,
        site_id: "reddit".to_string(),
        performers: author.as_ref().map(|a| vec![a.clone()]).unwrap_or_default(),
        tags: Vec::new(),
        description: selftext.map(|s| s.to_string()),
        channel: author,
        is_live: None,
        viewers: None,
        age: None,
        gender: None,
        stream_url: None,
        embed_url: None,
    })
}

pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let info = extract_info(ctx, url).await?;
    Ok(Some(info.url))
}
