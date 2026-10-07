pub const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS download_jobs (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    adapter TEXT NOT NULL,
    status TEXT NOT NULL,
    progress REAL NOT NULL DEFAULT 0,
    output_path TEXT,
    error TEXT,
    title TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS performers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    aliases TEXT NOT NULL DEFAULT '[]',
    image TEXT,
    favorite INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    parent_id TEXT
);

CREATE TABLE IF NOT EXISTS scenes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    path TEXT,
    source_url TEXT,
    thumb TEXT,
    phash TEXT,
    oshash TEXT,
    duration INTEGER,
    studio_id TEXT,
    date TEXT,
    rating INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scene_performers (
    scene_id TEXT NOT NULL,
    performer_id TEXT NOT NULL,
    PRIMARY KEY (scene_id, performer_id)
);

CREATE TABLE IF NOT EXISTS scene_tags (
    scene_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    PRIMARY KEY (scene_id, tag_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS scenes_fts USING fts5(
    title,
    notes,
    content='scenes',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS scenes_ai AFTER INSERT ON scenes BEGIN
    INSERT INTO scenes_fts(rowid, title, notes) VALUES (new.rowid, new.title, new.notes);
END;

CREATE TRIGGER IF NOT EXISTS scenes_ad AFTER DELETE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title, notes) VALUES('delete', old.rowid, old.title, old.notes);
END;

CREATE TRIGGER IF NOT EXISTS scenes_au AFTER UPDATE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title, notes) VALUES('delete', old.rowid, old.title, old.notes);
    INSERT INTO scenes_fts(rowid, title, notes) VALUES (new.rowid, new.title, new.notes);
END;
"#;

pub const MIGRATION_002: &str = r#"
CREATE TABLE IF NOT EXISTS site_cookies (
    site_id TEXT PRIMARY KEY,
    encrypted_data BLOB NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

pub const MIGRATION_003: &str = r#"
ALTER TABLE download_jobs ADD COLUMN metadata TEXT;
"#;

pub const MIGRATION_004: &str = r#"
ALTER TABLE scenes ADD COLUMN channel TEXT;
"#;

pub const MIGRATION_005: &str = r#"
ALTER TABLE scenes ADD COLUMN file_size INTEGER;
"#;

pub const MIGRATION_006: &str = r#"
ALTER TABLE scenes ADD COLUMN notes TEXT;
"#;

pub const MIGRATION_007: &str = r#"
ALTER TABLE download_jobs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE download_jobs ADD COLUMN last_retry_at TEXT;
"#;

pub const MIGRATION_008: &str = r#"
ALTER TABLE scenes ADD COLUMN width INTEGER;
ALTER TABLE scenes ADD COLUMN height INTEGER;
"#;

pub const MIGRATION_009: &str = r#"
CREATE TABLE IF NOT EXISTS watch_history (
    scene_id TEXT PRIMARY KEY,
    position_secs REAL NOT NULL DEFAULT 0,
    duration_secs REAL NOT NULL DEFAULT 0,
    watched INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
"#;

pub const MIGRATION_010: &str = r#"
CREATE TABLE IF NOT EXISTS saved_searches (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    site_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    slug TEXT NOT NULL,
    orientation TEXT,
    last_checked_at TEXT,
    last_item_keys TEXT NOT NULL DEFAULT '[]',
    new_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
"#;

pub const MIGRATION_011: &str = r#"
ALTER TABLE saved_searches ADD COLUMN auto_queue INTEGER NOT NULL DEFAULT 0;
"#;

pub const MIGRATION_012: &str = r#"
CREATE TABLE IF NOT EXISTS watchlist_poll_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    finished_at TEXT NOT NULL,
    checked INTEGER NOT NULL DEFAULT 0,
    queued INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0
);
"#;

pub const MIGRATION_013: &str = r#"
DROP TRIGGER IF EXISTS scenes_ai;
DROP TRIGGER IF EXISTS scenes_ad;
DROP TRIGGER IF EXISTS scenes_au;
DROP TABLE IF EXISTS scenes_fts;

CREATE VIRTUAL TABLE scenes_fts USING fts5(
    title,
    notes,
    content='scenes',
    content_rowid='rowid'
);

CREATE TRIGGER scenes_ai AFTER INSERT ON scenes BEGIN
    INSERT INTO scenes_fts(rowid, title, notes) VALUES (new.rowid, new.title, new.notes);
END;

CREATE TRIGGER scenes_ad AFTER DELETE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title, notes) VALUES('delete', old.rowid, old.title, old.notes);
END;

CREATE TRIGGER scenes_au AFTER UPDATE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title, notes) VALUES('delete', old.rowid, old.title, old.notes);
    INSERT INTO scenes_fts(rowid, title, notes) VALUES (new.rowid, new.title, new.notes);
END;

INSERT INTO scenes_fts(rowid, title, notes)
SELECT rowid, title, notes FROM scenes;
"#;

pub const MIGRATION_014: &str = r#"
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'collection',
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_scenes (
    collection_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    added_at TEXT NOT NULL,
    PRIMARY KEY (collection_id, scene_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_collection_scenes_scene ON collection_scenes(scene_id);
CREATE INDEX IF NOT EXISTS idx_collection_scenes_order ON collection_scenes(collection_id, position);
"#;

pub const MIGRATION_015: &str = r#"
ALTER TABLE collections ADD COLUMN filter_json TEXT;

CREATE INDEX IF NOT EXISTS idx_collections_type ON collections(type);
"#;

pub const MIGRATION_016: &str = r#"
CREATE TABLE IF NOT EXISTS browse_cache (
    cache_key TEXT PRIMARY KEY,
    site_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    slug TEXT NOT NULL,
    page INTEGER NOT NULL,
    orientation TEXT,
    payload TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    ttl_secs INTEGER NOT NULL DEFAULT 86400
);

CREATE INDEX IF NOT EXISTS idx_browse_cache_fetched ON browse_cache(fetched_at);
"#;

/// V2 BodyMatch: performer body attributes (cup size + hair color).
/// Values are validated in Rust (see `normalize_cup_size` / `normalize_hair_color`);
/// `NULL` means "unset". Old DBs get the columns via `column_exists` gating in
/// `Database::new` (bundled SQLite may not support `ADD COLUMN IF NOT EXISTS`).
pub const MIGRATION_017: &str = r#"
ALTER TABLE performers ADD COLUMN cup_size TEXT;
ALTER TABLE performers ADD COLUMN hair_color TEXT;

CREATE INDEX IF NOT EXISTS idx_performers_cup ON performers(cup_size);
CREATE INDEX IF NOT EXISTS idx_performers_hair ON performers(hair_color);
"#;

/// StashDB enrichment: remote stash-box ids + local studio cache.
/// `scenes.studio_name` mirrors the matched studio (kept denormalized so
/// library lists don't need a join). `vault_secrets` holds stash-box API
/// keys encrypted (same AES-256-GCM cipher as cookies).
pub const MIGRATION_018: &str = r#"
ALTER TABLE scenes ADD COLUMN stash_id TEXT;
ALTER TABLE scenes ADD COLUMN stashdb_updated_at TEXT;
ALTER TABLE scenes ADD COLUMN studio_name TEXT;
ALTER TABLE performers ADD COLUMN stash_id TEXT;

CREATE TABLE IF NOT EXISTS studios (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    stash_id TEXT,
    url TEXT,
    image TEXT
);

CREATE TABLE IF NOT EXISTS vault_secrets (
    key TEXT PRIMARY KEY,
    encrypted_data BLOB NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scenes_stash_id ON scenes(stash_id);
CREATE INDEX IF NOT EXISTS idx_performers_stash_id ON performers(stash_id);
CREATE INDEX IF NOT EXISTS idx_studios_stash_id ON studios(stash_id);
"#;

/// StashDB fingerprint fix: `scenes.md5` (full-file MD5, StashDB's most common
/// fingerprint — previously never computed) plus cleanup of bogus `oshash`
/// rows. The old `compute_oshash` wrote a 64-char SHA-256 digest; real
/// OSHASH values are always exactly 16 hex chars, so anything else is
/// unreachable junk that would break batch Identify counts.
pub const MIGRATION_019: &str = r#"
ALTER TABLE scenes ADD COLUMN md5 TEXT;
UPDATE scenes SET oshash = NULL WHERE oshash IS NOT NULL AND length(oshash) != 16;
"#;
