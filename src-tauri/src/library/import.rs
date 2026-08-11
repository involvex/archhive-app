use crate::db::Database;
use crate::error::AppResult;

#[allow(clippy::too_many_arguments)]
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
    duration: Option<u32>,
    channel: Option<&str>,
) -> AppResult<String> {
    if let Some(p) = path {
        if let Some(existing) = db.scene_by_path(p)? {
            db.update_scene_hashes(&existing.id, phash, oshash, thumb)?;
            if let Some(dur) = duration {
                db.update_scene_duration(&existing.id, dur)?;
            }
            if !performers.is_empty() {
                db.replace_scene_performers(&existing.id, performers)?;
            }
            if !tags.is_empty() {
                db.replace_scene_tags(&existing.id, tags)?;
            }
            return Ok(existing.id);
        }
    }
    db.insert_scene(
        title, path, source_url, performers, tags, thumb, phash, oshash, duration, channel,
    )
}
