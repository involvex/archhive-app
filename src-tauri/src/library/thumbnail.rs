use std::path::{Path, PathBuf};
use std::time::Duration;

/// Minimum valid thumbnail size in bytes (256 bytes filters out error pages).
const MIN_THUMB_BYTES: usize = 256;
/// Maximum thumbnail size we'll accept (5 MB guard against huge images).
const MAX_THUMB_BYTES: usize = 5 * 1024 * 1024;
/// Per-request timeout for thumbnail downloads.
const THUMB_TIMEOUT: Duration = Duration::from_secs(15);
/// Maximum retries for transient network errors.
const MAX_RETRIES: u32 = 2;

/// Download a remote thumbnail to a sidecar `.jpg` next to the video file.
///
/// Uses a proper User-Agent, retries on transient failures, and validates
/// that the response is a reasonable image before writing.
pub async fn download_remote_thumbnail(thumbnail_url: &str, video_path: &Path) -> Option<PathBuf> {
    if thumbnail_url.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(THUMB_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .ok()?;

    let mut last_error: Option<String> = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            // Exponential backoff: 500ms, 1000ms, ...
            let delay = Duration::from_millis(500 * attempt as u64);
            tokio::time::sleep(delay).await;
        }

        match client.get(thumbnail_url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    last_error = Some(format!("HTTP {}", resp.status()));
                    continue;
                }

                // Validate content-type is an image (if server provides it).
                if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                    if let Ok(ct_str) = ct.to_str() {
                        if !ct_str.contains("image/") && !ct_str.contains("octet-stream") {
                            last_error = Some(format!("Not an image: {ct_str}"));
                            continue;
                        }
                    }
                }

                let bytes = match resp.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        last_error = Some(e.to_string());
                        continue;
                    }
                };

                if bytes.len() < MIN_THUMB_BYTES {
                    last_error = Some(format!("Too small: {} bytes", bytes.len()));
                    continue;
                }

                if bytes.len() > MAX_THUMB_BYTES {
                    last_error = Some(format!("Too large: {} bytes", bytes.len()));
                    continue;
                }

                // Validate JPEG/PNG magic bytes.
                if !is_valid_image_bytes(&bytes) {
                    last_error = Some("Not a valid image (bad magic bytes)".into());
                    continue;
                }

                let thumb_path = thumb_path_for_video(video_path);
                if std::fs::write(&thumb_path, &bytes).is_ok() {
                    return Some(thumb_path);
                }
                last_error = Some("Failed to write file".into());
            }
            Err(e) => {
                last_error = Some(e.to_string());
                continue;
            }
        }
    }

    if let Some(err) = last_error {
        tracing::debug!("thumbnail download failed for {thumbnail_url}: {err}");
    }
    None
}

/// Check if bytes start with JPEG or PNG magic bytes.
fn is_valid_image_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    // JPEG: FF D8 FF
    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return true;
    }
    // PNG: 89 50 4E 47
    if bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47 {
        return true;
    }
    // GIF: 47 49 46 38
    if bytes[0] == 0x47 && bytes[1] == 0x49 && bytes[2] == 0x46 && bytes[3] == 0x38 {
        return true;
    }
    // WebP: RIFF....WEBP
    if bytes.len() >= 12
        && bytes[0] == 0x52
        && bytes[1] == 0x49
        && bytes[2] == 0x46
        && bytes[3] == 0x46
        && &bytes[8..12] == b"WEBP"
    {
        return true;
    }
    false
}

fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("thumb");
    video_path.with_file_name(format!("{stem}.jpg"))
}
