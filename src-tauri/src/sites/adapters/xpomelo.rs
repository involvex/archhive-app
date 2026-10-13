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
    BrowseKind, BrowseOptions, BrowseOrientation, BrowsePage, BrowseQuery, DownloadPlan,
    DownloadTool, FilterOption, MediaItem, SiteCategory,
};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use std::sync::LazyLock;

const BASE: &str = "https://xpomelo.com";
/// Tags the site's own editor shows at most; the index runs to ~6k.
const POPULAR_TAGS: usize = 600;
/// `BrowseFilters::source` id that keeps paysite results.
const ALL_SOURCES: &str = "all";

/// Result hosts that only sell the video (trailers / store pages) — hidden
/// unless the Source filter says otherwise.
const PAYSITE_HOSTS: &[&str] = &[
    "faphouse.com",
    "clips4sale.com",
    "brazzers.com",
    "realitykings.com",
    "puretaboo.com",
    "analvids.com",
];

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

    /// The most popular tags (the index is ordered by popularity).
    async fn list_categories(&self, ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
        Ok(tag_index(ctx)
            .await?
            .into_iter()
            .take(POPULAR_TAGS)
            .collect())
    }

    /// "stars" = pornstar index; a…z / "0" = every tag starting with it.
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
        let url = listing_url(&query);
        tracing::info!("[xpomelo] browse: {url}");
        let html = fetch(ctx, &url, query.orientation).await?;
        let cards = parse_listing(&html);
        let has_more = cards.len() >= 40 || html.contains(&format!("page={}", query.page + 1));
        let keep_paysites = query.filters.source.is_some();
        let items = cards
            .into_iter()
            .filter(|i| keep_paysites || !is_paysite(&i.url))
            .map(|mut i| {
                // xPomelo's own `/image?key=` thumbnails sit behind the same
                // Cloudflare rules; on the phone fetch them via the app.
                if let Some(t) = i.thumbnail.as_deref().filter(|t| t.starts_with(BASE)) {
                    i.thumbnail = Some(crate::sites::adapters::pornhub::maybe_loopback_proxy(
                        ctx,
                        t,
                        "https://xpomelo.com/",
                    ));
                }
                i
            })
            .collect();
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
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

fn xpomelo_options() -> BrowseOptions {
    let o = |id: &str, label: &str| FilterOption::new(id, label);
    BrowseOptions {
        sorts: vec![
            o("date", "Newest"),
            o("views", "Most viewed"),
            o("duration", "Longest"),
        ],
        periods: vec![o("30", "Past month"), o("365", "Past year")],
        durations: vec![
            o("1m", "1+ min"),
            o("5m", "5+ min"),
            o("10m", "10+ min"),
            o("20m", "20+ min"),
            o("30m", "30+ min"),
        ],
        qualities: vec![o("hd", "HD"), o("4k", "4K")],
        sources: std::iter::once(o(ALL_SOURCES, "All (incl. paysites)"))
            .chain(SOURCES.iter().map(|s| o(s, s)))
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
        ..Default::default()
    }
}

async fn fetch(
    ctx: &SiteContext,
    url: &str,
    orientation: Option<BrowseOrientation>,
) -> AppResult<String> {
    let cookie = match orientation {
        Some(BrowseOrientation::Gay) => Some("orientation=gay"),
        Some(BrowseOrientation::Transgender) => Some("orientation=trans"),
        _ => None,
    };
    let mut headers = vec![
        (
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
        ("Accept-Language", "en-US,en;q=0.9"),
    ];
    if let Some(c) = cookie {
        headers.push(("Cookie", c));
    }
    ctx.fetch_with_headers(url, "xpomelo", Some(&headers))
        .await
        .map_err(|e| match e {
            AppError::Site(msg) if msg.contains("HTTP 403") => AppError::Site(
                "xPomelo's Cloudflare protection refused the request (HTTP 403); \
                 try again in a minute"
                    .to_string(),
            ),
            other => other,
        })
}

/// Tag index (`<script id="data-tags">[{"key":"big tits","name":"big tits"},…]`
/// on every category page), most popular first. Cached for the session: the
/// page is ~1.3 MB.
async fn tag_index(ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
    use std::time::{Duration, Instant};
    type Cached = Option<(Instant, Vec<SiteCategory>)>;
    static CACHE: LazyLock<std::sync::Mutex<Cached>> = LazyLock::new(Default::default);
    if let Some((at, tags)) = CACHE.lock().ok().and_then(|c| c.clone()) {
        if at.elapsed() < Duration::from_secs(6 * 3600) {
            return Ok(tags);
        }
    }
    let html = fetch(ctx, &format!("{BASE}/category"), None).await?;
    let tags = parse_tag_index(&html);
    if tags.is_empty() {
        return Err(AppError::Site(
            "xPomelo category index not found on the page".to_string(),
        ));
    }
    if let Ok(mut c) = CACHE.lock() {
        *c = Some((Instant::now(), tags.clone()));
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
            (
                "duration",
                &pick(&f.duration, &["1m", "5m", "10m", "20m", "30m"]),
            ),
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
                if src.starts_with('/') {
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
