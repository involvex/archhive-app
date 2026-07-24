use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MEDIA_EXTS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp", ".mp4", ".webm", ".mkv", ".mov",
];

/// Snapshot of files under a directory before a gallery-dl run.
pub struct DirSnapshot {
    files: HashMap<PathBuf, SystemTime>,
}

impl DirSnapshot {
    pub fn capture(root: &str) -> AppResult<Self> {
        let root = Path::new(root);
        if !root.exists() {
            return Ok(Self {
                files: HashMap::new(),
            });
        }
        let mut files = HashMap::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path().to_path_buf();
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    files.insert(path, modified);
                }
            }
        }
        Ok(Self { files })
    }

    /// Files created or modified since the snapshot.
    pub fn new_files(&self, root: &str) -> AppResult<Vec<PathBuf>> {
        let root = Path::new(root);
        if !root.exists() {
            return Ok(vec![]);
        }
        let mut found = Vec::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path().to_path_buf();
            let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
            let is_new = match (self.files.get(&path), modified) {
                (None, Some(_)) => true,
                (Some(prev), Some(cur)) => cur > *prev,
                _ => false,
            };
            if is_new && is_media_file(&path) {
                found.push(path);
            }
        }
        found.sort_by(|a, b| {
            let ma = file_mtime(a).unwrap_or(SystemTime::UNIX_EPOCH);
            let mb = file_mtime(b).unwrap_or(SystemTime::UNIX_EPOCH);
            mb.cmp(&ma)
        });
        Ok(found)
    }
}

pub fn parse_gallery_dl_path(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("# ") {
        let path = rest.trim().trim_matches('"');
        if is_media_file(Path::new(path)) || path.contains('.') {
            return Some(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        }
    }
    if trimmed.starts_with("Writing ") {
        let path = trimmed.trim_start_matches("Writing ").trim();
        if !path.is_empty() {
            return Some(path.to_string());
        }
    }
    None
}

pub fn resolve_output_paths(
    parsed: &[String],
    snapshot: &DirSnapshot,
    output_dir: &str,
) -> AppResult<Vec<String>> {
    let mut paths: Vec<String> = parsed
        .iter()
        .filter_map(|p| normalize_output_path(p, output_dir))
        .collect();
    paths.sort();
    paths.dedup();

    if paths.is_empty() {
        paths = snapshot
            .new_files(output_dir)?
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
    }

    if paths.is_empty() {
        return Err(AppError::Download(
            "gallery-dl finished but no output files were found in the library folder.".into(),
        ));
    }

    Ok(paths)
}

fn normalize_output_path(raw: &str, output_dir: &str) -> Option<String> {
    let path = Path::new(raw.trim());
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(output_dir).join(path)
    };
    if candidate.exists() && candidate.is_file() {
        Some(candidate.to_string_lossy().to_string())
    } else {
        None
    }
}

fn is_media_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    MEDIA_EXTS.iter().any(|ext| name.ends_with(ext))
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_hash_prefixed_path() {
        let p = parse_gallery_dl_path("# .\\reddit\\pics\\001.jpg").unwrap();
        assert!(p.contains("001.jpg"));
    }

    #[test]
    fn parses_writing_line() {
        let p = parse_gallery_dl_path("Writing downloads/file.png").unwrap();
        assert_eq!(p, "downloads/file.png");
    }

    #[test]
    fn snapshot_empty_job_dir_is_cheap() {
        let dir = std::env::temp_dir().join(format!("archhive-dl-snap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let snap = DirSnapshot::capture(dir.to_str().unwrap()).unwrap();
        assert!(snap.files.is_empty());

        let file = dir.join("a.jpg");
        fs::write(&file, b"x").unwrap();
        let new = snap.new_files(dir.to_str().unwrap()).unwrap();
        assert_eq!(new.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
