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
    if lower.contains(".m3u8") {
        return DownloadTool::FfmpegHls;
    }
    DownloadTool::YtDlp
}

/// Direct HTTP download with optional Referer/cookies and progress.
///
/// Streams the body to disk in chunks (safe for multi-hundred-MB videos),
/// unlike the one-shot buffered write. `on_progress(downloaded, total)`
/// is invoked per chunk; callers should throttle UI updates themselves.
pub async fn download_direct(
    url: &str,
    output_dir: &str,
    title: Option<&str>,
    referer: Option<&str>,
    cookie_header: Option<&str>,
    mut on_progress: Option<impl FnMut(u64, Option<u64>)>,
) -> AppResult<String> {
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; ArcHive/1.0)")
        .build()
        .map_err(|e| AppError::Download(e.to_string()))?;

    let mut req = client.get(url);
    if let Some(referer) = referer {
        req = req.header(reqwest::header::REFERER, referer);
    }
    if let Some(cookies) = cookie_header {
        if !cookies.is_empty() {
            req = req.header(reqwest::header::COOKIE, cookies);
        }
    }
    let mut resp = req
        .send()
        .await
        .map_err(|e| AppError::Download(format!("fetch file: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::Download(format!(
            "HTTP {} for file URL",
            resp.status()
        )));
    }

    let total = resp.content_length();
    let ext = extension_from_response(url, resp.headers().get(reqwest::header::CONTENT_TYPE));

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
                .unwrap_or_else(|| "download".to_string())
        });
    let filename = if base.contains('.') {
        base
    } else {
        format!("{base}{ext}")
    };
    let path = Path::new(output_dir).join(&filename);
    let path = unique_path(path);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| AppError::Download(format!("create file: {e}")))?;

    let mut downloaded: u64 = 0;
    loop {
        match resp
            .chunk()
            .await
            .map_err(|e| AppError::Download(format!("read body: {e}")))?
        {
            Some(bytes) => {
                file.write_all(&bytes)
                    .await
                    .map_err(|e| AppError::Download(format!("write file: {e}")))?;
                downloaded += bytes.len() as u64;
                if let Some(cb) = on_progress.as_mut() {
                    cb(downloaded, total);
                }
            }
            None => break,
        }
    }
    file.flush()
        .await
        .map_err(|e| AppError::Download(format!("flush file: {e}")))?;
    Ok(path.to_string_lossy().to_string())
}

fn extension_from_response(
    url: &str,
    content_type: Option<&reqwest::header::HeaderValue>,
) -> String {
    if let Some(ct) = content_type.and_then(|v| v.to_str().ok()) {
        let ct = ct.to_lowercase();
        if ct.contains("mp4") {
            return ".mp4".to_string();
        }
        if ct.contains("webm") {
            return ".webm".to_string();
        }
        if ct.contains("quicktime") {
            return ".mov".to_string();
        }
        if ct.contains("x-m4v") || (ct.contains("m4v")) {
            return ".m4v".to_string();
        }
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
    // Compare against the URL path without query/fragment so
    // `.../video.mp4?token=...` still matches.
    let path = url
        .split(&['?', '#'][..])
        .next()
        .unwrap_or(url)
        .to_lowercase();
    for ext in IMAGE_EXTS {
        if path.ends_with(ext) {
            return ext.to_string();
        }
    }
    for ext in [".mp4", ".webm", ".m4v", ".mov"] {
        if path.ends_with(ext) {
            return ext.to_string();
        }
    }
    ".jpg".to_string()
}

pub(crate) fn sanitize_filename(name: &str) -> String {
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

pub(crate) fn unique_path(path: PathBuf) -> PathBuf {
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
