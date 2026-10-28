use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

/// CamsFinder is a cam-site *aggregator* (chaturbate, stripchat, bongacams,
/// livejasmin, jerkmate, ...) with server-rendered listing and search pages.
///
/// In-app playback works for rooms on sites ArcHive already supports
/// (Chaturbate, Stripchat) by delegating to their resolvers. All other
/// underlying sites are affiliate-walled / JS-only, so those rooms error with
/// a pointer to the CamsFinder page. CamsFinder itself blocks framing
/// (`X-Frame-Options: SAMEORIGIN` on both the site and `/embed/...`), so
/// embed-iframe playback is not possible for this source.
const BASE: &str = "https://www.camsfinder.com";

const MAX_ROOMS: usize = 48;

pub struct CamsFinderAdapter;

#[async_trait]
impl SiteAdapter for CamsFinderAdapter {
    fn id(&self) -> &str {
        "camsfinder"
    }

    fn display_name(&self) -> &str {
        "CamsFinder"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Livestream, BrowseKind::Tag, BrowseKind::Search]
    }

    fn requires_cookies(&self) -> bool {
        false
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = match query.kind {
            BrowseKind::Tag => with_page(
                &format!("{BASE}/girl/{}", tag_slug(&query.slug)),
                query.page,
            ),
            BrowseKind::Search => format!(
                "{BASE}/models?search={}&page={}",
                query_slug(&query.slug),
                query.page.max(1)
            ),
            _ => {
                if query.slug.starts_with("http") {
                    query.slug.clone()
                } else {
                    with_page(&format!("{BASE}/girl"), query.page)
                }
            }
        };

        let html = ctx.fetch_html(&url, self.id()).await?;
        let rooms = parse_listing_html(&html).ok_or_else(|| {
            AppError::Site(format!(
                "No rooms found on CamsFinder ({url}) - the listing page may have changed."
            ))
        })?;
        Ok(build_browse_page(rooms, query.page))
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        let (site, room_url) = resolve_underlying_room(ctx, url).await?;
        match site.as_str() {
            "chaturbate" => {
                crate::sites::urls::resolve_cam_stream_url(ctx, "chaturbate", &room_url).await
            }
            "stripchat" => {
                // Native __PRELOADED_STATE__ resolver first (muxed master).
                if let Ok(master) =
                    crate::sites::adapters::stripchat::resolve_stream_url_native(ctx, &room_url)
                        .await
                {
                    return Ok(master);
                }
                crate::sites::urls::resolve_cam_stream_url(ctx, "stripchat", &room_url).await
            }
            other => Err(AppError::Site(format!(
                "CamsFinder proxies {other} rooms, which ArcHive can't stream in-app yet. Open the room on camsfinder.com instead."
            ))),
        }
    }

    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "camsfinder/%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "camsfinder".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: None,
            referer: Some(BASE.to_string()),
            initial_status: None,
        })
    }
}

/// Resolve a CamsFinder live-player URL into `(underlying_site, room_url)`.
///
/// Accepts both URL shapes:
/// - `https://www.camsfinder.com/model/{site}/{name}` (listing item URL)
/// - `https://camsfinder.com/{name}/` (constructed by the live player from
///   site id + slug) — the model's site is looked up via the search page.
pub(crate) async fn resolve_underlying_room(
    ctx: &SiteContext,
    url: &str,
) -> AppResult<(String, String)> {
    let path = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, path)| path.trim_end_matches('/'))
        .unwrap_or("");
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() >= 3 && segments[0] == "model" {
        return Ok((
            segments[1].to_lowercase(),
            model_page_url(segments[1], segments[2]),
        ));
    }

    // Single segment: find the model by username via the search page.
    let name = segments.last().copied().unwrap_or("");
    if name.is_empty() {
        return Err(AppError::Site("Invalid CamsFinder room URL".to_string()));
    }
    let html = ctx
        .fetch_html(
            &format!("{BASE}/models?search={}", query_slug(name)),
            "camsfinder",
        )
        .await?;
    if let Some((site, model_name)) = find_model_in_html(&html, name) {
        let page = model_page_url(&site, &model_name);
        return Ok((site, page));
    }
    Err(AppError::Site(format!(
        "Model '{name}' not found on CamsFinder"
    )))
}

/// Live-player resolution: returns `(video_url, audio_url)` for the
/// underlying site when it is supported.
pub(crate) async fn resolve_livestream_urls(
    ctx: &SiteContext,
    url: &str,
) -> AppResult<(String, Option<String>)> {
    let (site, room_url) = resolve_underlying_room(ctx, url).await?;
    match site.as_str() {
        "chaturbate" => {
            crate::sites::urls::resolve_cam_stream_urls(ctx, "chaturbate", &room_url).await
        }
        "stripchat" => {
            // Native muxed master first. Private-show/offline verdicts are
            // final (surfaced to the UI); only page-shape failures fall
            // through to yt-dlp's split-A/V path.
            match crate::sites::adapters::stripchat::resolve_stream_url_native(ctx, &room_url).await
            {
                Ok(master) => return Ok((master, None)),
                Err(e) if crate::sites::adapters::stripchat::is_room_unavailable(&e) => {
                    return Err(e);
                }
                Err(_) => {}
            }
            crate::sites::urls::resolve_cam_stream_urls(ctx, "stripchat", &room_url).await
        }
        other => Err(AppError::Site(format!(
            "CamsFinder proxies {other} rooms, which ArcHive can't stream in-app yet. Open the room on camsfinder.com instead."
        ))),
    }
}

fn model_page_url(site: &str, name: &str) -> String {
    format!("{BASE}/model/{site}/{name}")
}

fn with_page(url: &str, page: u32) -> String {
    if page <= 1 {
        url.to_string()
    } else {
        format!("{url}?page={page}")
    }
}

fn tag_slug(slug: &str) -> String {
    slug.trim()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

fn query_slug(slug: &str) -> String {
    slug.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            b' ' => "+".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A model card scraped from a CamsFinder listing/search page.
struct CamRoom {
    /// Underlying cam site brand (`chaturbate`, `stripchat`, `bongacams`, ...).
    site: String,
    /// Model username (as shown on CamsFinder).
    name: String,
    thumbnail: Option<String>,
    age: Option<u32>,
    country: Option<String>,
}

fn parse_listing_html(html: &str) -> Option<Vec<CamRoom>> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let link_sel = Selector::parse("a[href^='/model/']").ok()?;
    let img_sel = Selector::parse("img[src*='/imgproxy/']").ok()?;

    let mut rooms: Vec<CamRoom> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for el in document.select(&link_sel) {
        let href = el.value().attr("href")?;
        let Some((site, name)) = split_model_href(href) else {
            continue;
        };
        if !seen.insert(format!("{site}/{name}")) {
            continue;
        }

        // Card metadata: aria-label is
        // "{Name}, {age} years old, currently live, from {CC}".
        let label = el.value().attr("aria-label").unwrap_or("");
        let age = parse_age_from_label(label);
        let country = parse_country_from_label(label);

        // Thumbnail: first imgproxy image inside the card link.
        let thumbnail = el.select(&img_sel).next().and_then(|img| {
            img.value()
                .attr("src")
                .and_then(decode_imgproxy_src)
                .filter(|s| s.starts_with("http"))
        });

        rooms.push(CamRoom {
            site,
            name,
            thumbnail,
            age,
            country,
        });
        if rooms.len() >= MAX_ROOMS {
            break;
        }
    }

    if rooms.is_empty() {
        None
    } else {
        Some(rooms)
    }
}

/// Case-insensitive exact-username lookup on a search page.
fn find_model_in_html(html: &str, name: &str) -> Option<(String, String)> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let link_sel = Selector::parse("a[href^='/model/']").ok()?;
    let wanted = name.to_lowercase();
    for el in document.select(&link_sel) {
        let href = el.value().attr("href")?;
        if let Some((site, model_name)) = split_model_href(href) {
            if model_name.to_lowercase() == wanted {
                return Some((site, model_name));
            }
        }
    }
    None
}

/// `/model/{site}/{name}` -> `(site, name)`.
fn split_model_href(href: &str) -> Option<(String, String)> {
    let mut parts = href.trim_start_matches('/').split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("model"), Some(site), Some(name)) if !site.is_empty() && !name.is_empty() => {
            let name = name.split('?').next().unwrap_or(name);
            Some((site.to_lowercase(), crate::sites::urls::html_unescape(name)))
        }
        _ => None,
    }
}

/// CamsFinder thumbnails are served through `/imgproxy/insecure/w:384/q:65/
/// plain/<percent-encoded original URL>`; extract the original URL.
fn decode_imgproxy_src(src: &str) -> Option<String> {
    if !src.contains("/imgproxy/") {
        return None;
    }
    let idx = src.find("plain/")?;
    let encoded = &src[idx + "plain/".len()..];
    let end = encoded.find(['"', '\\', ' ']).unwrap_or(encoded.len());
    percent_decode(&encoded[..end])
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let byte = u8::from_str_radix(hex, 16).ok()?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn parse_age_from_label(label: &str) -> Option<u32> {
    let idx = label.find(" years old")?;
    let before = &label[..idx];
    let num: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    num.parse::<u32>().ok()
}

fn parse_country_from_label(label: &str) -> Option<String> {
    let idx = label.find(" from ")?;
    let rest = &label[idx + " from ".len()..];
    let cc: String = rest
        .chars()
        .take_while(|c| c.is_ascii_uppercase())
        .collect();
    if cc.len() == 2 {
        Some(cc)
    } else {
        None
    }
}

fn room_to_item(room: &CamRoom) -> MediaItem {
    let room_url = model_page_url(&room.site, &room.name);
    let thumb = room.thumbnail.clone();
    let country = room.country.clone();
    let name = room.name.clone();
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: name.clone(),
        url: room_url,
        thumbnail: thumb,
        duration: None,
        site_id: "camsfinder".to_string(),
        performers: vec![name.clone()],
        tags: vec![],
        description: country.as_ref().map(|cc| format!("Country: {cc}")),
        channel: Some(name),
        is_live: Some(true),
        viewers: None,
        age: room.age,
        gender: Some("girl".to_string()),
        stream_url: None,
        embed_url: None,
    }
}

fn build_browse_page(rooms: Vec<CamRoom>, page: u32) -> BrowsePage {
    let items: Vec<MediaItem> = rooms.iter().map(room_to_item).collect();
    let has_more = items.len() >= MAX_ROOMS;
    BrowsePage {
        items,
        page,
        has_more,
        total: None,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_model_href_parses_site_and_name() {
        assert_eq!(
            split_model_href("/model/bongacams/wild-angel777"),
            Some(("bongacams".to_string(), "wild-angel777".to_string()))
        );
        assert_eq!(
            split_model_href("/model/livejasmin/VenusPrescot?ref=x"),
            Some(("livejasmin".to_string(), "VenusPrescot".to_string()))
        );
        assert_eq!(split_model_href("/girl/masturbation"), None);
        assert_eq!(split_model_href("/model/chaturbate"), None);
    }

    #[test]
    fn label_parsers_extract_age_and_country() {
        let label = "VenusPrescot, 45 years old, currently live, from FR";
        assert_eq!(parse_age_from_label(label), Some(45));
        assert_eq!(parse_country_from_label(label), Some("FR".to_string()));
        assert_eq!(parse_age_from_label("no metadata here"), None);
    }

    #[test]
    fn imgproxy_src_decodes_original_url() {
        let src = "/imgproxy/insecure/w:384/q:65/plain/https%3A%2F%2Fgalleryn3.vcmdiawe.com%2Fff2%2Fpic.jpg%3Fcno%3D0b4d";
        assert_eq!(
            decode_imgproxy_src(src).as_deref(),
            Some("https://galleryn3.vcmdiawe.com/ff2/pic.jpg?cno=0b4d")
        );
        // Not an imgproxy URL - reject.
        assert_eq!(decode_imgproxy_src("https://example.com/plain/x.jpg"), None);
    }

    #[test]
    fn parses_real_camsfinder_listing() {
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/camsfinder_listing.html");
        if !fixture_path.exists() {
            return;
        }
        let html = std::fs::read_to_string(&fixture_path).expect("fixture");
        let rooms = parse_listing_html(&html).expect("rooms");
        assert!(rooms.len() >= 2, "expected >= 2 rooms, got {}", rooms.len());
        let first = &rooms[0];
        assert_eq!(first.site, "jerkmate");
        assert_eq!(first.name, "colettedreamxo");
        assert_eq!(first.age, Some(27));
        assert_eq!(first.country.as_deref(), Some("US"));
        assert!(first.thumbnail.as_deref().unwrap_or("").starts_with("http"));
    }
}
