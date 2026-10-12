//! Generic "video grid" parser shared by HTML-scraped tube adapters.
//!
//! Listing pages usually link each video several times: a thumbnail anchor
//! (image + duration/quality badges, no text title) and a title anchor (text
//! and/or `title=`). Taking only the first anchor per URL yields badge text
//! such as "1080p" as the title, or drops the card entirely when the first
//! anchor has no usable text. Instead, every anchor that points at the same
//! video is merged into one card, and the best title/thumbnail/duration
//! across all of them wins.

use crate::models::{BrowseKind, MediaItem, SiteCategory};
use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;

/// Upper bound on cards returned from one listing page.
pub const MAX_LISTING_ITEMS: usize = 60;

static DURATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:(\d{1,2}):)?(\d{1,2}):(\d{2})\b").expect("duration regex"));
static MINUTES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,3})\s*min\b").expect("minutes regex"));
static EDGE_DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:\d{1,2}:)?\d{1,2}:\d{2}\s*[-:|]?\s*|\s*[-:|]?\s*(?:(?:\d{1,2}:)?\d{1,2}:\d{2}|\d{1,3}\s*min)$")
        .expect("edge duration regex")
});
static BADGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:\d{3,4}p(?:\s*\d{2})?|[48]k|hd|uhd|fhd|vr|new|premium|cc|\d+\s*fps|[\d\s.,:%kmb]+|views?)(?:\s*(?:cc|hd|vr))*$")
        .expect("badge regex")
});
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("ws regex"));

/// How a site's listing page should be read.
pub struct ListingSpec<'a> {
    pub site_id: &'a str,
    /// Site origin used to absolutize relative hrefs / image paths.
    pub base: &'a str,
    /// CSS selector matching every candidate video anchor.
    pub anchor_selector: &'a str,
    /// Returns true when an href points at a single video page.
    pub is_video_href: &'a dyn Fn(&str) -> bool,
    /// Generic titles to discard (site nav links etc.).
    pub is_junk_title: &'a dyn Fn(&str) -> bool,
}

#[derive(Default)]
struct Card {
    title: Option<(u8, String)>,
    thumbnail: Option<String>,
    duration: Option<u32>,
}

/// Parse a listing page into media items, merging duplicate anchors per video.
pub fn parse_listing(html: &str, spec: &ListingSpec<'_>) -> Vec<MediaItem> {
    let document = Html::parse_document(html);
    let (Ok(anchor_sel), Ok(img_sel)) = (
        Selector::parse(spec.anchor_selector),
        Selector::parse("img"),
    ) else {
        return Vec::new();
    };

    let mut order: Vec<String> = Vec::new();
    let mut cards: HashMap<String, Card> = HashMap::new();

    for el in document.select(&anchor_sel) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        if !(spec.is_video_href)(href) {
            continue;
        }
        let url = absolutize(spec.base, href.split('#').next().unwrap_or(href));
        if !cards.contains_key(&url) {
            if order.len() >= MAX_LISTING_ITEMS {
                continue;
            }
            order.push(url.clone());
        }
        let card = cards.entry(url).or_default();

        let imgs: Vec<ElementRef<'_>> = el.select(&img_sel).collect();
        let mut candidates: Vec<(u8, Option<&str>)> = vec![
            (4, el.value().attr("title")),
            (4, el.value().attr("aria-label")),
            (3, el.value().attr("data-title")),
        ];
        for img in &imgs {
            candidates.push((2, img.value().attr("alt")));
            candidates.push((2, img.value().attr("title")));
        }
        let text = visible_text(&el);
        candidates.push((1, Some(text.as_str())));
        for (score, raw) in candidates {
            let Some(raw) = raw else { continue };
            if card.title.as_ref().is_some_and(|(s, _)| *s >= score) {
                continue;
            }
            if let Some(t) = clean_listing_title(raw, spec.is_junk_title) {
                card.title = Some((score, t));
            }
        }

        if card.thumbnail.is_none() {
            card.thumbnail = el
                .value()
                .attr("data-mediumthumb")
                .or_else(|| el.value().attr("data-thumb"))
                .and_then(|s| usable_image_url(spec.base, s))
                .or_else(|| imgs.iter().find_map(|img| image_from_img(spec.base, img)));
        }
        if card.duration.is_none() {
            card.duration = parse_duration_text(&text);
        }
    }

    order
        .into_iter()
        .filter_map(|url| {
            let card = cards.remove(&url)?;
            let title = card
                .title
                .map(|(_, t)| t)
                .or_else(|| title_from_url_slug(&url))?;
            Some(MediaItem {
                id: Uuid::new_v4().to_string(),
                title,
                url,
                thumbnail: card.thumbnail,
                duration: card.duration,
                site_id: spec.site_id.to_string(),
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
            })
        })
        .collect()
}

/// Text content of an element, skipping `<noscript>`, `<script>` and `<style>`
/// subtrees (xHamster ships its `<img>` markup inside `<noscript>`, which the
/// parser exposes as raw text).
pub fn visible_text(el: &ElementRef<'_>) -> String {
    let mut out = String::new();
    for node in el.descendants() {
        let Node::Text(text) = node.value() else {
            continue;
        };
        let hidden = node.ancestors().any(|a| {
            a.value()
                .as_element()
                .is_some_and(|e| matches!(e.name(), "noscript" | "script" | "style"))
        });
        if !hidden {
            out.push_str(text);
            out.push(' ');
        }
    }
    WS_RE.replace_all(out.trim(), " ").into_owned()
}

/// Normalize a title candidate; `None` when it is a badge, duration, nav label
/// or leaked markup rather than a real title.
pub fn clean_listing_title(raw: &str, is_junk: &dyn Fn(&str) -> bool) -> Option<String> {
    let unescaped = crate::sites::urls::html_unescape(raw);
    let mut t = WS_RE.replace_all(unescaped.trim(), " ").into_owned();
    // Strip leading/trailing durations ("05:45 Title", "Title 10 min").
    loop {
        let stripped = EDGE_DURATION_RE.replace(&t, "").trim().to_string();
        if stripped == t {
            break;
        }
        t = stripped;
    }
    if t.chars().count() < 3 || t.starts_with('<') || t.contains("src=\"") {
        return None;
    }
    if BADGE_RE.is_match(&t) || is_junk(&t) {
        return None;
    }
    Some(t)
}

/// A count at the end of a label: "tag 100,466", "Cat (1.2K)", "x 12 videos".
/// Requires whitespace before it so "1-on-1" keeps its digits.
static TRAILING_COUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+\(?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s*([km])?\)?(?:\s*videos?)?\s*$")
        .expect("trailing count regex")
});

/// How a site's category / tag index page should be read.
pub struct CategorySpec<'a> {
    pub base: &'a str,
    /// CSS selector matching every category anchor.
    pub anchor_selector: &'a str,
    /// Browse slug from an anchor href (`None` skips the anchor).
    pub slug_from_href: &'a dyn Fn(&str) -> Option<String>,
    pub kind: BrowseKind,
    /// Keep at most this many entries (largest counts first when counts exist).
    pub max: usize,
}

/// Parse a category / tag index page. Names come from `title`/`alt`/text,
/// counts from numbers in the anchor (or its parent, e.g. XNXX
/// `<li><a>tag</a><strong>1,234</strong></li>`).
pub fn parse_category_index(html: &str, spec: &CategorySpec<'_>) -> Vec<SiteCategory> {
    let document = Html::parse_document(html);
    let (Ok(sel), Ok(img_sel)) = (
        Selector::parse(spec.anchor_selector),
        Selector::parse("img"),
    ) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for el in document.select(&sel) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        let Some(slug) = (spec.slug_from_href)(href) else {
            continue;
        };
        if slug.is_empty() || !seen.insert(slug.clone()) {
            continue;
        }
        let text = visible_text(&el);
        let img = el.select(&img_sel).next();
        let text_name = TRAILING_COUNT_RE.replace(&text, "").trim().to_string();
        let text_name = text_name
            .strip_suffix(" Category")
            .unwrap_or(&text_name)
            .to_string();
        let name = [
            el.value().attr("title").map(str::to_string),
            (!text_name.is_empty()).then_some(text_name),
            img.and_then(|i| i.value().attr("alt")).map(str::to_string),
        ]
        .into_iter()
        .flatten()
        .map(|n| crate::sites::urls::html_unescape(n.trim()))
        .find(|n| n.chars().any(char::is_alphanumeric))
        .unwrap_or_else(|| slug.replace(['-', '_'], " "));
        // Count inside the anchor, else in a parent that wraps only this
        // anchor (XNXX: <li><a>tag</a><strong>1,234</strong></li>).
        let count = parse_count(&text).or_else(|| {
            let parent = el.parent().and_then(ElementRef::wrap)?;
            let a_sel = Selector::parse("a").ok()?;
            (parent.select(&a_sel).count() == 1)
                .then(|| parse_count(&visible_text(&parent)))
                .flatten()
        });
        let thumbnail = img.and_then(|i| image_from_img(spec.base, &i));
        out.push(SiteCategory {
            name,
            slug,
            kind: spec.kind,
            thumbnail,
            count,
        });
    }
    if out.len() > spec.max {
        if out.iter().any(|c| c.count.is_some()) {
            out.sort_by_key(|c| std::cmp::Reverse(c.count.unwrap_or(0)));
        }
        out.truncate(spec.max);
    }
    out
}

/// Trailing count of a label ("tag 100,466" → 100466, "x (1.2K)" → 1200).
fn parse_count(text: &str) -> Option<u32> {
    let c = TRAILING_COUNT_RE.captures(text.trim_end())?;
    let raw = c.get(1)?.as_str();
    let mult: f64 = match c.get(2).map(|m| m.as_str().to_ascii_lowercase()) {
        Some(ref s) if s == "k" => 1_000.0,
        Some(ref s) if s == "m" => 1_000_000.0,
        _ => 1.0,
    };
    let n: f64 = if mult > 1.0 {
        raw.replace(',', "").parse().ok()?
    } else {
        // Without a suffix, '.' and ',' are thousands separators.
        raw.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()?
    };
    let v = (n * mult).round();
    (v >= 0.0 && v <= u32::MAX as f64).then_some(v as u32)
}

/// Path segment after `prefix` in an href (`/tags/foo/` + "/tags/" → "foo").
pub fn segment_after(href: &str, prefix: &str) -> Option<String> {
    let path = href.split(['?', '#']).next()?;
    let idx = path.find(prefix)?;
    let rest = &path[idx + prefix.len()..];
    let seg = rest.split('/').next()?.trim();
    (!seg.is_empty()).then(|| seg.to_string())
}

/// Plain text from an HTML fragment (tags removed, entities decoded,
/// whitespace collapsed).
pub fn strip_html_tags(fragment: &str) -> String {
    let doc = Html::parse_fragment(fragment);
    let text: String = doc.root_element().text().collect::<Vec<_>>().join(" ");
    let text = crate::sites::urls::html_unescape(&text);
    WS_RE.replace_all(text.trim(), " ").into_owned()
}

/// First duration-looking token in card text, in seconds.
pub fn parse_duration_text(text: &str) -> Option<u32> {
    if let Some(c) = DURATION_RE.captures(text) {
        let h: u32 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let m: u32 = c.get(2)?.as_str().parse().ok()?;
        let s: u32 = c.get(3)?.as_str().parse().ok()?;
        if s < 60 {
            return Some(h * 3600 + m * 60 + s);
        }
    }
    MINUTES_RE
        .captures(text)
        .and_then(|c| c.get(1)?.as_str().parse::<u32>().ok())
        .map(|m| m * 60)
}

/// Human title from the last URL path segment (`/video.abc/some_title` →
/// "Some title"). Trailing id tokens such as `-xhZ1D1q` / `-4861165` are dropped.
pub fn title_from_url_slug(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let seg = parsed.path().trim_end_matches('/').rsplit('/').find(|s| {
        s.chars().any(|c| c.is_ascii_alphabetic())
            && s.len() > 3
            && !matches!(
                s.to_ascii_lowercase().as_str(),
                "watch" | "video" | "videos" | "view_video.php" | "embed"
            )
    })?;
    let mut words: Vec<&str> = seg
        .split(['-', '_', '+'])
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() > 1
        && words
            .last()
            .is_some_and(|w| w.chars().any(|c| c.is_ascii_digit()))
    {
        words.pop();
    }
    if words.len() < 2 && words.first().is_some_and(|w| w.starts_with("video")) {
        return None;
    }
    let joined = words.join(" ");
    let mut chars = joined.chars();
    let first = chars.next()?;
    let title: String = first.to_uppercase().chain(chars).collect();
    (title.chars().count() >= 3).then_some(title)
}

pub fn absolutize(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else if href.starts_with('/') {
        format!("{}{href}", base.trim_end_matches('/'))
    } else {
        format!("{}/{href}", base.trim_end_matches('/'))
    }
}

fn usable_image_url(base: &str, raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty()
        || s.starts_with("data:")
        || s.contains("lightbox-blank")
        || s.contains("blank.gif")
        || s.ends_with("/placeholder.png")
    {
        return None;
    }
    Some(absolutize(base, s))
}

fn image_from_img(base: &str, img: &ElementRef<'_>) -> Option<String> {
    let v = img.value();
    [
        "data-mediumthumb",
        "data-src",
        "data-thumb",
        "data-original",
        "data-poster",
        "data-thumb_url",
        "src",
    ]
    .iter()
    .find_map(|attr| v.attr(attr).and_then(|s| usable_image_url(base, s)))
    .or_else(|| {
        v.attr("srcset")
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| usable_image_url(base, s))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_junk(_: &str) -> bool {
        false
    }

    fn spec<'a>(
        is_video: &'a dyn Fn(&str) -> bool,
        junk: &'a dyn Fn(&str) -> bool,
    ) -> ListingSpec<'a> {
        ListingSpec {
            site_id: "test",
            base: "https://example.com",
            anchor_selector: "a[href]",
            is_video_href: is_video,
            is_junk_title: junk,
        }
    }

    #[test]
    fn merges_thumbnail_and_title_anchors() {
        let html = r#"
          <div class="thumb-block">
            <a href="/video.abc/some_slug"><img src="blank.gif" data-src="//cdn.example.com/t.jpg"/>
              <span class="video-hd-mark">1080p</span></a>
            <p class="title"><a href="/video.abc/some_slug" title="Real Title">Real Title <span>10 min</span></a></p>
          </div>"#;
        let is_video = |h: &str| h.contains("/video.");
        let items = parse_listing(html, &spec(&is_video, &no_junk));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Real Title");
        assert_eq!(items[0].url, "https://example.com/video.abc/some_slug");
        assert_eq!(
            items[0].thumbnail.as_deref(),
            Some("https://cdn.example.com/t.jpg")
        );
        assert_eq!(items[0].duration, Some(600));
    }

    #[test]
    fn keeps_card_when_first_anchor_has_no_text() {
        let html = r#"
          <a href="/watch/1/"><img alt="Image Alt Title" data-src="https://c/x.jpg"/><span>05:45</span></a>
          <a href="/watch/1/"><span>Text Title</span></a>
          <a href="/watch/2/"><img data-src="https://c/y.jpg"/></a>
          <a href="/watch/2/"> Second Video </a>"#;
        let is_video = |h: &str| h.contains("/watch/");
        let items = parse_listing(html, &spec(&is_video, &no_junk));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Image Alt Title");
        assert_eq!(items[0].duration, Some(345));
        assert_eq!(items[1].title, "Second Video");
    }

    #[test]
    fn ignores_noscript_markup_text() {
        let html = r#"<a href="https://example.com/videos/a-b-xh12" aria-label="Aria Title">
            <noscript><img src="https://c/z.webp" alt="x"/></noscript> 12:07</a>"#;
        let is_video = |h: &str| h.contains("/videos/");
        let items = parse_listing(html, &spec(&is_video, &no_junk));
        assert_eq!(items[0].title, "Aria Title");
        assert_eq!(items[0].duration, Some(727));
    }

    #[test]
    fn rejects_badges_and_durations() {
        for bad in [
            "1080p", "720pCC", "360p CC", "HD", "4K", "12:34", "10 min", "584K",
        ] {
            assert_eq!(clean_listing_title(bad, &no_junk), None, "{bad}");
        }
        assert_eq!(
            clean_listing_title("05:47 Some &amp; Title", &no_junk).as_deref(),
            Some("Some & Title")
        );
    }

    #[test]
    fn category_index_names_and_counts() {
        let html = r#"<ul>
          <li><a href="/tags/0-pussy"><b>0-pussy </b><span class="navbadge">100,466</span></a></li>
          <li><a href="/tags/1-on-1"><b>1-on-1 </b><span class="navbadge">205,885</span></a></li>
          <li><a href="/search/big-ass">big ass</a><strong>2,500</strong></li>
          <li><a href="/tags/small" title="Small Tag">x</a></li>
          <li><a href="/tags/0-pussy">dupe</a></li>
          <li><a href="/other/x">skip</a></li>
        </ul>"#;
        let slug = |h: &str| segment_after(h, "/tags/").or_else(|| segment_after(h, "/search/"));
        let cats = parse_category_index(
            html,
            &CategorySpec {
                base: "https://example.com",
                anchor_selector: "a[href]",
                slug_from_href: &slug,
                kind: BrowseKind::Tag,
                max: 3,
            },
        );
        assert_eq!(cats.len(), 3);
        assert_eq!(cats[0].name, "1-on-1");
        assert_eq!(cats[0].count, Some(205_885));
        assert_eq!(cats[1].name, "0-pussy");
        assert_eq!(cats[1].count, Some(100_466));
        assert_eq!(cats[2].slug, "big-ass");
        assert_eq!(cats[2].count, Some(2_500));
        assert_eq!(parse_count("tag (1.2K)"), Some(1_200));
        assert_eq!(parse_count("4K Porn"), None);
        assert_eq!(parse_count("Young (18+) and Old"), None);
        assert_eq!(
            segment_after("https://x.com/categories/amateur/?p=2", "/categories/").as_deref(),
            Some("amateur")
        );
    }

    #[test]
    fn strips_room_subject_markup() {
        assert_eq!(
            strip_html_tags(r#"hi &amp; welcome <a href="/tag/new/">#new</a>  <b>x</b>"#),
            "hi & welcome #new x"
        );
    }

    #[test]
    fn slug_title_fallback() {
        assert_eq!(
            title_from_url_slug("https://xhamster.com/videos/willing-housewife-xhZ1D1q").as_deref(),
            Some("Willing housewife")
        );
        assert_eq!(
            title_from_url_slug("https://www.xvideos.com/video.abc123/sexy_amateur").as_deref(),
            Some("Sexy amateur")
        );
        assert_eq!(
            title_from_url_slug("https://www.youporn.com/watch/205140451/"),
            None
        );
    }
}
