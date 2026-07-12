use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct ApiState {
    app: Arc<AppState>,
    token: String,
}

#[derive(Deserialize)]
struct BrowseParams {
    kind: String,
    slug: String,
    page: Option<u32>,
    orientation: Option<String>,
}

#[derive(Deserialize)]
struct QueueBody {
    url: String,
    adapter: Option<String>,
}

#[derive(Deserialize)]
struct ScenesQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
struct CategoriesQuery {
    orientation: Option<String>,
}

#[derive(Deserialize)]
struct CookieBody {
    cookies: String,
}

#[derive(Deserialize)]
struct MergeDuplicatesBody {
    keep_id: String,
    remove_ids: Vec<String>,
    delete_files: Option<bool>,
}

mod files;
mod streaming;

use files::{list_files, stream_file};
use streaming::serve_file_with_range;

use crate::models::{UpdateSceneRequest, BatchUpdateScenesRequest};
use serde::Deserialize;

pub struct LanServer {
    shutdown: Option<oneshot::Sender<()>>,
    _mdns: Option<ServiceDaemon>,
}

impl LanServer {
    pub async fn start(
        app: Arc<AppState>,
        port: u16,
        token: String,
        static_dir: Option<PathBuf>,
    ) -> AppResult<Self> {
        let api = ApiState { app, token: token.clone() };
        let api_router = Router::new()
            .route("/api/health", get(health))
            .route("/api/sites", get(list_sites))
            .route("/api/sites/{id}/browse", get(browse))
            .route("/api/downloads", get(list_downloads).post(queue_download))
            .route("/api/downloads/{id}/cancel", post(cancel_download))
            .route("/api/scenes", get(list_scenes))
            .route("/api/scenes/{id}", get(get_scene).patch(update_scene))
            .route("/api/scenes/{id}/thumb", get(scene_thumb))
            .route("/api/scenes/{id}/media", get(scene_media))
            .route("/api/scenes/batch", post(batch_update_scenes))
            .route("/api/files", get(list_files))
            .route("/api/files/stream", get(stream_file))
            .route("/api/sites/pornhub/categories", get(pornhub_categories))
            .route("/api/performers", get(list_performers))
            .route("/api/tags", get(list_tags))
            .route("/api/duplicates", get(list_duplicates))
            .route("/api/duplicates/merge", post(merge_duplicates))
            .route("/api/cookies", get(list_cookies))
            .route("/api/cookies/{id}", post(save_cookies).delete(delete_cookies))
            .route("/api/settings", get(get_settings).put(put_settings))
            .route("/api/library/scan", post(scan_library))
            .layer(middleware::from_fn_with_state(api.clone(), auth_middleware))
            .with_state(api);

        let router = if let Some(dir) = static_dir.filter(|d| d.join("index.html").exists()) {
            let index = dir.join("index.html");
            Router::new()
                .merge(api_router)
                .fallback_service(
                    ServeDir::new(dir).not_found_service(ServeFile::new(index)),
                )
        } else {
            api_router
        };

        let router = router.layer(CorsLayer::permissive());

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let (tx, rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .ok();
        });

        let mdns = advertise_mdns(port).ok();

        Ok(Self {
            shutdown: Some(tx),
            _mdns: mdns,
        })
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

fn advertise_mdns(port: u16) -> AppResult<ServiceDaemon> {
    let mdns = ServiceDaemon::new().map_err(|e| AppError::Other(e.to_string()))?;
    let ip = crate::discovery::local_ipv4_for_mdns();
    let ip_str = ip.to_string();
    let host_name = format!("{ip_str}.local.");
    let properties = [("version", env!("CARGO_PKG_VERSION"))];
    let info = ServiceInfo::new(
        crate::discovery::SERVICE_TYPE,
        "ArcHive",
        &host_name,
        &ip_str,
        port,
        &properties[..],
    )
    .map_err(|e| AppError::Other(e.to_string()))?;
    mdns.register(info)
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(mdns)
}

async fn auth_middleware(
    State(state): State<ApiState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if req.uri().path() == "/api/health" || !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }
    if state.token.is_empty() {
        return next.run(req).await;
    }
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let expected = format!("Bearer {}", state.token);
    if auth != expected {
        let query_ok = req.uri().query().is_some_and(|q| {
            url::form_urlencoded::parse(q.as_bytes()).any(|(k, v)| {
                k == "token" && v == state.token
            })
        });
        if !query_ok {
            return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
        }
    }
    next.run(req).await
}

async fn health(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let h = AppState::health();
    let settings = state.app.get_settings().ok();
    let library_path = settings.as_ref().map(|s| s.library_path.clone());
    let port = settings.as_ref().map(|s| s.lan_port).unwrap_or(8787);
    let ip = crate::discovery::local_ipv4_for_mdns();
    let lan_url = format!("http://{ip}:{port}");
    Json(serde_json::json!({
        "status": h.status,
        "version": h.version,
        "lan": true,
        "auth_required": !state.token.is_empty(),
        "library_path": library_path,
        "lan_url": lan_url,
    }))
}

async fn list_sites(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let sites = state.app.list_sites().await;
    Json(serde_json::json!(sites))
}

async fn browse(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(params): Query<BrowseParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kind = parse_kind(&params.kind).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let orientation = params
        .orientation
        .as_deref()
        .map(parse_orientation)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let page = state
        .app
        .browse(&id, kind, &params.slug, params.page.unwrap_or(1), orientation)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(serde_json::json!(page)))
}

async fn list_downloads(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let jobs = state
        .app
        .list_downloads()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(jobs)))
}

async fn queue_download(
    State(state): State<ApiState>,
    Json(body): Json<QueueBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let job = state
        .app
        .queue_download(&body.url, body.adapter.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(job)))
}

async fn cancel_download(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state.app.cancel_download(&id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_scenes(
    State(state): State<ApiState>,
    Query(q): Query<ScenesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let scenes = state
        .app
        .list_scenes(q.q.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(scenes)))
}

async fn get_scene(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let scene = state
        .app
        .get_scene(&id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::json!(scene)))
}

async fn update_scene(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSceneRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let scene = state
        .app
        .update_scene(
            &id,
            body.title.as_deref(),
            body.performers.as_deref(),
            body.tags.as_deref(),
            body.rename_file.unwrap_or(false),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(scene)))
}

async fn scene_thumb(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Response, StatusCode> {
    let scene = state
        .app
        .get_scene(&id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let thumb_path = scene
        .thumb
        .or(scene.path)
        .ok_or(StatusCode::NOT_FOUND)?;
    let bytes = tokio::fs::read(&thumb_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let content_type = if thumb_path.to_lowercase().ends_with(".png") {
        "image/png"
    } else if thumb_path.to_lowercase().ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

async fn scene_media(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<Response, StatusCode> {
    let scene = state
        .app
        .get_scene(&id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let media_path = scene.path.ok_or(StatusCode::NOT_FOUND)?;
    let path = std::path::Path::new(&media_path);
    if !path.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    let settings = state
        .app
        .get_settings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let library_root = AppState::validate_library_path(&settings.library_path, &state.app.data_dir)
        .map(std::path::PathBuf::from)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let canonical_root = library_root
        .canonicalize()
        .unwrap_or(library_root);
    let canonical_file = path
        .canonicalize()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(StatusCode::FORBIDDEN);
    }
    serve_file_with_range(&canonical_file, &headers).await
}

async fn batch_update_scenes(
    State(state): State<ApiState>,
    Json(body): Json<BatchUpdateScenesRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let result = state
        .app
        .batch_update_scenes(
            &body.scene_ids,
            body.performers_add.as_deref(),
            body.tags_add.as_deref(),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(result)))
}

async fn pornhub_categories(
    State(state): State<ApiState>,
    Query(q): Query<CategoriesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let orientation = q
        .orientation
        .as_deref()
        .map(parse_orientation)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .unwrap_or(crate::models::BrowseOrientation::Straight);
    let categories = state
        .app
        .list_pornhub_categories(orientation)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(categories)))
}

async fn list_performers(
    State(state): State<ApiState>,
    Query(q): Query<ScenesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let performers = state
        .app
        .list_performers(q.q.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(performers)))
}

async fn list_tags(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let tags = state.app.list_tags().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(tags)))
}

async fn list_duplicates(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let groups = state
        .app
        .find_duplicates()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(groups)))
}

async fn merge_duplicates(
    State(state): State<ApiState>,
    Json(body): Json<MergeDuplicatesBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let result = state
        .app
        .merge_duplicates(&body.keep_id, &body.remove_ids, body.delete_files.unwrap_or(false))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(result)))
}

async fn list_cookies(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let sites = state
        .app
        .list_cookie_sites()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(sites)))
}

async fn save_cookies(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<CookieBody>,
) -> Result<StatusCode, StatusCode> {
    state
        .app
        .save_site_cookies(&id, &body.cookies)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_cookies(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .app
        .delete_site_cookies(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_settings(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let settings = state
        .app
        .get_settings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(settings)))
}

async fn put_settings(
    State(state): State<ApiState>,
    Json(body): Json<crate::models::AppSettings>,
) -> Result<StatusCode, StatusCode> {
    state
        .app
        .save_settings(&body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn scan_library(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let app = state.app.clone();
    let result = tokio::task::spawn_blocking(move || app.scan_library())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(result)))
}

fn parse_kind(s: &str) -> AppResult<crate::models::BrowseKind> {
    use crate::models::BrowseKind;
    Ok(match s {
        "tag" => BrowseKind::Tag,
        "model" => BrowseKind::Model,
        "channel" => BrowseKind::Channel,
        "search" => BrowseKind::Search,
        "video" => BrowseKind::Video,
        "category" => BrowseKind::Category,
        _ => return Err(AppError::InvalidInput(format!("unknown kind: {s}"))),
    })
}

fn parse_orientation(s: &str) -> AppResult<crate::models::BrowseOrientation> {
    use crate::models::BrowseOrientation;
    Ok(match s {
        "straight" => BrowseOrientation::Straight,
        "gay" => BrowseOrientation::Gay,
        "lesbian" => BrowseOrientation::Lesbian,
        "transgender" => BrowseOrientation::Transgender,
        _ => {
            return Err(AppError::InvalidInput(format!(
                "unknown orientation: {s}"
            )));
        }
    })
}

pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[idx] as char
        })
        .collect()
}
