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
    content='scenes',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS scenes_ai AFTER INSERT ON scenes BEGIN
    INSERT INTO scenes_fts(rowid, title) VALUES (new.rowid, new.title);
END;

CREATE TRIGGER IF NOT EXISTS scenes_ad AFTER DELETE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title) VALUES('delete', old.rowid, old.title);
END;

CREATE TRIGGER IF NOT EXISTS scenes_au AFTER UPDATE ON scenes BEGIN
    INSERT INTO scenes_fts(scenes_fts, rowid, title) VALUES('delete', old.rowid, old.title);
    INSERT INTO scenes_fts(rowid, title) VALUES (new.rowid, new.title);
END;
"#;

pub const MIGRATION_002: &str = r#"
CREATE TABLE IF NOT EXISTS site_cookies (
    site_id TEXT PRIMARY KEY,
    encrypted_data BLOB NOT NULL,
    updated_at TEXT NOT NULL
);
"#;
