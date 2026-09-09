use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use crate::sites::SiteContext;
use regex::Regex;
use serde_json::Value;

pub async fn extract_info(ctx: &SiteContext, url: &str) -> AppResult<MediaItem> {
    let html = ctx.fetch_html(url, "youtube").await?;

    let json = extract_yt_initial_player_response(&html)
        .ok_or_else(|| AppError::Site("Could not extract video data from YouTube page".into()))?;

    let video_details = json
        .get("videoDetails")
        .ok_or_else(|| AppError::Site("Missing videoDetails in YouTube response".into()))?;

    let title = video_details
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let video_id = video_details
        .get("videoId")
        .and_then(|v| v.as_str())
        .unwrap_or(url)
        .to_string();

    let channel = video_details
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let duration_secs = video_details
        .get("lengthSeconds")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok());

    let thumbnail = extract_best_thumbnail(&json);

    let mut tags = Vec::new();
    if let Some(cats) = json
        .get("microformat")
        .and_then(|m| m.get("playerMicroformatRenderer"))
    {
        if let Some(tags_arr) = cats.get("tags").and_then(|t| t.as_array()) {
            tags = tags_arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    Ok(MediaItem {
        id: video_id,
        title,
        url: url.to_string(),
        thumbnail,
        duration: duration_secs,
        site_id: "youtube".to_string(),
        performers: channel
            .as_ref()
            .map(|c| vec![c.clone()])
            .unwrap_or_default(),
        tags,
        description: None,
        channel,
        is_live: None,
        viewers: None,
        age: None,
        gender: None,
        stream_url: None,
        embed_url: None,
    })
}

pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "youtube").await?;

    let json = extract_yt_initial_player_response(&html)
        .ok_or_else(|| AppError::Site("Could not extract video data from YouTube page".into()))?;

    let streaming_data = json
        .get("streamingData")
        .ok_or_else(|| AppError::Site("Missing streamingData in YouTube response".into()))?;

    let formats = streaming_data
        .get("formats")
        .or_else(|| streaming_data.get("adaptiveFormats"))
        .and_then(|v| v.as_array());

    if let Some(formats) = formats {
        for fmt in formats {
            if let Some(mime) = fmt.get("mimeType").and_then(|v| v.as_str()) {
                if mime.contains("video/mp4") {
                    if let Some(url) = fmt.get("url").and_then(|v| v.as_str()) {
                        return Ok(Some(url.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn extract_yt_initial_player_response(html: &str) -> Option<Value> {
    let re = Regex::new(r"ytInitialPlayerResponse\s*=\s*(\{.*?\});\s*(?:var\s+|</script>)").ok()?;
    let caps = re.captures(html)?;
    let json_str = caps.get(1)?.as_str();
    serde_json::from_str(json_str).ok()
}

fn extract_best_thumbnail(json: &Value) -> Option<String> {
    let video_details = json.get("videoDetails")?;
    let thumbnail = video_details.get("thumbnail")?;
    let thumbs = thumbnail.get("thumbnails")?.as_array()?;
    thumbs.last()?.get("url")?.as_str().map(|s| s.to_string())
}
