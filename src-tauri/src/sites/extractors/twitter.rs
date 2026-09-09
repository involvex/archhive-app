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
        return Err(AppError::Site(format!(
            "HTTP {} for Twitter",
            resp.status()
        )));
    }

    let html = resp.text().await?;

    let title = extract_meta(&html, "og:title")
        .or_else(|| extract_meta(&html, "twitter:title"))
        .unwrap_or_else(|| "Twitter / X Post".to_string());

    let thumbnail =
        extract_meta(&html, "og:image").or_else(|| extract_meta(&html, "twitter:image"));

    let description = extract_meta(&html, "og:description")
        .or_else(|| extract_meta(&html, "twitter:description"));

    let author = extract_from_html(
        &html,
        r#"<meta[^>]+name=["']twitter:creator["'][^>]+content=["']([^"']+)["']"#,
    )
    .or_else(|| {
        extract_from_html(
            &html,
            r#"<meta[^>]+property=["']twitter:creator["'][^>]+content=["']([^"']+)["']"#,
        )
    });

    Ok(MediaItem {
        id: url.to_string(),
        title,
        url: url.to_string(),
        thumbnail,
        duration: None,
        site_id: "twitter".to_string(),
        performers: author.as_ref().map(|a| vec![a.clone()]).unwrap_or_default(),
        tags: Vec::new(),
        description,
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

    if let Some(video_url) = extract_from_json(&html) {
        return Ok(Some(video_url));
    }

    Ok(None)
}

fn extract_meta(html: &str, prop: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"<meta[^>]+(?:property|name)=["']{}["'][^>]+content=["']([^"']+)["']"#,
        prop
    ))
    .ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_from_html(html: &str, pattern: &str) -> Option<String> {
    let re = Regex::new(pattern).ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_video_url(html: &str) -> Option<String> {
    for cap in html.match_indices("video.twimg.com") {
        let start = cap.0.saturating_sub(100);
        let end = (cap.0 + 200).min(html.len());
        let snippet = &html[start..end];
        if let Some(url_start) = snippet.find("http") {
            let rest = &snippet[url_start..];
            if let Some(url_end) = rest.find(['"', '\'', '<', ' ']) {
                return Some(rest[..url_end].to_string());
            }
            return Some(rest.to_string());
        }
    }
    None
}

fn extract_from_json(html: &str) -> Option<String> {
    for cap in html.match_indices("video.twimg.com") {
        let start = cap.0.saturating_sub(100);
        let end = (cap.0 + 200).min(html.len());
        let snippet = &html[start..end];
        if let Some(url_start) = snippet.find("http") {
            let rest = &snippet[url_start..];
            if let Some(url_end) = rest.find(['"', '\'', '<', ' ']) {
                let url = rest[..url_end].to_string();
                if url.contains(".mp4") {
                    return Some(url);
                }
            }
        }
    }
    None
}
