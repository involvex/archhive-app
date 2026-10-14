//! xPomelo (xpomelo.com) — a porn search engine aggregating PornHub, XVIDEOS,
//! xHamster, XNXX, Eporner, SpankBang and ~40 other tubes. It hosts nothing:
//! every result links to the source site, so items keep the source URL (and,
//! where ArcHive has an adapter, that site's id) and play / download / show
//! details through the source site's own pipeline.
//!
//! Listings:
//! - `/category/<a>%2C<b>%2C-<c>` — results in every listed category, none in
//!   the `-` ones (the site's own include / exclude editor writes this path).
//! - `/search?query=…`, `/pornstar/<name>`, `/videos` (everything).
//! - `?sort=date|duration|views&date=30|365&duration=10m&quality=hd&site=Pornhub&page=N`.
//! - Orientation is a cookie (`orientation=gay|trans`).
//!
//! Cloudflare blocks HTTP/1.1 clients here; the main (HTTP/2) client passes.

use crate::error::{AppError, AppResult};
use crate::models::{
    BrowseFilters, BrowseKind, BrowseOptions, BrowseOrientation, BrowsePage, BrowseQuery,
    DownloadPlan, DownloadTool, FilterOption, MediaItem, SiteCategory,
};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use std::sync::LazyLock;

const BASE: &str = "https://xpomelo.com";
/// Tags the site's own editor shows at most; the index runs to ~6k.
const POPULAR_TAGS: usize = 600;
/// Vault key of cookies added for xpomelo.com by domain.
const HOST_COOKIE_KEY: &str = "host-xpomelo-com";
/// `list_category_group` id for every tag (not shown as a tab).
const ALL_TAGS_GROUP: &str = "every-tag";
/// `BrowseFilters::source` id that keeps paysite results.
const ALL_SOURCES: &str = "all";

/// Result hosts that only sell the video (trailers / store pages) — hidden
/// unless the Source filter says otherwise.
pub(crate) const PAYSITE_HOSTS: &[&str] = &[
    "faphouse.com",
    "clips4sale.com",
    "brazzers.com",
    "realitykings.com",
    "puretaboo.com",
    "analvids.com",
];

/// Sources whose videos ArcHive can't play or download (Beeg's API no longer
/// returns streams and yt-dlp's extractor is broken; yt-dlp marks SxyPrn
/// unsupported; VipTube's player config comes back empty). Hidden by
/// default like paysites.
pub(crate) const UNSUPPORTED_HOSTS: &[&str] = &["beeg.com", "sxyprn.com", "viptube.com"];

/// `site=` values from the site's Source menu (its own spelling).
const SOURCES: &[&str] = &[
    "Pornhub",
    "XVideos",
    "xHamster",
    "XNXX",
    "Eporner",
    "SpankBang",
    "Noodlemagazine",
    "Youporn",
    "Redtube",
    "PornZog",
    "XGroovy",
    "4KPorn",
    "TNAFlix",
    "Love4Porn",
    "PornOne",
    "Hoes",
    "HDtube",
    "CrazyPorn",
    "Its",
    "FapCat",
    "YouJizz",
    "Tube8",
    "SxyPrn",
    "XXXBP",
    "EroThots1",
    "HQPorner",
    "PornHat",
    "SexVid",
    "TXXX",
    "IPornTV",
    "DrTuber",
    "VipTube",
    "inXXX",
    "AnySex",
    "Analdin",
    "Beeg",
    "Xozilla",
    "Nuvid",
    "Javmost",
    "ThisVid",
    "FapHouse",
    "Clips4sale",
    "Brazzers",
    "AnalVids",
    "RealityKings",
    "PureTaboo",
];

pub struct XpomeloAdapter;

#[async_trait]
impl SiteAdapter for XpomeloAdapter {
    fn id(&self) -> &str {
        "xpomelo"
    }

    fn display_name(&self) -> &str {
        "xPomelo"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Search, BrowseKind::Category, BrowseKind::Model]
    }

    fn browse_options(&self) -> BrowseOptions {
        xpomelo_options()
    }

    fn forget_category_cache(&self) {
        if let Ok(mut c) = TAG_INDEX.lock() {
            *c = None;
        }
    }

    /// The most popular tags (the index is ordered by popularity).
    async fn list_categories(&self, ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
        Ok(tag_index(ctx)
            .await?
            .into_iter()
            .take(POPULAR_TAGS)
            .collect())
    }

    /// "stars" = pornstar index; a…z / "0" = every tag starting with it;
    /// "every-tag" = every tag.
    async fn list_category_group(
        &self,
        ctx: &SiteContext,
        group: &str,
    ) -> AppResult<Vec<SiteCategory>> {
        if group == "stars" {
            let html = fetch(ctx, &format!("{BASE}/pornstar"), None).await?;
            return Ok(parse_star_index(&html));
        }
        let tags = tag_index(ctx).await?;
        if group == ALL_TAGS_GROUP {
            return Ok(tags);
        }
        Ok(tags
            .into_iter()
            .filter(|c| {
                let first = c.name.chars().next().map(|ch| ch.to_ascii_lowercase());
                match first {
                    Some(ch) if ch.is_ascii_lowercase() => ch.to_string() == group,
                    _ => group == "0",
                }
            })
            .collect())
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        // A range ("10m+") is several of the site's buckets: fetch the same
        // page of each and merge them.
        // Low ranges ("10m+", "5m+", "under 10") come from the unfiltered
        // listing, kept to the length here: one request per page and the
        // site's own order, where buckets cost one request each (about 37
        // of 80 videos on a page are 10+ minutes).
        let local_range = local_length_range(query.filters.duration.as_deref());
        let buckets = if local_range.is_some() {
            Vec::new()
        } else {
            duration_buckets(query.filters.duration.as_deref())
        };
        let queries: Vec<BrowseQuery> = if buckets.len() > 1 {
            buckets
                .iter()
                .map(|b| {
                    let mut q = query.clone();
                    q.filters.duration = Some((*b).to_string());
                    q
                })
                .collect()
        } else {
            let mut q = query.clone();
            q.filters.duration = buckets.first().map(|b| (*b).to_string());
            vec![q]
        };
        // One after another, not in parallel: xPomelo's Cloudflare rate
        // limits bursts.
        let mut pages = Vec::with_capacity(queries.len());
        for q in &queries {
            // A bucket that ended on an earlier page isn't asked again.
            let first_page_url = listing_url(&BrowseQuery {
                page: 1,
                ..q.clone()
            });
            if buckets.len() > 1 && bucket_ended_before(&first_page_url, q.page) {
                pages.push(Ok((Vec::new(), false, None)));
                continue;
            }
            let url = listing_url(q);
            tracing::info!("[xpomelo] browse: {url}");
            let mut fetched = fetch(ctx, &url, q.orientation).await;
            // A full page has PAGE_SIZE cards (fewer only on the last page).
            // A short one can be a throttled answer: ask once more (at the
            // normal pace) and keep whichever has more cards.
            if let Ok(html) = &fetched {
                let got = raw_card_count(html);
                if let Some(expected) = expected_cards(html, q.page).filter(|e| got < *e) {
                    tracing::warn!(
                        "[xpomelo] page {} had {got} of {expected} results; fetching it again",
                        q.page
                    );
                    match fetch(ctx, &url, q.orientation).await {
                        Ok(again) if raw_card_count(&again) > got => fetched = Ok(again),
                        Ok(_) => tracing::warn!("[xpomelo] page {} still short", q.page),
                        Err(e) => tracing::warn!("[xpomelo] refetch failed: {e}"),
                    }
                }
            }
            let page = match fetched {
                Ok(html) => {
                    let cards = parse_listing(&html);
                    let more = raw_card_count(&html) >= PAGE_SIZE
                        || html.contains(&format!("page={}", q.page + 1));
                    Ok((cards, more, result_count(&html)))
                }
                // Past the last page xPomelo answers 404: the end, not an error.
                Err(AppError::Site(m)) if q.page > 1 && m.contains("HTTP 404") => {
                    Ok((Vec::new(), false, None))
                }
                Err(e) => Err(e),
            };
            if buckets.len() > 1 {
                if let Ok((_, false, _)) = &page {
                    note_bucket_end(&first_page_url, q.page);
                }
            }
            // Rate limited: the other buckets would be refused too.
            let limited = matches!(&page, Err(AppError::Site(m)) if m.contains("429") || m.contains("Cloudflare"));
            pages.push(page);
            if limited {
                break;
            }
        }
        let mut lists = Vec::new();
        let mut has_more = false;
        let mut first_err = None;
        // A range adds up its buckets' counts (all of them or none).
        let mut total: Option<u32> = Some(0);
        for page in pages {
            match page {
                Ok((cards, more, count)) => {
                    has_more |= more;
                    lists.push(cards);
                    total = total.zip(count).map(|(a, b)| a.saturating_add(b));
                }
                Err(e) => first_err = first_err.or(Some(e)),
            }
        }
        if lists.is_empty() {
            return Err(
                first_err.unwrap_or_else(|| AppError::Site("xPomelo returned nothing".into()))
            );
        }
        let by_duration = query.filters.sort.as_deref() == Some("duration");
        let mut cards = dedupe(merge_lists(lists, by_duration));
        if let Some((min, max)) = local_range {
            cards.retain(|c| {
                c.duration
                    .is_some_and(|d| d >= min && max.is_none_or(|m| d < m))
            });
            // The heading counts the unfiltered listing.
            total = None;
        }
        // The default ("Free sites") hides paysites and unsupported sources;
        // picking a source (or "All") shows them.
        let keep_hidden = query.filters.source.is_some();
        let items = cards
            .into_iter()
            .filter(|i| keep_hidden || !hidden_by_default(&i.url))
            .map(|mut i| {
                // xPomelo's own `/image?key=` thumbnails sit behind the same
                // Cloudflare rules; on the phone fetch them via the app.
                if let Some(t) = i.thumbnail.as_deref().filter(|t| t.starts_with(BASE)) {
                    let session = browser_session();
                    i.thumbnail = Some(crate::sites::adapters::pornhub::loopback_proxy_with(
                        ctx,
                        t,
                        "https://xpomelo.com/",
                        session.as_ref().map(|(c, _)| c.as_str()),
                        session.as_ref().map(|(_, ua)| ua.as_str()),
                    ));
                }
                i
            })
            .collect();
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total,
            ..Default::default()
        })
    }

    /// Results are source-site URLs; plain yt-dlp resolves the ones without
    /// an ArcHive adapter of their own (those carry the source site's id).
    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            // Jobs take the Settings naming template (`downloads::naming`,
            // which falls back to the site's domain when there's no
            // uploader); this one is only a placeholder.
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.id().to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: item.channel.clone(),
            referer: None,
            initial_status: None,
        })
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        crate::sites::adapters::custom::CustomUrlAdapter
            .resolve_stream_url(ctx, url)
            .await
    }
}

/// xPomelo serves at most this many pages of any listing (page 11 is a 404
/// however many results the heading reports).
pub(crate) const MAX_PAGES: u32 = 10;
/// Results one listing can show: MAX_PAGES full pages.
pub(crate) const MAX_RESULTS: u32 = MAX_PAGES * PAGE_SIZE as u32;

/// Slices a full fetch starts from: a length range becomes its buckets
/// (each its own listing of up to MAX_PAGES pages).
pub(crate) fn crawl_start(filters: &BrowseFilters) -> Vec<BrowseFilters> {
    let buckets = duration_buckets(filters.duration.as_deref());
    if buckets.len() > 1 {
        buckets
            .into_iter()
            .map(|b| BrowseFilters {
                duration: Some(b.to_string()),
                ..filters.clone()
            })
            .collect()
    } else {
        vec![filters.clone()]
    }
}

/// Narrower, non-overlapping slices for a listing with more results than
/// MAX_PAGES can show: by length bucket first, then by source site
/// (paysites and unsupported sources left out). Empty when it can't be
/// split any further.
pub(crate) fn crawl_split(filters: &BrowseFilters) -> Vec<BrowseFilters> {
    let one_bucket = filters
        .duration
        .as_deref()
        .is_some_and(|d| DURATION_BUCKETS.contains(&d));
    if !one_bucket {
        return DURATION_BUCKETS
            .iter()
            .map(|b| BrowseFilters {
                duration: Some((*b).to_string()),
                ..filters.clone()
            })
            .collect();
    }
    if filters.source.is_some() {
        return Vec::new();
    }
    SOURCES
        .iter()
        .filter(|s| source_label(s) == **s)
        .map(|s| BrowseFilters {
            source: Some((*s).to_string()),
            ..filters.clone()
        })
        .collect()
}

/// Short description of a slice for progress messages.
pub(crate) fn crawl_slice_label(filters: &BrowseFilters) -> String {
    let length = match filters.duration.as_deref() {
        Some("1m") => "1–5 min",
        Some("5m") => "5–10 min",
        Some("10m") => "10–20 min",
        Some("20m") => "20–30 min",
        Some("30m") => "30+ min",
        _ => "",
    };
    [length, filters.source.as_deref().unwrap_or("")]
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
}

/// Source menu label: paysites and unsupported sources are marked.
fn source_label(source: &str) -> String {
    let lower = source.to_ascii_lowercase();
    let is = |hosts: &[&str]| {
        hosts
            .iter()
            .any(|h| h.split('.').next() == Some(lower.as_str()))
    };
    if is(PAYSITE_HOSTS) {
        format!("{source} (paysite)")
    } else if is(UNSUPPORTED_HOSTS) {
        format!("{source} (unsupported)")
    } else {
        source.to_string()
    }
}

fn xpomelo_options() -> BrowseOptions {
    let o = |id: &str, label: &str| FilterOption::new(id, label);
    BrowseOptions {
        sorts: vec![
            o("date", "Newest"),
            o("views", "Most viewed"),
            o("duration", "Longest"),
        ],
        periods: vec![o("30", "Past month"), o("365", "Past year")],
        // xPomelo's own values are buckets (10m = 10–19:59); the ranges are
        // merged from several bucket listings (`duration_buckets`).
        durations: vec![
            o("-10m", "Under 10 min"),
            o("1m", "1–5 min"),
            o("5m", "5–10 min"),
            o("10m", "10–20 min"),
            o("10m-30m", "10–30 min"),
            o("20m", "20–30 min"),
            o("5m+", "5+ min"),
            o("10m+", "10+ min"),
            o("20m+", "20+ min"),
            o("30m", "30+ min"),
        ],
        qualities: vec![o("hd", "HD"), o("4k", "4K")],
        sources: std::iter::once(o(ALL_SOURCES, "All (incl. paysites, unsupported)"))
            .chain(SOURCES.iter().map(|s| o(s, &source_label(s))))
            .collect(),
        filter_kinds: vec![BrowseKind::Search, BrowseKind::Category, BrowseKind::Model],
        has_categories: true,
        combine_tags: 10,
        category_groups: std::iter::once(o("stars", "Pornstars"))
            .chain(('a'..='z').map(|c| o(&c.to_string(), &c.to_ascii_uppercase().to_string())))
            .chain(std::iter::once(o("0", "0–9")))
            .collect(),
        include_exclude: true,
        include_exclude_kinds: vec![BrowseKind::Category],
        tag_index_group: Some(ALL_TAGS_GROUP.into()),
        ..Default::default()
    }
}

/// Cookies + User-Agent of an in-app browser visit that got past xPomelo's
/// Cloudflare after it refused the app; later requests reuse them so they
/// look like that browser session.
static BROWSER_SESSION: LazyLock<std::sync::Mutex<Option<(String, String)>>> =
    LazyLock::new(Default::default);

fn browser_session() -> Option<(String, String)> {
    BROWSER_SESSION.lock().ok().and_then(|s| s.clone())
}

/// Cloudflare's bot check refused the app (HTTP 403). The in-app browser
/// gets past that one.
fn is_bot_check(e: &AppError) -> bool {
    e.to_string().contains("HTTP 403")
}

/// Rate limited (429 / Cloudflare 1015) or ArcHive pausing after one. A
/// browser can't get past an IP rate limit, so nothing more is sent.
fn is_rate_limit(e: &AppError) -> bool {
    let m = e.to_string().to_ascii_lowercase();
    m.contains("429")
        || m.contains("too many requests")
        || m.contains("rate limit")
        || m.contains("pausing")
}

/// HTTP 5xx: xPomelo's own server isn't answering (Cloudflare 520-526,
/// 502/503/504) or a connection that couldn't be made at all.
fn is_site_down(e: &AppError) -> bool {
    let m = e.to_string();
    (500..=599).any(|code| m.contains(&format!("HTTP {code}")))
        || m.contains("error sending request")
        || m.contains("connection")
}

const SITE_DOWN: &str = "xPomelo is down right now: its server isn't answering (Cloudflare \
     error 52x). Nothing to fix in the app; try again later. Saved searches still open \
     from the Online library.";

const RATE_LIMITED: &str = "xPomelo's Cloudflare is rate limiting this connection (error 1015). \
     ArcHive has paused xPomelo requests and tries again on its own; the block \
     usually lifts within 10-60 minutes. Saved searches still open from the \
     Online library meanwhile.";

async fn fetch(
    ctx: &SiteContext,
    url: &str,
    orientation: Option<BrowseOrientation>,
) -> AppResult<String> {
    let session = browser_session();
    let mut cookie = match orientation {
        Some(BrowseOrientation::Gay) => "orientation=gay".to_string(),
        Some(BrowseOrientation::Transgender) => "orientation=trans".to_string(),
        _ => String::new(),
    };
    if let Some((c, _)) = &session {
        if !c.is_empty() {
            if !cookie.is_empty() {
                cookie.push_str("; ");
            }
            cookie.push_str(c);
        }
    }
    let mut headers = vec![
        (
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
        ("Accept-Language", "en-US,en;q=0.9"),
    ];
    if !cookie.is_empty() {
        headers.push(("Cookie", cookie.as_str()));
    }
    if let Some((_, ua)) = session.as_ref().filter(|(_, ua)| !ua.is_empty()) {
        headers.push(("User-Agent", ua.as_str()));
    }
    // Cookies saved for xPomelo itself, or added by domain ("Other site":
    // host-xpomelo-com) before xPomelo had its own entry.
    let cookie_key = if !ctx.has_cookies("xpomelo") && ctx.has_cookies(HOST_COOKIE_KEY) {
        HOST_COOKIE_KEY
    } else {
        "xpomelo"
    };
    match ctx
        .fetch_with_headers(url, cookie_key, Some(&headers))
        .await
    {
        Ok(html) => Ok(html),
        Err(e) if is_rate_limit(&e) => Err(AppError::Site(RATE_LIMITED.to_string())),
        Err(e) if is_site_down(&e) => Err(AppError::Site(SITE_DOWN.to_string())),
        Err(e) if is_bot_check(&e) => {
            #[cfg(target_os = "android")]
            {
                match fetch_in_browser(ctx, url).await {
                    Ok(html) => return Ok(html),
                    Err(be) => tracing::warn!("[xpomelo] in-app browser fallback failed: {be}"),
                }
            }
            Err(AppError::Site(
                "xPomelo's Cloudflare protection refused the request (HTTP 403); \
                 try again in a minute"
                    .to_string(),
            ))
        }
        Err(e) => Err(e),
    }
}

/// Load the page in the in-app browser (a real WebView, which Cloudflare
/// lets through like the phone's browser) and keep its session.
#[cfg(target_os = "android")]
async fn fetch_in_browser(ctx: &SiteContext, url: &str) -> AppResult<String> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOCK.lock().await;
    tracing::info!("[xpomelo] app refused; loading {url} in the in-app browser");
    let page = crate::cookie_login::open_page(ctx.app(), url, "Loading xPomelo").await;
    crate::cookie_login::close_page(ctx.app()).await;
    let page = page?;
    let lower = page.html.to_ascii_lowercase();
    if lower.contains("error 1015") || lower.contains("you are being rate limited") {
        crate::sites::throttle::note_rate_limited(url, None);
        return Err(AppError::Site(RATE_LIMITED.to_string()));
    }
    if let Ok(mut s) = BROWSER_SESSION.lock() {
        *s = Some((page.cookies.clone(), page.user_agent.clone()));
    }
    Ok(page.html)
}

/// Tag index (`<script id="data-tags">[{"key":"big tits","name":"big tits"},…]`
/// on every category page), most popular first. Cached for the session: the
/// page is ~1.3 MB.
/// The full tag index, kept for the session so every letter group and the
/// full list come from one page fetch (state caches each group on disk).
static TAG_INDEX: LazyLock<std::sync::Mutex<Option<Vec<SiteCategory>>>> =
    LazyLock::new(Default::default);

async fn tag_index(ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
    if let Some(tags) = TAG_INDEX.lock().ok().and_then(|c| c.clone()) {
        return Ok(tags);
    }
    let html = fetch(ctx, &format!("{BASE}/category"), None).await?;
    let tags = parse_tag_index(&html);
    if tags.is_empty() {
        return Err(AppError::Site(
            "xPomelo category index not found on the page".to_string(),
        ));
    }
    if let Ok(mut c) = TAG_INDEX.lock() {
        *c = Some(tags.clone());
    }
    Ok(tags)
}

fn embedded_json(html: &str, id: &str) -> Option<serde_json::Value> {
    let marker = format!(r#"id="{id}""#);
    let start = html.find(&marker)?;
    let body_start = start + html[start..].find('>')? + 1;
    let body_end = body_start + html[body_start..].find("</script>")?;
    serde_json::from_str(html[body_start..body_end].trim()).ok()
}

fn parse_tag_index(html: &str) -> Vec<SiteCategory> {
    let Some(serde_json::Value::Array(tags)) = embedded_json(html, "data-tags") else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    tags.iter()
        .filter_map(|t| {
            let key = t.get("key")?.as_str()?.trim();
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or(key).trim();
            let slug = tag_slug(key);
            (!slug.is_empty() && seen.insert(slug.clone())).then(|| SiteCategory {
                name: crate::sites::urls::html_unescape(name),
                slug,
                kind: BrowseKind::Category,
                thumbnail: None,
                count: None,
            })
        })
        .collect()
}

/// `<script id="data-stars">["riley reid", …]` on /pornstar.
fn parse_star_index(html: &str) -> Vec<SiteCategory> {
    let Some(serde_json::Value::Array(stars)) = embedded_json(html, "data-stars") else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    stars
        .iter()
        .filter_map(|s| {
            let name = s.as_str()?.trim();
            let slug = tag_slug(name);
            (!slug.is_empty() && seen.insert(slug.clone())).then(|| SiteCategory {
                name: title_case(name),
                slug,
                kind: BrowseKind::Model,
                thumbnail: None,
                count: None,
            })
        })
        .collect()
}

fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// "Big Tits" → "big-tits" (the site's own path form).
fn tag_slug(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Percent-encode one path segment (keeps `[a-z0-9-]`).
fn encode_segment(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn listing_url(query: &BrowseQuery) -> String {
    let f = &query.filters;
    let path = match query.kind {
        BrowseKind::Search if !query.slug.trim().is_empty() => crate::sites::urls::with_query(
            &format!("{BASE}/search"),
            &[("query", query.slug.trim())],
        ),
        BrowseKind::Model if !query.slug.trim().is_empty() => {
            format!("{BASE}/pornstar/{}", encode_segment(&tag_slug(&query.slug)))
        }
        _ => {
            // Category slug "a,b,-c" (combined in the category browser or a
            // pasted site path) plus the include / exclude filters.
            let mut include = Vec::new();
            let mut exclude = Vec::new();
            for t in crate::models::split_tags(&query.slug) {
                match t.strip_prefix('-') {
                    Some(x) => exclude.push(tag_slug(x)),
                    None => include.push(tag_slug(&t)),
                }
            }
            let list = |v: &Option<String>| {
                v.as_deref()
                    .map(crate::models::split_tags)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| tag_slug(&t))
            };
            include.extend(list(&f.include));
            exclude.extend(list(&f.exclude));
            include.retain(|t| !t.is_empty());
            exclude.retain(|t| !t.is_empty() && !include.contains(t));
            include.dedup();
            exclude.dedup();
            if include.is_empty() && exclude.is_empty() {
                format!("{BASE}/videos")
            } else {
                let parts: Vec<String> = include
                    .iter()
                    .map(|t| encode_segment(t))
                    .chain(exclude.iter().map(|t| format!("-{}", encode_segment(t))))
                    .collect();
                format!("{BASE}/category/{}", parts.join("%2C"))
            }
        }
    };
    let pick = |v: &Option<String>, allowed: &[&str]| {
        v.as_deref()
            .filter(|v| allowed.contains(v))
            .unwrap_or("")
            .to_string()
    };
    let source = f
        .source
        .as_deref()
        .and_then(|s| SOURCES.iter().find(|x| **x == s))
        .copied()
        .unwrap_or("");
    let page = if query.page > 1 {
        query.page.to_string()
    } else {
        String::new()
    };
    crate::sites::urls::with_query(
        &path,
        &[
            ("sort", &pick(&f.sort, &["date", "duration", "views"])),
            ("date", &pick(&f.period, &["30", "365"])),
            ("duration", &pick(&f.duration, DURATION_BUCKETS)),
            ("quality", &pick(&f.quality, &["hd", "4k"])),
            ("site", source),
            ("page", &page),
        ],
    )
}

/// ArcHive site id for a result host — those play, download and show details
/// through their own adapters; anything else stays "xpomelo" (yt-dlp).
fn source_site_id(url: &str) -> &'static str {
    let host = host_of(url);
    let is = |domain: &str| host_matches(&host, domain);
    for (domain, id) in [
        ("pornhub.com", "pornhub"),
        ("xvideos.com", "xvideos"),
        ("xnxx.com", "xnxx"),
        ("xhamster.com", "xhamster"),
        ("eporner.com", "eporner"),
        ("youporn.com", "youporn"),
        ("thisvid.com", "thisvid"),
    ] {
        if is(domain) {
            return id;
        }
    }
    "xpomelo"
}

fn hidden_by_default(url: &str) -> bool {
    is_paysite(url) || {
        let host = host_of(url);
        UNSUPPORTED_HOSTS.iter().any(|d| host_matches(&host, d))
    }
}

fn is_paysite(url: &str) -> bool {
    let host = host_of(url);
    PAYSITE_HOSTS.iter().any(|d| host_matches(&host, d))
}

fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|r| r.split(['/', '?', '#']).next())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn host_matches(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// xPomelo's `duration` values; each is a bucket up to the next one
/// (`10m` = 10:00–19:59), `30m` is open-ended.
const DURATION_BUCKETS: &[&str] = &["1m", "5m", "10m", "20m", "30m"];

/// Buckets for a duration filter: one bucket ("10m"), from ("10m+"),
/// under ("-10m") or a range ("10m-30m", upper bound exclusive).
fn duration_buckets(value: Option<&str>) -> Vec<&'static str> {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return vec![];
    };
    if let Some(b) = DURATION_BUCKETS.iter().find(|b| **b == v) {
        return vec![b];
    }
    let mins = |s: &str| s.trim().trim_end_matches('m').parse::<u32>().ok();
    let (lo, hi) = if let Some(from) = v.strip_suffix('+') {
        (mins(from), Some(u32::MAX))
    } else if let Some(under) = v.strip_prefix('-') {
        (Some(0), mins(under))
    } else if let Some((a, b)) = v.split_once('-') {
        (mins(a), mins(b))
    } else {
        (None, None)
    };
    let (Some(lo), Some(hi)) = (lo, hi) else {
        return vec![];
    };
    DURATION_BUCKETS
        .iter()
        .filter(|b| mins(b).is_some_and(|start| start >= lo && start < hi))
        .copied()
        .collect()
}

/// Ranges filtered in the app from the unfiltered listing: (min, max) in
/// seconds, max exclusive.
fn local_length_range(value: Option<&str>) -> Option<(u32, Option<u32>)> {
    match value? {
        "10m+" => Some((600, None)),
        "5m+" => Some((300, None)),
        "-10m" => Some((0, Some(600))),
        _ => None,
    }
}

/// Last page of each bucket listing (by its page-1 URL) seen to have no
/// next page, so later pages of a range skip it instead of asking for a
/// page that doesn't exist.
static BUCKET_END: LazyLock<std::sync::Mutex<std::collections::HashMap<String, u32>>> =
    LazyLock::new(Default::default);

fn bucket_ended_before(first_page_url: &str, page: u32) -> bool {
    BUCKET_END
        .lock()
        .ok()
        .and_then(|m| m.get(first_page_url).copied())
        .is_some_and(|last| page > last)
}

fn note_bucket_end(first_page_url: &str, page: u32) {
    if let Ok(mut m) = BUCKET_END.lock() {
        if m.len() > 500 {
            m.clear();
        }
        m.insert(first_page_url.to_string(), page);
    }
}

/// Interleave bucket listings (longest first when sorting by duration).
fn merge_lists(lists: Vec<Vec<MediaItem>>, by_duration: bool) -> Vec<MediaItem> {
    let mut iters: Vec<_> = lists.into_iter().map(Vec::into_iter).collect();
    let mut out = Vec::new();
    loop {
        let before = out.len();
        out.extend(iters.iter_mut().filter_map(Iterator::next));
        if out.len() == before {
            break;
        }
    }
    if by_duration {
        out.sort_by_key(|i| std::cmp::Reverse(i.duration.unwrap_or(0)));
    }
    out
}

/// Drop repeats: the same video (see [`crate::sites::urls::video_key`]) or the same title with
/// the same length (one upload mirrored on several sources).
fn dedupe(items: Vec<MediaItem>) -> Vec<MediaItem> {
    let mut keys = std::collections::HashSet::new();
    let mut titles = std::collections::HashSet::new();
    items
        .into_iter()
        .filter_map(|mut i| {
            let key = crate::sites::urls::video_key(&i.url);
            let title: String = i
                .title
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            let same_title = i
                .duration
                .filter(|_| title.len() >= 8)
                .is_some_and(|d| !titles.insert((title, d)));
            if !keys.insert(key.clone()) || same_title {
                return None;
            }
            i.id = key;
            Some(i)
        })
        .collect()
}

/// Result cards per xPomelo page (ads not counted).
const PAGE_SIZE: usize = 80;

/// Result cards on a page before any filtering (ad cards excluded).
fn raw_card_count(html: &str) -> usize {
    html.split(r#"<div class="qy-mod-li"#)
        .skip(1)
        .filter(|b| !b.split('"').next().unwrap_or("").contains("ad-li"))
        .filter(|b| b.contains("qy-mod-link"))
        .count()
}

/// Cards page `page` should have: a full PAGE_SIZE when it links to a next
/// page; None for the last page. (The heading's count is approximate, so
/// it can't tell how many the last page holds; trusting it made the app
/// refetch complete last pages and run into the rate limit.)
fn expected_cards(html: &str, page: u32) -> Option<usize> {
    html.contains(&format!("page={}", page + 1))
        .then_some(PAGE_SIZE)
}

/// Result count from the page heading: `<h1 class="main cat-count-header">
/// Blonde Porn Videos (575,432)</h1>` on category / pornstar pages (it follows
/// the filters). Search pages have none.
fn result_count(html: &str) -> Option<u32> {
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"<h1[^>]*>[^<]*\(([\d,.]+)\)\s*</h1>"#).expect("count")
    });
    RE.captures(html)?[1]
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

/// `/image?key=…&src=https%3A%2F%2Fthumbs…` carries the source site's own
/// thumbnail; xPomelo's image server is only needed when it doesn't.
fn original_thumb(src: &str) -> Option<String> {
    let query = src.strip_prefix("/image?")?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "src")
        .map(|(_, v)| v.into_owned())
        .filter(|v| v.starts_with("https://") || v.starts_with("http://"))
}

/// Result cards: `<div class="qy-mod-li " data-id="…">` with
/// `a.qy-mod-link[href]` (source URL), `<img src>`, `span.CrkfKf` (duration),
/// `a.qy-title-link[title]` and `a.qy-source` (source site name). Ad cards
/// carry `hilltop-ad-li` / `adsterra-ad-li`.
fn parse_listing(html: &str) -> Vec<MediaItem> {
    static LINK: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"class="qy-mod-link[^"]*" href="(https?://[^"]+)""#).expect("link")
    });
    static TITLE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"class="qy-title-link" title="([^"]*)""#).expect("title")
    });
    static THUMB: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"<img [^>]*src="([^"]+)""#).expect("thumb"));
    static DUR: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"class="CrkfKf">([\d:]+)<"#).expect("duration"));
    static SOURCE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"class="qy-source"[^>]*>([^<]+)<"#).expect("source"));
    let mut seen = std::collections::HashSet::new();
    html.split(r#"<div class="qy-mod-li"#)
        .skip(1)
        .filter_map(|block| {
            let class_rest = block.split('"').next().unwrap_or("");
            if class_rest.contains("ad-li") {
                return None;
            }
            let url = crate::sites::urls::html_unescape(LINK.captures(block)?.get(1)?.as_str());
            if !seen.insert(url.clone()) {
                return None;
            }
            let title = TITLE
                .captures(block)
                .map(|c| crate::sites::urls::html_unescape(&c[1]).trim().to_string())
                .filter(|t| !t.is_empty())?;
            let thumbnail = THUMB.captures(block).map(|c| {
                let src = crate::sites::urls::html_unescape(&c[1]);
                if let Some(original) = original_thumb(&src) {
                    // The source site's own CDN: no request to xPomelo.
                    original
                } else if src.starts_with('/') {
                    format!("{BASE}{src}")
                } else {
                    src
                }
            });
            let duration = DUR.captures(block).and_then(|c| {
                c[1].split(':')
                    .try_fold(0u32, |acc, p| Some(acc * 60 + p.parse::<u32>().ok()?))
            });
            let source = SOURCE
                .captures(block)
                .map(|c| crate::sites::urls::html_unescape(c[1].trim()));
            Some(MediaItem {
                id: url.clone(),
                title,
                site_id: source_site_id(&url).to_string(),
                url,
                thumbnail,
                duration,
                performers: vec![],
                tags: vec![],
                description: None,
                channel: source,
                is_live: None,
                viewers: None,
                age: None,
                gender: None,
                stream_url: None,
                embed_url: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BrowseFilters;

    fn q(kind: BrowseKind, slug: &str, page: u32, filters: BrowseFilters) -> BrowseQuery {
        BrowseQuery {
            kind,
            slug: slug.into(),
            page,
            orientation: None,
            filters,
        }
    }

    #[test]
    fn category_include_exclude_path() {
        let url = listing_url(&q(
            BrowseKind::Category,
            "prostate-massage,-pegging",
            2,
            BrowseFilters {
                include: Some("Big Tits".into()),
                exclude: Some("anal,big-tits".into()),
                sort: Some("date".into()),
                duration: Some("10m".into()),
                source: Some("Pornhub".into()),
                ..Default::default()
            },
        ));
        assert_eq!(
            url,
            "https://xpomelo.com/category/prostate-massage%2Cbig-tits%2C-pegging%2C-anal\
             ?sort=date&duration=10m&site=Pornhub&page=2"
        );
    }

    #[test]
    fn search_model_and_everything_urls() {
        let none = BrowseFilters::default;
        assert_eq!(
            listing_url(&q(BrowseKind::Search, "big ass", 1, none())),
            "https://xpomelo.com/search?query=big+ass"
        );
        assert_eq!(
            listing_url(&q(BrowseKind::Model, "Angela White", 1, none())),
            "https://xpomelo.com/pornstar/angela-white"
        );
        assert_eq!(
            listing_url(&q(BrowseKind::Category, "", 1, none())),
            "https://xpomelo.com/videos"
        );
        // Unknown ids are dropped, "all" sources = no site filter.
        let url = listing_url(&q(
            BrowseKind::Search,
            "milf",
            1,
            BrowseFilters {
                sort: Some("bogus".into()),
                source: Some(ALL_SOURCES.into()),
                quality: Some("4k".into()),
                ..Default::default()
            },
        ));
        assert_eq!(url, "https://xpomelo.com/search?query=milf&quality=4k");
    }

    #[test]
    fn listing_cards_skip_ads_and_map_sources() {
        let html = r#"
            <div class="qy-mod-li " data-id="pornhub.com_1"><div class="qy-mod-img">
              <a class="qy-mod-link eA0Zlc" href="https://www.pornhub.com/view_video.php?viewkey=1" target="_blank">
              <img src="/image?key=-14" alt="x"><span class="CrkfKf">15:21</span></a>
              <a class="qy-title-link" title="Cherry &amp; Kiss" href="https://www.pornhub.com/view_video.php?viewkey=1"></a>
              <a class="qy-source" href="/source/pornhub">Pornhub</a></div></div>
            <div class="qy-mod-li hilltop-ad-li"><a class="qy-mod-link" href="https://ads.example/x"></a>
              <a class="qy-title-link" title="Ad"></a></div>
            <div class="qy-mod-li " data-id="spankbang.com_2"><div class="qy-mod-img">
              <a class="qy-mod-link eA0Zlc" href="https://spankbang.com/abc/video/x" target="_blank">
              <img src="https://tb.sb-cd.com/t.jpg"><span class="CrkfKf">1:02:03</span></a>
              <a class="qy-title-link" title="Second"></a>
              <a class="qy-source" href="/source/spankbang">SpankBang</a></div></div>"#;
        let items = parse_listing(html);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Cherry & Kiss");
        assert_eq!(items[0].site_id, "pornhub");
        assert_eq!(
            items[0].thumbnail.as_deref(),
            Some("https://xpomelo.com/image?key=-14")
        );
        assert_eq!(items[0].duration, Some(921));
        assert_eq!(items[0].channel.as_deref(), Some("Pornhub"));
        assert_eq!(items[1].site_id, "xpomelo");
        assert_eq!(items[1].duration, Some(3723));
    }

    #[test]
    fn low_ranges_filter_locally_and_buckets_remember_their_end() {
        assert_eq!(local_length_range(Some("10m+")), Some((600, None)));
        assert_eq!(local_length_range(Some("-10m")), Some((0, Some(600))));
        assert_eq!(local_length_range(Some("20m+")), None);
        assert!(!bucket_ended_before(
            "https://xpomelo.com/x?duration=20m",
            3
        ));
        note_bucket_end("https://xpomelo.com/x?duration=20m", 2);
        assert!(bucket_ended_before("https://xpomelo.com/x?duration=20m", 3));
        assert!(!bucket_ended_before(
            "https://xpomelo.com/x?duration=20m",
            2
        ));
    }

    #[test]
    fn crawl_slices_split_by_length_then_source() {
        let base = BrowseFilters {
            duration: Some("10m+".into()),
            sort: Some("date".into()),
            ..Default::default()
        };
        let start = crawl_start(&base);
        assert_eq!(
            start
                .iter()
                .map(|f| f.duration.clone().unwrap())
                .collect::<Vec<_>>(),
            ["10m", "20m", "30m"]
        );
        assert_eq!(start[0].sort.as_deref(), Some("date"));
        assert_eq!(crawl_start(&BrowseFilters::default()).len(), 1);
        // No length yet: split by bucket; one bucket: by free source.
        assert_eq!(crawl_split(&BrowseFilters::default()).len(), 5);
        let by_source = crawl_split(&start[0]);
        assert!(by_source
            .iter()
            .any(|f| f.source.as_deref() == Some("Pornhub")));
        assert!(!by_source
            .iter()
            .any(|f| f.source.as_deref() == Some("Brazzers")));
        assert!(!by_source
            .iter()
            .any(|f| f.source.as_deref() == Some("Beeg")));
        assert!(crawl_split(&by_source[0]).is_empty());
        assert_eq!(crawl_slice_label(&by_source[0]), "10–20 min, Pornhub");
    }

    #[test]
    fn duration_ranges_map_to_buckets() {
        assert_eq!(duration_buckets(None), Vec::<&str>::new());
        assert_eq!(duration_buckets(Some("10m")), ["10m"]);
        assert_eq!(duration_buckets(Some("10m+")), ["10m", "20m", "30m"]);
        assert_eq!(duration_buckets(Some("5m+")), ["5m", "10m", "20m", "30m"]);
        assert_eq!(duration_buckets(Some("-10m")), ["1m", "5m"]);
        assert_eq!(duration_buckets(Some("10m-30m")), ["10m", "20m"]);
        assert_eq!(duration_buckets(Some("bogus")), Vec::<&str>::new());
        // Every option the filter bar offers resolves to something.
        for o in xpomelo_options().durations {
            assert!(!duration_buckets(Some(&o.id)).is_empty(), "{}", o.id);
        }
    }

    #[test]
    fn translated_and_mirrored_duplicates_dropped() {
        let item = |url: &str, title: &str, d: u32| MediaItem {
            id: url.into(),
            title: title.into(),
            url: url.into(),
            duration: Some(d),
            ..Default::default()
        };
        let items = dedupe(merge_lists(
            vec![
                vec![
                    item(
                        "https://www.xvideos.com/video.oiplkcmcc89/minha_madrasta",
                        "My stepmother",
                        630,
                    ),
                    item(
                        "https://www.pornhub.com/view_video.php?viewkey=a",
                        "Same Upload Here",
                        900,
                    ),
                ],
                vec![
                    item(
                        "https://www.xvideos.com/video.oiplkcmcc89/my_stepmother",
                        "My stepmother",
                        630,
                    ),
                    item(
                        "https://spankbang.com/x/video/same",
                        "Same upload here!",
                        900,
                    ),
                    item(
                        "https://spankbang.com/y/video/other",
                        "Same upload here!",
                        901,
                    ),
                ],
            ],
            false,
        ));
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "xvideos.com/video.oiplkcmcc89",
                "pornhub.com/view_video.php?viewkey=a",
                "spankbang.com/y/video/other"
            ]
        );
    }

    #[test]
    fn expected_cards_per_page() {
        let card = r#"<div class="qy-mod-li " data-id="x"><a class="qy-mod-link" href="https://x/1"></a></div>"#;
        let ad = r#"<div class="qy-mod-li hilltop-ad-li"><a class="qy-mod-link" href="https://ad"></a></div>"#;
        assert_eq!(raw_card_count(&format!("{}{ad}", card.repeat(80))), 80);
        // A next-page link means a full page is due; the last page has no
        // expectation (the heading's count is approximate).
        let h = r#"<h1 class="main">Blonde Porn Videos (771)</h1>"#;
        assert_eq!(
            expected_cards(&format!(r#"{h}<a href="?page=3">"#), 2),
            Some(80)
        );
        assert_eq!(expected_cards(h, 10), None);
    }

    #[test]
    fn site_down_errors() {
        assert!(is_site_down(&AppError::Site(
            "HTTP 522 <unknown status code> for https://xpomelo.com/".into()
        )));
        assert!(is_site_down(&AppError::Site(
            "HTTP 503 Service Unavailable for x".into()
        )));
        assert!(!is_site_down(&AppError::Site(
            "HTTP 404 Not Found for x".into()
        )));
    }

    #[test]
    fn result_count_from_heading() {
        assert_eq!(
            result_count(r#"<h1 class="main cat-count-header">Blonde Porn Videos (575,432)</h1>"#),
            Some(575_432)
        );
        assert_eq!(
            result_count(r#"<h1 class="main dbgbXb">Riley Reid Porn Videos (16,639)</h1>"#),
            Some(16_639)
        );
        assert_eq!(
            result_count(r#"<h1 class="main">Search Results for "x"</h1>"#),
            None
        );
    }

    #[test]
    fn thumbnails_prefer_the_source_cdn() {
        assert_eq!(
            original_thumb(
                "/image?key=1851250585&domain=xvideos.com&id=video.x&src=https%3A%2F%2Fthumbs-gcore.xvideos-cdn.com%2Fa%2F3%2Fxv_15_t.jpg"
            )
            .as_deref(),
            Some("https://thumbs-gcore.xvideos-cdn.com/a/3/xv_15_t.jpg")
        );
        assert_eq!(original_thumb("/image?key=-999701320"), None);
        assert_eq!(original_thumb("https://cdn.example/t.jpg"), None);
    }

    #[test]
    fn tag_and_star_indexes() {
        let html = r#"<script type="text/x-json" id="data-tags">
            [{"key":"big tits","name":"big tits"},{"key":"hd porn","name":"HD porn"},{"key":"big tits","name":"dup"}]
            </script><script id="data-stars">["riley reid","abella danger"]</script>"#;
        let tags = parse_tag_index(html);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[1].slug, "hd-porn");
        assert_eq!(tags[1].name, "HD porn");
        let stars = parse_star_index(html);
        assert_eq!(stars[0].slug, "riley-reid");
        assert_eq!(stars[0].name, "Riley Reid");
        assert_eq!(stars[0].kind, BrowseKind::Model);
    }

    #[test]
    fn paysites_detected_by_host() {
        assert!(is_paysite("https://www.brazzers.com/video/1/x"));
        assert!(is_paysite("https://faphouse.com/videos/x"));
        assert!(is_paysite("https://www.clips4sale.com/studio/1/x"));
        assert!(hidden_by_default("https://www.clips4sale.com/studio/1/x"));
        assert!(hidden_by_default("https://beeg.com/video/1"));
        assert!(hidden_by_default("https://sxyprn.com/post/x.html"));
        assert!(hidden_by_default("https://www.viptube.com/video/1/x"));
        assert_eq!(source_label("VipTube"), "VipTube (unsupported)");
        assert!(!hidden_by_default("https://hoes.tube/videos/1/x/"));
        assert_eq!(source_label("Clips4sale"), "Clips4sale (paysite)");
        assert_eq!(source_label("Beeg"), "Beeg (unsupported)");
        assert_eq!(source_label("Hoes"), "Hoes");
        assert_eq!(
            crate::sites::cookie_keys::host_key("https://xpomelo.com/videos").as_deref(),
            Some(HOST_COOKIE_KEY)
        );
        assert!(!is_paysite(
            "https://www.pornhub.com/view_video.php?viewkey=brazzers.com"
        ));
        assert_eq!(
            source_site_id("https://de.xhamster.com/videos/x"),
            "xhamster"
        );
        assert_eq!(source_site_id("https://notxvideos.com/x"), "xpomelo");
    }
}
