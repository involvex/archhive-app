use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use crate::sites::SiteContext;
use regex::Regex;

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub async fn extract_info(ctx: &SiteContext, url: &str) -> AppResult<MediaItem> {
    let mut req = ctx.client.get(url);
    req = req.header("User-Agent", USER_AGENT);
    req = req.header(
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    );
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(AppError::Site(format!("HTTP {} for TikTok", resp.status())));
    }

    let html = resp.text().await?;

    let title = extract_meta(&html, "og:title")
        .or_else(|| extract_meta(&html, "twitter:title"))
        .unwrap_or_else(|| {
            extract_from_json(&html, "title").unwrap_or_else(|| "TikTok Video".to_string())
        });

    let thumbnail =
        extract_meta(&html, "og:image").or_else(|| extract_meta(&html, "twitter:image"));

    let author =
        extract_from_json(&html, "author").or_else(|| extract_from_json(&html, "nickname"));

    let video_id = extract_video_id(url).unwrap_or_else(|| url.to_string());

    Ok(MediaItem {
        id: video_id,
        title,
        url: url.to_string(),
        thumbnail,
        duration: None,
        site_id: "tiktok".to_string(),
        performers: author.as_ref().map(|a| vec![a.clone()]).unwrap_or_default(),
        tags: Vec::new(),
        description: extract_meta(&html, "og:description"),
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
    let mut req = ctx.client.get(url);
    req = req.header("User-Agent", USER_AGENT);
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }

    let html = resp.text().await?;

    if let Some(video_url) = extract_video_url(&html) {
        return Ok(Some(video_url));
    }

    if let Some(video_url) = extract_from_json(&html, "playUrl") {
        return Ok(Some(video_url));
    }

    Ok(None)
}

fn extract_meta(html: &str, prop: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"<meta[^>]+property=["']{}["'][^>]+content=["']([^"']+)["']"#,
        prop
    ))
    .ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_from_json(html: &str, field: &str) -> Option<String> {
    let re = Regex::new(r#"<script[^>]*>([^<]*JSON\.parse[^<]*)</script>"#).ok()?;
    for cap in re.captures_iter(html) {
        let script = cap.get(1)?.as_str();
        if script.contains(&format!("\"{}\"", field)) {
            let json_re = Regex::new(&format!(r#""{}"\s*:\s*"([^"]+)""#, field)).ok()?;
            if let Some(m) = json_re.captures(script) {
                return m.get(1).map(|m| m.as_str().to_string());
            }
        }
    }
    None
}

fn extract_video_url(html: &str) -> Option<String> {
    let re = Regex::new(r#""playAddr"\s*:\s*"(https?://[^"]+)"#).ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_video_id(url: &str) -> Option<String> {
    let re = Regex::new(r"/video/(\d+)").ok()?;
    re.captures(url)?.get(1).map(|m| m.as_str().to_string())
}
