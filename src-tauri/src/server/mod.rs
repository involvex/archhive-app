use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::{
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
struct CookieBody {
    cookies: String,
}

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
            .route("/api/performers", get(list_performers))
            .route("/api/tags", get(list_tags))
            .route("/api/duplicates", get(list_duplicates))
            .route("/api/cookies", get(list_cookies))
            .route("/api/cookies/{id}", post(save_cookies).delete(delete_cookies))
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
    let host = "scrawler-host";
    let service_type = "_scrawler._tcp.local.";
    let instance = "Scrawler";
    let info = ServiceInfo::new(
        service_type,
        instance,
        &format!("{host}.local."),
        "",
        port,
        None,
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
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let expected = format!("Bearer {}", state.token);
    if auth != expected {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    next.run(req).await
}

async fn health(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let h = AppState::health();
    Json(serde_json::json!({
        "status": h.status,
        "version": h.version,
        "lan": true,
        "library_path": state.app.get_settings().ok().map(|s| s.library_path),
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
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = parse_kind(&params.kind).map_err(|_| StatusCode::BAD_REQUEST)?;
    let page = state
        .app
        .browse(&id, kind, &params.slug, params.page.unwrap_or(1))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

fn parse_kind(s: &str) -> AppResult<crate::models::BrowseKind> {
    use crate::models::BrowseKind;
    Ok(match s {
        "tag" => BrowseKind::Tag,
        "model" => BrowseKind::Model,
        "channel" => BrowseKind::Channel,
        "search" => BrowseKind::Search,
        "video" => BrowseKind::Video,
        _ => return Err(AppError::InvalidInput(format!("unknown kind: {s}"))),
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
