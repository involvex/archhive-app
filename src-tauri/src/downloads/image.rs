use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

const IMAGE_EXTS: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];

pub fn is_direct_image_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    if lower.contains("i.redd.it") || lower.contains("i.imgur.com") {
        return true;
    }
    if let Ok(parsed) = url::Url::parse(&lower) {
        let path = parsed.path().to_lowercase();
        return IMAGE_EXTS.iter().any(|ext| path.ends_with(ext));
    }
    false
}

pub fn resolve_download_tool(url: &str, adapter: &str) -> crate::models::DownloadTool {
    use crate::models::DownloadTool;
    if adapter == "redgifs" {
        return DownloadTool::GalleryDl;
    }
    let lower = url.to_lowercase();
    if is_direct_image_url(&lower) {
        return DownloadTool::DirectHttp;
    }
    if lower.contains("v.redd.it") {
        return DownloadTool::YtDlp;
    }
    if lower.contains("reddit.com") && lower.contains("/comments/") {
        return DownloadTool::GalleryDl;
    }
    if lower.contains("redd.it") && !lower.contains("/comments/") {
        return DownloadTool::DirectHttp;
    }
    DownloadTool::YtDlp
}

pub async fn download_direct(
    url: &str,
    output_dir: &str,
    title: Option<&str>,
) -> AppResult<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; ArcHive/1.0)")
        .build()
        .map_err(|e| AppError::Download(e.to_string()))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Download(format!("fetch image: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::Download(format!(
            "HTTP {} for image URL",
            resp.status()
        )));
    }

    let ext = extension_from_response(url, resp.headers().get(reqwest::header::CONTENT_TYPE));
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Download(format!("read image body: {e}")))?;

    std::fs::create_dir_all(output_dir)?;
    let base = title
        .map(sanitize_filename)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            url::Url::parse(url)
                .ok()
                .and_then(|u| {
                    u.path_segments()
                        .and_then(|s| s.last().map(|s| s.to_string()))
                })
                .map(|s| sanitize_filename(&s))
                .unwrap_or_else(|| "image".to_string())
        });
    let filename = if base.contains('.') {
        base
    } else {
        format!("{base}{ext}")
    };
    let path = Path::new(output_dir).join(&filename);
    let path = unique_path(path);
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| AppError::Download(format!("write image: {e}")))?;
    Ok(path.to_string_lossy().to_string())
}

fn extension_from_response(
    url: &str,
    content_type: Option<&reqwest::header::HeaderValue>,
) -> String {
    if let Some(ct) = content_type.and_then(|v| v.to_str().ok()) {
        if ct.contains("png") {
            return ".png".to_string();
        }
        if ct.contains("gif") {
            return ".gif".to_string();
        }
        if ct.contains("webp") {
            return ".webp".to_string();
        }
        if ct.contains("jpeg") || ct.contains("jpg") {
            return ".jpg".to_string();
        }
    }
    let lower = url.to_lowercase();
    for ext in IMAGE_EXTS {
        if lower.ends_with(ext) {
            return ext.to_string();
        }
    }
    ".jpg".to_string()
}

fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else if c == ' ' {
                '_'
            } else {
                '_'
            }
        })
        .collect();
    safe.trim_matches('_').to_string()
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for i in 1..1000 {
        let candidate = parent.join(format!("{stem}_{i}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direct_image_urls() {
        assert!(is_direct_image_url("https://i.redd.it/abc123.jpeg"));
        assert!(is_direct_image_url("https://example.com/photo.png"));
        assert!(!is_direct_image_url(
            "https://www.reddit.com/r/pics/comments/abc/title/"
        ));
    }

    #[test]
    fn reddit_comment_uses_gallery_dl() {
        use crate::models::DownloadTool;
        let tool = resolve_download_tool(
            "https://www.reddit.com/r/pics/comments/abc/title/",
            "reddit",
        );
        assert_eq!(tool, DownloadTool::GalleryDl);
    }

    #[test]
    fn v_redd_it_uses_ytdlp() {
        use crate::models::DownloadTool;
        let tool = resolve_download_tool("https://v.redd.it/abc123", "reddit");
        assert_eq!(tool, DownloadTool::YtDlp);
    }
}
