//! Video-page metadata for the browse Info dialog, scraped from the site's own
//! HTML. yt-dlp's `-J` output is thin for most tube sites (XVIDEOS / XNXX give
//! no tags, models or channel), slow on phones, and returns hotlink-protected
//! thumbnails for some (ThotHub) — so the sites below are read directly.

use scraper::{ElementRef, Html, Selector};

/// Sites with a page scraper; others use yt-dlp metadata.
pub const SCRAPED_SITES: &[&str] = &["xvideos", "xnxx", "pornhub", "thothub", "thisvid"];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PageDetails {
    pub title: Option<String>,
    pub description: Option<String>,
    pub channel: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<u32>,
    pub performers: Vec<String>,
    /// Categories first, then tags (deduplicated, case-insensitive).
    pub tags: Vec<String>,
}

impl PageDetails {
    /// Found anything beyond what a listing card already has.
    pub fn is_useful(&self) -> bool {
        !self.tags.is_empty()
            || !self.performers.is_empty()
            || self.channel.is_some()
            || self.description.is_some()
    }
}

pub fn scrape(site_id: &str, html: &str) -> PageDetails {
    let doc = Html::parse_document(html);
    let mut d = PageDetails {
        title: meta(&doc, "og:title"),
        description: meta(&doc, "og:description"),
        thumbnail: meta(&doc, "og:image"),
        duration: meta(&doc, "og:duration")
            .and_then(|v| v.parse().ok())
            .or_else(|| ld_json_duration(html)),
        ..Default::default()
    };
    let mut tags = Vec::new();
    match site_id {
        "xvideos" => {
            d.channel = first_text(&doc, "li.main-uploader a .name");
            d.performers = texts(&doc, ".video-metadata a.is-pornstar .name");
            if d.performers.is_empty() {
                d.performers = texts(&doc, ".video-metadata a.is-pornstar");
            }
            tags = texts(&doc, ".video-metadata a.is-keyword")
                .into_iter()
                .map(|t| t.replace('-', " "))
                .collect();
            // og:description is "XVIDEOS <title> free" — no information.
            d.description = None;
        }
        "xnxx" => {
            d.channel = first_text(&doc, ".metadata a.gold-plate");
            d.performers = texts(&doc, ".video-tags a.is-pornstar");
            tags = texts(&doc, ".video-tags a.is-keyword");
            d.description = None;
        }
        "pornhub" => {
            // Desktop and mobile layouts differ (mobile: the swipeable pill
            // row above the title, picked by a `platform=mobile` cookie from
            // a phone browser login); both mark the links the same way.
            d.channel = first_text(&doc, ".video-detailed-info .usernameWrap a")
                .or_else(|| first_text(&doc, ".videoUploaderBlock .usernameWrap a"))
                .or_else(|| first_text(&doc, ".userInfoContainer .usernameWrapper a"));
            d.performers = texts(&doc, ".pornstarsWrapper a.pstar-list-btn");
            d.performers.extend(texts(
                &doc,
                r#"a[data-event="video_underplayer"][href^="/pornstar/"]"#,
            ));
            tags = texts(
                &doc,
                r#"a[data-event="video_underplayer"][data-label="category"]"#,
            );
            tags.extend(texts(
                &doc,
                r#"a[data-event="video_underplayer"][data-label="tag"]"#,
            ));
            if tags.is_empty() {
                tags = texts(&doc, ".categoriesWrapper a.item");
                tags.extend(texts(&doc, ".tagsWrapper a.item"));
            }
            // og:description is site boilerplate ("Watch … on Pornhub.com …").
            d.description = None;
        }
        "thothub" | "thisvid" => {
            tags = labelled_links(&doc, "Categories");
            tags.extend(labelled_links(&doc, "Tags"));
            d.performers = labelled_links(&doc, "Models");
            d.channel = labelled_links(&doc, "Uploaded by")
                .into_iter()
                .next()
                .or_else(|| labelled_links(&doc, "Added by").into_iter().next());
        }
        _ => {}
    }
    d.tags = dedupe(tags);
    d.performers = dedupe(std::mem::take(&mut d.performers));
    if d.description
        .as_deref()
        .is_some_and(|desc| d.title.as_deref() == Some(desc))
    {
        d.description = None;
    }
    d
}

/// Any tube site without its own scraper: structured data first (KVS player
/// config, JSON-LD, `video:tag` meta), then tag / category / model / channel
/// links on the page outside menus and "related" blocks, and meta keywords
/// only when nothing else turned up.
pub fn scrape_generic(html: &str) -> PageDetails {
    let mut d = scrape("", html);
    let doc = Html::parse_document(html);
    let mut tags = Vec::new();
    let mut performers = Vec::new();

    // KVS (ThisVid, Hoes, Analdin, Xozilla, 4KPorn, …): flashvars lists;
    // XGroovy's pageContext uses `_title` / `_slug` variants.
    for key in ["video_categories", "video_tags"] {
        let mut found = kvs_list(html, key);
        if found.is_empty() {
            found = kvs_list(html, &format!("{key}_title"));
        }
        if found.is_empty() {
            found = kvs_list(html, &format!("{key}_slug"))
                .into_iter()
                .map(|t| t.replace('-', " "))
                .collect();
        }
        tags.extend(found);
    }
    performers.extend(kvs_list(html, "video_models"));

    // JSON-LD VideoObject.
    for script in doc.select(&sel(r#"script[type="application/ld+json"]"#)) {
        let raw: String = script.text().collect();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw.trim()) {
            ld_collect(&v, &mut tags, &mut performers, 0);
        }
    }
    for prop in ["video:tag", "og:video:tag", "article:tag"] {
        let s = sel(&format!(r#"meta[property="{prop}"], meta[name="{prop}"]"#));
        tags.extend(
            doc.select(&s)
                .filter_map(|m| m.value().attr("content"))
                .map(clean),
        );
    }

    // Page links only when the page had no structured lists (they'd add
    // the same tags again, plus whatever index blocks slip through).
    let links = page_links(&doc);
    if tags.is_empty() {
        tags.extend(labelled_links(&doc, "Categories"));
        tags.extend(labelled_links(&doc, "Tags"));
    }
    if tags.is_empty() {
        tags.extend(links.categories);
        tags.extend(links.tags);
    }
    if performers.is_empty() {
        performers.extend(labelled_links(&doc, "Pornstars"));
        performers.extend(labelled_links(&doc, "Models"));
    }
    if performers.is_empty() {
        performers.extend(links.models);
    }
    if d.channel.is_none() {
        d.channel = links.channels.into_iter().next();
    }

    if tags.is_empty() {
        if let Some(kw) = meta(&doc, "keywords") {
            tags.extend(
                kw.split(',')
                    .map(|t| {
                        // "big tits porn" → "big tits" (HQPorner and the like).
                        let t = t.trim();
                        t.strip_suffix(" porn").unwrap_or(t).to_string()
                    })
                    .filter(|t| !t.is_empty() && t.len() <= 40 && !is_junk_keyword(t)),
            );
        }
    }
    d.tags = dedupe(tags).into_iter().take(60).collect();
    d.performers = dedupe(performers).into_iter().take(20).collect();
    d
}

/// `video_tags: 'Amateur, Big Tits, …'` in a KVS flashvars block.
fn kvs_list(html: &str, key: &str) -> Vec<String> {
    let re = match regex::Regex::new(&format!(r#"{key}\s*:\s*['"]([^'"]*)['"]"#)) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    re.captures(html)
        .map(|c| {
            c[1].split(',')
                .map(|t| clean(t.trim()))
                .filter(|t| !t.is_empty() && t.len() <= 60)
                .collect()
        })
        .unwrap_or_default()
}

/// `keywords` / `genre` / `actor` of JSON-LD objects (also inside `@graph`).
fn ld_collect(
    v: &serde_json::Value,
    tags: &mut Vec<String>,
    performers: &mut Vec<String>,
    depth: u8,
) {
    use serde_json::Value;
    if depth > 4 {
        return;
    }
    let strings = |v: &Value| -> Vec<String> {
        match v {
            Value::String(s) => s.split(',').map(|t| clean(t.trim())).collect(),
            Value::Array(a) => a
                .iter()
                .filter_map(|x| x.as_str().map(|s| clean(s.trim())))
                .collect(),
            _ => Vec::new(),
        }
    };
    match v {
        Value::Array(a) => a
            .iter()
            .for_each(|x| ld_collect(x, tags, performers, depth + 1)),
        Value::Object(o) => {
            for key in ["keywords", "genre"] {
                if let Some(k) = o.get(key) {
                    tags.extend(strings(k).into_iter().filter(|t| t.len() <= 60));
                }
            }
            for key in ["actor", "actors", "performer"] {
                let names = match o.get(key) {
                    Some(Value::Array(a)) => a
                        .iter()
                        .filter_map(|x| x.get("name").and_then(Value::as_str).map(clean))
                        .collect(),
                    Some(x) => x
                        .get("name")
                        .and_then(Value::as_str)
                        .map(clean)
                        .into_iter()
                        .collect::<Vec<_>>(),
                    None => Vec::new(),
                };
                performers.extend(names);
            }
            if let Some(g) = o.get("@graph") {
                ld_collect(g, tags, performers, depth + 1);
            }
        }
        _ => {}
    }
}

#[derive(Default)]
struct PageLinks {
    tags: Vec<String>,
    categories: Vec<String>,
    models: Vec<String>,
    channels: Vec<String>,
}

/// Path segment that marks a link's kind: `/tags/x`, `/categories/x`,
/// `/pornstar/x`, `/channels/x`, … (followed by a slug).
fn link_kind(href: &str) -> Option<(u8, String)> {
    let path = href
        .split("://")
        .nth(1)
        .map(|r| r.split_once('/').map(|(_, p)| p).unwrap_or(""))
        .unwrap_or(href.trim_start_matches('/'));
    let path = path.split(['?', '#']).next().unwrap_or("");
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for (i, seg) in segs.iter().enumerate() {
        let kind = match seg.to_ascii_lowercase().as_str() {
            "tag" | "tags" | "keyword" | "keywords" => 0,
            "category" | "categories" | "cat" => 1,
            "pornstar" | "pornstars" | "model" | "models" | "star" | "stars" | "actor"
            | "actors" | "performer" | "performers" => 2,
            "channel" | "channels" | "studio" | "studios" => 3,
            _ => continue,
        };
        let slug = segs.get(i + 1)?;
        if slug.is_empty() || slug.chars().all(|c| c.is_ascii_digit()) && segs.len() == i + 2 {
            // "/categories/2/" is a listing page number, not a category.
            return None;
        }
        return Some((kind, slug.to_string()));
    }
    None
}

/// Links inside these are site-wide navigation, not the video's own tags.
/// Walking up from the link, the nearest marker wins: a "tags" / "categories"
/// block is the video's own (xHamster even wraps it in `<nav
/// id="video-tags-list">`), a menu / dropdown / footer is not.
fn in_navigation(el: ElementRef<'_>) -> bool {
    const NAV: &[&str] = &[
        "menu",
        "nav",
        "dropdown",
        "drop_",
        "drop-",
        "footer",
        "header",
        "sidebar",
        "related",
        "recommend",
        "popular",
        "trending",
        "breadcrumb",
        "filter",
        "sort",
        "pagination",
        "suggest",
        "top-",
        "mobile-",
        "lang",
    ];
    const CONTENT: &[&str] = &[
        "tag",
        "categor",
        "pornstar",
        "model",
        "video-info",
        "video_info",
    ];
    for a in el.ancestors().take(10).filter_map(ElementRef::wrap) {
        let v = a.value();
        let attrs = format!(
            "{} {}",
            v.attr("class").unwrap_or(""),
            v.attr("id").unwrap_or("")
        )
        .to_ascii_lowercase();
        if CONTENT.iter().any(|w| attrs.contains(w)) && !attrs.contains("menu") {
            return false;
        }
        if matches!(v.name(), "nav" | "header" | "footer") || NAV.iter().any(|w| attrs.contains(w))
        {
            return true;
        }
    }
    false
}

fn page_links(doc: &Html) -> PageLinks {
    use std::collections::HashMap;
    let count_re = regex::Regex::new(r"\s*\(?[\d.,]+\s*[KkMm]?\)?$").expect("count");
    // Group by the nearest ancestor with a class / id: a block of dozens of
    // links is a category index or menu, not this video's tags.
    let mut groups: HashMap<_, Vec<(u8, String)>> = HashMap::new();
    for a in doc.select(&sel("a[href]")) {
        let Some((kind, slug)) = a.value().attr("href").and_then(link_kind) else {
            continue;
        };
        if in_navigation(a) {
            continue;
        }
        let mut text = element_text(a);
        text = count_re.replace(&text, "").trim().to_string();
        if text.is_empty() || text.len() > 50 || text.starts_with('#') && text.len() == 1 {
            text = urlencoding_decode(&slug).replace(['-', '_', '+'], " ");
        }
        let text = text.trim_start_matches('#').trim();
        let text = text
            .strip_prefix("Submitted by ")
            .or_else(|| text.strip_prefix("Uploaded by "))
            .unwrap_or(text)
            .to_string();
        let lower = text.to_ascii_lowercase();
        // Menu labels ("Big Tits Category"), "Rank:" style captions.
        if text.is_empty()
            || lower.starts_with("see all")
            || lower.starts_with("show all")
            || lower.ends_with(" category")
            || lower.ends_with(" pornstar")
            || lower.ends_with(':')
        {
            continue;
        }
        let group = a
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find(|p| p.value().attr("class").is_some() || p.value().attr("id").is_some())
            .map(|p| p.id())
            .unwrap_or_else(|| a.id());
        groups.entry(group).or_default().push((kind, text));
    }
    let mut out = PageLinks::default();
    for (_, links) in groups.into_iter().filter(|(_, l)| l.len() <= 60) {
        for (kind, text) in links {
            match kind {
                0 => out.tags.push(text),
                1 => out.categories.push(text),
                2 => out.models.push(text),
                _ => out.channels.push(text),
            }
        }
    }
    out
}

fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Site-wide meta keywords that say nothing about the video.
fn is_junk_keyword(k: &str) -> bool {
    let k = k.to_ascii_lowercase();
    [
        "porn",
        "porno",
        "xxx",
        "sex",
        "free",
        "free porn",
        "porn videos",
        "sex videos",
        "xxx videos",
        "hd porn",
        "porn tube",
        "free sex",
        "adult",
        "videos",
        "video",
        "tube",
        "free porn videos",
        "porn movies",
        "sex movies",
        "hd",
        "online",
    ]
    .contains(&k.as_str())
        || k.contains(".com")
        || k.contains(".xxx")
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}

fn clean(s: &str) -> String {
    crate::sites::urls::html_unescape(&s.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn meta(doc: &Html, property: &str) -> Option<String> {
    let selector = sel(&format!(
        r#"meta[property="{property}"], meta[name="{property}"]"#
    ));
    doc.select(&selector)
        .find_map(|m| m.value().attr("content"))
        .map(clean)
        .filter(|s| !s.is_empty())
}

/// Visible text of an element, minus nested counters / hidden helper spans.
fn element_text(el: ElementRef<'_>) -> String {
    let skip = sel(".count, .user-subscribe, .psbox-link-container, .display-none");
    let hidden: Vec<_> = el.select(&skip).flat_map(|e| e.text()).collect();
    let mut text: String = el.text().collect::<Vec<_>>().join(" ");
    for h in hidden {
        if !h.trim().is_empty() {
            text = text.replacen(h, " ", 1);
        }
    }
    clean(&text)
}

fn texts(doc: &Html, selector: &str) -> Vec<String> {
    doc.select(&sel(selector))
        .map(element_text)
        .filter(|t| !t.is_empty() && t.len() <= 80)
        .collect()
}

fn first_text(doc: &Html, selector: &str) -> Option<String> {
    texts(doc, selector).into_iter().next()
}

/// KVS info blocks: `<div class="item">Categories: <a>…</a> <a>…</a></div>`
/// (ThotHub) or `<span>Tags:</span> <a>…</a>` inside a list item (ThisVid).
fn labelled_links(doc: &Html, label: &str) -> Vec<String> {
    let anchors = sel("a");
    let wanted = format!("{label}:");
    for el in doc.select(&sel("div, li, p")) {
        // The label must be the element's own leading text (or its first
        // child's), not something deep inside a nested block.
        let lead = el
            .children()
            .find_map(|c| {
                if let Some(t) = c.value().as_text() {
                    let t = t.trim();
                    (!t.is_empty()).then(|| t.to_string())
                } else {
                    // Inline label (`<span>Tags:</span>`) only — a nested block
                    // means this is a wrapper around the real labelled element.
                    ElementRef::wrap(c)
                        .filter(|e| {
                            matches!(e.value().name(), "span" | "strong" | "b" | "em" | "label")
                        })
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .or(Some(String::new()))
                }
            })
            .unwrap_or_default();
        if !lead.starts_with(&wanted) {
            continue;
        }
        let links: Vec<String> = el
            .select(&anchors)
            .map(element_text)
            .filter(|t| !t.is_empty() && t.len() <= 80)
            .collect();
        if !links.is_empty() {
            return links;
        }
    }
    Vec::new()
}

/// JSON-LD `"duration": "PT00H10M22S"`.
fn ld_json_duration(html: &str) -> Option<u32> {
    let re = regex::Regex::new(r#""duration"\s*:\s*"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?""#).ok()?;
    let c = re.captures(html)?;
    let n = |i: usize| {
        c.get(i)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0)
    };
    let secs = n(1) * 3600 + n(2) * 60 + n(3);
    (secs > 0).then_some(secs)
}

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|t| t.chars().any(char::is_alphanumeric))
        .filter(|t| seen.insert(t.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_kvs_jsonld_and_links() {
        let kvs = r#"<script>var flashvars = { video_categories: 'Amateur, Big Tits',
            video_tags: 'blonde, pov', video_models: 'Jane Doe' };</script>"#;
        let d = scrape_generic(kvs);
        assert_eq!(d.tags, ["Amateur", "Big Tits", "blonde", "pov"]);
        assert_eq!(d.performers, ["Jane Doe"]);

        let ld = r#"<script type="application/ld+json">{"@type":"VideoObject",
            "keywords":"milf, stepmom","genre":["Hardcore"],
            "actor":[{"@type":"Person","name":"Ann"}]}</script>"#;
        let d = scrape_generic(ld);
        assert_eq!(d.tags, ["milf", "stepmom", "Hardcore"]);
        assert_eq!(d.performers, ["Ann"]);

        let mut menu = String::from(r#"<nav class="main-menu">"#);
        for i in 0..5 {
            menu.push_str(&format!(r#"<a href="/categories/menu{i}/">Menu {i}</a>"#));
        }
        menu.push_str("</nav><div class=\"cat-index\">");
        for i in 0..70 {
            menu.push_str(&format!(r#"<a href="/categories/idx{i}/">Idx {i}</a>"#));
        }
        menu.push_str(
            r#"</div><div class="video-info"><div class="tags-row">
            <a href="/tags/outdoor/">Outdoor <span>12K</span></a>
            <a href="/categories/teen/">Teen</a>
            <a href="/pornstar/riley-reid/">Riley Reid</a>
            <a href="/channels/brazzers/">Brazzers</a>
            <a href="/categories/2/">2</a></div></div>
            <meta name="keywords" content="porn, free porn, ignored">"#,
        );
        let d = scrape_generic(&menu);
        assert_eq!(d.tags, ["Teen", "Outdoor"]);
        assert_eq!(d.performers, ["Riley Reid"]);
        assert_eq!(d.channel.as_deref(), Some("Brazzers"));

        let kw = r#"<meta name="keywords" content="Porn, Free Porn, homemade, Big Ass, site.com">"#;
        assert_eq!(scrape_generic(kw).tags, ["homemade", "Big Ass"]);
    }

    /// `SCRAPE_DIR=/path cargo test generic_on_saved_pages -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn generic_on_saved_pages() {
        let dir = std::env::var("SCRAPE_DIR").expect("SCRAPE_DIR");
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        files.sort_by_key(|f| f.file_name());
        for f in files {
            let html = std::fs::read_to_string(f.path()).unwrap_or_default();
            if html.len() < 20_000 {
                continue;
            }
            let d = scrape_generic(&html);
            println!(
                "{:15} tags {:2} {:?} | perf {:?} | chan {:?}",
                f.file_name().to_string_lossy().trim_end_matches(".html"),
                d.tags.len(),
                d.tags.iter().take(8).collect::<Vec<_>>(),
                d.performers.iter().take(3).collect::<Vec<_>>(),
                d.channel
            );
        }
    }

    #[test]
    fn xvideos_tags_uploader_models() {
        let html = r#"<meta property="og:title" content="Some &amp; Title"/>
            <meta property="og:duration" content="640"/>
            <meta property="og:image" content="https://thumb/x.jpg"/>
            <div class="video-metadata video-tags-list"><ul>
              <li class="main-uploader"><a href="/chan" class="uploader-tag"><span class="name">Chan Name</span><span class="user-subscribe"><span class="count">746k</span></span></a></li>
              <li><a href="/pornstars/a" class="is-pornstar"><span class="name">Ann Model</span><span class="count">1k</span></a></li>
              <li><a href="/tags/teen" class="is-keyword btn">teen</a></li>
              <li><a href="/tags/anal" class="is-keyword btn">anal</a></li>
            </ul></div>"#;
        let d = scrape("xvideos", html);
        assert_eq!(d.title.as_deref(), Some("Some & Title"));
        assert_eq!(d.channel.as_deref(), Some("Chan Name"));
        assert_eq!(d.performers, vec!["Ann Model"]);
        assert_eq!(d.tags, vec!["teen", "anal"]);
        assert_eq!(d.duration, Some(640));
        assert!(d.is_useful());
    }

    #[test]
    fn pornhub_mobile_layout() {
        // `platform=mobile`: tags are the pill row above the title.
        let html = r#"<div class="clearfix videoCtaPill videoCtaMixed">
              <a href="/video?c=3" data-event="video_underplayer" data-label="category"
                 class="gtm-event-video-underplayer item"><span>Amateur</span></a></div>
            <div class="clearfix videoCtaPill videoCtaMixed">
              <a data-event="video_underplayer" data-label="tag" class="gtm-event-video-underplayer item isTag" href="/video/search?search=prostate+massage"><span>prostate massage</span></a></div>
            <div class="userInfoContainer"><span class="usernameWrapper"><a href="/model/pinkloving">pinkloving</a></span></div>"#;
        let d = scrape("pornhub", html);
        assert_eq!(d.tags, vec!["Amateur", "prostate massage"]);
        assert_eq!(d.channel.as_deref(), Some("pinkloving"));
    }

    #[test]
    fn pornhub_categories_tags_pornstars_channel() {
        let html = r#"<script type="application/ld+json">{"duration": "PT00H10M22S"}</script>
            <div class="pornstarsWrapper"><p>Pornstars</p>
              <a class="pstar-list-btn" href="/pornstar/x"><img alt="avatar"> Lacey Johnson <span class="psbox-link-container display-none"></span></a></div>
            <div class="categoriesWrapper"><p>Categories</p><a class="item" href="/video?c=7">Big Dick</a></div>
            <div class="tagsWrapper"><p>Tags</p><a class="item isTag"><span>blonde</span></a><a class="item isTag"><span>Big Dick</span></a></div>
            <div class="video-detailed-info"><div class="usernameWrap"><a href="/channels/x">Sis Loves Me</a></div></div>"#;
        let d = scrape("pornhub", html);
        assert_eq!(d.performers, vec!["Lacey Johnson"]);
        assert_eq!(d.tags, vec!["Big Dick", "blonde"]);
        assert_eq!(d.channel.as_deref(), Some("Sis Loves Me"));
        assert_eq!(d.duration, Some(622));
    }

    #[test]
    fn kvs_labelled_blocks() {
        let html = r#"<div class="info"><div class="item"> Categories: <a href="/c/a">Big Tits</a> <a href="/c/b">Solo Female</a> </div>
            <div class="item"> Tags: <a href="/t/a">christina khalil</a> </div>
            <div class="item"> Models: <a href="/m/a">Christina</a> </div></div>"#;
        let d = scrape("thothub", html);
        assert_eq!(d.tags, vec!["Big Tits", "Solo Female", "christina khalil"]);
        assert_eq!(d.performers, vec!["Christina"]);
    }
}
