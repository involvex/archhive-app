use crate::error::AppResult;
use crate::models::{
    BrowseKind, BrowseOptions, BrowseOrientation, BrowsePage, BrowseQuery, DownloadPlan,
    DownloadTool, FilterOption, MediaItem, SiteCategory,
};
use crate::sites::browse_fallback::ytdlp_browse_fallback;
use crate::sites::urls::{path_slug, query_slug};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;

macro_rules! ytdlp_tube_adapter {
    ($struct_name:ident, $id:expr, $name:expr, $base:expr) => {
        pub struct $struct_name;

        #[async_trait]
        impl SiteAdapter for $struct_name {
            fn id(&self) -> &str {
                $id
            }

            fn display_name(&self) -> &str {
                $name
            }

            fn base_url(&self) -> &str {
                $base
            }

            fn supported_kinds(&self) -> Vec<BrowseKind> {
                vec![
                    BrowseKind::Tag,
                    BrowseKind::Search,
                    BrowseKind::Channel,
                    BrowseKind::Category,
                    BrowseKind::Video,
                    BrowseKind::Model,
                ]
            }

            fn requires_cookies(&self) -> bool {
                false
            }

            fn browse_options(&self) -> BrowseOptions {
                tube_browse_options($id)
            }

            async fn list_categories(&self, ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
                tube_list_categories(ctx, $id, $base).await
            }

            async fn list_category_group(
                &self,
                ctx: &SiteContext,
                group: &str,
            ) -> AppResult<Vec<SiteCategory>> {
                tube_list_category_group(ctx, $id, $base, group).await
            }

            async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
                let url = tube_browse_url($id, $base, &query);
                let html = ctx.fetch_html(&url, $id).await.unwrap_or_default();
                let mut items = parse_video_links(&html, $base, $id).unwrap_or_default();
                // Drop obvious nav junk so we fall through to yt-dlp.
                items.retain(|i| !is_junk_nav_title(&i.title));
                if $id == "xhamster" {
                    fill_from_xhamster_initials(&html, &mut items);
                }
                if items.is_empty() {
                    items = match ytdlp_browse_fallback(ctx, $id, &url, query.page, 24).await {
                        Ok(fallback) => fallback,
                        Err(_) => Vec::new(),
                    };
                }
                if items.is_empty() {
                    return Err(crate::error::AppError::Site(format!(
                        "No videos found on {}. Try another query, or import cookies if the site is blocking.",
                        $name
                    )));
                }
                let has_more = items.len() >= 20;
                Ok(BrowsePage {
                    items,
                    page: query.page,
                    has_more,
                    total: None,
                ..Default::default()
                })
            }

            async fn resolve_download(
                &self,
                ctx: &SiteContext,
                item: &MediaItem,
            ) -> AppResult<DownloadPlan> {
                let (performers, tags, channel) = ctx
                    .fetch_html(&item.url, $id)
                    .await
                    .map(|html| scrape_tube_video_page(&html, $id))
                    .unwrap_or_default();
                Ok(DownloadPlan {
                    url: item.url.clone(),
                    output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
                    tool: DownloadTool::YtDlp,
                    title: Some(item.title.clone()),
                    performers,
                    tags,
                    adapter_id: $id.to_string(),
                    thumbnail_url: item.thumbnail.clone(),
                    duration: item.duration,
                    channel,
                    referer: None,
                    initial_status: None,
                })
            }

            async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
                let runner = crate::sites::yt_dlp::SidecarRunner::new(ctx.app().clone());
                let cookies = ctx.cookie_file_for_site($id);
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
                    .find(|line| !line.is_empty())
                    .ok_or_else(|| crate::error::AppError::Other("No stream URL resolved".to_string()))?
                    .to_string();
                // YouPorn's CDN (Aylo, like PornHub) never starts in the
                // phone's WebView although downloads work: play it through
                // the loopback proxy, which fetches with the site Referer and
                // a browser User-Agent like the downloader does.
                if $id == "youporn" {
                    return Ok(maybe_loopback_proxy(ctx, &stream_url, "https://www.youporn.com/"));
                }
                Ok(stream_url)
            }
        }
    };
}

/// No practical cap on tag indexes: XVIDEOS / XNXX list ~2,000 tags and a
/// popularity cut-off hid most rare ones (e.g. late-alphabet tags).
pub(crate) const ALL_CATEGORIES: usize = 5000;

/// Group id for XVIDEOS' ~38 categories (/c/<Name>-<id>), listed apart from
/// its tags; categories cannot be combined with tags in one URL.
const XV_CATEGORIES_GROUP: &str = "categories";

/// XVIDEOS splits its full tag index by first character: /tags/09, /tags/a …
/// /tags/z (each the top 2,000 tags for that letter, ~50k in total); the
/// plain /tags page only has the overall top ~2,000.
fn xvideos_tag_groups() -> Vec<FilterOption> {
    [
        FilterOption::new(XV_CATEGORIES_GROUP, "Categories"),
        FilterOption::new("09", "0-9"),
    ]
    .into_iter()
    .chain(
        ('a'..='z').map(|c| FilterOption::new(&c.to_string(), &c.to_ascii_uppercase().to_string())),
    )
    .collect()
}

async fn tube_list_category_group(
    ctx: &SiteContext,
    site_id: &str,
    base: &str,
    group: &str,
) -> AppResult<Vec<SiteCategory>> {
    if site_id == "xnxx" {
        // Same per-letter index as XVIDEOS (/tags/09, /tags/a …), linking to
        // /search/<tag> — XNXX has no separate tag pages.
        if !xnxx_tag_groups().iter().any(|g| g.id == group) {
            return Ok(Vec::new());
        }
        let html = ctx
            .fetch_html(&format!("{base}/tags/{group}"), site_id)
            .await?;
        return Ok(parse_xnxx_tag_letter(&html));
    }
    if site_id == "xhamster" {
        if !xhamster_tag_groups().iter().any(|g| g.id == group) {
            return Ok(Vec::new());
        }
        let html = ctx
            .fetch_html(&format!("{base}/categories-{group}"), site_id)
            .await?;
        return Ok(parse_xhamster_tag_letter(&html));
    }
    if site_id != "xvideos" || !xvideos_tag_groups().iter().any(|g| g.id == group) {
        return Ok(Vec::new());
    }
    if group == XV_CATEGORIES_GROUP {
        let html = ctx.fetch_html(&format!("{base}/"), site_id).await?;
        return Ok(parse_xvideos_categories(&html));
    }
    let html = ctx
        .fetch_html(&format!("{base}/tags/{group}"), site_id)
        .await?;
    Ok(parse_xvideos_tag_letter(&html))
}

fn xnxx_tag_groups() -> Vec<FilterOption> {
    std::iter::once(FilterOption::new("09", "0-9"))
        .chain(
            ('a'..='z')
                .map(|c| FilterOption::new(&c.to_string(), &c.to_ascii_uppercase().to_string())),
        )
        .collect()
}

/// `<li><a href="/search/z-asian">z asian</a><strong>30,913</strong></li>`.
fn parse_xnxx_tag_letter(html: &str) -> Vec<SiteCategory> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"<li><a href="/search/([^"/?#]+)">([^<]*)</a>\s*<strong>([\d,]+)</strong>"#,
        )
        .expect("xnxx tag letter regex")
    });
    let mut seen = std::collections::HashSet::new();
    RE.captures_iter(html)
        .filter_map(|c| {
            let slug = c.get(1)?.as_str().to_string();
            if !seen.insert(slug.clone()) {
                return None;
            }
            let name = html_escape_decode(c.get(2)?.as_str().trim());
            Some(SiteCategory {
                name: if name.is_empty() { slug.clone() } else { name },
                slug,
                kind: BrowseKind::Tag,
                thumbnail: None,
                count: c[3]
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .ok(),
            })
        })
        .collect()
}

/// xHamster's A–Z tag index: /categories-0 ("#") and /categories-a … -z,
/// up to 2,000 tags each (no further paging).
/// xHamster renders only the first handful of result cards with real
/// `<img>`/duration markup; the rest are filled in client-side from the
/// `window.initials` JSON. Fill missing thumbnails / durations (and titles)
/// from every video object in that JSON, matched by page URL.
fn fill_from_xhamster_initials(html: &str, items: &mut [MediaItem]) {
    if items
        .iter()
        .all(|i| i.thumbnail.is_some() && i.duration.is_some())
    {
        return;
    }
    let Some(start) = html.find("window.initials") else {
        return;
    };
    let rest = &html[start..];
    let Some(json_start) = rest.find('{') else {
        return;
    };
    let json = &rest[json_start..];
    let end = json.find("</script>").unwrap_or(json.len());
    let json = json[..end].trim_end().trim_end_matches(';');
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return;
    };

    #[derive(Default)]
    struct Info {
        thumb: Option<String>,
        duration: Option<u32>,
        title: Option<String>,
    }
    fn key(url: &str) -> String {
        url.trim_end_matches('/').to_ascii_lowercase()
    }
    fn walk(v: &serde_json::Value, out: &mut std::collections::HashMap<String, Info>) {
        match v {
            serde_json::Value::Object(map) => {
                if let Some(page) = map.get("pageURL").and_then(|p| p.as_str()) {
                    if page.contains("/videos/") {
                        let thumb = ["thumbURL", "imageURL"]
                            .iter()
                            .find_map(|k| map.get(*k).and_then(|t| t.as_str()))
                            .map(str::to_string);
                        let entry = out.entry(key(page)).or_default();
                        entry.thumb = entry.thumb.take().or(thumb);
                        entry.duration = entry.duration.or_else(|| {
                            map.get("duration")
                                .and_then(|d| d.as_u64())
                                .map(|d| d as u32)
                        });
                        entry.title = entry.title.take().or_else(|| {
                            map.get("title")
                                .and_then(|t| t.as_str())
                                .map(str::to_string)
                        });
                    }
                }
                for child in map.values() {
                    walk(child, out);
                }
            }
            serde_json::Value::Array(list) => {
                for child in list {
                    walk(child, out);
                }
            }
            _ => {}
        }
    }
    let mut info = std::collections::HashMap::new();
    walk(&root, &mut info);
    for item in items.iter_mut() {
        let Some(found) = info.get(&key(&item.url)) else {
            continue;
        };
        if item.thumbnail.is_none() {
            item.thumbnail = found.thumb.clone();
        }
        if item.duration.is_none() {
            item.duration = found.duration;
        }
        if item.title.trim().is_empty() || item.title == "Untitled" {
            if let Some(t) = &found.title {
                item.title = t.clone();
            }
        }
    }
}

fn xhamster_tag_groups() -> Vec<FilterOption> {
    std::iter::once(FilterOption::new("0", "0-9"))
        .chain(
            ('a'..='z')
                .map(|c| FilterOption::new(&c.to_string(), &c.to_ascii_uppercase().to_string())),
        )
        .collect()
}

/// `<a … href="https://xhamster.com/tags/<slug>" data-role="tag-link">…
/// <span …><!--…-->Name<!--…--></span>` (Svelte comment markers inside).
fn parse_xhamster_tag_letter(html: &str) -> Vec<SiteCategory> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"href="https://xhamster\.com/tags/([a-z0-9-]+)" data-role="tag-link">(.*?)</span>"#,
        )
        .expect("xhamster tag letter regex")
    });
    static MARKUP: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<!--.*?-->|<[^>]+>").expect("markup"));
    let mut seen = std::collections::HashSet::new();
    RE.captures_iter(html)
        .filter_map(|c| {
            let slug = c.get(1)?.as_str().to_string();
            if !seen.insert(slug.clone()) {
                return None;
            }
            let name = html_escape_decode(MARKUP.replace_all(c.get(2)?.as_str(), "").trim());
            Some(SiteCategory {
                name: if name.is_empty() {
                    slug.replace('-', " ")
                } else {
                    name
                },
                slug,
                kind: BrowseKind::Tag,
                thumbnail: None,
                count: None,
            })
        })
        .collect()
}

/// Entries on an XVIDEOS letter page:
/// `<li><a href="/tags/<slug>"><b><name> </b><span class="navbadge…"><n,nnn></span>`.
/// The A–Z navigation links have no count badge, so they are not matched.
fn parse_xvideos_tag_letter(html: &str) -> Vec<SiteCategory> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"<li><a href="/tags/([^"/?#]+)"><b>([^<]*)</b><span class="navbadge[^"]*">([\d,.]+)</span>"#,
        )
        .expect("xvideos tag letter regex")
    });
    let mut seen = std::collections::HashSet::new();
    RE.captures_iter(html)
        .filter_map(|c| {
            let slug = c.get(1)?.as_str().to_string();
            if !seen.insert(slug.clone()) {
                return None;
            }
            let name = html_escape_decode(c.get(2)?.as_str().trim());
            Some(SiteCategory {
                name: if name.is_empty() { slug.clone() } else { name },
                slug,
                kind: BrowseKind::Tag,
                thumbnail: None,
                count: c
                    .get(3)?
                    .as_str()
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .ok(),
            })
        })
        .collect()
}

/// `<a href="/c/Asian_Woman-32">Asian</a>` → slug `Asian_Woman-32`, name `Asian`.
fn parse_xvideos_categories(html: &str) -> Vec<SiteCategory> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"<a href="/c/([A-Za-z0-9_]+-\d+)">([^<]+)</a>"#)
            .expect("xvideos category regex")
    });
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<SiteCategory> = RE
        .captures_iter(html)
        .filter_map(|c| {
            let slug = c.get(1)?.as_str().to_string();
            seen.insert(slug.clone()).then(|| SiteCategory {
                name: html_escape_decode(c.get(2).map_or("", |m| m.as_str()).trim()),
                slug,
                kind: BrowseKind::Category,
                thumbnail: None,
                count: None,
            })
        })
        .collect();
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// Category / tag index for the macro-generated tube adapters.
async fn tube_list_categories(
    ctx: &SiteContext,
    site_id: &str,
    base: &str,
) -> AppResult<Vec<SiteCategory>> {
    use crate::sites::listing::{parse_category_index, segment_after, CategorySpec};
    let (path, selector, prefix, kind, max) = match site_id {
        "xvideos" => (
            "/tags",
            "a[href^='/tags/']",
            "/tags/",
            BrowseKind::Tag,
            ALL_CATEGORIES,
        ),
        "xnxx" => (
            "/tags",
            "a[href^='/search/']",
            "/search/",
            BrowseKind::Tag,
            ALL_CATEGORIES,
        ),
        "xhamster" => (
            "/categories",
            "a[href*='/categories/']",
            "/categories/",
            BrowseKind::Category,
            ALL_CATEGORIES,
        ),
        "youporn" => (
            "/categories/",
            "a[href*='/category/']",
            "/category/",
            BrowseKind::Category,
            ALL_CATEGORIES,
        ),
        _ => return Ok(Vec::new()),
    };
    let html = ctx.fetch_html(&format!("{base}{path}"), site_id).await?;
    let slug = |href: &str| segment_after(href, prefix);
    Ok(parse_category_index(
        &html,
        &CategorySpec {
            base,
            anchor_selector: selector,
            slug_from_href: &slug,
            kind,
            max,
        },
    ))
}

pub struct PornhubAdapter;

const PH_BASE: &str = "https://www.pornhub.com";

#[async_trait]
impl SiteAdapter for PornhubAdapter {
    fn id(&self) -> &str {
        "pornhub"
    }

    fn display_name(&self) -> &str {
        "PornHub"
    }

    fn base_url(&self) -> &str {
        PH_BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Category,
            BrowseKind::Search,
            BrowseKind::Model,
            BrowseKind::Channel,
            BrowseKind::Tag,
            BrowseKind::Video,
        ]
    }

    fn requires_cookies(&self) -> bool {
        true
    }

    fn browse_options(&self) -> BrowseOptions {
        tube_browse_options("pornhub")
    }

    async fn list_categories(&self, ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
        let url = categories_page_url(BrowseOrientation::Straight);
        let html = ctx.fetch_html(&url, self.id()).await?;
        Ok(parse_pornhub_categories(&html, BrowseOrientation::Straight)
            .into_iter()
            .map(|c| SiteCategory {
                slug: c
                    .category_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| c.slug.clone()),
                name: c.name,
                kind: BrowseKind::Category,
                thumbnail: None,
                count: c.video_count,
            })
            .collect())
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = match query.kind {
            BrowseKind::Category => {
                // PornHub retired /categories/<slug> (404); browse by numeric
                // `c=` id resolved from the live catalog instead.
                let mut q = query.clone();
                let orientation = q.orientation.unwrap_or(BrowseOrientation::Straight);
                // Trans categories browse by slug (/incategories/<slug>/transgender).
                let by_slug = q.slug.starts_with("language-")
                    || (orientation == BrowseOrientation::Transgender
                        && !is_trans_hub_slug(&q.slug));
                if !by_slug && !q.slug.contains(',') && !q.slug.chars().all(|c| c.is_ascii_digit())
                {
                    if let Some(id) = resolve_category_id(ctx, &q.slug, orientation).await {
                        q.slug = id.to_string();
                    }
                }
                pornhub_filtered(build_pornhub_category_url(&q), &q)
            }
            BrowseKind::Search | BrowseKind::Tag => {
                pornhub_filtered(build_pornhub_search_url(&query), &query)
            }
            BrowseKind::Model => format!("{PH_BASE}/pornstar/{}", path_slug(&query.slug)),
            BrowseKind::Channel => format!("{PH_BASE}/channels/{}", path_slug(&query.slug)),
            BrowseKind::Livestream => {
                return Err(crate::error::AppError::Site(
                    "PornHub does not expose a livestream browse kind.".to_string(),
                ));
            }
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!(
                        "{PH_BASE}/view_video.php?viewkey={}",
                        path_slug(&query.slug)
                    )
                }
            }
        };
        assert_pornhub_url(&url)?;
        let html = match ctx.fetch_html(&url, self.id()).await {
            // Not every category pair has a page; fall back to a word search.
            Err(e) if url.contains("/incategories/") && e.to_string().contains("HTTP 404") => {
                let fallback = incategories_search_fallback(&url).ok_or(e)?;
                let fallback = pornhub_filtered(fallback, &query);
                assert_pornhub_url(&fallback)?;
                ctx.fetch_html(&fallback, self.id()).await?
            }
            other => other?,
        };
        let mut items = parse_video_links(&html, PH_BASE, self.id())?;
        if items.is_empty() {
            items = match ytdlp_browse_fallback(ctx, self.id(), &url, query.page, 24).await {
                Ok(fallback) => fallback,
                Err(_) => Vec::new(),
            };
        }
        if items.is_empty() {
            return Err(crate::error::AppError::Site(
                "No videos found. Import PornHub cookies in Settings → Cookies.".into(),
            ));
        }
        let has_more = items.len() >= 20;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
            ..Default::default()
        })
    }

    async fn resolve_download(
        &self,
        ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        let html = ctx.fetch_html(&item.url, self.id()).await.ok();
        let (performers, tags, channel) = html
            .as_deref()
            .map(|h| scrape_tube_video_page(h, self.id()))
            .unwrap_or_default();
        let title = html
            .as_deref()
            .and_then(scrape_tube_video_title)
            .or_else(|| {
                let t = item.title.trim();
                if !t.is_empty() && !t.starts_with("http://") && !t.starts_with("https://") {
                    Some(t.to_string())
                } else {
                    None
                }
            })
            .or_else(|| viewkey_title_fallback(&item.url));
        // On mobile, yt-dlp is 403-blocked by PornHub's edge, so fetch the
        // direct MP4 via the Rust extractor and download it with a Referer
        // header (phncdn requires one). Falls through to yt-dlp on failure.
        #[cfg(mobile)]
        {
            if let Ok(Some(media_url)) =
                crate::sites::extractors::pornhub::extract_download_url(ctx, &item.url).await
            {
                return Ok(DownloadPlan {
                    url: media_url,
                    output_template: "%(title)s.%(ext)s".to_string(),
                    tool: DownloadTool::DirectHttp,
                    title: title.clone(),
                    performers: performers.clone(),
                    tags: tags.clone(),
                    adapter_id: self.id().to_string(),
                    thumbnail_url: item.thumbnail.clone(),
                    duration: item.duration,
                    channel: channel.clone(),
                    referer: Some(item.url.clone()),
                    initial_status: None,
                });
            }
            // HLS-only pages: download the playlist via ffmpeg stream-copy
            // with Referer/Cookie headers (plain HTTP would fetch playlists).
            if let Ok(Some(hls_url)) =
                crate::sites::extractors::pornhub::extract_hls_url(ctx, &item.url).await
            {
                return Ok(DownloadPlan {
                    url: hls_url,
                    output_template: "%(title)s.%(ext)s".to_string(),
                    tool: DownloadTool::FfmpegHls,
                    title,
                    performers,
                    tags,
                    adapter_id: self.id().to_string(),
                    thumbnail_url: item.thumbnail.clone(),
                    duration: item.duration,
                    channel,
                    referer: Some(item.url.clone()),
                    initial_status: None,
                });
            }
        }
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title,
            performers,
            tags,
            adapter_id: self.id().to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel,
            referer: None,
            initial_status: None,
        })
    }
    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        // yt-dlp first: it is what successfully downloads PornHub on-device.
        // The in-house page extractor is a mobile fallback. Either way the
        // CDN URL goes through the loopback proxy, which adds the Referer
        // that HTML5 <video> cannot send.
        let ytdlp = self.resolve_stream_url_ytdlp(ctx, url).await;
        #[cfg(mobile)]
        if ytdlp.is_err() {
            if let Ok(Some(stream_url)) =
                crate::sites::extractors::pornhub::extract_download_url(ctx, url).await
            {
                return Ok(maybe_loopback_proxy(
                    ctx,
                    &stream_url,
                    "https://www.pornhub.com/",
                ));
            }
            if let Ok(Some(hls_url)) =
                crate::sites::extractors::pornhub::extract_hls_url(ctx, url).await
            {
                return Ok(maybe_loopback_proxy(
                    ctx,
                    &hls_url,
                    "https://www.pornhub.com/",
                ));
            }
        }
        let stream_url = ytdlp?;
        Ok(maybe_loopback_proxy(
            ctx,
            &stream_url,
            "https://www.pornhub.com/",
        ))
    }
}

impl PornhubAdapter {
    async fn resolve_stream_url_ytdlp(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        let runner = crate::sites::yt_dlp::SidecarRunner::new(ctx.app().clone());
        let cookies = ctx.cookie_file_for_site(self.id());
        let mut args = vec![
            url.to_string(),
            "--get-url".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "-f".to_string(),
            "b".to_string(), // best single file (prefer progressive for in-app preview)
        ];
        if let Some(cookies) = cookies.as_ref() {
            args.push("--cookies".to_string());
            args.push(cookies.to_string_lossy().to_string());
        }
        let raw = runner.run_capture_for_stream_url("yt-dlp", &args).await?;
        raw.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_string)
            .ok_or_else(|| crate::error::AppError::Other("No stream URL resolved".to_string()))
    }
}

/// Rewrite CDN URLs through the local LAN proxy so the WebView can play them
/// with a proper Referer (HTML5 <video> cannot set Referer itself).
pub(crate) fn maybe_loopback_proxy(ctx: &SiteContext, stream_url: &str, referer: &str) -> String {
    #[cfg(mobile)]
    {
        if !(stream_url.starts_with("http://") || stream_url.starts_with("https://")) {
            return stream_url.to_string();
        }
        if stream_url.contains("127.0.0.1") || stream_url.contains("localhost") {
            return stream_url.to_string();
        }
        let port = ctx.lan_port();
        return format!(
            "http://127.0.0.1:{port}/api/media/proxy?url={}&referer={}",
            urlencoding_encode(stream_url),
            urlencoding_encode(referer)
        );
    }
    #[cfg(not(mobile))]
    {
        let _ = (ctx, referer);
        stream_url.to_string()
    }
}

/// [`maybe_loopback_proxy`] that also forwards browser cookies and the
/// browser's User-Agent (Cloudflare clearance is bound to the UA).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn loopback_proxy_with(
    ctx: &SiteContext,
    stream_url: &str,
    referer: &str,
    cookie: Option<&str>,
    user_agent: Option<&str>,
) -> String {
    let base = maybe_loopback_proxy(ctx, stream_url, referer);
    if base == stream_url {
        return base;
    }
    let mut out = base;
    if let Some(c) = cookie.filter(|c| !c.is_empty()) {
        out.push_str(&format!("&cookie={}", urlencoding_encode(c)));
    }
    if let Some(ua) = user_agent.filter(|u| !u.is_empty()) {
        out.push_str(&format!("&ua={}", urlencoding_encode(ua)));
    }
    out
}

fn urlencoding_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

ytdlp_tube_adapter!(
    XHamsterAdapter,
    "xhamster",
    "xHamster",
    "https://xhamster.com"
);
ytdlp_tube_adapter!(
    XVideosAdapter,
    "xvideos",
    "XVIDEOS",
    "https://www.xvideos.com"
);
ytdlp_tube_adapter!(
    YouPornAdapter,
    "youporn",
    "YouPorn",
    "https://www.youporn.com"
);
ytdlp_tube_adapter!(XnxxAdapter, "xnxx", "XNXX", "https://www.xnxx.com");

fn tube_browse_url(site_id: &str, base: &str, query: &BrowseQuery) -> String {
    let url = tube_base_url(site_id, base, query);
    if matches!(query.kind, BrowseKind::Video | BrowseKind::Livestream) {
        return url;
    }
    let f = &query.filters;
    let opt = |v: &Option<String>| {
        v.as_deref()
            .filter(|s| allowed_option(site_id, s))
            .unwrap_or("")
            .to_string()
    };
    match site_id {
        "xvideos" if matches!(query.kind, BrowseKind::Search | BrowseKind::Tag) => {
            crate::sites::urls::with_query(
                &url,
                &[
                    ("sort", &opt(&f.sort)),
                    ("datef", &opt(&f.period)),
                    ("durf", &opt(&f.duration)),
                    ("quality", &opt(&f.quality)),
                ],
            )
        }
        "xhamster" => {
            // Category and single-tag pages carry sort in the path.
            let category = query.kind == BrowseKind::Category
                || (query.kind == BrowseKind::Tag && !query.slug.contains(','));
            let sort = opt(&f.sort);
            // Categories: "/hd" path + ?quality=1080p|2160p (720p is plain /hd).
            let quality = match (category, opt(&f.quality).as_str()) {
                (true, "720p") => String::new(),
                (_, q) => q.to_string(),
            };
            let (min, max) = match opt(&f.duration).as_str() {
                "0-10" => ("", "10"),
                "10-30" => ("10", "30"),
                "30-plus" => ("30", ""),
                _ => ("", ""),
            };
            crate::sites::urls::with_query(
                &url,
                &[
                    // Categories carry sort in the path (see tube_base_url).
                    ("sort", if category { "" } else { &sort }),
                    ("min-duration", min),
                    ("max-duration", max),
                    ("quality", &quality),
                ],
            )
        }
        _ => url,
    }
}

/// True when `value` is one of the option ids `site_id` advertises (filter
/// values end up in URLs, so unknown ids are dropped instead of forwarded).
fn allowed_option(site_id: &str, value: &str) -> bool {
    let o = tube_browse_options(site_id);
    o.sorts
        .iter()
        .chain(&o.periods)
        .chain(&o.durations)
        .chain(&o.qualities)
        .any(|opt| opt.id == value)
}

/// Sort / filter options per tube site (ids are the sites' own URL tokens,
/// verified against their filter menus).
pub fn tube_browse_options(site_id: &str) -> BrowseOptions {
    let o = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(id, label)| FilterOption::new(id, label))
            .collect::<Vec<_>>()
    };
    let search = vec![BrowseKind::Search, BrowseKind::Tag];
    match site_id {
        "pornhub" => BrowseOptions {
            sorts: o(&[
                ("mr", "Newest"),
                ("mv", "Most viewed"),
                ("tr", "Top rated"),
                ("lg", "Longest"),
                ("ht", "Hottest"),
            ]),
            periods: o(&[
                ("t", "Today"),
                ("w", "This week"),
                ("m", "This month"),
                ("y", "This year"),
                ("a", "All time"),
            ]),
            durations: o(&[
                ("0-10", "Under 10 min"),
                ("10-20", "10–20 min"),
                ("20-30", "20–30 min"),
                ("30-plus", "30+ min"),
            ]),
            qualities: o(&[("hd", "HD only")]),
            filter_kinds: vec![BrowseKind::Search, BrowseKind::Tag, BrowseKind::Category],
            has_categories: true,
            combine_tags: 2,
            category_groups: Vec::new(),
            // Searches take one required category (`filter_category`) and up
            // to 10 excluded ones (`exclude_category=a-b`); category pages
            // only the excludes (`filter_category` is ignored there).
            include_exclude: true,
            category_tag_filter: false,
            max_include: 1,
            max_exclude: 10,
            exclude_only_kinds: vec![BrowseKind::Category],
            ..Default::default()
        },
        "xvideos" => BrowseOptions {
            sorts: o(&[
                ("relevance", "Relevance"),
                ("uploaddate", "Newest"),
                ("rating", "Top rated"),
                ("views", "Most viewed"),
                ("length", "Longest"),
                ("random", "Random"),
            ]),
            periods: o(&[
                ("today", "Today"),
                ("week", "This week"),
                ("month", "This month"),
                ("3month", "Last 3 months"),
                ("6month", "Last 6 months"),
            ]),
            durations: o(&[
                ("1-3min", "1–3 min"),
                ("3-10min", "3–10 min"),
                ("10min_more", "10+ min"),
                ("10-20min", "10–20 min"),
                ("20min_more", "20+ min"),
            ]),
            qualities: o(&[("hd", "HD"), ("1080P", "1080p+")]),
            filter_kinds: vec![BrowseKind::Search, BrowseKind::Tag, BrowseKind::Category],
            has_categories: true,
            // /tags/a+b is a keyword search over titles and tags, not a tag
            // intersection (verified on result pages), so combined tags run as
            // a word search.
            combine_tags: 0,
            category_groups: xvideos_tag_groups(),
            include_exclude: false,
            category_tag_filter: false,
            ..Default::default()
        },
        "xnxx" => BrowseOptions {
            sorts: o(&[("hits", "Most viewed"), ("random", "Random")]),
            periods: o(&[("month", "This month"), ("year", "This year")]),
            durations: o(&[
                ("0-10min", "Under 10 min"),
                ("10-20min", "10–20 min"),
                ("10min+", "10+ min"),
                ("20min+", "20+ min"),
            ]),
            qualities: o(&[("hd-only", "HD"), ("fullhd", "Full HD")]),
            filter_kinds: vec![BrowseKind::Search, BrowseKind::Tag, BrowseKind::Category],
            has_categories: true,
            combine_tags: 0,
            category_groups: xnxx_tag_groups(),
            include_exclude: false,
            category_tag_filter: false,
            ..Default::default()
        },
        "xhamster" => BrowseOptions {
            sorts: o(&[
                ("relevance", "Relevance"),
                ("newest", "Newest"),
                ("views", "Most viewed"),
                ("best", "Top rated"),
                ("longest", "Longest"),
            ]),
            periods: vec![],
            durations: o(&[
                ("0-10", "Under 10 min"),
                ("10-30", "10–30 min"),
                ("30-plus", "30+ min"),
            ]),
            qualities: o(&[("720p", "720p+"), ("1080p", "1080p+"), ("2160p", "4K")]),
            filter_kinds: vec![BrowseKind::Search, BrowseKind::Tag, BrowseKind::Category],
            has_categories: true,
            combine_tags: 0,
            category_groups: xhamster_tag_groups(),
            include_exclude: false,
            category_tag_filter: false,
            ..Default::default()
        },
        "youporn" => BrowseOptions {
            sorts: o(&[
                ("relevance", "Relevance"),
                ("date", "Newest"),
                ("views", "Most viewed"),
                ("rating", "Top rated"),
                ("duration", "Longest"),
            ]),
            filter_kinds: search,
            has_categories: true,
            ..Default::default()
        },
        _ => BrowseOptions::default(),
    }
}

fn tube_base_url(site_id: &str, base: &str, query: &BrowseQuery) -> String {
    let slug = path_slug(&query.slug);
    let q = query_slug(&query.slug);
    match site_id {
        "xvideos" => match query.kind {
            BrowseKind::Search | BrowseKind::Tag => {
                let page = if query.page > 1 {
                    format!("&p={}", query.page.saturating_sub(1))
                } else {
                    String::new()
                };
                let q = if query.slug.contains(',') {
                    query_slug(
                        &crate::models::split_tags(&query.slug)
                            .join(" ")
                            .replace('-', " "),
                    )
                } else {
                    q
                };
                format!("{base}/?k={q}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/models/{slug}"),
            // Categories cannot be intersected on XVIDEOS: search their names.
            BrowseKind::Category if query.slug.contains(',') => {
                let words: Vec<String> = crate::models::split_tags(&query.slug)
                    .iter()
                    .map(|c| {
                        let name = c.rsplit_once('-').map_or(c.as_str(), |(n, id)| {
                            if id.chars().all(|d| d.is_ascii_digit()) {
                                n
                            } else {
                                c.as_str()
                            }
                        });
                        name.replace(['_', '-'], " ")
                    })
                    .collect();
                format!("{base}/?k={}", query_slug(&words.join(" ")))
            }
            // /c/[s:<sort>/][d:<length>/][q:<quality>/]<Name>-<id>[/<page-1>]
            BrowseKind::Category => {
                let f = &query.filters;
                let pick = |v: &Option<String>, ok: &[&str]| {
                    v.as_deref().filter(|x| ok.contains(x)).map(str::to_string)
                };
                let mut path = format!("{base}/c/");
                if let Some(s) = pick(&f.sort, &["uploaddate", "rating", "views", "length"]) {
                    path.push_str(&format!("s:{s}/"));
                }
                if let Some(d) = pick(
                    &f.duration,
                    &["1-3min", "3-10min", "10-20min", "20min_more"],
                ) {
                    path.push_str(&format!("d:{d}/"));
                }
                if let Some(q) = pick(&f.quality, &["hd", "1080P"]) {
                    path.push_str(&format!("q:{q}/"));
                }
                // Keep the canonical case (Asian_Woman-32); path_slug lowercases
                // and the site only redirects bare category URLs back.
                let cat = query.slug.trim();
                let valid = !cat.is_empty()
                    && cat
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                path.push_str(if valid { cat } else { &slug });
                if query.page > 1 {
                    path.push_str(&format!("/{}", query.page - 1));
                }
                path
            }
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{base}/video.{slug}")
                }
            }
            BrowseKind::Livestream => format!("{base}/live/{slug}"),
        },
        "xhamster" => match query.kind {
            // Single tags have real listing pages (/tags/<slug>) that page and
            // sort like categories; combined tags fall back to search.
            BrowseKind::Search | BrowseKind::Tag
                if query.kind == BrowseKind::Search || query.slug.contains(',') =>
            {
                let page = if query.page > 1 {
                    format!("?page={}", query.page)
                } else {
                    String::new()
                };
                let words = if query.slug.contains(',') {
                    // xHamster's own multi-word links use '+' (/search/redhead+milf).
                    query_slug(
                        &crate::models::split_tags(&query.slug)
                            .join(" ")
                            .replace('-', " "),
                    )
                } else {
                    slug.clone()
                };
                format!("{base}/search/{words}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/pornstars/{slug}"),
            // (Search always takes the arm above; listed for exhaustiveness.)
            BrowseKind::Category | BrowseKind::Tag | BrowseKind::Search => {
                // Category and tag listings take sort / HD as path segments:
                // /categories|tags/<slug>/[newest|best]/[hd]/<page>
                let f = &query.filters;
                let section = if query.kind == BrowseKind::Tag {
                    "tags"
                } else {
                    "categories"
                };
                let mut path = format!("{base}/{section}/{slug}");
                if let Some(sort @ ("newest" | "best")) = f.sort.as_deref() {
                    path.push('/');
                    path.push_str(sort);
                }
                if f.quality
                    .as_deref()
                    .is_some_and(|q| allowed_option(site_id, q))
                {
                    path.push_str("/hd");
                }
                if query.page > 1 {
                    path.push_str(&format!("/{}", query.page));
                }
                path
            }
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{base}/videos/{slug}")
                }
            }
            BrowseKind::Livestream => format!("{base}/live/{slug}"),
        },
        "youporn" => match query.kind {
            BrowseKind::Search | BrowseKind::Tag => {
                // /search/<sort>/?query=… (views, rating, date, duration).
                let sort = query
                    .filters
                    .sort
                    .as_deref()
                    .filter(|s| allowed_option(site_id, s) && *s != "relevance")
                    .map(|s| format!("{s}/"))
                    .unwrap_or_default();
                format!("{base}/search/{sort}?query={q}{}", page_amp(query.page))
            }
            BrowseKind::Category => {
                if slug.is_empty() || slug == "categories" {
                    format!("{base}/categories/{}", page_query(query.page))
                } else {
                    format!("{base}/category/{slug}/{}", page_query(query.page))
                }
            }
            BrowseKind::Model | BrowseKind::Channel => {
                format!("{base}/pornstar/{slug}/{}", page_query(query.page))
            }
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    format!("{base}/watch/{slug}/")
                }
            }
            BrowseKind::Livestream => format!("{base}/live/{slug}"),
        },
        "xnxx" => match query.kind {
            BrowseKind::Search | BrowseKind::Tag | BrowseKind::Category => {
                // /search/[sort]/[period]/[duration]/[quality]/<query>/[page-1]
                let f = &query.filters;
                let mut path = String::from("/search");
                for part in [&f.sort, &f.period, &f.duration, &f.quality]
                    .into_iter()
                    .flatten()
                {
                    if allowed_option(site_id, part) {
                        path.push('/');
                        path.push_str(part);
                    }
                }
                let page = if query.page > 1 {
                    // XNXX pages are 0-based: page 2 is /1.
                    format!("/{}", query.page - 1)
                } else {
                    String::new()
                };
                format!("{base}{path}/{slug}{page}")
            }
            BrowseKind::Model | BrowseKind::Channel => format!("{base}/pornstars/{slug}"),
            BrowseKind::Livestream => format!("{base}/live/{slug}"),
            BrowseKind::Video => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else if slug.starts_with("video-") {
                    format!("{base}/{slug}")
                } else {
                    format!("{base}/video-{slug}")
                }
            }
        },
        _ => format!("{base}/?k={q}"),
    }
}

fn is_junk_nav_title(title: &str) -> bool {
    let t = title.trim().to_lowercase();
    matches!(
        t.as_str(),
        "red videos"
            | "liked videos"
            | "join"
            | "login"
            | "sign in"
            | "sign up"
            | "upload"
            | "home"
            | "premium"
            | "remove ads"
            | "my favourites"
            | "my favorites"
    ) || t.len() < 3
}

fn page_query(page: u32) -> String {
    if page > 1 {
        format!("?page={page}")
    } else {
        String::new()
    }
}

fn page_amp(page: u32) -> String {
    if page > 1 {
        format!("&page={page}")
    } else {
        String::new()
    }
}

fn orientation_path(orientation: BrowseOrientation) -> &'static str {
    match orientation {
        BrowseOrientation::Straight => "",
        BrowseOrientation::Gay => "gay",
        BrowseOrientation::Lesbian => "lesbian",
        BrowseOrientation::Transgender => "transgender",
    }
}

/// Append PornHub sort (`o`), period (`t`), duration and HD filters.
fn pornhub_filtered(url: String, query: &BrowseQuery) -> String {
    let f = &query.filters;
    let ok = |v: &Option<String>| {
        v.as_deref()
            .filter(|s| allowed_option("pornhub", s))
            .unwrap_or("")
            .to_string()
    };
    let (min, max) = match ok(&f.duration).as_str() {
        "0-10" => ("", "10"),
        "10-20" => ("10", "20"),
        "20-30" => ("20", "30"),
        "30-plus" => ("30", ""),
        _ => ("", ""),
    };
    let hd = if ok(&f.quality) == "hd" { "1" } else { "" };
    // Category filters are numeric `c=` ids (the category index's slugs).
    // One required category on searches only (category pages ignore it);
    // up to 10 exclusions, dash-separated as the site's own UI sends them.
    let ids = |v: &Option<String>, max: usize| {
        crate::models::split_tags(v.as_deref().unwrap_or(""))
            .into_iter()
            .filter(|id| !id.is_empty() && id.len() <= 6 && id.bytes().all(|b| b.is_ascii_digit()))
            .take(max)
            .collect::<Vec<_>>()
            .join("-")
    };
    let include = if query.kind == BrowseKind::Category {
        String::new()
    } else {
        ids(&f.include, 1)
    };
    let exclude = ids(&f.exclude, 10);
    crate::sites::urls::with_query(
        &url,
        &[
            ("o", &ok(&f.sort)),
            ("t", &ok(&f.period)),
            ("min_duration", min),
            ("max_duration", max),
            ("hd", hd),
            ("filter_category", &include),
            ("exclude_category", &exclude),
        ],
    )
}

fn build_pornhub_search_url(query: &BrowseQuery) -> String {
    let orientation = query.orientation.unwrap_or(BrowseOrientation::Straight);
    let prefix = orientation_path(orientation);
    let slug = query_slug(&query.slug);
    let page = page_amp(query.page);
    if prefix.is_empty() {
        format!("{PH_BASE}/video/search?search={slug}{page}")
    } else {
        format!("{PH_BASE}/{prefix}/video/search?search={slug}{page}")
    }
}

/// Defense-in-depth: every URL this adapter fetches must stay on
/// `https://www.pornhub.com`. All browse URLs are constructed internally from
/// `PH_BASE` + sanitized slugs, so a failure here means a sanitizer regressed
/// (or a future arm interpolates raw input) — fail closed instead of letting
/// `fetch_html` follow a crafted URL with the user's cookies attached.
fn assert_pornhub_url(url: &str) -> AppResult<()> {
    let parsed = url::Url::parse(url)
        .map_err(|e| crate::error::AppError::InvalidInput(format!("bad browse url: {e}")))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("www.pornhub.com") {
        return Err(crate::error::AppError::InvalidInput(format!(
            "browse url left pornhub.com: {url}"
        )));
    }
    Ok(())
}

fn build_pornhub_category_url(query: &BrowseQuery) -> String {
    let orientation = query.orientation.unwrap_or(BrowseOrientation::Straight);
    // Two combined categories: /video/incategories/a/b (straight only; the
    // orientation hubs 404 and three or more categories 404 as well).
    // "Language in videos" listings: /language/<name>.
    if let Some(lang) = query
        .slug
        .strip_prefix("language-")
        .filter(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_lowercase()))
    {
        return format!("{PH_BASE}/language/{lang}{}", page_query(query.page));
    }
    let combined = crate::models::split_tags(&query.slug);
    // Trans has no category index of its own: a trans category is the
    // straight category intersected with Transgender (/incategories/x/transgender).
    if orientation == BrowseOrientation::Transgender
        && combined.len() == 1
        && !is_trans_hub_slug(&combined[0])
        && !combined[0].chars().all(|c| c.is_ascii_digit())
    {
        return incategories_url("", &combined[0], "transgender", query.page);
    }
    if combined.len() == 2 {
        // Straight and gay intersect exactly two categories; gay catalog
        // slugs carry a `-gay` suffix the incategories path drops.
        match orientation {
            BrowseOrientation::Straight => {
                return incategories_url("", &combined[0], &combined[1], query.page);
            }
            BrowseOrientation::Gay => {
                let strip = |s: &str| s.strip_suffix("-gay").unwrap_or(s).to_string();
                return incategories_url(
                    "/gay",
                    &strip(&combined[0]),
                    &strip(&combined[1]),
                    query.page,
                );
            }
            _ => {}
        }
    }
    if combined.len() > 1 {
        let words = combined.join(" ").replace('-', " ");
        let root = match orientation {
            BrowseOrientation::Straight | BrowseOrientation::Transgender => PH_BASE.to_string(),
            o => format!("{PH_BASE}/{}", orientation_path(o)),
        };
        return format!(
            "{root}/video/search?search={}{}",
            query_slug(&words),
            page_amp(query.page)
        );
    }
    let slug = path_slug(&query.slug);
    // /transgender/video… paths 404 now; trans browsing uses the main index.
    let prefix = match orientation {
        BrowseOrientation::Transgender => "",
        o => orientation_path(o),
    };
    let root = if prefix.is_empty() {
        PH_BASE.to_string()
    } else {
        format!("{PH_BASE}/{prefix}")
    };

    let category_id = if query.slug.chars().all(|c| c.is_ascii_digit()) {
        Some(query.slug.clone())
    } else {
        pornhub_category_id_for_slug(&slug, orientation).map(|id| id.to_string())
    };
    if let Some(id) = category_id {
        return format!("{root}/video?c={id}{}", page_amp(query.page));
    }

    // Unknown slug: search for it. Route through query_slug (hyphens → spaces
    // first) so `&`/unicode in a slug can't break the query string.
    let search = query_slug(&slug.replace('-', " "));
    format!(
        "{root}/video/search?search={search}{}",
        page_amp(query.page)
    )
}

fn is_trans_hub_slug(slug: &str) -> bool {
    matches!(slug, "transgender" | "trans" | "shemale" | "83")
}

/// `/[gay/]video/incategories/a/b[?page=N]` — a strict two-category listing.
fn incategories_url(prefix: &str, a: &str, b: &str, page: u32) -> String {
    let page = (page > 1).then(|| page.to_string());
    crate::sites::urls::with_query(
        &format!(
            "{PH_BASE}{prefix}/video/incategories/{}/{}",
            path_slug(a),
            path_slug(b)
        ),
        &[("page", page.as_deref().unwrap_or(""))],
    )
}

/// Word-search URL used when PornHub has no page for a category pair
/// (e.g. `/incategories/bbw/transgender` is a 404).
fn incategories_search_fallback(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let segs: Vec<&str> = parsed.path_segments()?.collect();
    let i = segs.iter().position(|s| *s == "incategories")?;
    let words: Vec<String> = segs[i + 1..]
        .iter()
        .map(|s| s.strip_suffix("-gay").unwrap_or(s).replace('-', " "))
        .collect();
    let prefix = if segs.first() == Some(&"gay") {
        "/gay"
    } else {
        ""
    };
    let page = parsed
        .query_pairs()
        .find(|(k, _)| k == "page")
        .map(|(_, v)| format!("&page={v}"))
        .unwrap_or_default();
    Some(format!(
        "{PH_BASE}{prefix}/video/search?search={}{page}",
        query_slug(&words.join(" "))
    ))
}

/// Slug → PornHub `c=` id from the live category catalog (embedded JSON on
/// /categories, /gay/categories, /lesbian/categories), cached for 12 hours.
/// Gay slugs carry a `-gay` suffix there ("amateur" → "amateur-gay").
async fn resolve_category_id(
    ctx: &SiteContext,
    slug: &str,
    orientation: BrowseOrientation,
) -> Option<u32> {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};
    type Catalog = HashMap<String, u32>;
    static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Catalog)>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    const TTL: Duration = Duration::from_secs(12 * 3600);

    let slug = slug.trim().to_ascii_lowercase();
    let catalogs: &[BrowseOrientation] = match orientation {
        BrowseOrientation::Gay => &[BrowseOrientation::Gay],
        BrowseOrientation::Lesbian => &[BrowseOrientation::Lesbian, BrowseOrientation::Straight],
        _ => &[BrowseOrientation::Straight],
    };
    for &catalog in catalogs {
        let key = format!("{catalog:?}");
        let cached = CACHE
            .lock()
            .ok()
            .and_then(|c| c.get(&key).cloned())
            .filter(|(at, _)| at.elapsed() < TTL)
            .map(|(_, m)| m);
        let map = match cached {
            Some(m) => m,
            None => {
                let html = ctx
                    .fetch_html(&categories_page_url(catalog), "pornhub")
                    .await
                    .ok()?;
                let m: Catalog = parse_pornhub_categories(&html, catalog)
                    .into_iter()
                    .filter_map(|c| Some((c.slug.to_ascii_lowercase(), c.category_id?)))
                    .collect();
                if !m.is_empty() {
                    if let Ok(mut c) = CACHE.lock() {
                        c.insert(key, (Instant::now(), m.clone()));
                    }
                }
                m
            }
        };
        if let Some(id) = map
            .get(&slug)
            .or_else(|| map.get(&format!("{slug}-gay")))
            .copied()
        {
            return Some(id);
        }
    }
    None
}

/// Known PornHub `c=` ids for human-readable category slugs (mirrors frontend catalog).
fn pornhub_category_id_for_slug(slug: &str, _orientation: BrowseOrientation) -> Option<u32> {
    match slug.to_ascii_lowercase().as_str() {
        // Frontend catalog sets categoryId for Lesbian — slug paths are unreliable.
        "lesbian" => Some(27),
        _ => None,
    }
}

fn parse_video_links(html: &str, base: &str, site_id: &str) -> AppResult<Vec<MediaItem>> {
    let anchor_selector = match site_id {
        // Scope to the result grid when present: the page header carries
        // "hottest" / recommended / playlist dropdowns full of unrelated videos.
        "pornhub" if html.contains("id=\"videoSearchResult\"") => {
            "#videoSearchResult a[href*='view_video']"
        }
        "pornhub" if html.contains("id=\"videoCategory\"") => {
            "#videoCategory a[href*='view_video']"
        }
        // Pornstar / model / channel pages: their grids share this class.
        "pornhub" if html.contains("search-video-thumbs") => {
            "ul.search-video-thumbs a[href*='view_video']"
        }
        "pornhub" => "a[href*='view_video']",
        "xvideos" => "a[href*='/video.']",
        "xhamster" => "a[href*='/videos/']",
        "youporn" => "a[href*='/watch/']",
        "xnxx" => "a[href*='/video-']",
        _ => "a[href*='/video.'], a[href*='/videos/'], a[href*='/watch/']",
    };
    let is_video_href = |href: &str| is_site_video_href(site_id, href);
    Ok(crate::sites::listing::parse_listing(
        html,
        &crate::sites::listing::ListingSpec {
            site_id,
            base,
            anchor_selector,
            is_video_href: &is_video_href,
            is_junk_title: &is_junk_nav_title,
        },
    ))
}

fn is_site_video_href(site_id: &str, href: &str) -> bool {
    let lower = href.to_lowercase();
    match site_id {
        "pornhub" => is_pornhub_video_href(href),
        "xvideos" => {
            // Require /video.ID/ — excludes /videos-i-like, /account/..., etc.
            lower.contains("/video.")
                && !lower.contains("/videos-")
                && !lower.contains("/account")
                && !lower.contains("/profiles/")
        }
        "xhamster" => {
            // Single videos live at /videos/<slug>-<id>; hubs such as
            // /creators/videos/... or /videos/best are listings.
            lower
                .rsplit("xhamster.com")
                .next()
                .is_some_and(|path| path.starts_with("/videos/"))
                && !lower.contains("/videos/best")
                && !lower.contains("/videos/newest")
                && !lower.contains("/categories/")
                && !lower.contains("/users/")
                && !lower.contains("/my/")
        }
        "youporn" => {
            // /watch/12345/title — skip category/pornstar hubs
            if !lower.contains("/watch/") {
                return false;
            }
            let skip = [
                "/categories/",
                "/category/",
                "/pornstar/",
                "/channel/",
                "/channels/",
                "/search/",
                "/info/",
            ];
            !skip.iter().any(|s| lower.contains(s))
        }
        "xnxx" => {
            lower.contains("/video-")
                && !lower.contains("/search/")
                && !lower.contains("/pornstars/")
                && !lower.contains("/profiles/")
        }
        _ => lower.contains("/video.") || lower.contains("/videos/") || lower.contains("/watch/"),
    }
}

fn is_pornhub_video_href(href: &str) -> bool {
    let lower = href.to_lowercase();
    // `pkey=` marks a playlist "Play All" link, not a single video card.
    if !lower.contains("view_video") || lower.contains("pkey=") {
        return false;
    }
    let skip = [
        "/channel/",
        "/channels/",
        "/categories/",
        "/video/search",
        "/users/",
        "/model/",
        "/pornstar/",
    ];
    !skip.iter().any(|s| lower.contains(s))
}

pub fn categories_page_url(orientation: BrowseOrientation) -> String {
    match orientation {
        BrowseOrientation::Straight => format!("{PH_BASE}/categories"),
        BrowseOrientation::Gay => format!("{PH_BASE}/gay/categories"),
        BrowseOrientation::Lesbian => format!("{PH_BASE}/lesbian/categories"),
        BrowseOrientation::Transgender => format!("{PH_BASE}/transgender/categories"),
    }
}

/// Straight catalog re-tagged for the trans section: the Transgender
/// category itself becomes an "All trans" entry at the top.
pub fn trans_catalog(
    straight: Vec<crate::models::PornhubCategoryEntry>,
) -> Vec<crate::models::PornhubCategoryEntry> {
    let hub = straight.iter().find(|c| c.slug == "transgender").cloned();
    let mut out: Vec<_> = straight
        .into_iter()
        .filter(|c| c.slug != "transgender")
        .map(|mut c| {
            c.orientation = BrowseOrientation::Transgender;
            c
        })
        .collect();
    out.insert(
        0,
        crate::models::PornhubCategoryEntry {
            name: "All trans".into(),
            slug: "transgender".into(),
            orientation: BrowseOrientation::Transgender,
            category_id: Some(83),
            video_count: hub.and_then(|h| h.video_count),
        },
    );
    out
}

pub fn parse_pornhub_categories(
    html: &str,
    orientation: BrowseOrientation,
) -> Vec<crate::models::PornhubCategoryEntry> {
    use crate::models::PornhubCategoryEntry;
    use scraper::{Html, Selector};
    // Current markup embeds the full catalog as JSON objects
    // ({"id":722,"name":"Uncensored",…,"slug":"…",…,"video_count":10207,…});
    // the visible tiles only expose counts, which the HTML path used to
    // mistake for names.
    let from_json = pornhub_categories_from_json(html, orientation);
    if from_json.len() >= 10 {
        return from_json;
    }
    let document = Html::parse_document(html);
    let selectors = [
        "li.categoryWrapper a",
        ".categoriesList li a",
        ".categoryList li a",
        "a[href*='/categories/']",
        "a[href*='c=']",
    ];
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        for el in document.select(&sel) {
            let Some(href) = el.value().attr("href") else {
                continue;
            };
            let category_id = category_id_from_href(href);
            let slug =
                category_slug_from_href(href).or_else(|| category_id.map(|id| id.to_string()));
            let Some(slug) = slug else { continue };
            let key = format!("{orientation:?}:{slug}");
            if !seen.insert(key) {
                continue;
            }
            let name = category_name_from_element(&el);
            if name.is_empty() {
                continue;
            }
            let video_count = category_count_from_element(&el);
            items.push(PornhubCategoryEntry {
                name,
                slug,
                orientation,
                category_id,
                video_count,
            });
        }
        if items.len() >= 40 {
            break;
        }
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn pornhub_categories_from_json(
    html: &str,
    orientation: BrowseOrientation,
) -> Vec<crate::models::PornhubCategoryEntry> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"\{"id":(\d+),"name":("(?:[^"\\]|\\.)*")(?:,"english":"(?:[^"\\]|\\.)*")?,"slug":"([a-z0-9-]+)"[^{}]*?"video_count":(\d+)"#,
        )
        .expect("pornhub category json regex")
    });
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<_> = RE
        .captures_iter(html)
        .filter_map(|c| {
            let id: u32 = c.get(1)?.as_str().parse().ok()?;
            let name: String = serde_json::from_str(c.get(2)?.as_str()).ok()?;
            let name = name.trim().to_string();
            if name.is_empty() || !seen.insert(id) {
                return None;
            }
            Some(crate::models::PornhubCategoryEntry {
                name,
                slug: c.get(3)?.as_str().to_string(),
                orientation,
                category_id: Some(id),
                video_count: c.get(4)?.as_str().parse().ok(),
            })
        })
        .collect();
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

fn category_id_from_href(href: &str) -> Option<u32> {
    let lower = href.to_lowercase();
    for marker in ["?c=", "&c="] {
        if let Some(idx) = lower.find(marker) {
            let rest = &href[idx + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u32>() {
                return Some(id);
            }
        }
    }
    None
}

fn category_slug_from_href(href: &str) -> Option<String> {
    let lower = href.to_lowercase();
    let marker = "/categories/";
    let idx = lower.find(marker)?;
    let rest = &href[idx + marker.len()..];
    let slug = rest.split(&['/', '?', '#'][..]).next().unwrap_or("").trim();
    if slug.is_empty() || slug == "categories" || slug.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(slug.to_string())
}

fn category_name_from_element(el: &scraper::element_ref::ElementRef<'_>) -> String {
    use scraper::Selector;
    if let Ok(sel) = Selector::parse(".categoryTitle, .title, span") {
        if let Some(span) = el.select(&sel).next() {
            let text = span.text().collect::<String>().trim().to_string();
            if !text.is_empty() && !text.to_lowercase().contains("video") {
                return text;
            }
        }
    }
    el.text()
        .collect::<String>()
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn category_count_from_element(el: &scraper::element_ref::ElementRef<'_>) -> Option<u32> {
    use scraper::Selector;
    let text = if let Ok(sel) = Selector::parse(".videosNumber, .categoryCount, span") {
        el.select(&sel)
            .map(|s| s.text().collect::<String>())
            .collect::<String>()
    } else {
        el.text().collect::<String>()
    };
    parse_video_count(&text)
}

fn parse_video_count(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.replace(',', "").parse().ok()
}

fn scrape_tube_video_title(html: &str) -> Option<String> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    for sel in [
        r#"meta[property="og:title"]"#,
        r#"meta[name="twitter:title"]"#,
        "h1.title",
        "h1",
    ] {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        if let Some(el) = doc.select(&selector).next() {
            let text = if sel.starts_with("meta") {
                el.value().attr("content").unwrap_or("").trim().to_string()
            } else {
                el.text().collect::<String>().trim().to_string()
            };
            if !text.is_empty() && text.len() < 200 && !is_junk_nav_title(&text) {
                return Some(text);
            }
        }
    }
    None
}

fn viewkey_title_fallback(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let key = parsed
        .query_pairs()
        .find(|(k, _)| k == "viewkey")
        .map(|(_, v)| v.to_string())?;
    if key.is_empty() {
        return None;
    }
    Some(format!("pornhub-{key}"))
}

/// Scrape performers, tags, and channel from a tube site video page.
pub(crate) fn scrape_tube_video_page(
    html: &str,
    site_id: &str,
) -> (Vec<String>, Vec<String>, Option<String>) {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);

    let performer_selectors: &[&str] = match site_id {
        "pornhub" => &[
            ".video-info-row a[href*='/pornstar/']",
            ".pornstarsWrapper a[href*='/pornstar/']",
            "a[href*='/model/']",
        ],
        "xhamster" => &[
            ".pornstars-list a[href*='/pornstars/']",
            "a[href*='/pornstars/']",
        ],
        "xvideos" => &[".pornstars a[href*='/pornstar/']", "a[href*='/pornstar/']"],
        "xnxx" => &[".pornstar a[href*='/pornstar/']", "a[href*='/pornstar/']"],
        "youporn" => &[
            ".pornstar-list a[href*='/pornstar/']",
            "a[href*='/pornstar/']",
        ],
        _ => &[],
    };

    let tag_selectors: &[&str] = match site_id {
        "pornhub" => &[
            ".tagsWrapper a[href*='/video/search']",
            ".categoriesWrapper a[href*='?search=']",
            "a[href*='/video/search?search=']",
        ],
        "xhamster" => &[".tags a[href*='/tags/']", ".video-tags a[href*='/tags/']"],
        "xvideos" => &[".tags a.is-keyword", ".tags-label a[href*='/tags/']"],
        "xnxx" => &[".metadata-row a[href*='/tags/']", ".tags a[href*='/tags/']"],
        "youporn" => &[".tags a[href*='/tags/']", ".video-tags a"],
        _ => &[],
    };

    let channel_selectors: &[&str] = match site_id {
        "pornhub" => &[
            ".userInfo a[href*='/users/']",
            ".usernameWrap a[href*='/model/']",
            "a[href*='/channels/']",
        ],
        "xhamster" => &[
            ".author a[href*='/users/']",
            ".uploader-name a[href*='/users/']",
        ],
        "xvideos" => &[
            ".uploader a[href*='/channels/']",
            ".video-uploader a[href*='/channels/']",
        ],
        "xnxx" => &[".uploader a", ".video-uploader a[href*='/profiles/']"],
        "youporn" => &[
            ".username a[href*='/channels/']",
            ".uploader a[href*='/channels/']",
        ],
        _ => &[],
    };

    fn extract_texts(doc: &Html, selectors: &[&str]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for sel_str in selectors {
            let Ok(sel) = Selector::parse(sel_str) else {
                continue;
            };
            for el in doc.select(&sel) {
                let text = el.text().collect::<String>().trim().to_string();
                if text.is_empty() || text.len() > 80 {
                    continue;
                }
                if seen.insert(text.to_lowercase()) {
                    out.push(text);
                }
            }
        }
        out
    }

    let performers = extract_texts(&doc, performer_selectors);
    let tags = extract_texts(&doc, tag_selectors);
    let channels = extract_texts(&doc, channel_selectors);
    let channel = channels.into_iter().next();

    (performers, tags, channel)
}

#[cfg(test)]
mod category_tests {
    use super::*;

    #[test]
    fn parses_slug_from_href() {
        assert_eq!(
            category_slug_from_href("/categories/big-tits?page=2"),
            Some("big-tits".to_string())
        );
    }

    #[test]
    fn parses_category_id_from_href() {
        assert_eq!(category_id_from_href("/video?c=8"), Some(8));
        assert_eq!(
            category_id_from_href("/lesbian/video?c=27&page=2"),
            Some(27)
        );
        assert_eq!(category_id_from_href("/categories/big-tits"), None);
    }

    #[test]
    fn parses_embedded_category_json() {
        let mut html = String::new();
        for i in 0..12 {
            html.push_str(&format!(
                r#"{{"id":{i},"name":"Cat \u0026 {i}","english":"x","slug":"cat-{i}","segment":0,"video_count":{}}},"#,
                1000 + i
            ));
        }
        let cats = parse_pornhub_categories(&html, BrowseOrientation::Straight);
        assert_eq!(cats.len(), 12);
        assert_eq!(cats[0].name, "Cat & 0");
        assert_eq!(cats[0].category_id, Some(0));
        assert_eq!(cats[0].video_count, Some(1000));
    }

    #[test]
    fn parses_count_from_text() {
        assert_eq!(parse_video_count("544,892 Videos"), Some(544892));
    }

    #[test]
    fn lesbian_numeric_category_url() {
        let url = build_pornhub_category_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: "8".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(url, format!("{PH_BASE}/lesbian/video?c=8"));
    }

    #[test]
    fn lesbian_slug_maps_to_category_id() {
        let url = build_pornhub_category_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: "lesbian".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Straight),
        });
        assert_eq!(url, format!("{PH_BASE}/video?c=27"));
    }

    #[test]
    fn lesbian_slug_falls_back_to_search() {
        let url = build_pornhub_category_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: "big-tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/lesbian/video/search?search=big+tits")
        );
    }

    #[test]
    fn lesbian_search_url() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "big tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/lesbian/video/search?search=big+tits")
        );
    }

    #[test]
    fn gay_search_url() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "big tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Gay),
        });
        assert_eq!(url, format!("{PH_BASE}/gay/video/search?search=big+tits"));
    }

    #[test]
    fn transgender_search_url() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Tag,
            slug: "big tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Transgender),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/transgender/video/search?search=big+tits")
        );
    }

    #[test]
    fn straight_search_url_has_no_prefix() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "big tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Straight),
        });
        assert_eq!(url, format!("{PH_BASE}/video/search?search=big+tits"));
    }

    #[test]
    fn search_page_2_appends_page_param() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "big tits".into(),
            page: 2,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/lesbian/video/search?search=big+tits&page=2")
        );
    }

    #[test]
    fn search_term_with_ampersand_cannot_inject_param() {
        // `&` must be percent-encoded so it can't smuggle `&page=` / `&c=`.
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "big & busty".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Lesbian),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/lesbian/video/search?search=big+%26+busty")
        );
        assert!(!url.contains("&page=") && !url.contains("&c="));
    }

    #[test]
    fn search_term_with_unicode_is_encoded() {
        let url = build_pornhub_search_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Search,
            slug: "amateur €".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Straight),
        });
        assert!(url.starts_with(&format!("{PH_BASE}/video/search?search=amateur+")));
        assert!(!url.contains('€'));
    }

    #[test]
    fn category_fallback_encodes_specials() {
        // Same guarantee for the orientation category-fallback branch (M3):
        // routed through query_slug, so specials can't break the query string.
        let url = build_pornhub_category_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: "big-tits".into(),
            page: 1,
            orientation: Some(BrowseOrientation::Gay),
        });
        assert_eq!(url, format!("{PH_BASE}/gay/video/search?search=big+tits"));
    }

    #[test]
    fn assert_pornhub_url_accepts_own_pages() {
        assert!(assert_pornhub_url(&format!("{PH_BASE}/video/search?search=x")).is_ok());
        assert!(assert_pornhub_url(&format!("{PH_BASE}/lesbian/video?c=27")).is_ok());
    }

    #[test]
    fn assert_pornhub_url_rejects_off_host() {
        // Wrong host, wrong scheme, and suffix-impersonation all fail closed.
        assert!(assert_pornhub_url("https://evil.com/video?c=27").is_err());
        assert!(assert_pornhub_url("http://www.pornhub.com/video?c=27").is_err());
        assert!(assert_pornhub_url("https://www.pornhub.com.evil.com/video").is_err());
        assert!(assert_pornhub_url("not a url").is_err());
    }

    #[test]
    fn pornhub_listing_scopes_to_result_grid() {
        let html = r#"
          <ul id="hottestMenuSection"><li><a href="/view_video.php?viewkey=hot1" title="Header promo">x</a></li></ul>
          <ul id="videoSearchResult">
            <li><a href="/view_video.php?viewkey=res1" title="Real result">x</a></li>
            <li><a class="playAllLink" href="/view_video.php?viewkey=res2&pkey=99">Play All</a></li>
          </ul>"#;
        let items = parse_video_links(html, PH_BASE, "pornhub").unwrap();
        let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, ["Real result"]);
    }

    #[test]
    fn unresolved_straight_slug_searches() {
        // /categories/<slug> 404s now; unresolved slugs fall back to search.
        let url = build_pornhub_category_url(&BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: "big-tits".into(),
            page: 2,
            orientation: Some(BrowseOrientation::Straight),
        });
        assert_eq!(
            url,
            format!("{PH_BASE}/video/search?search=big+tits&page=2")
        );
    }

    #[test]
    fn trans_category_avoids_dead_prefix() {
        let q = |slug: &str| BrowseQuery {
            filters: Default::default(),
            kind: BrowseKind::Category,
            slug: slug.into(),
            page: 1,
            orientation: Some(BrowseOrientation::Transgender),
        };
        assert_eq!(
            build_pornhub_category_url(&q("83")),
            format!("{PH_BASE}/video?c=83")
        );
        assert_eq!(
            build_pornhub_category_url(&q("shemale")),
            format!("{PH_BASE}/video/search?search=shemale")
        );
    }

    #[test]
    fn xvideos_rejects_nav_hrefs() {
        assert!(!is_site_video_href("xvideos", "/videos-i-like"));
        assert!(!is_site_video_href("xvideos", "/account"));
        assert!(is_site_video_href(
            "xvideos",
            "/video.opammko7524/3_lesbians_1_staircase"
        ));
    }

    #[test]
    fn xvideos_search_url() {
        let url = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Search,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(url, "https://www.xvideos.com/?k=lesbian");
    }

    #[test]
    fn xhamster_search_url() {
        let url = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Search,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(url, "https://xhamster.com/search/lesbian");
    }

    #[test]
    fn youporn_category_and_pornstar_urls() {
        let cat = tube_browse_url(
            "youporn",
            "https://www.youporn.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Category,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(cat, "https://www.youporn.com/category/lesbian/");

        let star = tube_browse_url(
            "youporn",
            "https://www.youporn.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Model,
                slug: "alexis-fawx".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(star, "https://www.youporn.com/pornstar/alexis-fawx/");

        let search = tube_browse_url(
            "youporn",
            "https://www.youporn.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Search,
                slug: "lesbian".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(search, "https://www.youporn.com/search/?query=lesbian");
    }

    fn q(kind: BrowseKind, slug: &str, page: u32, f: &[(&str, &str)]) -> BrowseQuery {
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string());
        BrowseQuery {
            kind,
            slug: slug.into(),
            page,
            orientation: None,
            filters: crate::models::BrowseFilters {
                sort: get("sort"),
                period: get("period"),
                duration: get("duration"),
                quality: get("quality"),
                include: get("include"),
                exclude: get("exclude"),
                tag: get("tag"),
                source: get("source"),
            },
        }
    }

    #[test]
    fn filtered_tube_urls() {
        let xv = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &q(
                BrowseKind::Search,
                "milf",
                2,
                &[
                    ("sort", "uploaddate"),
                    ("duration", "10min_more"),
                    ("quality", "hd"),
                ],
            ),
        );
        assert_eq!(
            xv,
            "https://www.xvideos.com/?k=milf&p=1&sort=uploaddate&durf=10min_more&quality=hd"
        );
        let xn = tube_browse_url(
            "xnxx",
            "https://www.xnxx.com",
            &q(
                BrowseKind::Search,
                "milf",
                3,
                &[
                    ("sort", "hits"),
                    ("period", "month"),
                    ("duration", "20min+"),
                ],
            ),
        );
        assert_eq!(xn, "https://www.xnxx.com/search/hits/month/20min+/milf/2");
        let xh = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &q(
                BrowseKind::Search,
                "milf",
                1,
                &[
                    ("sort", "newest"),
                    ("duration", "10-30"),
                    ("quality", "1080p"),
                ],
            ),
        );
        assert_eq!(xh, "https://xhamster.com/search/milf?sort=newest&min-duration=10&max-duration=30&quality=1080p");
        let xhc = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &q(
                BrowseKind::Category,
                "amateur",
                2,
                &[
                    ("sort", "best"),
                    ("quality", "1080p"),
                    ("duration", "30-plus"),
                ],
            ),
        );
        assert_eq!(
            xhc,
            "https://xhamster.com/categories/amateur/best/hd/2?min-duration=30&quality=1080p"
        );
        let yp = tube_browse_url(
            "youporn",
            "https://www.youporn.com",
            &q(BrowseKind::Search, "milf", 2, &[("sort", "views")]),
        );
        assert_eq!(
            yp,
            "https://www.youporn.com/search/views/?query=milf&page=2"
        );
        // Unknown ids never reach the URL.
        let bad = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &q(BrowseKind::Search, "milf", 1, &[("sort", "x&evil=1")]),
        );
        assert_eq!(bad, "https://www.xvideos.com/?k=milf");
        let ph = pornhub_filtered(
            build_pornhub_search_url(&q(BrowseKind::Search, "milf", 1, &[])),
            &q(
                BrowseKind::Search,
                "milf",
                1,
                &[
                    ("sort", "mv"),
                    ("period", "w"),
                    ("duration", "30-plus"),
                    ("quality", "hd"),
                ],
            ),
        );
        assert_eq!(
            ph,
            "https://www.pornhub.com/video/search?search=milf&o=mv&t=w&min_duration=30&hd=1"
        );
        // One required category + dash-separated excludes on a search.
        let f = [("include", "29,8"), ("exclude", "35,8,x&y=1")];
        let ph = pornhub_filtered(
            build_pornhub_search_url(&q(BrowseKind::Search, "redhead", 2, &f)),
            &q(BrowseKind::Search, "redhead", 2, &f),
        );
        assert_eq!(
            ph,
            "https://www.pornhub.com/video/search?search=redhead&page=2&filter_category=29&exclude_category=35-8"
        );
        // Category pages ignore `filter_category`; only excludes are sent.
        let cat = q(BrowseKind::Category, "29", 1, &f);
        assert_eq!(
            pornhub_filtered(build_pornhub_category_url(&cat), &cat),
            "https://www.pornhub.com/video?c=29&exclude_category=35-8"
        );
    }

    #[test]
    fn parses_xvideos_tag_letter_page() {
        let html = r#"<ul><li><a href="/tags/z">Z</a></li>
          <li><a href="/tags/zorra"><b>zorra </b><span class="navbadge default">14,021</span></a></li>
          <li><a href="/tags/z-asian"><b>z-asian </b><span class="navbadge default">30,913</span></a></li>
          <li><a href="/tags/zorra"><b>zorra </b><span class="navbadge default">14,021</span></a></li></ul>"#;
        let tags = parse_xvideos_tag_letter(html);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].slug, "zorra");
        assert_eq!(tags[0].count, Some(14_021));
        assert_eq!(xvideos_tag_groups().len(), 28);
        let cats = parse_xvideos_categories(
            r#"<li><a href="/c/Femdom-235">Femdom</a></li><li><a href="/c/Asian_Woman-32">Asian</a></li>"#,
        );
        assert_eq!(cats.len(), 2);
        assert_eq!(
            (cats[0].name.as_str(), cats[0].slug.as_str()),
            ("Asian", "Asian_Woman-32")
        );
        let url = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &q(
                BrowseKind::Category,
                "Femdom-235",
                3,
                &[
                    ("sort", "uploaddate"),
                    ("duration", "10-20min"),
                    ("period", "week"),
                ],
            ),
        );
        assert_eq!(
            url,
            "https://www.xvideos.com/c/s:uploaddate/d:10-20min/Femdom-235/2"
        );
    }

    #[test]
    fn parses_xnxx_tag_letter_page() {
        let html = r#"<ul id="tags"><li><a href="/search/z-asian">z asian</a><strong>30,913</strong></li><li><a href="/search/zaad">zaad</a><strong>1,204</strong></li></ul><a href="/tags/b">B</a>"#;
        let tags = parse_xnxx_tag_letter(html);
        assert_eq!(tags.len(), 2);
        assert_eq!(
            (tags[0].slug.as_str(), tags[0].name.as_str()),
            ("z-asian", "z asian")
        );
        assert_eq!(tags[0].count, Some(30_913));
        assert_eq!(xnxx_tag_groups().len(), 27);
    }

    #[test]
    fn xhamster_initials_fill_lazy_cards() {
        let html = r#"<script>window.initials={"searchResult":{"videoThumbProps":[
            {"pageURL":"https:\/\/xhamster.com\/videos\/a-xh1","duration":552,"title":"A",
             "thumbURL":"https:\/\/ic-vt-nss.xhcdn.com\/a\/1.webp"}]}};</script>"#;
        let mut items = vec![MediaItem {
            id: "1".into(),
            title: "A".into(),
            url: "https://xhamster.com/videos/a-xh1".into(),
            thumbnail: None,
            duration: None,
            site_id: "xhamster".into(),
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
        }];
        fill_from_xhamster_initials(html, &mut items);
        assert_eq!(
            items[0].thumbnail.as_deref(),
            Some("https://ic-vt-nss.xhcdn.com/a/1.webp")
        );
        assert_eq!(items[0].duration, Some(552));
    }

    #[test]
    fn xhamster_tag_index_and_tag_pages() {
        let html = r#"<div class="item"><!----><a class="root-23e38 primary-23e38" href="https://xhamster.com/tags/a-family-affair" data-role="tag-link"><!----><span class="primary-fda7e"><!--[--><!--[!--><!---->A Family Affair<!----><!--]--><!----><!--]--><!----></span></a></div>"#;
        let tags = parse_xhamster_tag_letter(html);
        assert_eq!(tags.len(), 1);
        assert_eq!(
            (tags[0].slug.as_str(), tags[0].name.as_str()),
            ("a-family-affair", "A Family Affair")
        );
        assert_eq!(xhamster_tag_groups().len(), 27);
        let url = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &q(
                BrowseKind::Tag,
                "a-family-affair",
                2,
                &[("sort", "newest"), ("duration", "10-30")],
            ),
        );
        assert_eq!(
            url,
            "https://xhamster.com/tags/a-family-affair/newest/2?min-duration=10&max-duration=30"
        );
        let combined = tube_browse_url(
            "xhamster",
            "https://xhamster.com",
            &q(BrowseKind::Tag, "milf,big-tits", 1, &[]),
        );
        assert_eq!(combined, "https://xhamster.com/search/milf+big+tits");
    }

    #[test]
    fn combined_tag_urls() {
        // XVIDEOS: combined tags are a word search (its /tags/a+b pages are
        // keyword matches, not tag intersections).
        let xv = tube_browse_url(
            "xvideos",
            "https://www.xvideos.com",
            &q(
                BrowseKind::Tag,
                "milf,big-tits",
                1,
                &[("sort", "uploaddate")],
            ),
        );
        assert_eq!(
            xv,
            "https://www.xvideos.com/?k=milf+big+tits&sort=uploaddate"
        );

        // PornHub: two straight or gay categories intersect; others search.
        let ph = build_pornhub_category_url(&q(BrowseKind::Category, "milf,anal", 3, &[]));
        assert_eq!(ph, format!("{PH_BASE}/video/incategories/milf/anal?page=3"));
        let mut gay = q(BrowseKind::Category, "amateur-gay,bareback-gay", 1, &[]);
        gay.orientation = Some(BrowseOrientation::Gay);
        assert_eq!(
            build_pornhub_category_url(&gay),
            format!("{PH_BASE}/gay/video/incategories/amateur/bareback")
        );
        let mut lesbian = q(BrowseKind::Category, "milf,anal", 1, &[]);
        lesbian.orientation = Some(BrowseOrientation::Lesbian);
        assert_eq!(
            build_pornhub_category_url(&lesbian),
            format!("{PH_BASE}/lesbian/video/search?search=milf+anal")
        );
    }

    #[test]
    fn trans_category_urls() {
        let trans = |slug: &str, page: u32| {
            let mut t = q(BrowseKind::Category, slug, page, &[]);
            t.orientation = Some(BrowseOrientation::Transgender);
            build_pornhub_category_url(&t)
        };
        // A trans category is <category> ∩ Transgender.
        assert_eq!(
            trans("milf", 2),
            format!("{PH_BASE}/video/incategories/milf/transgender?page=2")
        );
        // Spoken-language listings.
        assert_eq!(
            trans("language-german", 2),
            format!("{PH_BASE}/language/german?page=2")
        );
        // The hub itself is category 83.
        assert_eq!(trans("83", 1), format!("{PH_BASE}/video?c=83"));
        // Missing pairs fall back to a word search, keeping the page.
        assert_eq!(
            incategories_search_fallback(&trans("bbw", 3)).as_deref(),
            Some("https://www.pornhub.com/video/search?search=bbw+transgender&page=3")
        );
        assert_eq!(
            incategories_search_fallback(&format!(
                "{PH_BASE}/gay/video/incategories/amateur/big-dick"
            ))
            .as_deref(),
            Some("https://www.pornhub.com/gay/video/search?search=amateur+big+dick")
        );
    }

    #[test]
    fn trans_catalog_retags_straight_list() {
        use crate::models::PornhubCategoryEntry;
        let entry = |slug: &str, id: u32| PornhubCategoryEntry {
            name: slug.into(),
            slug: slug.into(),
            orientation: BrowseOrientation::Straight,
            category_id: Some(id),
            video_count: Some(10),
        };
        let out = trans_catalog(vec![entry("milf", 29), entry("transgender", 83)]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "All trans");
        assert_eq!(out[0].video_count, Some(10));
        assert_eq!(out[1].slug, "milf");
        assert!(out
            .iter()
            .all(|c| c.orientation == BrowseOrientation::Transgender));
    }

    #[test]
    fn xnxx_search_url() {
        let url = tube_browse_url(
            "xnxx",
            "https://www.xnxx.com",
            &BrowseQuery {
                filters: Default::default(),
                kind: BrowseKind::Search,
                slug: "lesbians".into(),
                page: 1,
                orientation: None,
            },
        );
        assert_eq!(url, "https://www.xnxx.com/search/lesbians");
    }

    #[test]
    fn youporn_and_xnxx_video_hrefs() {
        assert!(is_site_video_href("youporn", "/watch/16877734/some-title/"));
        assert!(!is_site_video_href("youporn", "/category/lesbian/"));
        assert!(!is_site_video_href("youporn", "/pornstar/alexis-fawx/"));
        assert!(is_site_video_href("xnxx", "/video-abc123/title"));
        assert!(!is_site_video_href("xnxx", "/search/lesbians"));
    }

    #[test]
    fn junk_nav_titles_detected() {
        assert!(is_junk_nav_title("RED videos"));
        assert!(is_junk_nav_title("Liked videos"));
        assert!(is_junk_nav_title("Join"));
        assert!(!is_junk_nav_title("Lesbian massage"));
    }
}
