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

            async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
                let url = tube_browse_url($id, $base, &query);
                let html = ctx.fetch_html(&url, $id).await.unwrap_or_default();
                let mut items = parse_video_links(&html, $base, $id).unwrap_or_default();
                // Drop obvious nav junk so we fall through to yt-dlp.
                items.retain(|i| !is_junk_nav_title(&i.title));
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
                Ok(stream_url)
            }
        }
    };
}

/// Category / tag index for the macro-generated tube adapters.
async fn tube_list_categories(
    ctx: &SiteContext,
    site_id: &str,
    base: &str,
) -> AppResult<Vec<SiteCategory>> {
    use crate::sites::listing::{parse_category_index, segment_after, CategorySpec};
    let (path, selector, prefix, kind, max) = match site_id {
        "xvideos" => ("/tags", "a[href^='/tags/']", "/tags/", BrowseKind::Tag, 400),
        "xnxx" => (
            "/tags",
            "a[href^='/search/']",
            "/search/",
            BrowseKind::Tag,
            400,
        ),
        "xhamster" => (
            "/categories",
            "a[href*='/categories/']",
            "/categories/",
            BrowseKind::Category,
            300,
        ),
        "youporn" => (
            "/categories/",
            "a[href*='/category/']",
            "/category/",
            BrowseKind::Category,
            300,
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
                if !q.slug.chars().all(|c| c.is_ascii_digit()) {
                    let orientation = q.orientation.unwrap_or(BrowseOrientation::Straight);
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
        let html = ctx.fetch_html(&url, self.id()).await?;
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
        // Prefer a progressive MP4 (works through the loopback Referer proxy).
        // Fall back to proxied HLS (playlist + segments rewritten by the proxy).
        #[cfg(mobile)]
        {
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
        let stream_url = raw
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .ok_or_else(|| crate::error::AppError::Other("No stream URL resolved".to_string()))?
            .to_string();
        Ok(maybe_loopback_proxy(
            ctx,
            &stream_url,
            "https://www.pornhub.com/",
        ))
    }
}

/// Rewrite CDN URLs through the local LAN proxy so the WebView can play them
/// with a proper Referer (HTML5 <video> cannot set Referer itself).
fn maybe_loopback_proxy(ctx: &SiteContext, stream_url: &str, referer: &str) -> String {
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
            let category = query.kind == BrowseKind::Category;
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
            filter_kinds: search,
            has_categories: true,
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
                format!("{base}/?k={q}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/models/{slug}"),
            BrowseKind::Category => format!("{base}/c/{slug}"),
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
            BrowseKind::Search | BrowseKind::Tag => {
                let page = if query.page > 1 {
                    format!("?page={}", query.page)
                } else {
                    String::new()
                };
                format!("{base}/search/{slug}{page}")
            }
            BrowseKind::Channel => format!("{base}/channels/{slug}"),
            BrowseKind::Model => format!("{base}/pornstars/{slug}"),
            BrowseKind::Category => {
                // Category listings take sort / HD as path segments:
                // /categories/<slug>/[newest|best]/[hd]/<page>
                let f = &query.filters;
                let mut path = format!("{base}/categories/{slug}");
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
    crate::sites::urls::with_query(
        &url,
        &[
            ("o", &ok(&f.sort)),
            ("t", &ok(&f.period)),
            ("min_duration", min),
            ("max_duration", max),
            ("hd", hd),
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
