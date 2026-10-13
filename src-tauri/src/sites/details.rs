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
            d.channel = first_text(&doc, ".video-detailed-info .usernameWrap a")
                .or_else(|| first_text(&doc, ".videoUploaderBlock .usernameWrap a"));
            d.performers = texts(&doc, ".pornstarsWrapper a.pstar-list-btn");
            tags = texts(&doc, ".categoriesWrapper a.item");
            tags.extend(texts(&doc, ".tagsWrapper a.item"));
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
