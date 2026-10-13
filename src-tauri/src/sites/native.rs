//! Native stream resolvers for sites without their own adapter, tried before
//! plain yt-dlp:
//!
//! - KVS tubes (Hoes, Wankoz, FapCat, …): `get_file` links only work inside
//!   the page's session — opened from the player they 403 ("stream expired")
//!   or redirect to a placeholder (`video_error.mp4`). Resolved in-session
//!   to the CDN URL, which plays without cookies.
//! - XXXBP: the player asks `api.xxxbp.tv/hls` for its sources; yt-dlp has
//!   no extractor for it.

use crate::error::{AppError, AppResult};
use crate::sites::SiteContext;

/// A stream URL for `page_url`, or `None` when no native resolver applies.
pub async fn resolve_stream(
    ctx: &SiteContext,
    page_url: &str,
    site_id: &str,
) -> Option<AppResult<String>> {
    let host = host_of(page_url);
    if host_is(&host, "xxxbp.tv") {
        return Some(xxxbp_best(page_url).await);
    }
    match crate::sites::extractors::thothub::resolve_kvs_stream(ctx, page_url, site_id, &host).await
    {
        Ok(url) => Some(Ok(url)),
        // A KVS page whose video is private / a placeholder: say so rather
        // than let yt-dlp hand the player the same placeholder.
        Err(e) if is_kvs_verdict(&e.to_string()) => Some(Err(e)),
        Err(e) => {
            tracing::debug!("[native] {page_url}: not resolved as KVS: {e}");
            None
        }
    }
}

/// A directly downloadable media URL for pages yt-dlp has no extractor for
/// (no session needed). `None` when no native resolver applies.
pub async fn resolve_download(page_url: &str) -> Option<AppResult<String>> {
    let host = host_of(page_url);
    if host_is(&host, "xxxbp.tv") {
        return Some(xxxbp_best(page_url).await);
    }
    None
}

/// `resolve_kvs_stream` errors that are about the video itself, not about
/// the page being something other than a reachable KVS page.
fn is_kvs_verdict(err: &str) -> bool {
    err.contains("private") || err.contains("placeholder clip")
}

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_ascii_lowercase())
        })
        .unwrap_or_default()
}

fn host_is(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// `https://xxxbp.tv/video/<id>/<slug>` (also `/embed/<id>`).
fn xxxbp_video_id(page_url: &str) -> Option<String> {
    let path = url::Url::parse(page_url).ok()?.path().to_string();
    let mut parts = path.split('/').filter(|p| !p.is_empty());
    match parts.next()? {
        "video" | "embed" => parts
            .next()
            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            .map(str::to_string),
        _ => None,
    }
}

/// Highest-quality progressive MP4 from `POST api.xxxbp.tv/hls`, else the
/// HLS master playlist.
async fn xxxbp_best(page_url: &str) -> AppResult<String> {
    let id = xxxbp_video_id(page_url)
        .ok_or_else(|| AppError::InvalidInput(format!("not an XXXBP video URL: {page_url}")))?;
    let client = reqwest::Client::builder()
        .user_agent(crate::sites::BROWSER_UA)
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    crate::sites::throttle::acquire("https://api.xxxbp.tv/hls").await?;
    let resp = client
        .post("https://api.xxxbp.tv/hls")
        .header("Origin", "https://xxxbp.tv")
        .header("Referer", page_url)
        .header("Accept", "application/json")
        .form(&[("id", id.as_str())])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Site(format!(
            "XXXBP returned HTTP {} for the video sources",
            resp.status()
        )));
    }
    let body: serde_json::Value = resp.json().await?;
    pick_xxxbp(&body).ok_or_else(|| {
        AppError::Site("XXXBP returned no playable sources — the video may be removed.".into())
    })
}

fn pick_xxxbp(body: &serde_json::Value) -> Option<String> {
    let height = |t: &serde_json::Value| {
        t.get("title")
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches('p').parse::<u32>().ok())
            .unwrap_or(0)
    };
    body.get("mp4")
        .and_then(|v| v.as_array())
        .and_then(|list| {
            list.iter()
                .filter(|e| e.get("src").and_then(|s| s.as_str()).is_some())
                .max_by_key(|e| height(e))
        })
        .and_then(|e| e.get("src")?.as_str().map(str::to_string))
        .or_else(|| {
            body.get("master")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xxxbp_ids_and_source_pick() {
        assert_eq!(
            xxxbp_video_id("https://xxxbp.tv/video/488547/ikeepthemcuming-blowjob").as_deref(),
            Some("488547")
        );
        assert_eq!(
            xxxbp_video_id("https://xxxbp.tv/embed/12").as_deref(),
            Some("12")
        );
        assert_eq!(xxxbp_video_id("https://xxxbp.tv/model/x"), None);
        let body = serde_json::json!({
            "master": "https://cdn.test/multi/master.m3u8",
            "mp4": [
                {"src": "https://cdn.test/a_480p.mp4", "title": "480p"},
                {"src": "https://cdn.test/a_1080p.mp4", "title": "1080p"},
                {"src": "https://cdn.test/a_720p.mp4", "title": "720p"}
            ]
        });
        assert_eq!(
            pick_xxxbp(&body).as_deref(),
            Some("https://cdn.test/a_1080p.mp4")
        );
        let hls_only = serde_json::json!({"master": "https://cdn.test/m.m3u8", "mp4": []});
        assert_eq!(
            pick_xxxbp(&hls_only).as_deref(),
            Some("https://cdn.test/m.m3u8")
        );
    }
}
