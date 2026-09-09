use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use crate::sites::SiteContext;
use regex::Regex;
use serde_json::Value;

const REDGIFS_API: &str = "https://api.redgifs.com/v2/gifs/";

pub async fn extract_info(ctx: &SiteContext, url: &str) -> AppResult<MediaItem> {
    let id = extract_gif_id(url).ok_or_else(|| {
        AppError::Site("Invalid RedGifs URL. Expected https://www.redgifs.com/watch/ID".into())
    })?;

    let resp = ctx
        .client
        .get(format!("{}{}", REDGIFS_API, id))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Site(format!(
            "HTTP {} for RedGifs",
            resp.status()
        )));
    }

    let json: Value = resp.json().await?;
    let gif = json
        .get("gif")
        .ok_or_else(|| AppError::Site("Missing gif data in RedGifs response".into()))?;

    let title = gif
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("RedGifs")
        .to_string();

    let username = gif
        .get("userName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let duration = gif
        .get("duration")
        .and_then(|v| v.as_f64())
        .map(|d| d as u32);

    let thumbnail = gif
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tags = gif
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let video_url = gif
        .get("urls")
        .and_then(|u| u.get("hd"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            gif.get("urls")
                .and_then(|u| u.get("gif"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    Ok(MediaItem {
        id: format!("redgifs_{}", id),
        title,
        url: video_url.unwrap_or_else(|| url.to_string()),
        thumbnail,
        duration,
        site_id: "redgifs".to_string(),
        performers: username
            .as_ref()
            .map(|u| vec![u.clone()])
            .unwrap_or_default(),
        tags,
        description: None,
        channel: username,
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

fn extract_gif_id(url: &str) -> Option<String> {
    let re = Regex::new(r"redgifs\.com/watch/([A-Za-z0-9]+)").ok()?;
    re.captures(url)?.get(1).map(|m| m.as_str().to_string())
}
