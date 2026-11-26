//! Watch-folder helpers and (desktop) filesystem watcher for auto-import.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Same video extensions as [`super::LibraryScanner::scan`].
pub const DEFAULT_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v"];

/// Debounce quiet period before considering a path for import.
pub const DEBOUNCE: Duration = Duration::from_secs(2);

/// Gap between size samples to decide a file has finished writing.
pub const SIZE_SETTLE: Duration = Duration::from_secs(2);

/// Normalize user-supplied extensions (comma/newline/space separated OK when
/// callers split first). Empty input → defaults. Strips leading dots.
pub fn normalize_extensions(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = raw
        .iter()
        .map(|s| s.trim().trim_start_matches('.').to_lowercase())
        .filter(|s| !s.is_empty() && s.len() <= 16 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .collect();
    out.sort();
    out.dedup();
    if out.is_empty() {
        DEFAULT_EXTENSIONS.iter().map(|s| (*s).to_string()).collect()
    } else {
        out
    }
}

/// True for paths that look like incomplete downloads or temp sidecars.
pub fn is_incomplete_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name.is_empty() || name.starts_with('.') {
        return true;
    }
    if name.ends_with('~') {
        return true;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "part" | "ytdl" | "temp" | "tmp" | "crdownload" | "download" | "partial"
    )
}

/// True when the file extension is in the allowed (normalized) list.
pub fn extension_allowed(path: &Path, allowed: &[String]) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    !ext.is_empty() && allowed.iter().any(|a| a == &ext)
}

/// Resolve the directory to watch. `watch_folder_path` empty → library root.
/// Absolute or relative paths must canonicalize under `library_path`.
pub fn resolve_watch_root(
    library_path: &Path,
    watch_folder_path: Option<&str>,
) -> Result<PathBuf, String> {
    if library_path.as_os_str().is_empty() {
        return Err("library path is empty".into());
    }
    if !library_path.exists() {
        std::fs::create_dir_all(library_path)
            .map_err(|e| format!("create library root: {e}"))?;
    }
    let lib = library_path
        .canonicalize()
        .map_err(|e| format!("library root unreadable: {e}"))?;

    let raw = watch_folder_path.map(str::trim).filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return Ok(lib);
    };

    let candidate = {
        let p = Path::new(raw);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            lib.join(p)
        }
    };

    if !candidate.exists() {
        std::fs::create_dir_all(&candidate)
            .map_err(|e| format!("create watch folder: {e}"))?;
    }

    let canon = candidate
        .canonicalize()
        .map_err(|e| format!("watch path unreadable: {e}"))?;
    if !path_is_under(&lib, &canon) {
        return Err("watch folder must be inside the library folder".into());
    }
    Ok(canon)
}

/// `candidate` must already be canonical (or compared carefully). Uses
/// `starts_with` on the path components after both are absolute.
pub fn path_is_under(root: &Path, candidate: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cand = candidate
        .canonicalize()
        .unwrap_or_else(|_| candidate.to_path_buf());
    cand.starts_with(&root)
}

/// Depth of `path` relative to `root` (0 = file in root). Returns None if not under root.
pub fn relative_depth(root: &Path, path: &Path) -> Option<usize> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let rel = path.strip_prefix(&root).ok()?;
    // file depth = number of components in parent relative path
    let parent = rel.parent()?;
    Some(parent.components().count())
}

/// True when any ancestor directory is hidden / vault-marked.
pub fn is_under_hidden_dir(path: &Path) -> bool {
    for ancestor in path.ancestors().skip(1) {
        if super::is_hidden_dir(ancestor) {
            return true;
        }
    }
    false
}

/// Read file length twice with a settle delay; Ok(size) when stable and > 0.
pub fn wait_size_stable(path: &Path, settle: Duration) -> Result<u64, String> {
    let meta1 = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta1.is_file() {
        return Err("not a file".into());
    }
    let size1 = meta1.len();
    if size1 == 0 {
        return Err("empty file".into());
    }
    std::thread::sleep(settle);
    let meta2 = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let size2 = meta2.len();
    if size1 != size2 {
        return Err(format!("size still changing ({size1} → {size2})"));
    }
    Ok(size2)
}

/// Max WalkDir-equivalent nesting (file in library root = depth 0).
pub const MAX_WATCH_DEPTH: usize = 5;

/// Result of attempting to import one settled path.
#[derive(Debug, Clone)]
pub enum ImportAttempt {
    Imported { scene_id: String, title: String, path: String },
    Skipped(&'static str),
    Failed(String),
}

/// Shared import gate used by the desktop watcher after debounce + size settle.
pub fn try_import_settled_file(
    db: &crate::db::Database,
    library_root: &Path,
    file_path: &Path,
    rules: &[String],
    extensions: &[String],
) -> ImportAttempt {
    if is_incomplete_path(file_path) {
        return ImportAttempt::Skipped("incomplete");
    }
    if !extension_allowed(file_path, extensions) {
        return ImportAttempt::Skipped("extension");
    }
    if !path_is_under(library_root, file_path) {
        return ImportAttempt::Skipped("outside library");
    }
    if is_under_hidden_dir(file_path) {
        return ImportAttempt::Skipped("hidden/vault");
    }
    match relative_depth(library_root, file_path) {
        Some(d) if d <= MAX_WATCH_DEPTH => {}
        _ => return ImportAttempt::Skipped("too deep"),
    }
    if let Err(e) = wait_size_stable(file_path, SIZE_SETTLE) {
        return ImportAttempt::Failed(e);
    }

    match super::LibraryScanner::import_new_file(db, file_path, rules) {
        Ok(Some(scene_id)) => {
            let title = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            ImportAttempt::Imported {
                scene_id,
                title,
                path: file_path.to_string_lossy().to_string(),
            }
        }
        Ok(None) => ImportAttempt::Skipped("already known"),
        Err(e) => ImportAttempt::Failed(e.to_string()),
    }
}

/// Desktop-only watcher handle. Dropping it stops the notify watcher.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub struct WatchFolderHandle {
    _watcher: notify::RecommendedWatcher,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl Drop for WatchFolderHandle {
    fn drop(&mut self) {
        self.stop
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Start watching `watch_root` and import settled new files via `on_path`.
/// `on_path` is invoked on a background thread after debounce coalescing.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn start_watcher<F>(
    watch_root: PathBuf,
    mut on_path: F,
) -> Result<WatchFolderHandle, String>
where
    F: FnMut(PathBuf) + Send + 'static,
{
    use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use notify::event::{CreateKind, ModifyKind, RenameMode};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Instant;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_worker = stop.clone();
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| format!("watch folder: {e}"))?;

    watcher
        .watch(&watch_root, RecursiveMode::Recursive)
        .map_err(|e| format!("watch folder path: {e}"))?;

    std::thread::Builder::new()
        .name("archhive-watch-folder".into())
        .spawn(move || {
            let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
            loop {
                if stop_worker.load(Ordering::SeqCst) {
                    break;
                }
                // Drain events with a short timeout so we can flush debounce.
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(event) => {
                        let interesting = matches!(
                            event.kind,
                            EventKind::Create(CreateKind::File)
                                | EventKind::Create(CreateKind::Any)
                                | EventKind::Modify(ModifyKind::Data(_))
                                | EventKind::Modify(ModifyKind::Name(RenameMode::To))
                                | EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                                | EventKind::Modify(ModifyKind::Any)
                        );
                        if interesting {
                            for path in event.paths {
                                if path.is_file() || path.extension().is_some() {
                                    pending.insert(path, Instant::now());
                                }
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }

                let now = Instant::now();
                let ready: Vec<PathBuf> = pending
                    .iter()
                    .filter(|(_, t)| now.duration_since(**t) >= DEBOUNCE)
                    .map(|(p, _)| p.clone())
                    .collect();
                for path in ready {
                    pending.remove(&path);
                    if stop_worker.load(Ordering::SeqCst) {
                        break;
                    }
                    on_path(path);
                }
            }
        })
        .map_err(|e| format!("watch folder thread: {e}"))?;

    Ok(WatchFolderHandle {
        _watcher: watcher,
        stop,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_extensions_defaults_and_strips() {
        assert_eq!(
            normalize_extensions(&[]),
            DEFAULT_EXTENSIONS
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            normalize_extensions(&["".into(), " .MP4 ".into(), "mkv".into(), "mkv".into()]),
            vec!["mkv".to_string(), "mp4".to_string()]
        );
        assert_eq!(
            normalize_extensions(&[ "../x".into(), "mp4;rm".into(), "webm".into()]),
            vec!["webm".to_string()]
        );
    }

    #[test]
    fn incomplete_paths_detected() {
        assert!(is_incomplete_path(Path::new("clip.mp4.part")));
        assert!(is_incomplete_path(Path::new("clip.ytdl")));
        assert!(is_incomplete_path(Path::new(".hidden.mp4")));
        assert!(is_incomplete_path(Path::new("clip.mp4~")));
        assert!(is_incomplete_path(Path::new("foo.crdownload")));
        assert!(!is_incomplete_path(Path::new("clip.mp4")));
    }

    #[test]
    fn extension_allowed_checks_list() {
        let allowed = normalize_extensions(&["mp4".into(), "mkv".into()]);
        assert!(extension_allowed(Path::new("a.MP4"), &allowed));
        assert!(!extension_allowed(Path::new("a.avi"), &allowed));
        assert!(!extension_allowed(Path::new("a"), &allowed));
    }

    #[test]
    fn resolve_watch_root_rejects_outside() {
        let dir = tempdir().unwrap();
        let lib = dir.path().join("library");
        fs::create_dir_all(&lib).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();

        let root = resolve_watch_root(&lib, None).unwrap();
        assert_eq!(root, lib.canonicalize().unwrap());

        let inbox = resolve_watch_root(&lib, Some("inbox")).unwrap();
        assert!(inbox.starts_with(lib.canonicalize().unwrap()));
        assert!(inbox.ends_with("inbox"));

        let err = resolve_watch_root(&lib, Some(outside.to_str().unwrap())).unwrap_err();
        assert!(err.contains("inside the library"));
    }

    #[test]
    fn path_is_under_guards() {
        let dir = tempdir().unwrap();
        let lib = dir.path().join("library");
        let nested = lib.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let outside = dir.path().join("other");
        fs::create_dir_all(&outside).unwrap();

        assert!(path_is_under(&lib, &nested));
        assert!(!path_is_under(&lib, &outside));
    }

    #[test]
    fn relative_depth_counts() {
        let dir = tempdir().unwrap();
        let lib = dir.path().join("library");
        let file = lib.join("sub").join("clip.mp4");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"x").unwrap();
        assert_eq!(relative_depth(&lib, &file), Some(1));
        let root_file = lib.join("root.mp4");
        fs::write(&root_file, b"x").unwrap();
        assert_eq!(relative_depth(&lib, &root_file), Some(0));
    }

    #[test]
    fn wait_size_stable_ok_for_static_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.mp4");
        fs::write(&file, vec![0u8; 64]).unwrap();
        let size = wait_size_stable(&file, Duration::from_millis(20)).unwrap();
        assert_eq!(size, 64);
    }
}
