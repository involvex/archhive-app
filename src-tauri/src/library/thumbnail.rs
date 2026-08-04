use std::path::{Path, PathBuf};

pub async fn download_remote_thumbnail(thumbnail_url: &str, video_path: &Path) -> Option<PathBuf> {
    if thumbnail_url.is_empty() {
        return None;
    }
    let resp = reqwest::get(thumbnail_url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 256 {
        return None;
    }
    let thumb_path = thumb_path_for_video(video_path);
    std::fs::write(&thumb_path, &bytes).ok()?;
    Some(thumb_path)
}

fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("thumb");
    video_path
        .with_file_name(format!("{stem}.jpg"))
}
