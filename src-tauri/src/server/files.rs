use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::streaming::{mime_from_path, resolve_under_library, serve_file_with_range};
use super::ApiState;

#[derive(Deserialize)]
pub struct FilesPathQuery {
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub mime: Option<String>,
}

#[derive(Serialize)]
pub struct FilesListResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

fn library_root(state: &ApiState) -> Result<std::path::PathBuf, StatusCode> {
    let settings = state.app.get_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::state::AppState::validate_library_path(&settings.library_path, &state.app.data_dir)
        .map(std::path::PathBuf::from)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

fn relative_path(library_root: &Path, absolute: &Path) -> String {
    absolute
        .strip_prefix(library_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

pub async fn list_files(
    State(state): State<ApiState>,
    Query(query): Query<FilesPathQuery>,
) -> Result<Json<FilesListResponse>, StatusCode> {
    let root = library_root(&state)?;
    let rel = query.path.unwrap_or_default();
    let dir = resolve_under_library(&root, &rel).map_err(|_| StatusCode::FORBIDDEN)?;
    if !dir.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&dir).map_err(|_| StatusCode::NOT_FOUND)?;
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta
            .as_ref()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        let abs = entry.path();
        let rel_path = relative_path(&root, &abs);
        let mime = if is_dir {
            None
        } else {
            Some(mime_from_path(&abs).to_string())
        };
        entries.push(FileEntry {
            name,
            path: rel_path,
            is_dir,
            size,
            mime,
        });
    }
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    let current = relative_path(&root, &dir);
    Ok(Json(FilesListResponse {
        path: current,
        entries,
    }))
}

pub async fn stream_file(
    State(state): State<ApiState>,
    Query(query): Query<FilesPathQuery>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let rel = query
        .path
        .filter(|p| !p.trim().is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let root = library_root(&state)?;
    let file_path = resolve_under_library(&root, &rel).map_err(|_| StatusCode::FORBIDDEN)?;
    if !file_path.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut response = serve_file_with_range(&file_path, &headers).await?;
    let cd = super::streaming::content_disposition_inline(&file_path);
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, cd);
    Ok(response)
}
