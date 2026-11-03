//! Media vault: moves library files into app-private storage and gates access
//! behind a session lock.
//!
//! Storage model (v1, "hide" only - no encryption):
//! - vaulted files live under `<app_data_dir>/vault_media/`, which on Android
//!   is internal app storage that no gallery, file manager or other app can
//!   read, and which never appears in MediaStore;
//! - a `.nomedia` marker is written into the vault root as a belt-and-braces
//!   measure for platforms where the data dir sits on shared storage;
//! - thumbnails are sidecars next to the video, so they move with it. Leaving
//!   a `{stem}.jpg` behind would leak a preview into the gallery.
//!
//! Scene rows survive vaulting (flagged `is_vaulted = 1`) so ratings, tags and
//! watch history are not lost. While the vault is locked, listings filter those
//! rows out and every media/thumbnail read is refused.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{AppSettings, Scene, VaultStatus};
use parking_lot::RwLock;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
#[cfg(target_os = "android")]
use tauri::AppHandle;
use tracing;

/// Outcome of a single file move (copy+delete).
#[derive(Debug, Clone, Default)]
pub struct MoveOutcome {
    pub pending_delete_consent: bool,
    pub leftover_paths: Vec<String>,
}

/// Vault move result including Android leftover-delete hints.
#[derive(Debug, Clone)]
pub struct VaultedScene {
    pub scene: Scene,
    pub original_path: String,
    pub original_thumb: Option<String>,
    pub pending_delete_consent: bool,
    pub leftover_paths: Vec<String>,
}

/// Directory name under the app data dir.
const VAULT_DIR: &str = "vault_media";

/// Marker that keeps Android's media scanner (and any other indexer) out of
/// the vault directory. Android does not index dot-files.
const NOMEDIA: &str = ".nomedia";

pub struct MediaVault {
    root: PathBuf,
    state: RwLock<VaultState>,
    #[cfg(target_os = "android")]
    app_handle: Option<tauri::AppHandle>,
}

#[derive(Default)]
struct VaultState {
    unlocked_until: Option<Instant>,
}

impl MediaVault {
    pub fn new(data_dir: &Path) -> AppResult<Self> {
        let root = data_dir.join(VAULT_DIR);
        std::fs::create_dir_all(&root)?;
        let marker = root.join(NOMEDIA);
        if !marker.exists() {
            // Best effort: the vault works without it on internal storage.
            let _ = std::fs::write(&marker, b"");
        }
        Ok(Self {
            root,
            state: RwLock::new(VaultState::default()),
            #[cfg(target_os = "android")]
            app_handle: None,
        })
    }

    #[cfg(target_os = "android")]
    pub fn with_app_handle(mut self, app: tauri::AppHandle) -> Self {
        self.app_handle = Some(app);
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Lock state, with lazy expiry.
    pub fn is_unlocked(&self) -> bool {
        let mut st = self.state.write();
        match st.unlocked_until {
            Some(deadline) if Instant::now() < deadline => true,
            Some(_) => {
                st.unlocked_until = None;
                false
            }
            None => false,
        }
    }

    /// Unlock for `mins` minutes. `0` unlocks until the app locks it again.
    pub fn unlock_for(&self, mins: u32) {
        let mut st = self.state.write();
        st.unlocked_until = if mins == 0 {
            Some(Instant::now() + Duration::from_secs(60 * 60 * 24 * 365))
        } else {
            Some(Instant::now() + Duration::from_secs(u64::from(mins) * 60))
        };
    }

    pub fn lock(&self) {
        self.state.write().unlocked_until = None;
    }

    pub fn assert_unlocked(&self) -> AppResult<()> {
        if self.is_unlocked() {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "The vault is locked. Unlock it first.".into(),
            ))
        }
    }

    /// Seconds until the session re-locks (None when locked or never-expiring).
    pub fn expires_in(&self) -> Option<i64> {
        let st = self.state.read();
        st.unlocked_until
            .map(|deadline| deadline.saturating_duration_since(Instant::now()).as_secs() as i64)
            .filter(|secs| *secs < 60 * 60 * 24 * 300)
    }

    pub fn status(&self, db: &Database, settings: &AppSettings) -> AppResult<VaultStatus> {
        Ok(VaultStatus {
            enabled: settings.vault_enabled,
            unlocked: self.is_unlocked(),
            item_count: db.count_vaulted_scenes()?,
            auto_lock_mins: settings.vault_auto_lock_mins,
            expires_at: self
                .expires_in()
                .map(|secs| chrono::Utc::now().timestamp() + secs),
            lan_exposed: settings.vault_lan_exposed,
            biometric_available: cfg!(target_os = "android") || cfg!(target_os = "ios"),
            vault_root: Some(self.root.to_string_lossy().to_string()),
        })
    }

    /// True when `path` points inside the vault directory.
    ///
    /// Called with both stored paths (built from `root`, plain form) and
    /// canonicalized paths (Windows `\\?\` prefix). Compare lexically first,
    /// then canonically so both forms agree; never canonicalize a path that
    /// does not exist just to compare it.
    pub fn is_vault_path(&self, path: &str) -> bool {
        if path.trim().is_empty() {
            return false;
        }
        let candidate = Path::new(path);
        if lexical_contains(&self.root, candidate) {
            return true;
        }
        match (candidate.canonicalize(), self.root.canonicalize()) {
            (Ok(canonical_candidate), Ok(canonical_root)) => {
                lexical_contains(&canonical_root, &canonical_candidate)
            }
            _ => false,
        }
    }

    /// Move a scene's file (and its thumbnail sidecar) into the vault and flag
    /// the row. Idempotent: an already-vaulted scene is returned unchanged.
    pub fn vault_scene(&self, db: &Database, scene: &Scene) -> AppResult<VaultedScene> {
        if db.is_scene_vaulted(&scene.id)? {
            let existing = db.get_scene(&scene.id)?;
            return Ok(VaultedScene {
                scene: existing,
                original_path: String::new(),
                original_thumb: None,
                pending_delete_consent: false,
                leftover_paths: Vec::new(),
            });
        }
        let source = scene
            .path
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| AppError::InvalidInput("scene has no file on disk".into()))?;
        let source_path = Path::new(source);
        if !source_path.is_file() {
            return Err(AppError::NotFound(format!("file not found: {source}")));
        }
        if self.is_vault_path(source) {
            return Err(AppError::InvalidInput(
                "file is already inside the vault directory".into(),
            ));
        }

        let stem = source_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("video");
        let ext = source_path
            .extension()
            .and_then(|n| n.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();

        let dest = self
            .root
            .join(format!("{}_{}{}", &scene.id, sanitize(stem), ext));
        #[cfg(target_os = "android")]
        let move_result = if let Some(ref app) = self.app_handle {
            move_file_with_app(source_path, &dest, Some(app.clone()))
        } else {
            move_file(source_path, &dest)
        };
        #[cfg(not(target_os = "android"))]
        let move_result = move_file(source_path, &dest);
        let video_move = move_result?;

        // Thumbnail sidecar: prefer DB `scene.thumb`, else conventional `{stem}.jpg`
        // next to the video (library convention when thumb column is empty).
        let sidecar_candidate = source_path.with_extension("jpg");
        let thumb_source: Option<PathBuf> = scene
            .thumb
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_file() && !self.is_vault_path(&p.to_string_lossy()))
            .or_else(|| {
                if sidecar_candidate.is_file()
                    && !self.is_vault_path(&sidecar_candidate.to_string_lossy())
                {
                    Some(sidecar_candidate)
                } else {
                    None
                }
            });

        let mut thumb_move = MoveOutcome::default();
        let mut original_thumb_path: Option<String> = None;
        if let Some(ref thumb_path) = thumb_source {
            original_thumb_path = Some(thumb_path.to_string_lossy().to_string());
            let dest_thumb = self
                .root
                .join(format!("{}_{}.jpg", &scene.id, sanitize(stem)));
            #[cfg(target_os = "android")]
            let thumb_res = if let Some(ref app) = self.app_handle {
                move_file_with_app(thumb_path, &dest_thumb, Some(app.clone()))
            } else {
                move_file(thumb_path, &dest_thumb)
            };
            #[cfg(not(target_os = "android"))]
            let thumb_res = move_file(thumb_path, &dest_thumb);
            match thumb_res {
                Ok(outcome) => {
                    thumb_move = outcome;
                    let _ = db.set_scene_thumb(&scene.id, &dest_thumb.to_string_lossy());
                }
                Err(e) => {
                    tracing::warn!(
                        "vault: video vaulted but thumb move failed for {}: {e}",
                        thumb_path.display()
                    );
                }
            }
        }

        db.mark_scene_vaulted(&scene.id, &dest.to_string_lossy(), Some(source))?;
        let scene = db.get_scene(&scene.id)?;
        Ok(VaultedScene {
            scene,
            original_path: source.to_string(),
            original_thumb: original_thumb_path,
            pending_delete_consent: video_move.pending_delete_consent
                || thumb_move.pending_delete_consent,
            leftover_paths: {
                let mut left = video_move.leftover_paths;
                left.extend(thumb_move.leftover_paths);
                left
            },
        })
    }

    /// Move a vaulted scene's files back into the library.
    pub fn restore_scene(
        &self,
        db: &Database,
        scene: &Scene,
        library_root: &str,
    ) -> AppResult<Scene> {
        let (is_vaulted, path, _thumb, original, _vaulted_at) = db.scene_vault_meta(&scene.id)?;
        if !is_vaulted {
            return db.get_scene(&scene.id);
        }
        let current = path
            .as_deref()
            .ok_or_else(|| AppError::Other("vaulted scene has no path".into()))?;
        let current_path = Path::new(current);
        if !current_path.is_file() {
            // Row still flagged but file gone; clear the flag so the UI is not
            // stuck with a phantom item.
            db.mark_scene_unvaulted(&scene.id, current)?;
            return db.get_scene(&scene.id);
        }

        // Restore to the original location when its folder still exists,
        // otherwise drop it in the library root.
        let restore_dir = original
            .as_deref()
            .map(Path::new)
            .and_then(|p| p.parent())
            .filter(|dir| dir.is_dir())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(library_root));
        std::fs::create_dir_all(&restore_dir)?;

        let original_name = original
            .as_deref()
            .map(Path::new)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                let ext = current_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{e}"))
                    .unwrap_or_default();
                format!("{}{}", sanitize(&scene.title), ext)
            });
        let dest = unique_path(&restore_dir.join(original_name));
        #[cfg(target_os = "android")]
        let move_result = if let Some(ref app) = self.app_handle {
            move_file_with_app(current_path, &dest, Some(app.clone()))
        } else {
            move_file(current_path, &dest)
        };
        #[cfg(not(target_os = "android"))]
        let move_result = move_file(current_path, &dest);
        move_result?;

        // Thumbnail: prefer moving the vaulted sidecar next to the restored
        // video; fall back to whatever path the row already had.
        if let Some(thumb) = scene.thumb.as_deref().filter(|t| self.is_vault_path(t)) {
            let thumb_src = Path::new(thumb);
            if thumb_src.is_file() {
                let dest_thumb = dest.with_extension("jpg");
                #[cfg(target_os = "android")]
                let thumb_res = if let Some(ref app) = self.app_handle {
                    move_file_with_app(thumb_src, &dest_thumb, Some(app.clone()))
                } else {
                    move_file(thumb_src, &dest_thumb)
                };
                #[cfg(not(target_os = "android"))]
                let thumb_res = move_file(thumb_src, &dest_thumb);
                if thumb_res.is_ok() {
                    let _ = db.set_scene_thumb(&scene.id, &dest_thumb.to_string_lossy());
                }
            }
        }

        db.mark_scene_unvaulted(&scene.id, &dest.to_string_lossy())?;
        db.get_scene(&scene.id)
    }
}


/// Resolve a path for comparison without requiring it to exist.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Is `candidate` at or below `root`? Component-wise so `a/bc` is not treated
/// as being inside `a/b`, and case-insensitive on Windows.
fn lexical_contains(root: &Path, candidate: &Path) -> bool {
    let root_parts: Vec<String> = normalize(root).components().map(component_key).collect();
    let candidate_parts: Vec<String> = normalize(candidate)
        .components()
        .map(component_key)
        .collect();
    !root_parts.is_empty()
        && root_parts.len() <= candidate_parts.len()
        && root_parts
            .iter()
            .zip(candidate_parts.iter())
            .all(|(a, b)| a == b)
}

fn component_key(component: Component<'_>) -> String {
    let raw = component.as_os_str().to_string_lossy().to_string();
    if cfg!(windows) {
        raw.to_lowercase()
    } else {
        raw
    }
}

/// Strip path-hostile characters from a stem used in the vault filename.
fn sanitize(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "video".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

/// `name.ext`, `name (2).ext`, ... until the path is free.
fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let ext = path.extension().and_then(|e| e.to_str());
    for n in 2..10_000 {
        let name = match ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = path.with_file_name(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

/// Move a file across directories, falling back to copy + verify + delete when
/// the source and destination are on different filesystems (common on Android
/// where the library may live on shared storage and the vault is internal).
#[cfg_attr(not(target_os = "android"), allow(unused_mut))]
fn move_file(src: &Path, dest: &Path) -> AppResult<MoveOutcome> {
    move_file_with_app(src, dest, None)
}

#[cfg(target_os = "android")]
fn move_file_with_app(src: &Path, dest: &Path, app: Option<AppHandle>) -> AppResult<MoveOutcome> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let src_metadata = std::fs::metadata(src)?;
    let src_len = src_metadata.len();
    // Try atomic rename first (same filesystem)
    if std::fs::rename(src, dest).is_ok() {
        return Ok(MoveOutcome::default());
    }
    // Cross-filesystem: copy then verify then delete
    std::fs::copy(src, dest)?;
    let dest_len = std::fs::metadata(dest)?.len();
    if dest_len != src_len {
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Other(format!(
            "vault copy incomplete: {} -> {} ({} != {} bytes)",
            src.display(),
            dest.display(),
            src_len,
            dest_len
        )));
    }
    // Try to delete source; on Android with scoped storage this may fail even
    // if the app created the file. The copy is verified in the vault so the
    // move is still successful from the vault's perspective.
    let mut deleted = false;
    let mut pending_delete_consent = false;
    for attempt in 0..20 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(
                50 * ((attempt % 10) + 1) as u64,
            ));
        }
        if std::fs::remove_file(src).is_ok() {
            deleted = true;
            break;
        }
        // Try truncating first
        if let Ok(f) = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(src)
        {
            drop(f);
            if std::fs::remove_file(src).is_ok() {
                deleted = true;
                break;
            }
        }
        // Try to change permissions if possible
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(src) {
                let mut perms = meta.permissions();
                perms.set_mode(0o666);
                let _ = std::fs::set_permissions(src, perms);
                if std::fs::remove_file(src).is_ok() {
                    deleted = true;
                    break;
                }
            }
        }
    }
    if !deleted {
        // Try via MediaStore plugin if we have app handle
        if let Some(ref app_handle) = app {
            let path_str = src.to_string_lossy().to_string();
            let result = crate::mobile::media_scan::delete_paths(app_handle, &[path_str.clone()]);
            pending_delete_consent = result.pending_user_consent;
            std::thread::sleep(std::time::Duration::from_millis(300));
            if !src.exists() || std::fs::remove_file(src).is_ok() {
                deleted = true;
            } else if result.deleted && !src.exists() {
                deleted = true;
            } else {
                let result2 =
                    crate::mobile::media_scan::delete_paths(app_handle, &[path_str.clone()]);
                pending_delete_consent = pending_delete_consent || result2.pending_user_consent;
                crate::mobile::media_scan::rescan_paths(app_handle, &[path_str.clone()]);
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !src.exists() || std::fs::remove_file(src).is_ok() {
                    deleted = true;
                }
            }
        }
        if !deleted {
            tracing::warn!(
                "vault: copied {} to {} but could not delete source after multiple attempts; original may remain visible in file explorers due to Android scoped storage restrictions",
                src.display(),
                dest.display()
            );
            return Ok(MoveOutcome {
                pending_delete_consent,
                leftover_paths: vec![src.to_string_lossy().to_string()],
            });
        }
    }
    Ok(MoveOutcome {
        pending_delete_consent,
        leftover_paths: Vec::new(),
    })
}

#[cfg(not(target_os = "android"))]
fn move_file_with_app(src: &Path, dest: &Path, _app: Option<()>) -> AppResult<MoveOutcome> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let src_metadata = std::fs::metadata(src)?;
    let src_len = src_metadata.len();
    // Try atomic rename first (same filesystem)
    if std::fs::rename(src, dest).is_ok() {
        return Ok(MoveOutcome::default());
    }
    // Cross-filesystem: copy then verify then delete
    std::fs::copy(src, dest)?;
    let dest_len = std::fs::metadata(dest)?.len();
    if dest_len != src_len {
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Other(format!(
            "vault copy incomplete: {} -> {} ({} != {} bytes)",
            src.display(),
            dest.display(),
            src_len,
            dest_len
        )));
    }
    if let Err(e) = std::fs::remove_file(src) {
        tracing::warn!(
            "vault: copied {} to {} but failed to delete source ({}); file may remain visible",
            src.display(),
            dest.display(),
            e
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
        if src.exists() && std::fs::remove_file(src).is_err() {
            return Ok(MoveOutcome {
                pending_delete_consent: false,
                leftover_paths: vec![src.to_string_lossy().to_string()],
            });
        }
    }
    Ok(MoveOutcome::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with(path: &Path, thumb: Option<&Path>) -> Scene {
        Scene {
            id: "scene-1".into(),
            title: "Test".into(),
            path: Some(path.to_string_lossy().to_string()),
            duration: Some(10),
            thumb: thumb.map(|t| t.to_string_lossy().to_string()),
            source_url: None,
            studio_id: None,
            studio_name: None,
            date: None,
            rating: None,
            performers: Vec::new(),
            tags: Vec::new(),
            channel: None,
            phash: None,
            oshash: None,
            file_size: None,
            notes: None,
            width: None,
            height: None,
            stash_id: None,
            stashdb_updated_at: None,
        }
    }

    #[test]
    fn lock_gate_opens_and_closes() {
        let dir = tempfile::tempdir().unwrap();
        let vault = MediaVault::new(dir.path()).unwrap();
        assert!(vault.assert_unlocked().is_err());
        vault.unlock_for(5);
        assert!(vault.assert_unlocked().is_ok());
        vault.lock();
        assert!(vault.assert_unlocked().is_err());
    }

    #[test]
    fn zero_minutes_unlocks_without_timer() {
        let dir = tempfile::tempdir().unwrap();
        let vault = MediaVault::new(dir.path()).unwrap();
        vault.unlock_for(0);
        assert!(vault.is_unlocked());
        assert!(vault.expires_in().is_none());
    }

    #[test]
    fn vault_root_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let vault = MediaVault::new(dir.path()).unwrap();
        let inside = vault.root().join("scene-1_video.mp4");
        assert!(vault.is_vault_path(&inside.to_string_lossy()));
        assert!(!vault.is_vault_path("/tmp/elsewhere.mp4"));
    }

    #[test]
    fn vault_path_matches_canonical_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let vault = MediaVault::new(dir.path()).unwrap();
        // A real file compares through canonicalization (Windows `\\?\` form).
        let real = vault.root().join("real.mp4");
        std::fs::write(&real, b"x").unwrap();
        let canonical = real.canonicalize().unwrap();
        assert!(vault.is_vault_path(&canonical.to_string_lossy()));
        // A file that no longer exists must still be recognised, otherwise a
        // delete/restore path leaks out of the vault directory check.
        let gone = vault.root().join("gone.mp4");
        assert!(vault.is_vault_path(&gone.to_string_lossy()));
        // Sibling prefix is not containment: `vault_media2` is a different dir.
        let sibling = format!("{}2/gone.mp4", vault.root().to_string_lossy());
        assert!(!vault.is_vault_path(&sibling));
    }

    #[test]
    fn move_file_handles_same_and_cross_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.mp4");
        std::fs::write(&src, b"hello").unwrap();
        let dest = dir.path().join("nested").join("b.mp4");
        move_file(&src, &dest).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"hello");
    }

    #[test]
    fn sanitize_strips_separators() {
        assert_eq!(sanitize("a/b\\c:d"), "a_b_c_d");
        assert_eq!(sanitize("..."), "video");
    }

    #[test]
    fn unique_path_avoids_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("clip.mp4");
        std::fs::write(&first, b"x").unwrap();
        let second = unique_path(&first);
        assert_eq!(second.file_name().unwrap(), "clip (2).mp4");
    }

    #[test]
    fn scene_helper_is_used() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.mp4");
        std::fs::write(&file, b"x").unwrap();
        let scene = scene_with(&file, None);
        assert_eq!(scene.id, "scene-1");
    }
}
