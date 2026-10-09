use crate::error::{AppError, AppResult};
use crate::models::{
    BrowseKind, BrowseOrientation, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem,
};
use crate::sites::urls::query_slug;
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

const BASE: &str = "https://www.eporner.com";
const API_SEARCH: &str = "https://www.eporner.com/api/v2/video/search/";

pub struct EpornerAdapter;

#[async_trait]
impl SiteAdapter for EpornerAdapter {
    fn id(&self) -> &str {
        "eporner"
    }

    fn display_name(&self) -> &str {
        "Eporner"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Search,
            BrowseKind::Tag,
            BrowseKind::Category,
            BrowseKind::Video,
        ]
    }

    fn requires_cookies(&self) -> bool {
        false
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        // Video kind: return a single MediaItem pointing at the video page.
        // yt-dlp handles URL resolution in resolve_stream_url; this listing
        // entry is just a navigation card.
        if query.kind == BrowseKind::Video {
            let video_url = if query.slug.starts_with("http") {
                query.slug.clone()
            } else {
                format!("{BASE}/video-{}/", query.slug)
            };
            return Ok(BrowsePage {
                items: vec![MediaItem {
                    id: video_url.clone(),
                    title: query.slug.clone(),
                    url: video_url,
                    thumbnail: None,
                    duration: None,
                    site_id: self.id().to_string(),
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
                }],
                page: query.page,
                has_more: false,
                total: Some(1),
                ..Default::default()
            });
        }

        let api_url = build_api_url(&query);

        info!(
            "[eporner] browse: kind={:?}, slug={}, api_url={}",
            query.kind, query.slug, api_url
        );

        let body = ctx
            .fetch_with_headers(
                &api_url,
                self.id(),
                Some(&[("Accept", "application/json"), ("Referer", BASE)]),
            )
            .await?;

        let page = parse_api_json(&body, query.page, self.id())
            .ok_or_else(|| AppError::Site("No videos returned from Eporner API".to_string()))?;

        Ok(page)
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
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
            .find(|line| {
                !line.is_empty() && (line.starts_with("http://") || line.starts_with("https://"))
            })
            .ok_or_else(|| AppError::Other("No stream URL resolved from Eporner".to_string()))?
            .to_string();
        Ok(stream_url)
    }

    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.id().to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
            referer: Some(BASE.to_string()),
            initial_status: None,
        })
    }
}

/// Map BrowseOrientation to Eporner's `gay` parameter.
/// - Straight → gay=0 (no gay content)
/// - Gay → gay=2 (only gay content)
/// - Lesbian/Transgender → gay=1 (include gay content, since Eporner doesn't distinguish)
fn orient_to_eporner_gay(o: BrowseOrientation) -> &'static str {
    match o {
        BrowseOrientation::Straight => "0",
        BrowseOrientation::Gay => "2",
        BrowseOrientation::Lesbian | BrowseOrientation::Transgender => "1",
    }
}

/// Extract a category name from an Eporner category URL.
/// e.g. `https://www.eporner.com/cat/japanese/` → `japanese`
/// e.g. `https://www.eporner.com/cat/4k-porn/` → `4k-porn`
/// Returns `None` if the URL is not a `/cat/<name>/` path.
fn extract_category_from_url(url: &str) -> Option<String> {
    if !url.starts_with("http") {
        return None;
    }
    let u = url.parse::<url::Url>().ok()?;
    let segs: Vec<&str> = u.path_segments()?.collect();
    // Find "cat" in path segments, then take the next one.
    let idx = segs.iter().position(|s| *s == "cat")?;
    let cat = segs.get(idx + 1)?.trim_matches('/');
    if cat.is_empty() {
        None
    } else {
        Some(cat.to_string())
    }
}

fn build_api_url(query: &BrowseQuery) -> String {
    let q = if query.slug.starts_with("http") {
        // Try to extract a category name from /cat/<name>/ URLs first.
        if let Some(cat) = extract_category_from_url(&query.slug) {
            cat
        } else if matches!(query.kind, BrowseKind::Category | BrowseKind::Tag) {
            // Fallback: extract the last meaningful path segment from the URL
            // (e.g. /tag/japanese-uncensored/ → japanese-uncensored)
            let seg = query
                .slug
                .split('/')
                .rfind(|s| !s.is_empty())
                .filter(|s| *s != "cat")
                .map(|s| s.trim_matches('/').to_string())
                .filter(|s| !s.is_empty());
            seg.unwrap_or_default()
        } else {
            "all".to_string()
        }
    } else if query.slug.is_empty() {
        "all".to_string()
    } else {
        query.slug.clone()
    };

    // Fallback to "all" if we couldn't extract a query.
    let q = if q.is_empty() { "all" } else { q.as_str() };

    let mut url = format!(
        "{API_SEARCH}?query={}&per_page=24&page={}&thumbsize=big&order=latest",
        query_slug(q),
        query.page.max(1),
    );

    if let Some(orient) = &query.orientation {
        url.push_str(&format!("&gay={}", orient_to_eporner_gay(*orient)));
    }

    url
}

fn parse_api_json(body: &str, page: u32, site_id: &str) -> Option<BrowsePage> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let videos = v.get("videos")?.as_array()?;
    if videos.is_empty() {
        return None;
    }

    let mut items: Vec<MediaItem> = Vec::new();
    for vid in videos.iter().take(48) {
        let title = crate::sites::urls::html_unescape(
            vid.get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Untitled"),
        );

        let video_url = vid
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();

        let thumbnail = vid
            .get("default_thumb")
            .and_then(|t| t.get("src"))
            .and_then(|s| s.as_str())
            .or_else(|| {
                vid.get("thumbs")
                    .and_then(|t| t.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|t| t.get("src"))
                    .and_then(|s| s.as_str())
            })
            .map(|s| s.to_string());

        let duration = vid
            .get("length_sec")
            .and_then(|d| d.as_u64())
            .map(|s| s as u32);

        let keywords = vid
            .get("keywords")
            .and_then(|k| k.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = keywords
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let views = vid.get("views").and_then(|v| v.as_u64()).map(|n| n as u32);

        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title: title.clone(),
            url: video_url.clone(),
            thumbnail,
            duration,
            site_id: site_id.to_string(),
            performers: vec![],
            tags,
            description: None,
            channel: None,
            is_live: None,
            viewers: views,
            age: None,
            gender: None,
            stream_url: None,
            embed_url: vid
                .get("embed")
                .and_then(|e| e.as_str())
                .map(|s| s.to_string()),
        });
    }

    let total = v
        .get("total_count")
        .and_then(|t| t.as_u64())
        .map(|n| n as u32);
    let count = v.get("count").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
    let has_more = count >= 24 && items.len() >= 24;

    Some(BrowsePage {
        items,
        page,
        has_more,
        total,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_api_url_uses_query_param() {
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "teen".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=teen"));
        assert!(url.contains("per_page=24"));
        assert!(url.contains("page=1"));
        assert!(url.contains("order=latest"));
        assert!(!url.contains("gay="));
    }

    #[test]
    fn build_api_url_defaults_to_all_when_empty() {
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "".to_string(),
            page: 2,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=all"));
        assert!(url.contains("page=2"));
    }

    #[test]
    fn build_api_url_falls_back_to_all_for_non_category_url_slugs() {
        // Non-Tag/Category kinds with URL slugs should fall back to "all".
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "https://www.eporner.com/video-abc/".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=all"));
    }

    #[test]
    fn build_api_url_extracts_category_from_url() {
        let q = BrowseQuery {
            kind: BrowseKind::Category,
            slug: "https://www.eporner.com/cat/japanese/".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=japanese"));
        assert!(url.contains("page=1"));
    }

    #[test]
    fn build_api_url_extracts_category_with_hyphen() {
        let q = BrowseQuery {
            kind: BrowseKind::Category,
            slug: "https://www.eporner.com/cat/4k-porn/".to_string(),
            page: 3,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=4k-porn"));
        assert!(url.contains("page=3"));
    }

    #[test]
    fn build_api_url_extracts_tag_from_url() {
        let q = BrowseQuery {
            kind: BrowseKind::Tag,
            slug: "https://www.eporner.com/tag/japanese-uncensored/".to_string(),
            page: 1,
            orientation: None,
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=japanese-uncensored"));
    }

    #[test]
    fn extract_category_from_url_returns_category_name() {
        let url = "https://www.eporner.com/cat/japanese/";
        assert_eq!(extract_category_from_url(url), Some("japanese".to_string()));
        let url = "https://www.eporner.com/cat/4k-porn/";
        assert_eq!(extract_category_from_url(url), Some("4k-porn".to_string()));
        let url = "https://www.eporner.com/cat/anal/";
        assert_eq!(extract_category_from_url(url), Some("anal".to_string()));
        // Non-category URL returns None.
        let url = "https://www.eporner.com/tag/anal/";
        assert_eq!(extract_category_from_url(url), None);
        let url = "https://www.eporner.com/video-abc123/";
        assert_eq!(extract_category_from_url(url), None);
        // Plain slug (no http) returns None.
        assert_eq!(extract_category_from_url("japanese"), None);
        // Empty category name after /cat/ returns None.
        let url = "https://www.eporner.com/cat/";
        assert_eq!(extract_category_from_url(url), None);
    }

    #[test]
    fn build_api_url_applies_orientation() {
        let q = BrowseQuery {
            kind: BrowseKind::Search,
            slug: "japanese".to_string(),
            page: 1,
            orientation: Some(BrowseOrientation::Gay),
        };
        let url = build_api_url(&q);
        assert!(url.contains("query=japanese"));
        assert!(url.contains("gay=2"));
    }

    #[test]
    fn orient_to_eporner_gay_maps_correctly() {
        assert_eq!(orient_to_eporner_gay(BrowseOrientation::Straight), "0");
        assert_eq!(orient_to_eporner_gay(BrowseOrientation::Gay), "2");
        assert_eq!(orient_to_eporner_gay(BrowseOrientation::Lesbian), "1");
        assert_eq!(orient_to_eporner_gay(BrowseOrientation::Transgender), "1");
    }

    #[test]
    fn parse_api_json_extracts_video_fields() {
        let body = r#"{
            "count": 2,
            "total_count": 2,
            "total_pages": 1,
            "videos": [
                {
                    "id": "abc123",
                    "title": "Test Video",
                    "keywords": "teen, brunette, hd",
                    "views": 1000,
                    "rate": "4.50",
                    "url": "https://www.eporner.com/video-abc123/Test-Video/",
                    "added": "2024-01-01 00:00:00",
                    "length_sec": 300,
                    "length_min": "5:00",
                    "embed": "https://www.eporner.com/embed/abc123/",
                    "default_thumb": {"src": "https://example.com/thumb.jpg", "width": 640, "height": 360}
                },
                {
                    "id": "def456",
                    "title": "Second Video",
                    "keywords": "amateur, couple",
                    "views": 500,
                    "rate": "3.80",
                    "url": "https://www.eporner.com/video-def456/Second-Video/",
                    "added": "2024-01-02 00:00:00",
                    "length_sec": 600,
                    "length_min": "10:00",
                    "embed": "https://www.eporner.com/embed/def456/",
                    "default_thumb": {"src": "https://example.com/thumb2.jpg", "width": 640, "height": 360}
                }
            ]
        }"#;
        let page = parse_api_json(body, 1, "eporner").expect("page");
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].title, "Test Video");
        assert_eq!(
            page.items[0].url,
            "https://www.eporner.com/video-abc123/Test-Video/"
        );
        assert_eq!(
            page.items[0].thumbnail.as_deref(),
            Some("https://example.com/thumb.jpg")
        );
        assert_eq!(page.items[0].duration, Some(300));
        assert_eq!(page.items[0].tags, vec!["teen", "brunette", "hd"]);
        assert_eq!(page.items[0].viewers, Some(1000));
        assert_eq!(
            page.items[0].embed_url.as_deref(),
            Some("https://www.eporner.com/embed/abc123/")
        );
        assert_eq!(page.items[1].title, "Second Video");
        assert_eq!(page.items[1].tags, vec!["amateur", "couple"]);
        assert_eq!(page.total, Some(2));
        // 2 items < per_page(24), so no more pages.
        assert!(!page.has_more);
    }

    #[test]
    fn parse_api_json_decodes_html_entities_in_title() {
        let body = r#"{
            "count": 1,
            "total_count": 1,
            "videos": [
                {
                    "id": "abc123",
                    "title": "I&#x27;m a &#34;test&#34; video",
                    "keywords": "",
                    "views": 0,
                    "url": "https://www.eporner.com/video-abc123/",
                    "length_sec": 100,
                    "default_thumb": {"src": "https://example.com/thumb.jpg"}
                }
            ]
        }"#;
        let page = parse_api_json(body, 1, "eporner").expect("page");
        assert_eq!(page.items[0].title, "I'm a \"test\" video");
    }

    #[test]
    fn parse_api_json_returns_none_for_empty_videos() {
        let body = r#"{"count": 0, "total_pages": 0, "videos": []}"#;
        assert!(parse_api_json(body, 1, "eporner").is_none());
    }

    #[test]
    fn parse_api_json_returns_none_for_invalid_json() {
        assert!(parse_api_json("not json", 1, "eporner").is_none());
    }
}
