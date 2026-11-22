//! Full library database backup / restore (`.archhive-backup.zip`).
//!
//! Zip contents:
//! - `manifest.json` — version, migration id, original library path, counts
//! - `archhive.db` — SQLite snapshot via `VACUUM INTO` (safe while live)

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    LibraryBackupDryRun, LibraryBackupExportResult, LibraryBackupManifest,
    LibraryBackupRestoreOptions, LibraryBackupRestoreResult,
};
use rusqlite::{Connection, OptionalExtension};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const BACKUP_SCHEMA_VERSION: u32 = 1;
pub const MANIFEST_NAME: &str = "manifest.json";
pub const DB_ENTRY_NAME: &str = "archhive.db";
/// Marker written by `Database::new` after the latest additive migration.
pub const CURRENT_MIGRATION_ID: &str = "021";

fn map_io(ctx: &str, err: std::io::Error) -> AppError {
    AppError::Other(format!("{ctx}: {err}"))
}

fn map_zip(ctx: &str, err: zip::result::ZipError) -> AppError {
    AppError::Other(format!("{ctx}: {err}"))
}

fn map_sqlite(ctx: &str, err: rusqlite::Error) -> AppError {
    AppError::Other(format!("{ctx}: {err}"))
}

fn map_json(ctx: &str, err: serde_json::Error) -> AppError {
    AppError::Other(format!("{ctx}: {err}"))
}

fn build_manifest(db: &Database) -> AppResult<LibraryBackupManifest> {
    let settings = db.get_settings().unwrap_or_default();
    let scene_count = db.count_scenes().unwrap_or(0);
    let performer_count = db
        .list_performers(None, None, None)
        .map(|p| p.len() as u32)
        .unwrap_or(0);
    let tag_count = db.list_tags().map(|t| t.len() as u32).unwrap_or(0);
    Ok(LibraryBackupManifest {
        schema_version: BACKUP_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        migration_id: CURRENT_MIGRATION_ID.to_string(),
        library_path: settings.library_path,
        scene_count,
        performer_count,
        tag_count,
    })
}

/// Create a consistent SQLite snapshot next to the live DB, then zip it with a manifest.
pub fn export_library_backup(db: &Database, zip_path: &Path) -> AppResult<LibraryBackupExportResult> {
    let db_path = db.db_path();
    if !db_path.is_file() {
        return Err(AppError::NotFound(format!(
            "database not found at {}",
            db_path.display()
        )));
    }

    let manifest = build_manifest(db)?;
    let parent = db_path
        .parent()
        .ok_or_else(|| AppError::Other("database has no parent directory".into()))?;
    let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let snapshot_path = parent.join(format!("archhive-backup-snap-{stamp}.db"));
    let snapshot_str = snapshot_path.to_string_lossy().replace('\'', "''");

    {
        let conn_arc = db.connection();
        let conn = conn_arc
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        // VACUUM INTO produces a consistent copy without stopping readers.
        conn.execute_batch(&format!("VACUUM INTO '{snapshot_str}'"))
            .map_err(|e| map_sqlite("VACUUM INTO snapshot", e))?;
    }

    let result = (|| -> AppResult<LibraryBackupExportResult> {
        if let Some(parent) = zip_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| map_io("create backup dir", e))?;
        }
        let file = File::create(zip_path).map_err(|e| map_io("create backup zip", e))?;
        let mut zip = ZipWriter::new(BufWriter::new(file));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let manifest_json =
            serde_json::to_vec_pretty(&manifest).map_err(|e| map_json("manifest serialize", e))?;
        zip.start_file(MANIFEST_NAME, opts)
            .map_err(|e| map_zip("zip manifest", e))?;
        zip.write_all(&manifest_json)
            .map_err(|e| map_io("write manifest", e))?;

        zip.start_file(DB_ENTRY_NAME, opts)
            .map_err(|e| map_zip("zip database entry", e))?;
        let mut snap = File::open(&snapshot_path).map_err(|e| map_io("open snapshot", e))?;
        std::io::copy(&mut snap, &mut zip).map_err(|e| map_io("copy snapshot into zip", e))?;
        zip.finish().map_err(|e| map_zip("finish zip", e))?;

        Ok(LibraryBackupExportResult {
            path: zip_path.to_string_lossy().to_string(),
            manifest,
        })
    })();

    let _ = std::fs::remove_file(&snapshot_path);
    result
}

fn read_manifest_from_zip(zip_path: &Path) -> AppResult<(LibraryBackupManifest, PathBuf)> {
    let file = File::open(zip_path).map_err(|e| map_io("open backup zip", e))?;
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| map_zip("read zip", e))?;

    let mut manifest_bytes = Vec::new();
    {
        let mut entry = archive
            .by_name(MANIFEST_NAME)
            .map_err(|e| map_zip("missing manifest.json", e))?;
        std::io::copy(&mut entry, &mut manifest_bytes).map_err(|e| map_io("read manifest", e))?;
    }
    let manifest: LibraryBackupManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| map_json("parse manifest", e))?;
    if manifest.schema_version != BACKUP_SCHEMA_VERSION {
        return Err(AppError::InvalidInput(format!(
            "Unsupported backup schema_version {}",
            manifest.schema_version
        )));
    }

    let extract_dir = zip_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            "archhive-restore-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));
    std::fs::create_dir_all(&extract_dir).map_err(|e| map_io("create extract dir", e))?;
    let extracted_db = extract_dir.join(DB_ENTRY_NAME);

    {
        let mut entry = archive
            .by_name(DB_ENTRY_NAME)
            .map_err(|e| map_zip("missing archhive.db in zip", e))?;
        let mut out = File::create(&extracted_db).map_err(|e| map_io("create extracted db", e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| map_io("extract database", e))?;
    }

    Ok((manifest, extracted_db))
}

pub fn dry_run_library_backup(
    db: &Database,
    zip_path: &Path,
) -> AppResult<LibraryBackupDryRun> {
    let (manifest, extracted_db) = read_manifest_from_zip(zip_path)?;
    // Clean extract from dry-run — we only needed the manifest.
    if let Some(parent) = extracted_db.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }

    let current = db.get_settings().unwrap_or_default();
    let library_path_exists = !manifest.library_path.trim().is_empty()
        && Path::new(&manifest.library_path).exists();

    Ok(LibraryBackupDryRun {
        manifest,
        current_library_path: current.library_path,
        library_path_exists,
        library_path_needs_remap: !library_path_exists,
    })
}

fn patch_library_path_in_db(db_path: &Path, new_path: &str) -> AppResult<()> {
    let conn = Connection::open(db_path).map_err(|e| map_sqlite("open extracted db", e))?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'settings'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| map_sqlite("read settings", e))?;
    let mut settings: crate::models::AppSettings = match raw {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => crate::models::AppSettings::default(),
    };
    settings.library_path = new_path.to_string();
    // Never carry a remote host's LAN token into this device.
    settings.lan_token = None;
    let json =
        serde_json::to_string(&settings).map_err(|e| map_json("settings serialize", e))?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('settings', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![json],
    )
    .map_err(|e| map_sqlite("write remapped settings", e))?;
    Ok(())
}

/// Restore a backup zip into the live database. Writes a safety VACUUM copy first.
pub fn restore_library_backup(
    db: &Database,
    zip_path: &Path,
    options: &LibraryBackupRestoreOptions,
) -> AppResult<LibraryBackupRestoreResult> {
    let (manifest, extracted_db) = read_manifest_from_zip(zip_path)?;
    let mut remapped = false;

    if let Some(ref remap) = options.library_path_remap {
        let trimmed = remap.trim();
        if !trimmed.is_empty() {
            patch_library_path_in_db(&extracted_db, trimmed)?;
            remapped = true;
        }
    }

    let db_path = db.db_path();
    let safety_path = db_path.with_extension(format!(
        "db.pre-restore-{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    ));
    {
        let conn_arc = db.connection();
        let conn = conn_arc
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let safety_str = safety_path.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{safety_str}'"))
            .map_err(|e| map_sqlite("safety VACUUM INTO", e))?;
    }

    let src = Connection::open(&extracted_db).map_err(|e| map_sqlite("open restored db", e))?;
    {
        let conn_arc = db.connection();
        let mut dst = conn_arc
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let backup = rusqlite::backup::Backup::new(&src, &mut *dst)
            .map_err(|e| map_sqlite("init backup restore", e))?;
        backup
            .run_to_completion(100, Duration::from_millis(0), None)
            .map_err(|e| map_sqlite("run backup restore", e))?;
    }

    if let Some(parent) = extracted_db.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }

    Ok(LibraryBackupRestoreResult {
        restored: true,
        safety_copy_path: safety_path.to_string_lossy().to_string(),
        library_path_remapped: remapped,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn export_dry_run_and_restore_round_trip() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().to_path_buf()).unwrap();
        let tags = vec!["demo".to_string()];
        let _ = db.insert_scene(
            "Backup Scene",
            None,
            None,
            &[],
            &tags,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let zip_path = dir.path().join("test.archhive-backup.zip");
        let exported = export_library_backup(&db, &zip_path).unwrap();
        assert_eq!(exported.manifest.schema_version, 1);
        assert!(exported.manifest.scene_count >= 1);
        assert!(zip_path.is_file());

        let dry = dry_run_library_backup(&db, &zip_path).unwrap();
        assert_eq!(dry.manifest.scene_count, exported.manifest.scene_count);

        // Wipe live scenes then restore.
        for scene in db.list_scenes(None, crate::models::SceneSort::Newest, None, None).unwrap() {
            let _ = db.delete_scene(&scene.id, false);
        }
        assert!(db
            .list_scenes(None, crate::models::SceneSort::Newest, None, None)
            .unwrap()
            .is_empty());

        let restored = restore_library_backup(
            &db,
            &zip_path,
            &LibraryBackupRestoreOptions {
                library_path_remap: Some(dir.path().join("lib").to_string_lossy().to_string()),
            },
        )
        .unwrap();
        assert!(restored.restored);
        assert!(restored.library_path_remapped);
        assert!(Path::new(&restored.safety_copy_path).is_file());

        let scenes = db
            .list_scenes(None, crate::models::SceneSort::Newest, None, None)
            .unwrap();
        assert!(scenes.iter().any(|s| s.title == "Backup Scene"));
    }
}
