use crate::db::Database;
use crate::error::AppResult;

pub fn import_download(
    db: &Database,
    title: &str,
    path: Option<&str>,
    source_url: Option<&str>,
    performers: &[String],
    tags: &[String],
    thumb: Option<&str>,
    phash: Option<&str>,
    oshash: Option<&str>,
) -> AppResult<String> {
    if let Some(p) = path {
        if let Some(existing) = db.scene_by_path(p)? {
            db.update_scene_hashes(&existing.id, phash, oshash, thumb)?;
            return Ok(existing.id);
        }
    }
    db.insert_scene(title, path, source_url, performers, tags, thumb, phash, oshash)
}
