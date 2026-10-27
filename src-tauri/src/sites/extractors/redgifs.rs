//! RedGifs v2 API client.
//!
//! redgifs.com is a JavaScript single-page app: its HTML contains no listing
//! markup, so everything goes through `api.redgifs.com`. The API requires a
//! bearer token; anonymous clients get one from `/v2/auth/temporary`. The
//! token is bound to the caller's IP + User-Agent, so it is fetched with the
//! same shared `reqwest::Client` that issues the API calls and cached until
//! the API rejects it.

use crate::error::{AppError, AppResult};
use crate::models::MediaItem;
use crate::sites::SiteContext;
use regex::Regex;
use serde_json::Value;
use std::sync::{LazyLock, Mutex};

const API: &str = "https://api.redgifs.com/v2";
const WATCH_BASE: &str = "https://www.redgifs.com/watch/";

static TOKEN: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

async fn fetch_token(ctx: &SiteContext) -> AppResult<String> {
    let resp = ctx
        .client
        .get(format!("{API}/auth/temporary"))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Site(format!(
            "RedGifs auth failed: HTTP {}",
            resp.status()
        )));
    }
    let json: Value = resp.json().await?;
    let token = json
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Site("RedGifs auth response had no token".into()))?
        .to_string();
    if let Ok(mut slot) = TOKEN.lock() {
        *slot = Some(token.clone());
    }
    Ok(token)
}

async fn token(ctx: &SiteContext) -> AppResult<String> {
    let cached = TOKEN.lock().ok().and_then(|slot| slot.clone());
    match cached {
        Some(t) => Ok(t),
        None => fetch_token(ctx).await,
    }
}

/// Authenticated GET against the API; refreshes the token once on 401/403
/// (expired, or issued for a different IP after a network change).
async fn api_get(ctx: &SiteContext, path_and_query: &str) -> AppResult<Value> {
    let url = format!("{API}{path_and_query}");
    let mut tok = token(ctx).await?;
    for attempt in 0..2 {
        let resp = ctx
            .client
            .get(&url)
            .bearer_auth(&tok)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp.json().await?);
        }
        if attempt == 0 && (status.as_u16() == 401 || status.as_u16() == 403) {
            tok = fetch_token(ctx).await?;
            continue;
        }
        return Err(AppError::Site(format!(
            "RedGifs API HTTP {status} for {url}"
        )));
    }
    Err(AppError::Site("RedGifs API unavailable".into()))
}

/// One page of RedGifs results as `(items, total_pages)`.
pub async fn search(
    ctx: &SiteContext,
    text: &str,
    order: &str,
    page: u32,
    count: u32,
) -> AppResult<(Vec<MediaItem>, Option<u32>)> {
    let path = format!(
        "/gifs/search?search_text={}&order={order}&count={count}&page={}",
        crate::sites::urls::query_slug(text),
        page.max(1)
    );
    page_from_json(&api_get(ctx, &path).await?)
}

/// One page of a creator's uploads.
pub async fn user_gifs(
    ctx: &SiteContext,
    user: &str,
    page: u32,
    count: u32,
) -> AppResult<(Vec<MediaItem>, Option<u32>)> {
    let user = crate::sites::urls::path_slug(user);
    let path = format!(
        "/users/{user}/search?order=new&count={count}&page={}",
        page.max(1)
    );
    page_from_json(&api_get(ctx, &path).await?)
}

fn page_from_json(json: &Value) -> AppResult<(Vec<MediaItem>, Option<u32>)> {
    let gifs = json
        .get("gifs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Site("RedGifs response had no gifs".into()))?;
    let pages = json.get("pages").and_then(|v| v.as_u64()).map(|p| p as u32);
    Ok((gifs.iter().filter_map(media_item_from_gif).collect(), pages))
}

/// Map an API gif object to a listing item (`url` is the watch page).
fn media_item_from_gif(gif: &Value) -> Option<MediaItem> {
    let id = gif.get("id")?.as_str()?.to_string();
    let username = gif
        .get("userName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tags: Vec<String> = gif
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.as_str()
                        .or_else(|| t.get("text").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    let urls = gif.get("urls");
    let url_field = |k: &str| {
        urls.and_then(|u| u.get(k))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    Some(MediaItem {
        title: gif_title(gif, &id, username.as_deref(), &tags),
        url: format!("{WATCH_BASE}{id}"),
        id: format!("redgifs_{id}"),
        thumbnail: url_field("thumbnail").or_else(|| url_field("poster")),
        duration: gif
            .get("duration")
            .and_then(|v| v.as_f64())
            .map(|d| d.round() as u32),
        site_id: "redgifs".to_string(),
        performers: username.iter().cloned().collect(),
        tags,
        description: gif
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        channel: username,
        is_live: None,
        viewers: None,
        age: None,
        gender: None,
        // Direct MP4 — playable in-app without another resolve round trip.
        stream_url: url_field("hd").or_else(|| url_field("sd")),
        embed_url: None,
    })
}

/// RedGifs items have no title field; prefer the description, then
/// "creator · tags", then the id.
fn gif_title(gif: &Value, id: &str, user: Option<&str>, tags: &[String]) -> String {
    if let Some(desc) = gif
        .get("description")
        .and_then(|v| v.as_str())
        .and_then(|d| d.lines().map(str::trim).find(|l| !l.is_empty()))
    {
        return desc.chars().take(120).collect();
    }
    let tag_part = tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
    match (user, tag_part.is_empty()) {
        (Some(u), false) => format!("{u} · {tag_part}"),
        (Some(u), true) => format!("{u} · {id}"),
        (None, false) => tag_part,
        (None, true) => id.to_string(),
    }
}

pub async fn extract_info(ctx: &SiteContext, url: &str) -> AppResult<MediaItem> {
    let id = extract_gif_id(url).ok_or_else(|| {
        AppError::Site("Invalid RedGifs URL. Expected https://www.redgifs.com/watch/ID".into())
    })?;
    let json = api_get(ctx, &format!("/gifs/{id}")).await?;
    let gif = json
        .get("gif")
        .ok_or_else(|| AppError::Site("Missing gif data in RedGifs response".into()))?;
    media_item_from_gif(gif)
        .ok_or_else(|| AppError::Site("Malformed gif data in RedGifs response".into()))
}

/// Direct MP4 for a watch URL (`None` when the API lists no media URL).
pub async fn extract_download_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    Ok(extract_info(ctx, url).await?.stream_url)
}

pub fn extract_gif_id(url: &str) -> Option<String> {
    let re = Regex::new(r"redgifs\.com/(?:watch|ifr|i)/([A-Za-z0-9]+)").ok()?;
    re.captures(url)?
        .get(1)
        .map(|m| m.as_str().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gif_id_from_watch_and_embed_urls() {
        assert_eq!(
            extract_gif_id("https://www.redgifs.com/watch/FlimsyBlueWhale#rel=x").as_deref(),
            Some("flimsybluewhale")
        );
        assert_eq!(
            extract_gif_id("https://www.redgifs.com/ifr/abc").as_deref(),
            Some("abc")
        );
        assert_eq!(extract_gif_id("https://example.com/watch/abc"), None);
    }

    #[test]
    fn maps_search_page() {
        let json: Value = serde_json::from_str(
            r#"{"page":1,"pages":7,"total":300,"gifs":[
              {"id":"calmwildcat","duration":12.4,"userName":"someone","tags":["Amateur","Couple","POV","Extra"],
               "urls":{"sd":"https://media.redgifs.com/Calm-mobile.mp4","hd":"https://media.redgifs.com/Calm.mp4",
                       "thumbnail":"https://media.redgifs.com/Calm-mobile.jpg","poster":"https://media.redgifs.com/Calm-poster.jpg"}},
              {"id":"desc","description":"\n  First line\nsecond","urls":{}},
              {"nope":true}
            ]}"#,
        )
        .unwrap();
        let (items, pages) = page_from_json(&json).unwrap();
        assert_eq!(pages, Some(7));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].url, "https://www.redgifs.com/watch/calmwildcat");
        assert_eq!(items[0].title, "someone · Amateur, Couple, POV");
        assert_eq!(items[0].duration, Some(12));
        assert_eq!(
            items[0].stream_url.as_deref(),
            Some("https://media.redgifs.com/Calm.mp4")
        );
        assert_eq!(
            items[0].thumbnail.as_deref(),
            Some("https://media.redgifs.com/Calm-mobile.jpg")
        );
        assert_eq!(items[1].title, "First line");
        assert_eq!(items[1].stream_url, None);
    }
}
