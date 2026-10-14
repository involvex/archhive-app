//! Local "Online" library: listing results kept on the device so saved
//! searches (and everything browsed before) can be filtered, sorted and
//! searched without asking the site again.

use super::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    MediaItem, OnlineItem, OnlinePage, OnlineQuery, OnlineSearchInfo, OnlineTagCount,
};
use rusqlite::{params, params_from_iter, OptionalExtension};

/// Tag as stored for lookups: lowercase, single spaces.
fn norm_tag(tag: &str) -> String {
    tag.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn json_list(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn parse_list(raw: Option<String>) -> Vec<String> {
    raw.and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default()
}

const ITEM_COLUMNS: &str = "o.key, o.url, o.site_id, o.title, o.thumbnail, o.duration, o.channel, \
     o.performers, o.tags, o.first_seen, o.last_seen, o.details_at";

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<OnlineItem> {
    Ok(OnlineItem {
        key: row.get(0)?,
        url: row.get(1)?,
        site_id: row.get(2)?,
        title: row.get(3)?,
        thumbnail: row.get(4)?,
        duration: row.get::<_, Option<i64>>(5)?.map(|d| d as u32),
        channel: row.get(6)?,
        performers: parse_list(row.get(7)?),
        tags: parse_list(row.get(8)?),
        first_seen: row.get(9)?,
        last_seen: row.get(10)?,
        details_at: row.get(11)?,
    })
}

fn replace_tags(conn: &rusqlite::Connection, key: &str, tags: &[String]) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM online_item_tags WHERE item_key = ?1",
        params![key],
    )?;
    let mut stmt =
        conn.prepare("INSERT OR IGNORE INTO online_item_tags (item_key, tag) VALUES (?1, ?2)")?;
    for t in tags.iter().map(|t| norm_tag(t)).filter(|t| !t.is_empty()) {
        stmt.execute(params![key, t])?;
    }
    Ok(())
}

impl Database {
    /// Record listing results. With `search`, they also join that saved
    /// search's slice, ranked from `base_rank` in listing order. Returns how
    /// many were new (to the search when given, otherwise to the library).
    pub fn record_online_items(
        &self,
        items: &[MediaItem],
        search: Option<(&str, i64)>,
    ) -> AppResult<u32> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut new_count = 0u32;
        for (i, item) in items.iter().enumerate() {
            if item.url.trim().is_empty() || item.is_live == Some(true) {
                continue;
            }
            let key = crate::sites::urls::video_key(&item.url);
            let dismissed = |scope: &str| -> rusqlite::Result<bool> {
                tx.query_row(
                    "SELECT 1 FROM online_dismissed WHERE scope = ?1 AND item_key = ?2",
                    params![scope, key],
                    |_| Ok(()),
                )
                .optional()
                .map(|r| r.is_some())
            };
            // Deleted from history: not recorded again.
            if dismissed("")? {
                continue;
            }
            let existing: Option<Option<String>> = tx
                .query_row(
                    "SELECT details_at FROM online_items WHERE key = ?1",
                    params![key],
                    |r| r.get(0),
                )
                .optional()?;
            match &existing {
                None => {
                    tx.execute(
                        "INSERT INTO online_items (key, url, site_id, title, thumbnail, duration,
                             channel, performers, tags, first_seen, last_seen)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                        params![
                            key,
                            item.url,
                            item.site_id,
                            item.title,
                            item.thumbnail,
                            item.duration,
                            item.channel,
                            json_list(&item.performers),
                            json_list(&item.tags),
                            now,
                        ],
                    )?;
                    replace_tags(&tx, &key, &item.tags)?;
                    if search.is_none() {
                        new_count += 1;
                    }
                }
                Some(details_at) => {
                    tx.execute(
                        "UPDATE online_items SET url = ?2, site_id = ?3, title = ?4,
                             thumbnail = COALESCE(?5, thumbnail),
                             duration = COALESCE(?6, duration),
                             channel = COALESCE(channel, ?7),
                             last_seen = ?8
                         WHERE key = ?1",
                        params![
                            key,
                            item.url,
                            item.site_id,
                            item.title,
                            item.thumbnail,
                            item.duration,
                            item.channel,
                            now,
                        ],
                    )?;
                    // Listing tags only until the video page's own were fetched.
                    if details_at.is_none() && !item.tags.is_empty() {
                        tx.execute(
                            "UPDATE online_items SET tags = ?2 WHERE key = ?1",
                            params![key, json_list(&item.tags)],
                        )?;
                        replace_tags(&tx, &key, &item.tags)?;
                    }
                }
            }
            if let Some((search_id, base_rank)) = search {
                // Removed from this saved search: Update doesn't re-add it.
                if dismissed(search_id)? {
                    continue;
                }
                let added = tx.execute(
                    "INSERT OR IGNORE INTO saved_search_items (search_id, item_key, rank, first_seen)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![search_id, key, base_rank + i as i64, now],
                )?;
                if added > 0 {
                    new_count += 1;
                } else {
                    tx.execute(
                        "UPDATE saved_search_items SET rank = ?3
                         WHERE search_id = ?1 AND item_key = ?2",
                        params![search_id, key, base_rank + i as i64],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(new_count)
    }

    pub fn mark_online_search_updated(&self, search_id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "INSERT INTO online_search_updates (search_id, updated_at) VALUES (?1, ?2)
             ON CONFLICT(search_id) DO UPDATE SET updated_at = excluded.updated_at",
            params![search_id, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn online_search_updated_at(&self, search_id: &str) -> AppResult<Option<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        Ok(conn
            .query_row(
                "SELECT updated_at FROM online_search_updates WHERE search_id = ?1",
                params![search_id],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Tags / performers / channel from the video's own page.
    pub fn set_online_item_details(
        &self,
        key: &str,
        details: &MediaItem,
    ) -> AppResult<Option<OnlineItem>> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE online_items SET
                 performers = ?2, tags = ?3,
                 channel = COALESCE(?4, channel),
                 duration = COALESCE(duration, ?5),
                 details_at = ?6
             WHERE key = ?1",
            params![
                key,
                json_list(&details.performers),
                json_list(&details.tags),
                details.channel,
                details.duration,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;
        replace_tags(&tx, key, &details.tags)?;
        let item = tx
            .query_row(
                &format!("SELECT {ITEM_COLUMNS} FROM online_items o WHERE o.key = ?1"),
                params![key],
                map_item,
            )
            .optional()?;
        tx.commit()?;
        Ok(item)
    }

    pub fn get_online_item(&self, key: &str) -> AppResult<Option<OnlineItem>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        Ok(conn
            .query_row(
                &format!("SELECT {ITEM_COLUMNS} FROM online_items o WHERE o.key = ?1"),
                params![key],
                map_item,
            )
            .optional()?)
    }

    pub fn query_online_items(&self, q: &OnlineQuery) -> AppResult<OnlinePage> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut from = String::from("FROM online_items o");
        let mut wheres: Vec<String> = Vec::new();
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        let search_id = q.search_id.as_deref().filter(|s| !s.is_empty());
        if let Some(id) = search_id {
            from.push_str(" JOIN saved_search_items s ON s.item_key = o.key AND s.search_id = ?");
            args.push(id.to_string().into());
        }
        for word in q
            .text
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_lowercase)
        {
            wheres.push(
                "(lower(o.title) LIKE ? OR lower(COALESCE(o.channel, '')) LIKE ? \
                 OR lower(o.performers) LIKE ? OR lower(o.tags) LIKE ?)"
                    .into(),
            );
            let like = format!("%{word}%");
            for _ in 0..4 {
                args.push(like.clone().into());
            }
        }
        // A tag counts when the item has it, or (most items have no tags
        // until fetched) when the title contains it.
        let tag_match = "(EXISTS (SELECT 1 FROM online_item_tags t \
                          WHERE t.item_key = o.key AND t.tag = ?) OR lower(o.title) LIKE ?)";
        for tag in q
            .include_tags
            .iter()
            .map(|t| norm_tag(t))
            .filter(|t| !t.is_empty())
        {
            wheres.push(tag_match.into());
            args.push(tag.clone().into());
            args.push(format!("%{tag}%").into());
        }
        for tag in q
            .exclude_tags
            .iter()
            .map(|t| norm_tag(t))
            .filter(|t| !t.is_empty())
        {
            wheres.push(format!("NOT {tag_match}"));
            args.push(tag.clone().into());
            args.push(format!("%{tag}%").into());
        }
        if let Some(min) = q.min_duration.filter(|m| *m > 0) {
            wheres.push("o.duration >= ?".into());
            args.push((min as i64).into());
        }
        if let Some(max) = q.max_duration.filter(|m| *m > 0) {
            wheres.push("o.duration < ?".into());
            args.push((max as i64).into());
        }
        if q.missing_details {
            wheres.push("o.details_at IS NULL".into());
        }
        let where_sql = if wheres.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", wheres.join(" AND "))
        };
        let order = match q.sort.as_deref().unwrap_or("") {
            "longest" => "o.duration IS NULL, o.duration DESC",
            "shortest" => "o.duration IS NULL, o.duration ASC",
            "title" => "lower(o.title) ASC",
            "site" if search_id.is_some() => "s.rank ASC",
            "newest" if search_id.is_some() => "s.first_seen DESC, s.rank ASC",
            _ if search_id.is_some() => "s.rank ASC",
            _ => "o.last_seen DESC",
        };
        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) {from}{where_sql}"),
            params_from_iter(args.iter()),
            |r| r.get(0),
        )?;
        let limit = if q.limit == 0 { 60 } else { q.limit.min(500) };
        let mut page_args = args.clone();
        page_args.push((limit as i64).into());
        page_args.push((q.offset as i64).into());
        let mut stmt = conn.prepare(&format!(
            "SELECT {ITEM_COLUMNS} {from}{where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
        ))?;
        let items = stmt
            .query_map(params_from_iter(page_args.iter()), map_item)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OnlinePage {
            items,
            total: total as u32,
        })
    }

    /// Most common tags (of one saved search, or the whole library).
    pub fn online_tag_counts(
        &self,
        search_id: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<OnlineTagCount>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let limit = limit.clamp(1, 2000) as i64;
        let rows = match search_id.filter(|s| !s.is_empty()) {
            Some(id) => {
                let mut stmt = conn.prepare(
                    "SELECT t.tag, COUNT(*) AS n FROM online_item_tags t
                     JOIN saved_search_items s ON s.item_key = t.item_key AND s.search_id = ?1
                     GROUP BY t.tag ORDER BY n DESC, t.tag LIMIT ?2",
                )?;
                let r = stmt
                    .query_map(params![id, limit], |r| {
                        Ok(OnlineTagCount {
                            tag: r.get(0)?,
                            count: r.get::<_, i64>(1)? as u32,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                r
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT tag, COUNT(*) AS n FROM online_item_tags
                     GROUP BY tag ORDER BY n DESC, tag LIMIT ?1",
                )?;
                let r = stmt
                    .query_map(params![limit], |r| {
                        Ok(OnlineTagCount {
                            tag: r.get(0)?,
                            count: r.get::<_, i64>(1)? as u32,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                r
            }
        };
        Ok(rows)
    }

    /// Item count and last update per saved search.
    pub fn online_search_info(&self) -> AppResult<Vec<OnlineSearchInfo>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT ss.id,
                    (SELECT COUNT(*) FROM saved_search_items i WHERE i.search_id = ss.id),
                    u.updated_at
             FROM saved_searches ss
             LEFT JOIN online_search_updates u ON u.search_id = ss.id",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(OnlineSearchInfo {
                    search_id: r.get(0)?,
                    item_count: r.get::<_, i64>(1)? as u32,
                    updated_at: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Remove entries: from one saved search (`search_id`), or from the
    /// library altogether. Either way they are remembered as removed so
    /// updates / browsing don't bring them back. Returns how many went.
    pub fn delete_online_items(&self, search_id: Option<&str>, keys: &[String]) -> AppResult<u32> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let tx = conn.transaction()?;
        let scope = search_id.filter(|s| !s.is_empty()).unwrap_or("");
        let mut removed = 0u32;
        for key in keys {
            let n = if scope.is_empty() {
                tx.execute(
                    "DELETE FROM saved_search_items WHERE item_key = ?1",
                    params![key],
                )?;
                tx.execute(
                    "DELETE FROM online_item_tags WHERE item_key = ?1",
                    params![key],
                )?;
                tx.execute("DELETE FROM online_items WHERE key = ?1", params![key])?
            } else {
                tx.execute(
                    "DELETE FROM saved_search_items WHERE search_id = ?1 AND item_key = ?2",
                    params![scope, key],
                )?
            };
            tx.execute(
                "INSERT OR IGNORE INTO online_dismissed (scope, item_key) VALUES (?1, ?2)",
                params![scope, key],
            )?;
            removed += n as u32;
        }
        tx.commit()?;
        Ok(removed)
    }

    /// Remove everything the query currently shows (all pages).
    pub fn delete_online_matching(&self, q: &OnlineQuery) -> AppResult<u32> {
        let mut all = q.clone();
        all.offset = 0;
        all.limit = 500;
        let mut keys = Vec::new();
        loop {
            let page = self.query_online_items(&all)?;
            let n = page.items.len();
            keys.extend(page.items.into_iter().map(|i| i.key));
            if n < 500 {
                break;
            }
            all.offset += 500;
        }
        self.delete_online_items(q.search_id.as_deref(), &keys)
    }

    /// Forget browsing history (items not in any saved search).
    pub fn clear_online_history(&self) -> AppResult<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let removed = conn.execute(
            "DELETE FROM online_items
             WHERE key NOT IN (SELECT item_key FROM saved_search_items)",
            [],
        )?;
        conn.execute(
            "DELETE FROM online_item_tags
             WHERE item_key NOT IN (SELECT key FROM online_items)",
            [],
        )?;
        Ok(removed as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(url: &str, title: &str, dur: u32, tags: &[&str]) -> MediaItem {
        MediaItem {
            id: url.into(),
            url: url.into(),
            title: title.into(),
            site_id: "xpomelo".into(),
            duration: Some(dur),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn records_filters_and_ranks() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::new(dir.path().to_path_buf()).unwrap();
        let page = vec![
            item(
                "https://www.xvideos.com/video.abc/blonde_teacher",
                "Blonde teacher",
                1500,
                &[],
            ),
            item(
                "https://www.pornhub.com/view_video.php?viewkey=1",
                "Trailer",
                90,
                &["Anal"],
            ),
            item(
                "https://spankbang.com/x/video/y",
                "Long one",
                2400,
                &["Big Tits", "blonde"],
            ),
        ];
        assert_eq!(db.record_online_items(&page, None).unwrap(), 3);
        // Same xvideos video under its translated slug: not new.
        let again = vec![item(
            "https://www.xvideos.com/video.abc/professora_loira",
            "Blonde teacher",
            1500,
            &[],
        )];
        assert_eq!(db.record_online_items(&again, None).unwrap(), 0);
        assert_eq!(db.record_online_items(&page, Some(("s1", 0))).unwrap(), 3);
        assert_eq!(db.record_online_items(&page, Some(("s1", 0))).unwrap(), 0);

        let q = |f: &dyn Fn(&mut OnlineQuery)| {
            let mut q = OnlineQuery {
                search_id: Some("s1".into()),
                ..Default::default()
            };
            f(&mut q);
            db.query_online_items(&q).unwrap()
        };
        let all = q(&|_| {});
        assert_eq!(all.total, 3);
        assert_eq!(all.items[0].title, "Blonde teacher");
        // 10+ minutes drops the trailer.
        assert_eq!(q(&|q| q.min_duration = Some(600)).total, 2);
        // "blonde": tag on one, title word on the other.
        assert_eq!(q(&|q| q.include_tags = vec!["Blonde".into()]).total, 2);
        assert_eq!(
            q(&|q| {
                q.include_tags = vec!["blonde".into()];
                q.exclude_tags = vec!["big tits".into()];
            })
            .total,
            1
        );
        assert_eq!(q(&|q| q.exclude_tags = vec!["anal".into()]).total, 2);
        let longest = q(&|q| q.sort = Some("longest".into()));
        assert_eq!(longest.items[0].title, "Long one");
        assert_eq!(q(&|q| q.text = Some("long".into())).total, 1);

        let tags = db.online_tag_counts(Some("s1"), 10).unwrap();
        assert!(tags.iter().any(|t| t.tag == "big tits" && t.count == 1));

        let key = crate::sites::urls::video_key("https://www.xvideos.com/video.abc/x");
        let details = MediaItem {
            tags: vec!["Teacher".into()],
            performers: vec!["Riley Reid".into()],
            ..Default::default()
        };
        let updated = db.set_online_item_details(&key, &details).unwrap().unwrap();
        assert_eq!(updated.performers, ["Riley Reid"]);
        assert!(updated.details_at.is_some());
        assert_eq!(q(&|q| q.include_tags = vec!["teacher".into()]).total, 1);
        assert_eq!(q(&|q| q.missing_details = true).total, 2);

        // Removing from the search: gone there, and Update doesn't re-add it.
        let short = q(&|q| q.max_duration = Some(600));
        assert_eq!(short.total, 1);
        assert_eq!(
            db.delete_online_matching(&OnlineQuery {
                search_id: Some("s1".into()),
                max_duration: Some(600),
                ..Default::default()
            })
            .unwrap(),
            1
        );
        assert_eq!(q(&|_| {}).total, 2);
        assert_eq!(db.record_online_items(&page, Some(("s1", 0))).unwrap(), 0);
        assert_eq!(q(&|_| {}).total, 2);
        // Deleting from history removes it everywhere and for good.
        let long_key = crate::sites::urls::video_key("https://spankbang.com/x/video/y");
        assert_eq!(db.delete_online_items(None, &[long_key]).unwrap(), 1);
        assert_eq!(q(&|_| {}).total, 1);
        db.record_online_items(&page, None).unwrap();
        assert_eq!(
            db.query_online_items(&OnlineQuery::default())
                .unwrap()
                .total,
            2
        );

        db.mark_online_search_updated("s1").unwrap();
        // History-only items are removed (the trailer, taken out of s1),
        // saved-search ones kept.
        assert_eq!(db.clear_online_history().unwrap(), 1);
    }
}
