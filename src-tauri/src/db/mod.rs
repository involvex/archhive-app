mod migrations;

use crate::db::migrations::{MIGRATION_001, MIGRATION_002};
use crate::error::{AppError, AppResult};
use crate::models::{
    AppSettings, DownloadJob, DownloadStatus, DuplicateGroup, Performer, Scene, Tag,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

type SceneRow = (String, String, Option<String>, Option<String>, Option<String>);

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("scrawler.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch(MIGRATION_001)?;
        conn.execute_batch(MIGRATION_002)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let json = serde_json::to_string(settings)
            .map_err(|e| AppError::Other(format!("settings serialize: {e}")))?;
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }

    pub fn insert_download_job(&self, url: &str, adapter: &str, title: Option<&str>) -> AppResult<DownloadJob> {
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
        };
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "INSERT INTO download_jobs (id, url, adapter, status, progress, title, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                job.id,
                job.url,
                job.adapter,
                format!("{:?}", job.status).to_lowercase(),
                job.progress,
                job.title,
                job.created_at,
            ],
        )?;
        Ok(job)
    }

    pub fn update_download_job(&self, job: &DownloadJob) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let status = format!("{:?}", job.status).to_lowercase();
        conn.execute(
            "UPDATE download_jobs SET status = ?2, progress = ?3, output_path = ?4, error = ?5, title = ?6
             WHERE id = ?1",
            params![job.id, status, job.progress, job.output_path, job.error, job.title],
        )?;
        Ok(())
    }

    pub fn list_download_jobs(&self) -> AppResult<Vec<DownloadJob>> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, url, adapter, status, progress, output_path, error, title, created_at
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
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn get_download_job(&self, id: &str) -> AppResult<Option<DownloadJob>> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        conn.query_row(
            "SELECT id, url, adapter, status, progress, output_path, error, title, created_at
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
                })
            },
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn upsert_performer(&self, name: &str) -> AppResult<String> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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

    pub fn upsert_tag(&self, name: &str) -> AppResult<String> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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
    ) -> AppResult<String> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO scenes (id, title, path, source_url, thumb, phash, oshash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, title, path, source_url, thumb, phash, oshash, now],
        )?;
        for p in performers {
            let pid = self.upsert_performer(p)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_performers (scene_id, performer_id) VALUES (?1, ?2)",
                params![id, pid],
            )?;
        }
        for t in tags {
            let tid = self.upsert_tag(t)?;
            conn.execute(
                "INSERT OR IGNORE INTO scene_tags (scene_id, tag_id) VALUES (?1, ?2)",
                params![id, tid],
            )?;
        }
        Ok(id)
    }

    pub fn update_scene_path(&self, id: &str, path: &str, thumb: Option<&str>) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "UPDATE scenes SET phash = COALESCE(?2, phash), oshash = COALESCE(?3, oshash), thumb = COALESCE(?4, thumb) WHERE id = ?1",
            params![id, phash, oshash, thumb],
        )?;
        Ok(())
    }

    pub fn list_scenes(&self, query: Option<&str>) -> AppResult<Vec<Scene>> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let scenes: Vec<SceneRow> =
            if let Some(q) = query.filter(|s| !s.is_empty()) {
                let mut stmt = conn.prepare(
                    "SELECT s.id, s.title, s.path, s.thumb, s.source_url
                     FROM scenes s
                     JOIN scenes_fts fts ON s.rowid = fts.rowid
                     WHERE scenes_fts MATCH ?1
                     ORDER BY s.created_at DESC LIMIT 100",
                )?;
                let fts_q = format!("\"{}*\"", q.replace('"', ""));
                let rows = stmt.query_map(params![fts_q], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, title, path, thumb, source_url FROM scenes ORDER BY created_at DESC LIMIT 100",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };

        let mut result = Vec::new();
        for (id, title, path, thumb, source_url) in scenes {
            let performers = self.scene_performers(&conn, &id)?;
            let tags = self.scene_tags(&conn, &id)?;
            result.push(Scene {
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
            });
        }
        Ok(result)
    }

    pub fn scene_by_path(&self, path: &str) -> AppResult<Option<Scene>> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let row = conn
            .query_row(
                "SELECT id, title, path, thumb, source_url FROM scenes WHERE path = ?1",
                params![path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(id, title, path, thumb, source_url)| {
            let performers = self.scene_performers(&conn, &id).unwrap_or_default();
            let tags = self.scene_tags(&conn, &id).unwrap_or_default();
            Scene {
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
            }
        }))
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
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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

    pub fn scene_exists_by_path(&self, path: &str) -> AppResult<bool> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM scenes WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn find_duplicate_groups(&self, phash_threshold: u8) -> AppResult<Vec<DuplicateGroup>> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let mut groups = Vec::new();

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

        let clusters = crate::library::duplicates::cluster_phash_ids(&phash_entries, phash_threshold);
        for ids in clusters {
            let ids_csv = ids.join(",");
            let scenes = self.scenes_by_ids(&conn, &ids_csv)?;
            if scenes.len() < 2 {
                continue;
            }
            groups.push(crate::library::duplicates::build_phash_group(
                &phash_entries,
                &ids,
                scenes,
            )?);
        }

        let mut stmt = conn.prepare(
            "SELECT oshash, GROUP_CONCAT(id) FROM scenes
             WHERE oshash IS NOT NULL AND oshash != ''
             GROUP BY oshash HAVING COUNT(*) > 1",
        )?;
        let oshash_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in oshash_rows {
            let (oshash, ids_csv) = row?;
            let scenes = self.scenes_by_ids(&conn, &ids_csv)?;
            groups.push(DuplicateGroup {
                match_type: "oshash".to_string(),
                hash: oshash,
                scenes,
                max_distance: None,
            });
        }

        Ok(groups)
    }

    pub fn delete_scene(&self, id: &str, delete_files: bool) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let row: Option<(Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT path, thumb FROM scenes WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((path, thumb)) = row else {
            return Err(AppError::NotFound(format!("scene {id}")));
        };

        conn.execute("DELETE FROM scene_performers WHERE scene_id = ?1", params![id])?;
        conn.execute("DELETE FROM scene_tags WHERE scene_id = ?1", params![id])?;
        conn.execute("DELETE FROM scenes WHERE id = ?1", params![id])?;

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
        let conn = self.conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
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
                    "SELECT id, title, path, thumb, source_url FROM scenes WHERE id = ?1",
                    params![id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .optional()?
            {
                let (id, title, path, thumb, source_url) = row;
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
                });
            }
        }
        Ok(scenes)
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
        "completed" => DownloadStatus::Completed,
        "failed" => DownloadStatus::Failed,
        "cancelled" => DownloadStatus::Cancelled,
        _ => DownloadStatus::Pending,
    }
}
