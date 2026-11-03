//! CamGirlFinder API adapter — face/model discovery (not a live room grid).
//!
//! Free API: https://api.camgirlfinder.net/
//! Use `externalProfile` affiliate links when opening rooms externally, and send
//! a meaningful User-Agent (`ARCHHIVE_UA`).

use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

const API_BASE: &str = "https://api.camgirlfinder.net";
const SITE_BASE: &str = "https://camgirlfinder.net";

/// Meaningful UA required by the free API terms.
pub(crate) const ARCHHIVE_UA: &str =
    concat!("ArcHive/", env!("CARGO_PKG_VERSION"), " (+https://github.com/involvex/archhive-app)");

const RANDOM_FEED_SIZE: usize = 24;

pub struct CamgirlfinderAdapter;

#[async_trait]
impl SiteAdapter for CamgirlfinderAdapter {
    fn id(&self) -> &str {
        "camgirlfinder"
    }

    fn display_name(&self) -> &str {
        "CamGirlFinder"
    }

    fn base_url(&self) -> &str {
        SITE_BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Livestream, BrowseKind::Search, BrowseKind::Tag]
    }

    fn requires_cookies(&self) -> bool {
        false
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        match query.kind {
            BrowseKind::Search => {
                let model = query.slug.trim();
                if model.chars().count() < 3 {
                    return Err(AppError::InvalidInput(
                        "Search needs at least 3 characters".into(),
                    ));
                }
                let (platform, gender) = parse_filters_from_slug("");
                let items = search_models(ctx, model, platform.as_deref(), gender.as_deref()).await?;
                Ok(BrowsePage {
                    items,
                    page: query.page.max(1),
                    has_more: false,
                    total: None,
                    ..Default::default()
                })
            }
            BrowseKind::Tag => {
                // Tag slug conventions:
                // - `similar:cb/username` → similar accounts
                // - `platform:cb` / `gender:f` → random with filters
                // - `platform:cb,gender:f` combined
                let slug = query.slug.trim();
                if let Some(rest) = slug.strip_prefix("similar:") {
                    let (platform, model) = split_platform_model(rest)?;
                    let items = similar_models(ctx, &platform, &model).await?;
                    return Ok(BrowsePage {
                        items,
                        page: 1,
                        has_more: false,
                        total: None,
                        ..Default::default()
                    });
                }
                let (platform, gender) = parse_filters_from_slug(slug);
                let items =
                    random_feed(ctx, RANDOM_FEED_SIZE, platform.as_deref(), gender.as_deref())
                        .await?;
                Ok(BrowsePage {
                    items,
                    page: query.page.max(1),
                    has_more: true,
                    total: None,
                    ..Default::default()
                })
            }
            _ => {
                // Livestream / default → random explore feed (not live)
                let (platform, gender) = parse_filters_from_slug(&query.slug);
                let items =
                    random_feed(ctx, RANDOM_FEED_SIZE, platform.as_deref(), gender.as_deref())
                        .await?;
                Ok(BrowsePage {
                    items,
                    page: query.page.max(1),
                    has_more: true,
                    total: None,
                    ..Default::default()
                })
            }
        }
    }

    async fn resolve_stream_url(&self, ctx: &SiteContext, url: &str) -> AppResult<String> {
        let (site, room_url) = resolve_underlying_room(url)?;
        match site.as_str() {
            "chaturbate" => {
                crate::sites::urls::resolve_cam_stream_url(ctx, "chaturbate", &room_url).await
            }
            "stripchat" => {
                if let Ok(master) =
                    crate::sites::adapters::stripchat::resolve_stream_url_native(ctx, &room_url)
                        .await
                {
                    return Ok(master);
                }
                crate::sites::urls::resolve_cam_stream_url(ctx, "stripchat", &room_url).await
            }
            other => Err(AppError::Site(format!(
                "CamGirlFinder lists {other} rooms, which ArcHive can't stream in-app yet. Open the affiliate room link instead."
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
            output_template: "camgirlfinder/%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: "camgirlfinder".to_string(),
            thumbnail_url: item.thumbnail.clone(),
            duration: item.duration,
            channel: item.channel.clone(),
            referer: Some(SITE_BASE.to_string()),
            initial_status: None,
        })
    }
}

pub(crate) async fn resolve_livestream_urls(
    ctx: &SiteContext,
    url: &str,
) -> AppResult<(String, Option<String>)> {
    let (site, room_url) = resolve_underlying_room(url)?;
    match site.as_str() {
        "chaturbate" => {
            crate::sites::urls::resolve_cam_stream_urls(ctx, "chaturbate", &room_url).await
        }
        "stripchat" => {
            match crate::sites::adapters::stripchat::resolve_stream_url_native(ctx, &room_url).await
            {
                Ok(master) => Ok((master, None)),
                Err(e) if crate::sites::adapters::stripchat::is_room_unavailable(&e) => Err(e),
                Err(_) => {
                    crate::sites::urls::resolve_cam_stream_urls(ctx, "stripchat", &room_url).await
                }
            }
        }
        other => Err(AppError::Site(format!(
            "CamGirlFinder lists {other} rooms, which ArcHive can't stream in-app yet."
        ))),
    }
}

/// Resolve CGF out-link or mapped room URL to `(archhive_site, room_url)`.
pub(crate) fn resolve_underlying_room(url: &str) -> AppResult<(String, String)> {
    let lower = url.to_lowercase();
    // https://api.camgirlfinder.net/out/{platform}/{model} — keep model case from original URL
    for needle in ["camgirlfinder.net/out/", "camgirlfinder.to/out/"] {
        if let Some(idx) = lower.find(needle) {
            let rest = &url[idx + needle.len()..];
            let mut parts = rest.trim_matches('/').split('/');
            let platform = parts.next().unwrap_or("").to_lowercase();
            let model = parts
                .next()
                .unwrap_or("")
                .split('?')
                .next()
                .unwrap_or("");
            if platform.is_empty() || model.is_empty() {
                return Err(AppError::Site("Invalid CamGirlFinder out URL".into()));
            }
            return map_platform_room(&platform, model);
        }
    }
    // Already a CB/SC room URL
    if lower.contains("chaturbate.com/") {
        return Ok(("chaturbate".into(), url.to_string()));
    }
    if lower.contains("stripchat.com/") || lower.contains("xhamsterlive.com/") {
        return Ok(("stripchat".into(), url.to_string()));
    }
    Err(AppError::Site(
        "Not a CamGirlFinder / Chaturbate / Stripchat room URL".into(),
    ))
}

fn map_platform_room(platform: &str, model: &str) -> AppResult<(String, String)> {
    match platform {
        "cb" => Ok((
            "chaturbate".into(),
            format!("https://chaturbate.com/{model}/"),
        )),
        "sc" => Ok((
            "stripchat".into(),
            format!("https://stripchat.com/{model}/"),
        )),
        other => Err(AppError::Site(format!(
            "Platform '{other}' is not streamable in-app"
        ))),
    }
}

fn playable_or_external(platform: &str, model: &str, external: &str) -> String {
    match platform {
        "cb" => format!("https://chaturbate.com/{model}/"),
        "sc" => format!("https://stripchat.com/{model}/"),
        _ => external.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct ModelSearchHit {
    name: String,
    platform: String,
    gender: String,
    #[serde(default)]
    #[allow(dead_code)]
    distance: f64,
    #[serde(default, rename = "lastSeen")]
    last_seen: Option<String>,
    #[serde(default)]
    persons: Vec<Person>,
    urls: ProfileUrls,
}

#[derive(Debug, Deserialize)]
struct Person {
    #[serde(default)]
    urls: Option<PersonUrls>,
}

#[derive(Debug, Deserialize)]
struct PersonUrls {
    #[serde(default, rename = "faceImage")]
    face_image: Option<String>,
    #[serde(default, rename = "fullImage")]
    full_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileUrls {
    #[serde(default)]
    profile: Option<String>,
    #[serde(rename = "externalProfile")]
    external_profile: String,
}

#[derive(Debug, Deserialize)]
struct ModelProfile {
    name: String,
    platform: String,
    gender: String,
    #[serde(default, rename = "firstSeen")]
    first_seen: Option<String>,
    #[serde(default, rename = "lastSeen")]
    last_seen: Option<String>,
    urls: ProfileUrls,
    #[serde(default)]
    persons: Vec<PersonDetail>,
    #[serde(default)]
    schedule: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Deserialize)]
struct PersonDetail {
    person: i64,
    #[serde(default)]
    faces: i64,
    #[serde(default)]
    seen: Option<String>,
    #[serde(default, rename = "firstSeen")]
    first_seen: Option<String>,
    #[serde(default, rename = "lastSeen")]
    last_seen: Option<String>,
    urls: PersonUrls,
}

#[derive(Debug, Clone, Serialize)]
pub struct CamgirlfinderProfile {
    pub name: String,
    pub platform: String,
    pub gender: String,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub profile_url: Option<String>,
    pub external_profile: String,
    pub playable_url: Option<String>,
    pub persons: Vec<CamgirlfinderPerson>,
    pub schedule: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CamgirlfinderPerson {
    pub person: i64,
    pub faces: i64,
    pub seen: Option<String>,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub face_image: Option<String>,
    pub full_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Prediction {
    platform: String,
    model: String,
    gender: String,
    #[serde(default)]
    distance: f64,
    #[serde(default)]
    probability: Option<String>,
    #[serde(default, rename = "accountSeen")]
    account_seen: Option<String>,
    urls: PredictionUrls,
}

#[derive(Debug, Deserialize)]
struct PredictionUrls {
    #[serde(rename = "externalProfile")]
    external_profile: String,
    #[serde(default, rename = "fullImage")]
    full_image: Option<String>,
    #[serde(default, rename = "faceImage")]
    face_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobResponse {
    #[serde(default)]
    status: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    predictions: Vec<Prediction>,
}

async fn api_get(ctx: &SiteContext, path_and_query: &str) -> AppResult<String> {
    let url = format!("{API_BASE}{path_and_query}");
    let resp = ctx
        .client
        .get(&url)
        .header("User-Agent", ARCHHIVE_UA)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Site(format!("CamGirlFinder request failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| AppError::Site(format!("CamGirlFinder read failed: {e}")))?;
    if status.as_u16() == 429 {
        return Err(AppError::Site(
            "CamGirlFinder rate limit (429). Wait a moment and try again.".into(),
        ));
    }
    if !status.is_success() {
        return Err(AppError::Site(format!(
            "CamGirlFinder HTTP {status}: {}",
            body.chars().take(200).collect::<String>()
        )));
    }
    Ok(body)
}

async fn search_models(
    ctx: &SiteContext,
    model: &str,
    platform: Option<&str>,
    gender: Option<&str>,
) -> AppResult<Vec<MediaItem>> {
    let mut q = format!(
        "/models/search?model={}",
        urlencoding_minimal(model)
    );
    if let Some(p) = platform {
        q.push_str(&format!("&platform={p}"));
    }
    if let Some(g) = gender {
        q.push_str(&format!("&gender={g}"));
    }
    let body = api_get(ctx, &q).await?;
    let hits: Vec<ModelSearchHit> = serde_json::from_str(&body)
        .map_err(|e| AppError::Site(format!("CamGirlFinder search parse: {e}")))?;
    Ok(hits.iter().map(hit_to_item).collect())
}

async fn random_feed(
    ctx: &SiteContext,
    count: usize,
    platform: Option<&str>,
    gender: Option<&str>,
) -> AppResult<Vec<MediaItem>> {
    let mut items = Vec::with_capacity(count);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..count {
        let mut q = "/models/random".to_string();
        let mut sep = '?';
        if let Some(p) = platform {
            q.push_str(&format!("{sep}platform={p}"));
            sep = '&';
        }
        if let Some(g) = gender {
            q.push_str(&format!("{sep}gender={g}"));
        }
        let body = api_get(ctx, &q).await?;
        let profile: ModelProfile = serde_json::from_str(&body)
            .map_err(|e| AppError::Site(format!("CamGirlFinder random parse: {e}")))?;
        let key = format!("{}/{}", profile.platform, profile.name);
        if !seen.insert(key) {
            continue;
        }
        items.push(profile_to_item(&profile));
    }
    if items.is_empty() {
        return Err(AppError::Site("CamGirlFinder returned no models".into()));
    }
    Ok(items)
}

async fn similar_models(
    ctx: &SiteContext,
    platform: &str,
    model: &str,
) -> AppResult<Vec<MediaItem>> {
    let path = format!(
        "/models/{}/{}/similar",
        urlencoding_minimal(platform),
        urlencoding_minimal(model)
    );
    let body = api_get(ctx, &path).await?;
    let map: serde_json::Map<String, Value> = serde_json::from_str(&body)
        .map_err(|e| AppError::Site(format!("CamGirlFinder similar parse: {e}")))?;
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (_person_id, preds) in map {
        let Ok(list) = serde_json::from_value::<Vec<Prediction>>(preds) else {
            continue;
        };
        for pred in list {
            let key = format!("{}/{}", pred.platform, pred.model);
            if !seen.insert(key) {
                continue;
            }
            items.push(prediction_to_item(&pred));
        }
    }
    Ok(items)
}

/// Fetch a single model profile (Explore detail page).
pub async fn get_model_profile(
    ctx: &SiteContext,
    platform: &str,
    model: &str,
) -> AppResult<CamgirlfinderProfile> {
    let path = format!(
        "/models/{}/{}",
        urlencoding_minimal(platform),
        urlencoding_minimal(model)
    );
    let body = api_get(ctx, &path).await?;
    let profile: ModelProfile = serde_json::from_str(&body)
        .map_err(|e| AppError::Site(format!("CamGirlFinder profile parse: {e}")))?;
    let playable = match profile.platform.as_str() {
        "cb" => Some(format!("https://chaturbate.com/{}/", profile.name)),
        "sc" => Some(format!("https://stripchat.com/{}/", profile.name)),
        _ => None,
    };
    Ok(CamgirlfinderProfile {
        name: profile.name,
        platform: profile.platform,
        gender: profile.gender,
        first_seen: profile.first_seen,
        last_seen: profile.last_seen,
        profile_url: profile.urls.profile,
        external_profile: profile.urls.external_profile,
        playable_url: playable,
        persons: profile
            .persons
            .into_iter()
            .map(|p| CamgirlfinderPerson {
                person: p.person,
                faces: p.faces,
                seen: p.seen,
                first_seen: p.first_seen,
                last_seen: p.last_seen,
                face_image: p.urls.face_image,
                full_image: p.urls.full_image,
            })
            .collect(),
        schedule: profile.schedule,
    })
}

/// Face search: POST image bytes, poll job until finished.
pub async fn face_search(ctx: &SiteContext, image_bytes: &[u8]) -> AppResult<Vec<MediaItem>> {
    if image_bytes.is_empty() {
        return Err(AppError::InvalidInput("Empty image".into()));
    }
    if image_bytes.len() > 10 * 1024 * 1024 {
        return Err(AppError::InvalidInput("Image must be under 10 MiB".into()));
    }
    let url = format!("{API_BASE}/search");
    let resp = ctx
        .client
        .post(&url)
        .header("User-Agent", ARCHHIVE_UA)
        .header("Content-Type", "application/octet-stream")
        .header("Accept", "application/json")
        .body(image_bytes.to_vec())
        .send()
        .await
        .map_err(|e| AppError::Site(format!("CamGirlFinder face search failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| AppError::Site(format!("CamGirlFinder face search read: {e}")))?;
    if status.as_u16() == 429 {
        return Err(AppError::Site(
            "CamGirlFinder rate limit (429). Wait a moment and try again.".into(),
        ));
    }
    if !status.is_success() {
        return Err(AppError::Site(format!(
            "CamGirlFinder face search HTTP {status}: {}",
            body.chars().take(200).collect::<String>()
        )));
    }
    let mut job: JobResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::Site(format!("CamGirlFinder job parse: {e}")))?;

    // Job id may be in the initial response as `id`
    let job_id = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v.get("id").and_then(|i| i.as_str().map(str::to_string)));

    let mut attempts = 0u32;
    while job.status == "active" && attempts < 40 {
        attempts += 1;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let Some(ref id) = job_id else { break };
        let poll = api_get(ctx, &format!("/jobs/{id}")).await?;
        job = serde_json::from_str(&poll)
            .map_err(|e| AppError::Site(format!("CamGirlFinder job poll: {e}")))?;
    }

    match job.status.as_str() {
        "finished" => Ok(job.predictions.iter().map(prediction_to_item).collect()),
        "noface" => Err(AppError::Site(
            "No face detected in the image. Try a clearer face crop.".into(),
        )),
        "failed" => Err(AppError::Site(format!(
            "CamGirlFinder face search failed: {}",
            job.error.unwrap_or_else(|| "unknown".into())
        ))),
        other => Err(AppError::Site(format!(
            "CamGirlFinder job stuck in status '{other}'"
        ))),
    }
}

fn hit_to_item(hit: &ModelSearchHit) -> MediaItem {
    let thumb = hit
        .persons
        .first()
        .and_then(|p| p.urls.as_ref())
        .and_then(|u| u.full_image.clone().or(u.face_image.clone()));
    let external = hit.urls.external_profile.clone();
    let url = playable_or_external(&hit.platform, &hit.name, &external);
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: hit.name.clone(),
        url,
        thumbnail: thumb,
        duration: None,
        site_id: "camgirlfinder".to_string(),
        performers: vec![hit.name.clone()],
        tags: vec![
            format!("platform:{}", hit.platform),
            format!("gender:{}", hit.gender),
        ],
        description: hit
            .last_seen
            .as_ref()
            .map(|s| format!("Last seen: {s}"))
            .or(Some(format!("external:{external}"))),
        channel: Some(hit.name.clone()),
        is_live: None,
        viewers: None,
        age: None,
        gender: Some(gender_label(&hit.gender)),
        stream_url: None,
        embed_url: None,
    }
}

fn profile_to_item(profile: &ModelProfile) -> MediaItem {
    let thumb = profile
        .persons
        .first()
        .and_then(|p| p.urls.full_image.clone().or(p.urls.face_image.clone()));
    let external = profile.urls.external_profile.clone();
    let url = playable_or_external(&profile.platform, &profile.name, &external);
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: profile.name.clone(),
        url,
        thumbnail: thumb,
        duration: None,
        site_id: "camgirlfinder".to_string(),
        performers: vec![profile.name.clone()],
        tags: vec![
            format!("platform:{}", profile.platform),
            format!("gender:{}", profile.gender),
        ],
        description: profile
            .last_seen
            .as_ref()
            .map(|s| format!("Last seen: {s}")),
        channel: Some(profile.name.clone()),
        is_live: None,
        viewers: None,
        age: None,
        gender: Some(gender_label(&profile.gender)),
        stream_url: None,
        embed_url: None,
    }
}

fn prediction_to_item(pred: &Prediction) -> MediaItem {
    let external = pred.urls.external_profile.clone();
    let url = playable_or_external(&pred.platform, &pred.model, &external);
    let mut tags = vec![
        format!("platform:{}", pred.platform),
        format!("gender:{}", pred.gender),
    ];
    if let Some(ref p) = pred.probability {
        tags.push(format!("match:{p}"));
    }
    MediaItem {
        id: Uuid::new_v4().to_string(),
        title: pred.model.clone(),
        url,
        thumbnail: pred
            .urls
            .full_image
            .clone()
            .or(pred.urls.face_image.clone()),
        duration: None,
        site_id: "camgirlfinder".to_string(),
        performers: vec![pred.model.clone()],
        tags,
        description: Some(format!(
            "distance:{:.3}{}",
            pred.distance,
            pred.account_seen
                .as_ref()
                .map(|s| format!(" · last seen {s}"))
                .unwrap_or_default()
        )),
        channel: Some(pred.model.clone()),
        is_live: None,
        viewers: None,
        age: None,
        gender: Some(gender_label(&pred.gender)),
        stream_url: None,
        embed_url: None,
    }
}

fn gender_label(g: &str) -> String {
    match g {
        "f" => "female".into(),
        "m" => "male".into(),
        "c" => "couple".into(),
        "t" => "trans".into(),
        other => other.to_string(),
    }
}

fn parse_filters_from_slug(slug: &str) -> (Option<String>, Option<String>) {
    let mut platform = None;
    let mut gender = None;
    for part in slug.split([',', '&', ' ']) {
        let part = part.trim();
        if let Some(p) = part.strip_prefix("platform:") {
            if !p.is_empty() {
                platform = Some(p.to_string());
            }
        } else if let Some(g) = part.strip_prefix("gender:") {
            if matches!(g, "f" | "m" | "c" | "t") {
                gender = Some(g.to_string());
            }
        }
    }
    (platform, gender)
}

fn split_platform_model(s: &str) -> AppResult<(String, String)> {
    let mut parts = s.trim().trim_matches('/').splitn(2, '/');
    let platform = parts.next().unwrap_or("").to_string();
    let model = parts.next().unwrap_or("").to_string();
    if platform.is_empty() || model.is_empty() {
        return Err(AppError::InvalidInput(
            "similar: needs platform/model (e.g. similar:cb/username)".into(),
        ));
    }
    Ok((platform, model))
}

fn urlencoding_minimal(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cb_sc_rooms() {
        let (site, url) = resolve_underlying_room("https://api.camgirlfinder.net/out/cb/sexyana04")
            .unwrap();
        assert_eq!(site, "chaturbate");
        assert!(url.contains("chaturbate.com/sexyana04"));

        let (site, url) = resolve_underlying_room("https://api.camgirlfinder.net/out/sc/Maddie")
            .unwrap();
        assert_eq!(site, "stripchat");
        assert!(url.contains("stripchat.com/Maddie"));
    }

    #[test]
    fn rejects_unsupported_platform() {
        assert!(resolve_underlying_room("https://api.camgirlfinder.net/out/mfc/x").is_err());
    }

    #[test]
    fn parses_filter_slug() {
        let (p, g) = parse_filters_from_slug("platform:cb,gender:f");
        assert_eq!(p.as_deref(), Some("cb"));
        assert_eq!(g.as_deref(), Some("f"));
    }

    #[test]
    fn parses_search_fixture() {
        let json = r#"[
          {
            "name": "Kati3kat",
            "platform": "mfc",
            "gender": "f",
            "distance": 0,
            "faces": 1,
            "firstSeen": "2015-07-01T03:05:00Z",
            "lastSeen": "2023-12-01T08:22:39Z",
            "persons": [],
            "urls": {
              "externalProfile": "https://api.camgirlfinder.net/out/mfc/Kati3kat",
              "profile": "https://camgirlfinder.net/models/mfc/Kati3kat"
            }
          }
        ]"#;
        let hits: Vec<ModelSearchHit> = serde_json::from_str(json).unwrap();
        let item = hit_to_item(&hits[0]);
        assert_eq!(item.title, "Kati3kat");
        assert!(item.url.contains("camgirlfinder.net/out/mfc"));
        assert!(item.is_live.is_none());
    }
}
