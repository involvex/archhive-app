use crate::error::AppResult;
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use uuid::Uuid;

const BASE: &str = "https://chaturbate.com";

pub struct ChaturbateAdapter;

#[async_trait]
impl SiteAdapter for ChaturbateAdapter {
    fn id(&self) -> &str {
        "chaturbate"
    }

    fn display_name(&self) -> &str {
        "Chaturbate"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![
            BrowseKind::Livestream,
            BrowseKind::Tag,
            BrowseKind::Search,
            BrowseKind::Model,
        ]
    }

    fn requires_cookies(&self) -> bool {
        true
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        // Chaturbate is a JS SPA: room cards are hydrated client-side by
        // `web2.static.mmcdn.com/cachebust/roomlist-prefetch.bfbd95a4a9e3.js`. The
        // server-rendered HTML (root `/`, `/tags/<x>/`, `/discover/<x>/`) only contains
        // empty `<li class="roomCard placeholder camBgColor">` placeholders inside
        // `<div id="roomlist_root" data-testid="room-list">`.
        //
        // The internal API the SPA calls (`/api/ts/roomlist/room-list/`) returns the same
        // placeholder-only HTML page gated by the age-verification overlay, and the
        // affiliates endpoint (`affiliates/api/onlinerooms/?format=json`) returns `[]`
        // without a valid `wm=` token. yt-dlp's `[Chaturbate]` extractor only handles
        // individual room URLs (treats anything else as `Room is currently offline`), so
        // it cannot list rooms either.
        //
        // Net result: room LISTINGS (Livestream/Tag/Search) always come back empty.
        // Per-room navigation (`browse_model`) and `resolve_livestream` still work via
        // yt-dlp `--get-url`. The frontend surfaces a clear "no rooms from listing page"
        // message instead of a generic empty grid.

        match query.kind {
            BrowseKind::Model => self.browse_model(ctx, query).await,
            BrowseKind::Livestream => self.browse_listing(ctx, query).await,
            BrowseKind::Tag => self.browse_listing(ctx, query).await,
            BrowseKind::Search => self.browse_listing(ctx, query).await,
            _ => self.browse_listing(ctx, query).await,
        }
    }

    async fn resolve_download(
        &self,
        _ctx: &SiteContext,
        item: &MediaItem,
    ) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "chaturbate/%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "chaturbate".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
        })
    }
}

impl ChaturbateAdapter {
    /// `livestream` / `tag` / `search` / generic: open the listing URL in a hidden
    /// Tauri webview (which runs the site's JS bundle) and extract the hydrated
    /// room cards. yt-dlp's `[Chaturbate]` extractor rejects listing URLs, and
    /// the server-rendered HTML only contains placeholder `<li class="roomCard
    /// placeholder">` elements, so we must run the SPA to get real cards.
    async fn browse_listing(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_listing_url(&query);
        let rooms =
            crate::sites::adapters::chaturbate_webview::fetch_listing(ctx.app(), ctx.vault(), &url)
                .await?;
        let items: Vec<MediaItem> = rooms.into_iter().map(map_room).collect();
        let has_more = items.len() >= 30;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
        })
    }

    /// `model` kind: return a single synthetic `MediaItem` pointing at the model's room.
    /// `resolve_livestream` (yt-dlp `--get-url`) is what actually resolves the stream URL
    /// when the user opens the player page — this listing entry is just a navigation card.
    async fn browse_model(&self, _ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let slug_raw = query.slug.trim();
        if slug_raw.is_empty() {
            return Ok(BrowsePage {
                items: vec![],
                page: query.page,
                has_more: false,
                total: None,
            });
        }
        let (room_url, username) = if slug_raw.starts_with("http") {
            let u = extract_username_from_url(slug_raw).unwrap_or_default();
            (slug_raw.to_string(), u)
        } else {
            let u = path_slug(slug_raw);
            (format!("{BASE}/{u}/"), u)
        };
        if username.is_empty() {
            return Ok(BrowsePage {
                items: vec![],
                page: query.page,
                has_more: false,
                total: None,
            });
        }
        Ok(BrowsePage {
            items: vec![MediaItem {
                id: Uuid::new_v4().to_string(),
                title: username.clone(),
                url: room_url,
                thumbnail: None,
                duration: None,
                site_id: "chaturbate".to_string(),
                performers: vec![username.clone()],
                tags: vec![],
                description: None,
                channel: Some(username.clone()),
                is_live: Some(true),
                viewers: None,
                age: None,
                gender: None,
                stream_url: None,
                embed_url: Some(format!("{BASE}/embed/{username}/")),
            }],
            page: query.page,
            has_more: false,
            total: Some(1),
        })
    }
}

fn build_listing_url(query: &BrowseQuery) -> String {
    match query.kind {
        BrowseKind::Livestream => {
            if query.slug.starts_with("http") {
                query.slug.clone()
            } else if query.slug.is_empty() {
                format!("{BASE}/")
            } else {
                format!("{}/{}/", BASE, path_slug(&query.slug))
            }
        }
        BrowseKind::Tag => format!("{BASE}/tags/{}/", path_slug(&query.slug)),
        BrowseKind::Search => format!("{BASE}/search/?q={}", url_slug(&query.slug)),
        _ => {
            if query.slug.starts_with("http") {
                query.slug.clone()
            } else if query.slug.is_empty() {
                format!("{BASE}/")
            } else {
                format!("{}/{}/", BASE, path_slug(&query.slug))
            }
        }
    }
}

fn extract_username_from_url(url: &str) -> Option<String> {
    // Accept https://chaturbate.com/<user>/ or trailing path junk; return the first
    // path segment that looks like a username.
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let path = path.split('?').next().unwrap_or(path);
    let after_host = path.split_once('/').map(|(_, rest)| rest).unwrap_or("");
    let first = after_host.split('/').find(|s| !s.is_empty())?;
    if first.is_empty() || first == "tags" || first == "search" || first == "embed" {
        return None;
    }
    Some(first.to_string())
}

fn map_room(room: crate::sites::adapters::chaturbate_webview::RawRoom) -> MediaItem {
    let username = room.username;
    let room_url = format!("{BASE}/{username}/");
    let embed_url = format!("{BASE}/embed/{username}/");
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: room.title,
        url: room_url,
        thumbnail: room.thumbnail,
        duration: None,
        site_id: "chaturbate".to_string(),
        performers: vec![username.clone()],
        tags: vec![],
        description: None,
        channel: Some(username.clone()),
        is_live: Some(true),
        viewers: room.viewers,
        age: room.age,
        gender: room.gender,
        stream_url: None,
        embed_url: Some(embed_url),
    }
}

fn path_slug(slug: &str) -> String {
    slug.trim_start_matches('/')
        .trim_end_matches('/')
        .replace(' ', "_")
}

fn url_slug(slug: &str) -> String {
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
