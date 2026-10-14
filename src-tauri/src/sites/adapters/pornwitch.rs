//! Pornwitch (pornwitch.com) — a search engine over PornHub, XVIDEOS,
//! xHamster, Eporner, TXXX, beeg and many paysites. Like xPomelo it hosts
//! nothing: results keep the source site's URL (and ArcHive adapter) where
//! the id maps to one, otherwise Pornwitch's `/go?id=` redirect, which is
//! followed only when the item is played or downloaded.
//!
//! Search (`/search/{sort}?query=…`) matches titles and the source site's
//! tags. Every word is required (synonyms and plurals count), `-word`
//! excludes, so a tag combination `a,b,-c` is the query `a b -c`.
//! Filters: `freeOnly=1`, `sites=a,b`, `duration=MIN-MAX` (minutes, 60 =
//! 60+), `releaseDate=…`, `quality=…`. Gay / trans listings live under
//! `/gay/…` and `/trans/…`.
//!
//! Page 1 is the full page (36 cards plus "N results"); later pages come
//! from `/search/{sort}/partial?…&after=<cursor>`, the cursor being the last
//! card's `data-cursor`. Cursors are remembered per listing so paging on
//! asks for one page at a time.

use crate::error::{AppError, AppResult};
use crate::models::{
    split_tags, BrowseKind, BrowseOptions, BrowseOrientation, BrowsePage, BrowseQuery,
    DownloadPlan, DownloadTool, FilterOption, MediaItem, SiteCategory,
};
use crate::sites::urls::html_unescape;
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const BASE: &str = "https://pornwitch.com";
/// Cards per page (full page and `partial` alike).
const PAGE_SIZE: usize = 36;
/// `BrowseFilters::source` value for every indexed site, paysites included.
const ALL_SOURCES: &str = "all";
/// Free source sites (the site's `freeOnly=1` set).
const FREE_SOURCES: &[(&str, &str)] = &[
    ("pornhub", "PornHub"),
    ("xvideos", "XVIDEOS"),
    ("xhamster", "xHamster"),
    ("eporner", "Eporner"),
    ("txxx", "TXXX"),
    ("beeg", "beeg"),
    ("omgjav", "OMGJAV"),
    ("boyfriendtv", "BoyFriendTV (gay)"),
];

pub struct PornwitchAdapter;

#[async_trait]
impl SiteAdapter for PornwitchAdapter {
    fn id(&self) -> &str {
        "pornwitch"
    }

    fn display_name(&self) -> &str {
        "Pornwitch"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Category, BrowseKind::Search]
    }

    fn browse_options(&self) -> BrowseOptions {
        pornwitch_options()
    }

    /// The site's fixed category list (each one is the same as searching
    /// its slug, so categories combine like any other words).
    async fn list_categories(&self, ctx: &SiteContext) -> AppResult<Vec<SiteCategory>> {
        let html = ctx
            .fetch_html(&format!("{BASE}/categories"), self.id())
            .await?;
        Ok(parse_categories(&html))
    }

    async fn suggest(
        &self,
        ctx: &SiteContext,
        text: &str,
        orientation: Option<BrowseOrientation>,
    ) -> AppResult<Vec<String>> {
        let text = text.trim().to_lowercase();
        if text.chars().count() < 2 {
            return Ok(Vec::new());
        }
        let key = format!("{}|{text}", orientation_path(orientation));
        if let Some(hit) = SUGGESTIONS.lock().ok().and_then(|c| c.get(&key).cloned()) {
            return Ok(hit);
        }
        let url = format!(
            "{BASE}{}/suggest?q={}",
            orientation_path(orientation),
            enc(&text)
        );
        let raw = ctx
            .fetch_with_headers(&url, self.id(), Some(&[("Accept", "application/json")]))
            .await?;
        let words = parse_suggestions(&raw);
        if let Ok(mut cache) = SUGGESTIONS.lock() {
            if cache.len() > 2000 {
                cache.clear();
            }
            cache.insert(key, words.clone());
        }
        Ok(words)
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let first = listing_url(&query, None);
        if query.text().is_empty() {
            return Err(AppError::Site(
                "Pornwitch needs search words or tags".into(),
            ));
        }
        let page = query.page.max(1);
        // Cursor of the card before this page: walk forward from the last
        // page we know (normally just the previous one).
        let mut after = None;
        if page > 1 {
            loop {
                let known = cursors(&first);
                if let Some(c) = known.get(page as usize - 2) {
                    after = Some(c.clone());
                    break;
                }
                let next = known.len() as u32 + 1;
                let url = match known.last() {
                    Some(c) => listing_url(&query, Some(c)),
                    None => first.clone(),
                };
                let html = ctx.fetch_html(&url, self.id()).await?;
                let (items, last) = parse_listing(&html);
                match last {
                    Some(c) if items.len() >= PAGE_SIZE => note_cursor(&first, next, c),
                    // The listing ended before the requested page.
                    _ => {
                        return Ok(BrowsePage {
                            page,
                            ..Default::default()
                        })
                    }
                }
            }
        }
        let url = match &after {
            Some(c) => listing_url(&query, Some(c)),
            None => first.clone(),
        };
        let html = ctx.fetch_html(&url, self.id()).await?;
        let (items, last) = parse_listing(&html);
        let raw = items.len();
        if let Some(c) = last {
            note_cursor(&first, page, c);
        }
        Ok(BrowsePage {
            items,
            page,
            has_more: raw >= PAGE_SIZE,
            total: if page == 1 { result_count(&html) } else { None },
            ..Default::default()
        })
    }

    /// Items carry the source URL where Pornwitch's id maps to one; the
    /// others are `/go` links, followed here (one request) to the source.
    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        let url = follow_go(&item.url).await?;
        Ok(DownloadPlan {
            url,
            // Placeholder; jobs use the Settings naming template.
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
        let url = follow_go(url).await?;
        crate::sites::adapters::custom::CustomUrlAdapter
            .resolve_stream_url(ctx, &url)
            .await
    }
}

fn pornwitch_options() -> BrowseOptions {
    let o = |id: &str, label: &str| FilterOption::new(id, label);
    BrowseOptions {
        sorts: vec![
            o("hottest", "Hottest"),
            o("newest", "Newest"),
            o("oldest", "Oldest"),
            o("longest", "Longest"),
            o("shortest", "Shortest"),
        ],
        periods: vec![
            o("24hours", "Past 24 hours"),
            o("2days", "Past 2 days"),
            o("week", "Past week"),
            o("3months", "Past 3 months"),
            o("year", "Past year"),
        ],
        durations: vec![
            o("0-10", "Under 10 min"),
            o("5-60", "5+ min"),
            o("10-60", "10+ min"),
            o("10-20", "10–20 min"),
            o("20-60", "20+ min"),
            o("20-30", "20–30 min"),
            o("30-60", "30+ min"),
            o("40-60", "40+ min"),
            o("60-60", "60+ min"),
        ],
        qualities: vec![o("720", "720p+"), o("1080", "1080p+"), o("4K", "4K")],
        sources: std::iter::once(o(ALL_SOURCES, "All (incl. paysites)"))
            .chain(FREE_SOURCES.iter().map(|(id, label)| o(id, label)))
            .collect(),
        filter_kinds: vec![BrowseKind::Category, BrowseKind::Search],
        has_categories: true,
        // Words are strictly ANDed, so any number of tags combine exactly.
        combine_tags: 20,
        include_exclude: true,
        remote_suggest: true,
        ..Default::default()
    }
}

/// Suggestions already asked for (`orientation|text` → phrases).
static SUGGESTIONS: LazyLock<Mutex<HashMap<String, Vec<String>>>> = LazyLock::new(Default::default);

/// Page cursors per listing (page-1 URL → cursor after page 1, 2, …).
static CURSORS: LazyLock<Mutex<HashMap<String, Vec<String>>>> = LazyLock::new(Default::default);

fn cursors(listing: &str) -> Vec<String> {
    CURSORS
        .lock()
        .ok()
        .and_then(|c| c.get(listing).cloned())
        .unwrap_or_default()
}

/// Remember the cursor after `page` (pages already known beyond it are
/// dropped when page 1 is fetched again: the listing may have changed).
fn note_cursor(listing: &str, page: u32, cursor: String) {
    let Ok(mut all) = CURSORS.lock() else { return };
    if all.len() > 200 && !all.contains_key(listing) {
        all.clear();
    }
    let list = all.entry(listing.to_string()).or_default();
    let i = page as usize - 1;
    if page == 1 {
        list.clear();
    }
    if i < list.len() {
        list[i] = cursor;
    } else if i == list.len() {
        list.push(cursor);
    }
}

trait QueryText {
    fn text(&self) -> String;
}

impl QueryText for BrowseQuery {
    /// Search words: a search's text as typed, or a tag combination
    /// (`a,b,-c`) plus the include / exclude filters as `a b -c`.
    fn text(&self) -> String {
        let mut words: Vec<String> = Vec::new();
        if self.kind == BrowseKind::Search {
            let t = self.slug.trim();
            if !t.is_empty() {
                words.push(t.to_string());
            }
        } else {
            words.extend(split_tags(&self.slug).into_iter().map(|t| tag_word(&t)));
        }
        let f = &self.filters;
        for t in f.include.as_deref().map(split_tags).unwrap_or_default() {
            words.push(tag_word(&t));
        }
        for t in f.exclude.as_deref().map(split_tags).unwrap_or_default() {
            words.push(tag_word(&format!("-{}", t.trim_start_matches('-'))));
        }
        words.retain(|w| !w.is_empty() && w != "-");
        words.join(" ")
    }
}

/// A tag as query words: an excluded multi-word tag excludes each word.
fn tag_word(tag: &str) -> String {
    let tag = tag.trim().to_lowercase();
    match tag.strip_prefix('-') {
        Some(rest) => rest
            .split_whitespace()
            .map(|w| format!("-{w}"))
            .collect::<Vec<_>>()
            .join(" "),
        None => tag,
    }
}

fn orientation_path(o: Option<BrowseOrientation>) -> &'static str {
    match o {
        Some(BrowseOrientation::Gay) => "/gay",
        Some(BrowseOrientation::Transgender) => "/trans",
        _ => "",
    }
}

/// Page-1 URL of a listing, or with `after` its `partial` continuation.
fn listing_url(query: &BrowseQuery, after: Option<&str>) -> String {
    let f = &query.filters;
    let sort = f.sort.as_deref().unwrap_or("hottest");
    let mut params: Vec<(&str, String)> = Vec::new();
    match f.source.as_deref() {
        Some(ALL_SOURCES) => {}
        Some(site) => {
            params.push(("freeOnly", "1".into()));
            params.push(("sites", site.to_string()));
        }
        None => params.push(("freeOnly", "1".into())),
    }
    if let Some(d) = &f.duration {
        params.push(("duration", d.clone()));
    }
    if let Some(p) = &f.period {
        params.push(("releaseDate", p.clone()));
    }
    if let Some(q) = &f.quality {
        params.push(("quality", q.clone()));
    }
    params.push(("query", query.text()));
    if let Some(a) = after {
        params.push(("after", a.to_string()));
    }
    let qs = params
        .iter()
        .map(|(k, v)| format!("{k}={}", enc(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!(
        "{BASE}{}/search/{sort}{}?{qs}",
        orientation_path(query.orientation),
        if after.is_some() { "/partial" } else { "" }
    )
}

/// Source URL for a Pornwitch id (`site` + `id` from a card's cursor) and
/// the ArcHive adapter for it; `None` keeps the `/go` link.
fn source_url(site: &str, id: &str) -> Option<(String, &'static str)> {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric());
    if !ok {
        return None;
    }
    Some(match site {
        "pornhub" => (
            format!("https://www.pornhub.com/view_video.php?viewkey={id}"),
            "pornhub",
        ),
        "xvideos" => (format!("https://www.xvideos.com/video.{id}/x"), "xvideos"),
        "xhamster" => (
            format!("https://xhamster.com/videos/video-{id}"),
            "xhamster",
        ),
        "eporner" => (format!("https://www.eporner.com/video-{id}/"), "eporner"),
        _ => return None,
    })
}

/// Percent-encode a query value (spaces as `%20`, like the site's own links).
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn go_url(site: &str, id: &str) -> String {
    format!("{BASE}/go?id={}", enc(&format!("{site}{id}")))
}

fn is_go_url(url: &str) -> bool {
    url.starts_with(&format!("{BASE}/go?"))
}

/// Where a `/go` link points (other URLs are returned unchanged).
async fn follow_go(url: &str) -> AppResult<String> {
    if !is_go_url(url) {
        return Ok(url.to_string());
    }
    static NO_REDIRECT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .user_agent(crate::sites::BROWSER_UA)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default()
    });
    crate::sites::throttle::acquire(url).await?;
    let resp = NO_REDIRECT.get(url).send().await?;
    let target = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .filter(|l| l.starts_with("http"))
        .ok_or_else(|| {
            AppError::Site(format!(
                "Pornwitch did not redirect to the video (HTTP {})",
                resp.status()
            ))
        })?;
    Ok(target.to_string())
}

/// "1,356 results" on page 1.
fn result_count(html: &str) -> Option<u32> {
    static COUNT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#">\s*([\d,]+) results?\s*<"#).expect("count"));
    COUNT
        .captures(html)
        .and_then(|c| c[1].replace(',', "").parse().ok())
}

/// Cards of a listing page plus the last card's cursor (for the next page).
fn parse_listing(html: &str) -> (Vec<MediaItem>, Option<String>) {
    static CURSOR: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"data-cursor="([^"]+)""#).expect("cursor"));
    static IMG: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"<img src="([^"]*)" alt="([^"]*)""#).expect("img"));
    static DUR: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"bottom-2[^"]*">\s*([\d:]+)\s*<"#).expect("duration"));
    static SITE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"href="/sites/[^"]*"[^>]*>([^<]+)<"#).expect("site"));
    let mut last = None;
    let mut seen = std::collections::HashSet::new();
    let items = html
        .split("<pwx-video-card")
        .skip(1)
        .filter_map(|card| {
            let card = card.split("</pwx-video-card>").next().unwrap_or(card);
            let cursor = html_unescape(CURSOR.captures(card)?.get(1)?.as_str());
            last = Some(cursor.clone());
            // `…,<id>,<site>`: the leading sort keys differ per sort
            // (newest: timestamp; hottest: score, popularity).
            let mut parts = cursor.rsplit(',');
            let (site, id) = (parts.next()?, parts.next()?);
            let (url, site_id) = match source_url(site, id) {
                Some((u, s)) => (u, s),
                None => (go_url(site, id), "pornwitch"),
            };
            if !seen.insert(url.clone()) {
                return None;
            }
            let img = IMG.captures(card);
            let title = img
                .as_ref()
                .map(|c| html_unescape(&c[2]).trim().to_string())
                .filter(|t| !t.is_empty())?;
            let thumbnail = img
                .as_ref()
                .map(|c| html_unescape(&c[1]))
                .filter(|s| s.starts_with("http"));
            let duration = DUR.captures(card).and_then(|c| {
                c[1].split(':')
                    .try_fold(0u32, |acc, p| Some(acc * 60 + p.parse::<u32>().ok()?))
            });
            let channel = SITE
                .captures(card)
                .map(|c| html_unescape(c[1].trim()))
                .filter(|s| !s.is_empty());
            Some(MediaItem {
                id: url.clone(),
                title,
                url,
                thumbnail,
                duration,
                site_id: site_id.to_string(),
                channel,
                ..Default::default()
            })
        })
        .collect::<Vec<_>>();
    (items, last)
}

/// `/categories` tiles (the letter index links carry no image).
fn parse_categories(html: &str) -> Vec<SiteCategory> {
    static TILE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?s)<a[^>]*href="/categories/([a-z0-9-]+)"[^>]*>(.*?)</a>"#)
            .expect("tile")
    });
    static IMG: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"<img src="([^"]+)""#).expect("img"));
    static TAGS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<[^>]+>").expect("tags"));
    let mut seen = std::collections::HashSet::new();
    TILE.captures_iter(html)
        .filter_map(|c| {
            let slug = c[1].to_string();
            let inner = &c[2];
            let img = IMG.captures(inner)?;
            let name = html_unescape(TAGS.replace_all(inner, "").trim());
            if name.is_empty() || !seen.insert(slug.clone()) {
                return None;
            }
            let src = html_unescape(&img[1]);
            Some(SiteCategory {
                name,
                slug,
                kind: BrowseKind::Category,
                thumbnail: Some(if src.starts_with('/') {
                    format!("{BASE}{src}")
                } else {
                    src
                }),
                count: None,
            })
        })
        .collect()
}

/// `/suggest` answers a JSON list of phrases (strings, or objects with one).
fn parse_suggestions(raw: &str) -> Vec<String> {
    fn collect(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => out.push(s.trim().to_string()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
            serde_json::Value::Object(o) => {
                for key in ["suggestions", "results", "items"] {
                    if let Some(x) = o.get(key) {
                        return collect(x, out);
                    }
                }
                for key in ["text", "value", "query", "q", "name", "label"] {
                    if let Some(serde_json::Value::String(s)) = o.get(key) {
                        out.push(s.trim().to_string());
                        return;
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        collect(&v, &mut out);
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|s| !s.is_empty() && seen.insert(s.to_lowercase()));
    out.truncate(10);
    out
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
    fn tags_become_required_and_excluded_words() {
        let query = q(
            BrowseKind::Category,
            "prostate-massage,Blowjob,-solo",
            1,
            BrowseFilters {
                include: Some("pov".into()),
                exclude: Some("gay,big tits".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            query.text(),
            "prostate-massage blowjob -solo pov -gay -big -tits"
        );
        let search = q(
            BrowseKind::Search,
            " \"red hair\" -anal ",
            1,
            Default::default(),
        );
        assert_eq!(search.text(), "\"red hair\" -anal");
    }

    #[test]
    fn listing_and_partial_urls() {
        let mut query = q(
            BrowseKind::Category,
            "blowjob,-solo",
            1,
            BrowseFilters {
                sort: Some("newest".into()),
                duration: Some("10-60".into()),
                period: Some("year".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            listing_url(&query, None),
            "https://pornwitch.com/search/newest?freeOnly=1&duration=10-60&releaseDate=year&query=blowjob%20-solo"
        );
        query.orientation = Some(BrowseOrientation::Gay);
        query.filters.source = Some("pornhub".into());
        assert_eq!(
            listing_url(&query, Some("1791604050000,21809581,txxx")),
            "https://pornwitch.com/gay/search/newest/partial?freeOnly=1&sites=pornhub&duration=10-60&releaseDate=year&query=blowjob%20-solo&after=1791604050000%2C21809581%2Ctxxx"
        );
        query.filters.source = Some(ALL_SOURCES.into());
        query.filters.sort = None;
        assert!(listing_url(&query, None)
            .starts_with("https://pornwitch.com/gay/search/hottest?duration="));
    }

    #[test]
    fn cards_map_to_source_sites() {
        let html = r#"<div>1,356 results</div>
<pwx-video-card class="mb-6" data-cursor="1791604050000,21809581,txxx" data-timeline-src="x.mp4"><a href="/go?id=txxx21809581"><img src="https://tn.txxx.tube/1.jpg" alt="prostate massage &amp; more" loading="lazy"><div class="pointer-events-none absolute right-3 bottom-2 rounded-md">24:51</div></a><a href="/sites/txxx" class="hover:text-base-content">TXXX</a></pwx-video-card>
<pwx-video-card data-cursor="1791351939000,xhFrd4t,xhamster"><img src="https://ic.xhcdn.com/a.webp" alt="Brenna"><div class="absolute right-3 bottom-2 x">1:10:12</div><a href="/sites/xhamster">xHamster</a></pwx-video-card>
<pwx-video-card data-cursor="1.01,7549,660ee3061c342,pornhub"><img src="https://ei.phncdn.com/a.jpg" alt="PH"></pwx-video-card>"#;
        let (items, last) = parse_listing(html);
        assert_eq!(items.len(), 3);
        assert_eq!(last.as_deref(), Some("1.01,7549,660ee3061c342,pornhub"));
        assert_eq!(items[0].url, "https://pornwitch.com/go?id=txxx21809581");
        assert_eq!(items[0].site_id, "pornwitch");
        assert_eq!(items[0].title, "prostate massage & more");
        assert_eq!(items[0].duration, Some(24 * 60 + 51));
        assert_eq!(items[0].channel.as_deref(), Some("TXXX"));
        assert_eq!(items[1].url, "https://xhamster.com/videos/video-xhFrd4t");
        assert_eq!(items[1].site_id, "xhamster");
        assert_eq!(items[1].duration, Some(3600 + 10 * 60 + 12));
        assert_eq!(
            items[2].url,
            "https://www.pornhub.com/view_video.php?viewkey=660ee3061c342"
        );
        assert_eq!(result_count(html), Some(1356));
    }

    #[test]
    fn cursors_follow_pages() {
        let l = "test-listing";
        note_cursor(l, 1, "a".into());
        note_cursor(l, 2, "b".into());
        note_cursor(l, 4, "skip".into());
        assert_eq!(cursors(l), vec!["a", "b"]);
        note_cursor(l, 1, "a2".into());
        assert_eq!(cursors(l), vec!["a2"]);
    }

    #[test]
    fn categories_and_suggestions() {
        let html = r#"<a href="/categories/a" class="x">a</a>
<a href="/categories/close-up" class="y"><img src="/categories/straight/close-up.jpg" alt="Close Up Porn Category"><span>Close Up</span></a>
<a href="/categories/close-up"><img src="/x.jpg"><span>Close Up</span></a>"#;
        let cats = parse_categories(html);
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].slug, "close-up");
        assert_eq!(cats[0].name, "Close Up");
        assert_eq!(
            cats[0].thumbnail.as_deref(),
            Some("https://pornwitch.com/categories/straight/close-up.jpg")
        );
        assert_eq!(
            parse_suggestions(r#"["prostate massage","Prostate Massage","prostate milking"]"#),
            vec!["prostate massage", "prostate milking"]
        );
        assert_eq!(
            parse_suggestions(r#"{"suggestions":[{"text":"a b"}]}"#),
            vec!["a b"]
        );
    }

    #[test]
    fn fixture_pages_parse() {
        // Saved pages (not checked in): PORNWITCH_FIXTURES=<dir with
        // search.html and categories.html>.
        let Some(dir) = std::env::var_os("PORNWITCH_FIXTURES").map(std::path::PathBuf::from) else {
            return;
        };
        let Ok(html) = std::fs::read_to_string(dir.join("search.html")) else {
            return;
        };
        let (items, last) = parse_listing(&html);
        assert_eq!(items.len(), PAGE_SIZE);
        assert!(last.is_some());
        assert!(items
            .iter()
            .all(|i| i.duration.is_some() && i.thumbnail.is_some()));
        assert!(result_count(&html).is_some());
        if let Ok(html) = std::fs::read_to_string(dir.join("categories.html")) {
            let cats = parse_categories(&html);
            assert!(cats.len() > 100, "{}", cats.len());
        }
    }
}
