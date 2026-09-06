mod migrations;

use crate::db::migrations::{
    MIGRATION_001, MIGRATION_002, MIGRATION_003, MIGRATION_004, MIGRATION_005, MIGRATION_006,
    MIGRATION_007, MIGRATION_008, MIGRATION_009, MIGRATION_010, MIGRATION_011,
};
use crate::error::{AppError, AppResult};
use crate::models::{
    AppSettings, DownloadJob, DownloadStatus, DuplicateGroup, Performer, Scene, Tag,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info('{table}')");
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let names: Vec<String> = match stmt.query_map([], |row| row.get::<_, String>(1)) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => return false,
    };
    names.iter().any(|name| name == column)
}

type SceneRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
    Option<i64>,
    Option<u32>,
    Option<u32>,
);

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("archhive.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch(MIGRATION_001)?;
        conn.execute_batch(MIGRATION_002)?;
        if !column_exists(&conn, "download_jobs", "metadata") {
            conn.execute_batch(MIGRATION_003)?;
        }
        if !column_exists(&conn, "scenes", "channel") {
            conn.execute_batch(MIGRATION_004)?;
        }
        if !column_exists(&conn, "scenes", "file_size") {
            conn.execute_batch(MIGRATION_005)?;
        }
        if !column_exists(&conn, "scenes", "notes") {
            conn.execute_batch(MIGRATION_006)?;
        }
        if !column_exists(&conn, "download_jobs", "retry_count") {
            conn.execute_batch(MIGRATION_007)?;
        }
        if !column_exists(&conn, "scenes", "width") {
            conn.execute_batch(MIGRATION_008)?;
        }
        conn.execute_batch(MIGRATION_009)?;
        conn.execute_batch(MIGRATION_010)?;
        if !column_exists(&conn, "saved_searches", "auto_queue") {
            conn.execute_batch(MIGRATION_011)?;
        }
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'settings'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        match raw {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Other(format!("settings parse: {e}"))),
            None => Ok(AppSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let json = serde_json::to_string(settings)
            .map_err(|e| AppError::Other(format!("settings serialize: {e}")))?;
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }

    pub fn insert_download_job(
        &self,
        url: &str,
        adapter: &str,
        title: Option<&str>,
        metadata: Option<&str>,
    ) -> AppResult<DownloadJob> {
        let job = DownloadJob {
            id: Uuid::new_v4().to_string(),
            url: url.to_string(),
            adapter: adapter.to_string(),
            status: DownloadStatus::Pending,
            progress: 0.0,
            output_path: None,
            error: None,
            title: title.map(|s| s.to_string()),
            created_at: Utc::now().to_rfc3339(),
            retry_count: 0,
            last_retry_at: None,
        };
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "INSERT INTO download_jobs (id, url, adapter, status, progress, title, metadata, created_at, retry_count, last_retry_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                job.id,
                job.url,
                job.adapter,
                format!("{:?}", job.status).to_lowercase(),
                job.progress,
                job.title,
                metadata,
                job.created_at,
                job.retry_count,
                job.last_retry_at,
            ],
        )?;
        Ok(job)
    }

    pub fn update_download_job(&self, job: &DownloadJob) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let status = format!("{:?}", job.status).to_lowercase();
        conn.execute(
            "UPDATE download_jobs SET status = ?2, progress = ?3, output_path = ?4, error = ?5, title = ?6, retry_count = ?7, last_retry_at = ?8
             WHERE id = ?1",
            params![
                job.id,
                status,
                job.progress,
                job.output_path,
                job.error,
                job.title,
                job.retry_count,
                job.last_retry_at,
            ],
        )?;
        Ok(())
    }

    pub fn store_download_job_metadata(
        &self,
        job_id: &str,
        performers: &[String],
        tags: &[String],
        thumbnail_url: Option<&str>,
        duration: Option<u32>,
        channel: Option<&str>,
    ) -> AppResult<()> {
        let metadata = serde_json::json!({
            "performers": performers,
            "tags": tags,
            "thumbnail_url": thumbnail_url,
            "duration": duration,
            "channel": channel,
        });
        let json = serde_json::to_string(&metadata)
            .map_err(|e| AppError::Other(format!("metadata serialize: {e}")))?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE download_jobs SET metadata = ?2 WHERE id = ?1",
            params![job_id, json],
        )?;
        Ok(())
    }

    pub fn get_download_job_metadata(
        &self,
        job_id: &str,
    ) -> AppResult<(
        Vec<String>,
        Vec<String>,
        Option<String>,
        Option<u32>,
        Option<String>,
    )> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let metadata: Option<String> = conn
            .query_row(
                "SELECT metadata FROM download_jobs WHERE id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        match metadata {
            Some(json) => {
                let v: serde_json::Value = serde_json::from_str(&json)
                    .map_err(|e| AppError::Other(format!("metadata parse: {e}")))?;
                let performers = v
                    .get("performers")
                    .and_then(|p| p.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let tags = v
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let thumbnail_url = v
                    .get("thumbnail_url")
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
                let duration = v.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32);
                let channel = v
                    .get("channel")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
                Ok((performers, tags, thumbnail_url, duration, channel))
            }
            None => Ok((vec![], vec![], None, None, None)),
        }
    }

    pub fn list_download_jobs(&self) -> AppResult<Vec<DownloadJob>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, url, adapter, status, progress, output_path, error, title, created_at, retry_count, last_retry_at
             FROM download_jobs ORDER BY created_at DESC LIMIT 100",
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(3)?;
            Ok(DownloadJob {
                id: row.get(0)?,
                url: row.get(1)?,
                adapter: row.get(2)?,
                status: parse_status(&status_str),
                progress: row.get(4)?,
                output_path: row.get(5)?,
                error: row.get(6)?,
                title: row.get(7)?,
                created_at: row.get(8)?,
                retry_count: row.get(9)?,
                last_retry_at: row.get(10)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn delete_download_job(&self, id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute("DELETE FROM download_jobs WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_download_job(&self, id: &str) -> AppResult<Option<DownloadJob>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.query_row(
            "SELECT id, url, adapter, status, progress, output_path, error, title, created_at, retry_count, last_retry_at
             FROM download_jobs WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(3)?;
                Ok(DownloadJob {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    adapter: row.get(2)?,
                    status: parse_status(&status_str),
                    progress: row.get(4)?,
                    output_path: row.get(5)?,
                    error: row.get(6)?,
                    title: row.get(7)?,
                    created_at: row.get(8)?,
                    retry_count: row.get(9)?,
                    last_retry_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn upsert_performer(&self, name: &str) -> AppResult<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        Self::upsert_performer_with_conn(&conn, name)
    }

    fn upsert_performer_with_conn(conn: &Connection, name: &str) -> AppResult<String> {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM performers WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO performers (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
        Ok(id)
    }

    pub fn update_performer_image(&self, id: &str, image: Option<&str>) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE performers SET image = ?2 WHERE id = ?1",
            params![id, image],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn upsert_tag(&self, name: &str) -> AppResult<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        Self::upsert_tag_with_conn(&conn, name)
    }

    fn upsert_tag_with_conn(conn: &Connection, name: &str) -> AppResult<String> {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tags (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_scene(
        &self,
        title: &str,
        path: Option<&str>,
        source_url: Option<&str>,
        performers: &[String],
        tags: &[String],
        thumb: Option<&str>,
        phash: Option<&str>,
        oshash: Option<&str>,
        duration: Option<u32>,
        channel: Option<&str>,
        file_size: Option<u64>,
    ) -> AppResult<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO scenes (id, title, path, source_url, thumb, phash, oshash, duration, channel, file_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, title, path, source_url, thumb, phash, oshash, duration, channel, file_size.map(|v| v as i64), now],
        )?;
        for p in performers {
            let pid = Self::upsert_performer_with_conn(&conn, p)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_performers (scene_id, performer_id) VALUES (?1, ?2)",
                params![id, pid],
            )?;
        }
        for t in tags {
            let tid = Self::upsert_tag_with_conn(&conn, t)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_tags (scene_id, tag_id) VALUES (?1, ?2)",
                params![id, tid],
            )?;
        }
        Ok(id)
    }

    pub fn replace_scene_performers(&self, scene_id: &str, performers: &[String]) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "DELETE FROM scene_performers WHERE scene_id = ?1",
            params![scene_id],
        )?;
        for p in performers {
            let pid = Self::upsert_performer_with_conn(&conn, p)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_performers (scene_id, performer_id) VALUES (?1, ?2)",
                params![scene_id, pid],
            )?;
        }
        Ok(())
    }

    pub fn replace_scene_tags(&self, scene_id: &str, tags: &[String]) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "DELETE FROM scene_tags WHERE scene_id = ?1",
            params![scene_id],
        )?;
        for t in tags {
            let tid = Self::upsert_tag_with_conn(&conn, t)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_tags (scene_id, tag_id) VALUES (?1, ?2)",
                params![scene_id, tid],
            )?;
        }
        Ok(())
    }

    pub fn update_scene_path(&self, id: &str, path: &str, thumb: Option<&str>) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET path = ?2, thumb = COALESCE(?3, thumb) WHERE id = ?1",
            params![id, path, thumb],
        )?;
        Ok(())
    }

    pub fn update_scene_hashes(
        &self,
        id: &str,
        phash: Option<&str>,
        oshash: Option<&str>,
        thumb: Option<&str>,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET phash = COALESCE(?2, phash), oshash = COALESCE(?3, oshash), thumb = COALESCE(?4, thumb) WHERE id = ?1",
            params![id, phash, oshash, thumb],
        )?;
        Ok(())
    }

    pub fn update_scene_duration(&self, id: &str, duration_secs: u32) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET duration = ?2 WHERE id = ?1 AND (duration IS NULL OR duration = 0)",
            params![id, duration_secs],
        )?;
        Ok(())
    }

    pub fn update_scene_resolution(&self, id: &str, width: u32, height: u32) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET width = ?2, height = ?3 WHERE id = ?1 AND (width IS NULL OR width = 0 OR height IS NULL OR height = 0)",
            params![id, width, height],
        )?;
        Ok(())
    }

    pub fn update_scene_file_size(&self, id: &str, file_size: u64) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET file_size = ?2 WHERE id = ?1",
            params![id, file_size as i64],
        )?;
        Ok(())
    }

    /// Scenes with a video path but no usable thumbnail (thumb missing or thumb file deleted).
    pub fn list_scenes_missing_thumbs(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, path, thumb FROM scenes
             WHERE path IS NOT NULL AND path != ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, path, thumb): (String, String, Option<String>) = row?;
            let needs_thumb = match &thumb {
                None => true,
                Some(t) if t.is_empty() => true,
                Some(t) => !std::path::Path::new(t).is_file(),
            };
            if needs_thumb {
                out.push((id, path));
            }
        }
        Ok(out)
    }

    /// Scenes with a video path but no duration set.
    pub fn list_scenes_missing_durations(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, path FROM scenes
             WHERE path IS NOT NULL AND path != ''
             AND (duration IS NULL OR duration = 0)",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    /// Scenes with a video path but no resolution stored.
    pub fn list_scenes_missing_resolution(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, path FROM scenes
             WHERE path IS NOT NULL AND path != ''
             AND (width IS NULL OR width = 0 OR height IS NULL OR height = 0)",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    /// Upsert a watch-progress row. `watched` is computed by the caller.
    pub fn record_watch_progress(
        &self,
        scene_id: &str,
        position_secs: f64,
        duration_secs: f64,
        watched: bool,
        updated_at: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "INSERT INTO watch_history (scene_id, position_secs, duration_secs, watched, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(scene_id) DO UPDATE SET
               position_secs = excluded.position_secs,
               duration_secs = excluded.duration_secs,
               watched = excluded.watched,
               updated_at = excluded.updated_at",
            params![scene_id, position_secs, duration_secs, watched as i32, updated_at],
        )?;
        Ok(())
    }

    pub fn get_watch_progress(
        &self,
        scene_id: &str,
    ) -> AppResult<Option<crate::models::WatchProgress>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let row: Option<(f64, f64, i32, String)> = conn
            .query_row(
                "SELECT position_secs, duration_secs, watched, updated_at FROM watch_history WHERE scene_id = ?1",
                params![scene_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        Ok(
            row.map(|(position_secs, duration_secs, watched, updated_at)| {
                crate::models::WatchProgress {
                    scene_id: scene_id.to_string(),
                    position_secs,
                    duration_secs,
                    watched: watched != 0,
                    updated_at,
                }
            }),
        )
    }

    pub fn list_watch_progress(&self) -> AppResult<Vec<crate::models::WatchProgress>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT scene_id, position_secs, duration_secs, watched, updated_at FROM watch_history",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::models::WatchProgress {
                scene_id: row.get(0)?,
                position_secs: row.get(1)?,
                duration_secs: row.get(2)?,
                watched: row.get::<_, i32>(3)? != 0,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    /// Bulk mark scenes watched/unwatched. Marking watched keeps the stored
    /// position; unmarking resets position to 0.
    pub fn mark_watched(&self, ids: &[String], watched: bool, updated_at: &str) -> AppResult<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut updated = 0u32;
        for id in ids {
            let existing: Option<(f64, f64)> = conn
                .query_row(
                    "SELECT position_secs, duration_secs FROM watch_history WHERE scene_id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let (position, duration) = existing.unwrap_or((0.0, 0.0));
            let position = if watched { position } else { 0.0 };
            conn.execute(
                "INSERT INTO watch_history (scene_id, position_secs, duration_secs, watched, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(scene_id) DO UPDATE SET
                   position_secs = excluded.position_secs,
                   watched = excluded.watched,
                   updated_at = excluded.updated_at",
                params![id, position, duration, watched as i32, updated_at],
            )?;
            updated += 1;
        }
        Ok(updated)
    }

    /// Source URLs of scenes marked watched — used to hide watched matches
    /// in browse results (#28).
    pub fn list_watched_source_urls(&self) -> AppResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT s.source_url FROM scenes s
             JOIN watch_history w ON w.scene_id = s.id
             WHERE w.watched = 1 AND s.source_url IS NOT NULL AND s.source_url != ''",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    /// Insert a saved search (#28). Returns the created row.
    pub fn create_saved_search(
        &self,
        req: &crate::models::SaveSearchRequest,
    ) -> AppResult<crate::models::SavedSearch> {
        use crate::models::{BrowseKind, BrowseOrientation};
        let name = req.name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidInput("name is required".into()));
        }
        if req.slug.trim().is_empty() {
            return Err(AppError::InvalidInput("slug is required".into()));
        }
        let kind_str = match req.kind {
            BrowseKind::Tag => "tag",
            BrowseKind::Model => "model",
            BrowseKind::Channel => "channel",
            BrowseKind::Search => "search",
            BrowseKind::Video => "video",
            BrowseKind::Category => "category",
            BrowseKind::Livestream => "livestream",
        };
        let orientation_str = req.orientation.map(|o| match o {
            BrowseOrientation::Straight => "straight",
            BrowseOrientation::Gay => "gay",
            BrowseOrientation::Lesbian => "lesbian",
            BrowseOrientation::Transgender => "transgender",
        });
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO saved_searches (id, name, site_id, kind, slug, orientation, last_checked_at, last_item_keys, new_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, '[]', 0, ?7)",
            rusqlite::params![
                id,
                name,
                req.site_id,
                kind_str,
                req.slug.trim(),
                orientation_str,
                created_at
            ],
        )?;
        Ok(crate::models::SavedSearch {
            id,
            name: name.to_string(),
            site_id: req.site_id.clone(),
            kind: req.kind,
            slug: req.slug.trim().to_string(),
            orientation: req.orientation,
            last_checked_at: None,
            new_count: 0,
            auto_queue: false,
            created_at,
        })
    }

    fn map_saved_search_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<crate::models::SavedSearch> {
        use crate::models::{BrowseKind, BrowseOrientation};
        let kind_str: String = row.get(3)?;
        let kind = match kind_str.as_str() {
            "tag" => BrowseKind::Tag,
            "model" => BrowseKind::Model,
            "channel" => BrowseKind::Channel,
            "video" => BrowseKind::Video,
            "category" => BrowseKind::Category,
            "livestream" => BrowseKind::Livestream,
            _ => BrowseKind::Search,
        };
        let orientation_str: Option<String> = row.get(5)?;
        let orientation = orientation_str.and_then(|o| match o.as_str() {
            "gay" => Some(BrowseOrientation::Gay),
            "lesbian" => Some(BrowseOrientation::Lesbian),
            "transgender" => Some(BrowseOrientation::Transgender),
            _ => Some(BrowseOrientation::Straight),
        });
        Ok(crate::models::SavedSearch {
            id: row.get(0)?,
            name: row.get(1)?,
            site_id: row.get(2)?,
            kind,
            slug: row.get(4)?,
            orientation,
            last_checked_at: row.get(6)?,
            new_count: row.get::<_, i64>(8).unwrap_or(0).max(0) as u32,
            auto_queue: row.get::<_, i64>(10).unwrap_or(0) != 0,
            created_at: row.get(9)?,
        })
    }

    pub fn list_saved_searches(&self) -> AppResult<Vec<crate::models::SavedSearch>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, site_id, kind, slug, orientation, last_checked_at, last_item_keys, new_count, created_at, auto_queue
             FROM saved_searches ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], Self::map_saved_search_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn get_saved_search(&self, id: &str) -> AppResult<Option<crate::models::SavedSearch>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let row = conn
            .query_row(
                "SELECT id, name, site_id, kind, slug, orientation, last_checked_at, last_item_keys, new_count, created_at, auto_queue
                 FROM saved_searches WHERE id = ?1",
                rusqlite::params![id],
                Self::map_saved_search_row,
            )
            .optional()?;
        Ok(row)
    }

    pub fn set_saved_search_auto_queue(&self, id: &str, auto_queue: bool) -> AppResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let changed = conn.execute(
            "UPDATE saved_searches SET auto_queue = ?2 WHERE id = ?1",
            rusqlite::params![id, auto_queue as i32],
        )?;
        Ok(changed > 0)
    }

    pub fn delete_saved_search(&self, id: &str) -> AppResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let changed = conn.execute(
            "DELETE FROM saved_searches WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(changed > 0)
    }

    /// Persist a check snapshot. Returns the previous key set for diffing.
    pub fn update_saved_search_check(
        &self,
        id: &str,
        keys_json: &str,
        new_count: u32,
        checked_at: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE saved_searches SET last_item_keys = ?2, new_count = ?3, last_checked_at = ?4 WHERE id = ?1",
            rusqlite::params![id, keys_json, new_count as i64, checked_at],
        )?;
        Ok(())
    }

    pub fn saved_search_keys(&self, id: &str) -> AppResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT last_item_keys FROM saved_searches WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(raw
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default())
    }

    /// Clear every scene's thumbnail reference (sidecar files on disk are kept;
    /// use the orphan-sidecar cleaner to remove leftovers). Returns rows cleared.
    pub fn clear_all_thumbs(&self) -> AppResult<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let changed = conn.execute(
            "UPDATE scenes SET thumb = NULL WHERE thumb IS NOT NULL AND thumb != ''",
            [],
        )?;
        Ok(changed as u64)
    }

    /// Scenes whose `thumb` path points to a file that no longer exists on disk.
    #[allow(dead_code)]
    pub fn list_orphan_thumbs(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, thumb FROM scenes
             WHERE thumb IS NOT NULL AND thumb != ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, thumb): (String, String) = row?;
            if !std::path::Path::new(&thumb).is_file() {
                out.push((id, thumb));
            }
        }
        Ok(out)
    }

    /// List scenes matching a filter (used by the library UI filter bar).
    pub fn list_scenes_with_filter(
        &self,
        filter: &crate::models::SceneFilter,
    ) -> AppResult<Vec<Scene>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;

        // Build dynamic WHERE clause (only SQL-level filters; file-level checks are post-filtered).
        let mut conditions = Vec::new();
        let mut joins = Vec::new();

        if filter.missing_duration {
            conditions.push("(scenes.duration IS NULL OR scenes.duration = 0)".to_string());
        }
        if filter.hide_watched {
            conditions.push(
                "NOT EXISTS (SELECT 1 FROM watch_history w WHERE w.scene_id = scenes.id AND w.watched = 1)"
                    .to_string(),
            );
        }
        if let Some(min_dur) = filter.min_duration {
            conditions.push(format!("scenes.duration >= {min_dur}"));
        }
        if let Some(max_dur) = filter.max_duration {
            conditions.push(format!("scenes.duration <= {max_dur}"));
        }
        if !filter.performer_names.is_empty() {
            let in_clause = filter
                .performer_names
                .iter()
                .map(|n| format!("'{}'", n.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            joins.push("JOIN scene_performers spf ON scenes.id = spf.scene_id".to_string());
            joins.push("JOIN performers pf ON spf.performer_id = pf.id".to_string());
            conditions.push(format!("pf.name IN ({in_clause})"));
        }
        if !filter.tag_names.is_empty() {
            let in_clause = filter
                .tag_names
                .iter()
                .map(|n| format!("'{}'", n.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            joins.push("JOIN scene_tags stf ON scenes.id = stf.scene_id".to_string());
            joins.push("JOIN tags tf ON stf.tag_id = tf.id".to_string());
            conditions.push(format!("tf.name IN ({in_clause})"));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let joins_sql = if joins.is_empty() {
            String::new()
        } else {
            format!("\n             {}", joins.join("\n             "))
        };

        let mut group_by = String::new();
        if !filter.performer_names.is_empty() || !filter.tag_names.is_empty() {
            let mut conditions = Vec::new();
            if !filter.performer_names.is_empty() {
                conditions.push(format!(
                    "COUNT(DISTINCT pf.name) = {}",
                    filter.performer_names.len()
                ));
            }
            if !filter.tag_names.is_empty() {
                conditions.push(format!(
                    "COUNT(DISTINCT tf.name) = {}",
                    filter.tag_names.len()
                ));
            }
            group_by = format!(" GROUP BY scenes.id HAVING {}", conditions.join(" AND "));
        }

        let sql = format!(
            "SELECT scenes.id, scenes.title, scenes.path, scenes.thumb, scenes.source_url, scenes.duration, scenes.channel, scenes.file_size, scenes.width, scenes.height
             FROM scenes{joins_sql}
             {where_clause}{group_by}
             ORDER BY scenes.created_at DESC
             LIMIT 200"
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<u32>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<u32>>(8)?,
                row.get::<_, Option<u32>>(9)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (id, title, path, thumb, source_url, duration, channel, file_size, width, height) =
                row?;

            // Post-filter: missing_thumb (file-level check not possible in SQL)
            if filter.missing_thumb {
                let needs_thumb = match &thumb {
                    None => true,
                    Some(t) if t.is_empty() => true,
                    Some(t) => !std::path::Path::new(t).is_file(),
                };
                if !needs_thumb {
                    continue;
                }
            }

            // Post-filter: hash_named (GLOB-based pre-filter in SQL is unreliable across platforms)
            if filter.hash_named {
                let is_hash = title.len() > 15
                    && title.starts_with('-')
                    && title[1..].chars().all(|c| c.is_ascii_digit())
                    && title.contains('_');
                if !is_hash {
                    continue;
                }
            }

            let performers = self.scene_performers(&conn, &id)?;
            let tags = self.scene_tags(&conn, &id)?;
            result.push(Scene {
                id,
                title,
                path,
                duration,
                thumb,
                source_url,
                studio_id: None,
                studio_name: None,
                date: None,
                rating: None,
                performers,
                tags,
                channel,
                phash: None,
                oshash: None,
                file_size: file_size.map(|v| v as u64),
                notes: None,
                width,
                height,
            });
        }
        Ok(result)
    }

    /// Find jpg files in the library directory that have no matching video in the DB.
    pub fn list_orphan_sidecars(
        &self,
        library_path: &str,
    ) -> AppResult<Vec<crate::models::OrphanSidecar>> {
        use walkdir::WalkDir;

        let lib = std::path::Path::new(library_path);
        if !lib.exists() {
            return Ok(Vec::new());
        }

        // Collect DB paths under lock, then release before filesystem walk.
        let db_paths = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;
            let mut stmt =
                conn.prepare("SELECT path FROM scenes WHERE path IS NOT NULL AND path != ''")?;
            let set: std::collections::HashSet<String> = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
            set
        };

        let mut orphans = Vec::new();
        for entry in WalkDir::new(lib)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext != "jpg" && ext != "jpeg" {
                continue;
            }
            let p_str = p.to_string_lossy().to_string();
            if !db_paths.contains(&p_str) {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                orphans.push(crate::models::OrphanSidecar { path: p_str, size });
            }
        }
        Ok(orphans)
    }

    /// Clear a scene's thumb reference (set to NULL) — used when cleaning orphan thumbs.
    pub fn clear_scene_thumb(&self, id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute("UPDATE scenes SET thumb = NULL WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_scene_thumb(&self, id: &str, thumb: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET thumb = ?2 WHERE id = ?1",
            params![id, thumb],
        )?;
        Ok(())
    }

    pub fn list_scenes(
        &self,
        query: Option<&str>,
        sort: crate::models::SceneSort,
    ) -> AppResult<Vec<Scene>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let order_by_plain = match sort {
            crate::models::SceneSort::Newest => "created_at DESC",
            crate::models::SceneSort::Name => "title COLLATE NOCASE ASC",
            crate::models::SceneSort::Downloaded => "created_at DESC",
        };
        let scenes: Vec<SceneRow> = if let Some(q) = query.filter(|s| !s.is_empty()) {
            let fts_q = format!("\"{}*\"", q.replace('"', ""));
            let like_q = format!("%{}%", q.replace('\'', "''"));
            let order_direction = match sort {
                crate::models::SceneSort::Newest | crate::models::SceneSort::Downloaded => "DESC",
                crate::models::SceneSort::Name => "COLLATE NOCASE ASC",
            };
            let sql = format!(
                "SELECT * FROM (
                     SELECT s.id, s.title, s.path, s.thumb, s.source_url, s.duration, s.channel, s.file_size, s.width, s.height, s.created_at
                     FROM scenes s JOIN scenes_fts fts ON s.rowid = fts.rowid
                     WHERE scenes_fts MATCH ?1
                     ORDER BY s.created_at DESC LIMIT 100
                 )
                 UNION
                 SELECT * FROM (
                     SELECT s.id, s.title, s.path, s.thumb, s.source_url, s.duration, s.channel, s.file_size, s.width, s.height, s.created_at
                     FROM scenes s
                     JOIN scene_performers sp ON s.id = sp.scene_id
                     JOIN performers p ON sp.performer_id = p.id
                     WHERE p.name LIKE ?2
                     ORDER BY s.created_at DESC LIMIT 100
                 )
                 UNION
                  SELECT * FROM (
                     SELECT s.id, s.title, s.path, s.thumb, s.source_url, s.duration, s.channel, s.file_size, s.width, s.height, s.created_at
                     FROM scenes s
                     JOIN scene_tags st ON s.id = st.scene_id
                     JOIN tags t ON st.tag_id = t.id
                     WHERE t.name LIKE ?2
                     ORDER BY s.created_at DESC LIMIT 100
                 )
                 ORDER BY created_at {order_direction}
                 LIMIT 100"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![fts_q, like_q], |row| {
                Ok((
                    row.get(0)?, // id
                    row.get(1)?, // title
                    row.get(2)?, // path
                    row.get(3)?, // thumb
                    row.get(4)?, // source_url
                    row.get(5)?, // duration
                    row.get(6)?, // channel
                    row.get(7)?, // file_size
                    row.get(8)?, // width
                    row.get(9)?, // height
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let sql = format!(
                "SELECT id, title, path, thumb, source_url, duration, channel, file_size, width, height FROM scenes ORDER BY {order_by_plain} LIMIT 100"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut result = Vec::new();
        for (id, title, path, thumb, source_url, duration, channel, file_size, width, height) in
            scenes
        {
            let performers = self.scene_performers(&conn, &id)?;
            let tags = self.scene_tags(&conn, &id)?;
            result.push(Scene {
                id,
                title,
                path,
                duration,
                thumb,
                source_url,
                studio_id: None,
                studio_name: None,
                date: None,
                rating: None,
                performers,
                tags,
                channel,
                phash: None,
                oshash: None,
                file_size: file_size.map(|v| v as u64),
                notes: None,
                width,
                height,
            });
        }
        Ok(result)
    }

    pub fn scene_by_path(&self, path: &str) -> AppResult<Option<Scene>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let row = conn
            .query_row(
                "SELECT id, title, path, thumb, source_url, duration, channel, file_size, width, height FROM scenes WHERE path = ?1",
                params![path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<u32>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<i64>>(7)?,
                        row.get::<_, Option<u32>>(8)?,
                        row.get::<_, Option<u32>>(9)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(
            |(id, title, path, thumb, source_url, duration, channel, file_size, width, height)| {
                let performers = self.scene_performers(&conn, &id).unwrap_or_default();
                let tags = self.scene_tags(&conn, &id).unwrap_or_default();
                Scene {
                    id,
                    title,
                    path,
                    duration,
                    thumb,
                    source_url,
                    studio_id: None,
                    studio_name: None,
                    date: None,
                    rating: None,
                    performers,
                    tags,
                    channel,
                    phash: None,
                    oshash: None,
                    file_size: file_size.map(|v| v as u64),
                    notes: None,
                    width,
                    height,
                }
            },
        ))
    }

    fn scene_performers(&self, conn: &Connection, scene_id: &str) -> AppResult<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT p.name FROM performers p
             JOIN scene_performers sp ON sp.performer_id = p.id
             WHERE sp.scene_id = ?1",
        )?;
        let rows = stmt.query_map(params![scene_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    fn scene_tags(&self, conn: &Connection, scene_id: &str) -> AppResult<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT t.name FROM tags t
             JOIN scene_tags st ON st.tag_id = t.id
             WHERE st.scene_id = ?1",
        )?;
        let rows = stmt.query_map(params![scene_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn list_performers(&self, query: Option<&str>) -> AppResult<Vec<Performer>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut sql = String::from(
            "SELECT p.id, p.name, p.aliases, p.image, p.favorite,
                    (SELECT COUNT(*) FROM scene_performers sp WHERE sp.performer_id = p.id) as scene_count
             FROM performers p",
        );
        if query.filter(|s| !s.is_empty()).is_some() {
            sql.push_str(" WHERE p.name LIKE ?1");
        }
        sql.push_str(" ORDER BY p.name LIMIT 200");
        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(q) = query.filter(|s| !s.is_empty()) {
            let pattern = format!("%{q}%");
            stmt.query_map(params![pattern], map_performer)?
        } else {
            stmt.query_map([], map_performer)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn list_tags(&self) -> AppResult<Vec<Tag>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.parent_id,
                    (SELECT COUNT(*) FROM scene_tags st WHERE st.tag_id = t.id) as scene_count
             FROM tags t ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                scene_count: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    /// Load all known scene paths into a HashSet for O(1) lookups during scan.
    pub fn all_scene_paths(&self) -> AppResult<std::collections::HashSet<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt =
            conn.prepare("SELECT path FROM scenes WHERE path IS NOT NULL AND path != ''")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut set = std::collections::HashSet::new();
        for row in rows {
            set.insert(row?);
        }
        Ok(set)
    }

    pub fn find_duplicate_groups(&self, phash_threshold: u8) -> AppResult<Vec<DuplicateGroup>> {
        // Phase 1: Load all data under lock, then release.
        let (phash_entries, oshash_groups) = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;

            let mut phash_entries: Vec<(String, String)> = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT id, phash FROM scenes WHERE phash IS NOT NULL AND phash != ''",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                for row in rows {
                    phash_entries.push(row?);
                }
            }

            let mut oshash_groups: Vec<(String, String)> = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT oshash, GROUP_CONCAT(id) FROM scenes
                     WHERE oshash IS NOT NULL AND oshash != ''
                     GROUP BY oshash HAVING COUNT(*) > 1",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                for row in rows {
                    oshash_groups.push(row?);
                }
            }

            (phash_entries, oshash_groups)
        };

        // Phase 2: Compute duplicates without holding the lock.
        let mut groups = Vec::new();

        let clusters =
            crate::library::duplicates::cluster_phash_ids(&phash_entries, phash_threshold);
        for ids in clusters {
            let ids_csv = ids.join(",");
            let scenes = self.scenes_by_ids_from_csv(&ids_csv)?;
            if scenes.len() < 2 {
                continue;
            }
            groups.push(crate::library::duplicates::build_phash_group(
                &phash_entries,
                &ids,
                scenes,
            )?);
        }

        for (oshash, ids_csv) in oshash_groups {
            let scenes = self.scenes_by_ids_from_csv(&ids_csv)?;
            groups.push(DuplicateGroup {
                match_type: "oshash".to_string(),
                hash: oshash,
                scenes,
                max_distance: None,
            });
        }

        Ok(groups)
    }

    pub fn get_scene(&self, scene_id: &str) -> AppResult<Scene> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let row = conn
            .query_row(
                "SELECT id, title, path, thumb, source_url, phash, oshash, duration, channel, notes, width, height FROM scenes WHERE id = ?1",
                params![scene_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<u32>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<u32>>(10)?,
                        row.get::<_, Option<u32>>(11)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            id,
            title,
            path,
            thumb,
            source_url,
            phash,
            oshash,
            duration,
            channel,
            notes,
            width,
            height,
        )) = row
        else {
            return Err(AppError::NotFound(format!("scene {scene_id}")));
        };
        let performers = self.scene_performers(&conn, &id)?;
        let tags = self.scene_tags(&conn, &id)?;
        let file_size = path
            .as_deref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len());
        Ok(Scene {
            id,
            title,
            path,
            duration,
            thumb,
            source_url,
            studio_id: None,
            studio_name: None,
            date: None,
            rating: None,
            performers,
            tags,
            channel,
            phash,
            oshash,
            file_size,
            notes,
            width,
            height,
        })
    }

    pub fn batch_update_scenes(
        &self,
        ids: &[String],
        performers_add: Option<&[String]>,
        tags_add: Option<&[String]>,
    ) -> AppResult<u32> {
        let mut updated = 0u32;
        for id in ids {
            let scene = self.get_scene(id)?;
            let mut performers = scene.performers;
            if let Some(add) = performers_add {
                for name in add {
                    let name = name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    if !performers.iter().any(|p| p.eq_ignore_ascii_case(name)) {
                        performers.push(name.to_string());
                    }
                }
            }
            let mut tags = scene.tags;
            if let Some(add) = tags_add {
                for name in add {
                    let name = name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    if !tags.iter().any(|t| t.eq_ignore_ascii_case(name)) {
                        tags.push(name.to_string());
                    }
                }
            }
            self.update_scene(id, None, Some(&performers), Some(&tags), false, None)?;
            updated += 1;
        }
        Ok(updated)
    }

    pub fn update_scene(
        &self,
        id: &str,
        title: Option<&str>,
        performers: Option<&[String]>,
        tags: Option<&[String]>,
        rename_file: bool,
        notes: Option<&str>,
    ) -> AppResult<Scene> {
        let existing = self.get_scene(id)?;
        let new_title = title.unwrap_or(&existing.title);
        let new_notes = notes.unwrap_or(existing.notes.as_deref().unwrap_or(""));
        let mut new_path = existing.path.clone();

        if rename_file {
            if let Some(ref old_path) = existing.path {
                let old = std::path::Path::new(old_path);
                if old.exists() {
                    let ext = old.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
                    let safe: String = new_title
                        .chars()
                        .map(|c| {
                            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    let safe = safe.trim().replace(' ', "_");
                    let parent = old.parent().unwrap_or_else(|| std::path::Path::new("."));
                    let target = parent.join(format!("{safe}.{ext}"));
                    if target != old {
                        std::fs::rename(old, &target)?;
                        new_path = Some(target.to_string_lossy().to_string());
                    }
                }
            }
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET title = ?2, path = COALESCE(?3, path), notes = ?4 WHERE id = ?1",
            params![id, new_title, new_path, new_notes],
        )?;
        drop(conn);

        if let Some(p) = performers {
            {
                let conn = self
                    .conn
                    .lock()
                    .map_err(|e| AppError::Other(e.to_string()))?;
                conn.execute(
                    "DELETE FROM scene_performers WHERE scene_id = ?1",
                    params![id],
                )?;
            }
            let conn = self
                .conn
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;
            for name in p {
                let pid = Self::upsert_performer_with_conn(&conn, name)?;
                conn.execute(
                    "INSERT OR IGNORE INTO scene_performers (scene_id, performer_id) VALUES (?1, ?2)",
                    params![id, pid],
                )?;
            }
        }

        if let Some(t) = tags {
            {
                let conn = self
                    .conn
                    .lock()
                    .map_err(|e| AppError::Other(e.to_string()))?;
                conn.execute("DELETE FROM scene_tags WHERE scene_id = ?1", params![id])?;
            }
            let conn = self
                .conn
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;
            for name in t {
                let tid = Self::upsert_tag_with_conn(&conn, name)?;
                conn.execute(
                    "INSERT OR IGNORE INTO scene_tags (scene_id, tag_id) VALUES (?1, ?2)",
                    params![id, tid],
                )?;
            }
        }

        self.get_scene(id)
    }

    pub fn delete_scene(&self, id: &str, delete_files: bool) -> AppResult<()> {
        let (path, thumb) = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;
            let row: Option<(Option<String>, Option<String>)> = conn
                .query_row(
                    "SELECT path, thumb FROM scenes WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let Some(row) = row else {
                return Err(AppError::NotFound(format!("scene {id}")));
            };

            conn.execute(
                "DELETE FROM scene_performers WHERE scene_id = ?1",
                params![id],
            )?;
            conn.execute("DELETE FROM scene_tags WHERE scene_id = ?1", params![id])?;
            conn.execute("DELETE FROM watch_history WHERE scene_id = ?1", params![id])?;
            conn.execute("DELETE FROM scenes WHERE id = ?1", params![id])?;
            row
        };

        if delete_files {
            if let Some(p) = path {
                let _ = std::fs::remove_file(p);
            }
            if let Some(t) = thumb {
                let _ = std::fs::remove_file(t);
            }
        }
        Ok(())
    }

    pub fn merge_duplicates(
        &self,
        keep_id: &str,
        remove_ids: &[String],
        delete_files: bool,
    ) -> AppResult<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM scenes WHERE id = ?1",
                params![keep_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)?;
        if !exists {
            return Err(AppError::NotFound(format!("scene {keep_id}")));
        }

        let mut removed = 0u32;
        drop(conn);
        for id in remove_ids {
            if id == keep_id {
                continue;
            }
            self.delete_scene(id, delete_files)?;
            removed += 1;
        }
        Ok(removed)
    }

    fn scenes_by_ids(&self, conn: &Connection, ids_csv: &str) -> AppResult<Vec<Scene>> {
        let mut scenes = Vec::new();
        for id in ids_csv.split(',') {
            let id = id.trim();
            if id.is_empty() {
                continue;
            }
            if let Some(row) = conn
                .query_row(
                    "SELECT id, title, path, thumb, source_url, channel, file_size, width, height FROM scenes WHERE id = ?1",
                    params![id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<i64>>(6)?,
                            row.get::<_, Option<u32>>(7)?,
                            row.get::<_, Option<u32>>(8)?,
                        ))
                    },
                )
                .optional()?
            {
                let (id, title, path, thumb, source_url, channel, file_size, width, height) = row;
                let performers = self.scene_performers(conn, &id)?;
                let tags = self.scene_tags(conn, &id)?;
                scenes.push(Scene {
                    id,
                    title,
                    path,
                    duration: None,
                    thumb,
                    source_url,
                    studio_id: None,
                    studio_name: None,
                    date: None,
                    rating: None,
                    performers,
                    tags,
                    channel,
                    phash: None,
                    oshash: None,
                    file_size: file_size.map(|v| v as u64),
                notes: None,
                    width,
                    height,
                });
            }
        }
        Ok(scenes)
    }

    /// Like `scenes_by_ids` but acquires the lock internally — for use outside the lock.
    fn scenes_by_ids_from_csv(&self, ids_csv: &str) -> AppResult<Vec<Scene>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        self.scenes_by_ids(&conn, ids_csv)
    }
}

fn map_performer(row: &rusqlite::Row<'_>) -> rusqlite::Result<Performer> {
    let aliases_json: String = row.get(2)?;
    let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();
    Ok(Performer {
        id: row.get(0)?,
        name: row.get(1)?,
        aliases,
        image: row.get(3)?,
        favorite: row.get::<_, i32>(4)? != 0,
        scene_count: row.get(5)?,
    })
}

fn parse_status(s: &str) -> DownloadStatus {
    match s.to_lowercase().as_str() {
        "active" => DownloadStatus::Active,
        "paused" => DownloadStatus::Paused,
        "completed" => DownloadStatus::Completed,
        "failed" => DownloadStatus::Failed,
        "cancelled" => DownloadStatus::Cancelled,
        _ => DownloadStatus::Pending,
    }
}
