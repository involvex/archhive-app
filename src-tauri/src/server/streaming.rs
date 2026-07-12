use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::error::{AppError, AppResult};

/// Resolve a relative path under `library_root` and reject traversal escapes.
pub fn resolve_under_library(library_root: &Path, relative: &str) -> AppResult<PathBuf> {
    let trimmed = relative.trim().trim_start_matches(['/', '\\']);
    if trimmed.is_empty() {
        return Ok(library_root.to_path_buf());
    }
    let mut joined = library_root.to_path_buf();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => {}
            _ => {
                return Err(AppError::InvalidInput(
                    "Path must stay within the library directory.".into(),
                ));
            }
        }
    }
    let canonical_root = library_root
        .canonicalize()
        .unwrap_or_else(|_| library_root.to_path_buf());
    let canonical = joined
        .canonicalize()
        .map_err(|_| AppError::NotFound("Path not found".into()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::InvalidInput(
            "Path must stay within the library directory.".into(),
        ));
    }
    Ok(canonical)
}

pub fn mime_from_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

pub fn is_video_path(path: &Path) -> bool {
    matches!(
        mime_from_path(path),
        "video/mp4"
            | "video/webm"
            | "video/x-matroska"
            | "video/x-msvideo"
            | "video/quicktime"
            | "video/x-ms-wmv"
            | "video/x-flv"
    )
}

fn parse_range_header(range: &str, size: u64) -> Option<(u64, u64)> {
    let bytes = range.strip_prefix("bytes=")?;
    let (start_s, end_s) = bytes.split_once('-')?;
    if start_s.is_empty() {
        let suffix: u64 = end_s.parse().ok()?;
        if suffix == 0 || suffix > size {
            return None;
        }
        let start = size.saturating_sub(suffix);
        return Some((start, size - 1));
    }
    let start: u64 = start_s.parse().ok()?;
    let end = if end_s.is_empty() {
        size.saturating_sub(1)
    } else {
        end_s.parse().ok()?
    };
    if start > end || end >= size {
        return None;
    }
    Some((start, end))
}

/// Stream a file with optional HTTP Range support (for video seeking).
pub async fn serve_file_with_range(path: &Path, headers: &HeaderMap) -> Result<Response, StatusCode> {
    let meta = tokio::fs::metadata(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !meta.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    let size = meta.len();
    let content_type = mime_from_path(path);

    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|r| parse_range_header(r, size));

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if let Some((start, end)) = range {
        let len = end - start + 1;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let chunk = file.take(len);
        let stream = ReaderStream::with_capacity(chunk, 256 * 1024);
        let body = Body::from_stream(stream);
        return Ok(Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONNECTION, "keep-alive")
            .header(header::CACHE_CONTROL, "no-cache")
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{size}"),
            )
            .header(header::CONTENT_LENGTH, len.to_string())
            .body(body)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }

    let stream = ReaderStream::with_capacity(file, 256 * 1024);
    let body = Body::from_stream(stream);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONNECTION, "keep-alive")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONTENT_LENGTH, size.to_string())
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub fn content_disposition_inline(path: &Path) -> HeaderValue {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    HeaderValue::from_str(&format!("inline; filename=\"{name}\""))
        .unwrap_or_else(|_| HeaderValue::from_static("inline"))
}
