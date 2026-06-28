use rusqlite::Connection;

pub fn run(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current < 1 {
        v1(conn)?;
    }

    if current < 2 {
        v2(conn)?;
    }

    if current < 3 {
        v3(conn)?;
    }

    if current < 4 {
        v4(conn)?;
    }

    if current < 5 {
        v5(conn)?;
    }

    Ok(())
}

fn v1(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watched_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_scan_at TEXT
        );

        CREATE TABLE IF NOT EXISTS media_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_id INTEGER NOT NULL REFERENCES watched_folders(id),
            path TEXT NOT NULL UNIQUE,
            filename TEXT NOT NULL,
            media_type TEXT NOT NULL DEFAULT 'Unknown',
            size_bytes INTEGER NOT NULL,
            width INTEGER,
            height INTEGER,
            created_at TEXT,
            modified_at TEXT NOT NULL,
            blake3_hash TEXT,
            dhash INTEGER,
            phash INTEGER,
            latitude REAL,
            longitude REAL,
            city TEXT,
            country TEXT,
            micro_thumb BLOB,
            indexed_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_media_path ON media_files(path);
        CREATE INDEX IF NOT EXISTS idx_media_hash ON media_files(blake3_hash);
        CREATE INDEX IF NOT EXISTS idx_media_type ON media_files(media_type);
        CREATE INDEX IF NOT EXISTS idx_media_created ON media_files(created_at);
        CREATE INDEX IF NOT EXISTS idx_media_folder ON media_files(folder_id);

        CREATE TABLE IF NOT EXISTS albums (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            cover_media_id INTEGER REFERENCES media_files(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS album_items (
            album_id INTEGER NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
            media_id INTEGER NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
            sort_order INTEGER NOT NULL DEFAULT 0,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (album_id, media_id)
        );

        CREATE TABLE IF NOT EXISTS duplicate_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_type TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS duplicate_members (
            group_id INTEGER NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
            media_id INTEGER NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
            similarity REAL NOT NULL DEFAULT 1.0,
            PRIMARY KEY (group_id, media_id)
        );

        INSERT OR IGNORE INTO schema_version (version) VALUES (1);",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v2(conn: &Connection) -> catchlight_core::Result<()> {
    let has_scan_status: bool = conn
        .prepare("PRAGMA table_info(watched_folders)")
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .any(|name| name == "scan_status");

    if !has_scan_status {
        conn.execute(
            "ALTER TABLE watched_folders ADD COLUMN scan_status TEXT NOT NULL DEFAULT 'idle'",
            [],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (2)",
        [],
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v3(conn: &Connection) -> catchlight_core::Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(media_files)")
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "is_favorite") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
    }

    if !columns.iter().any(|c| c == "is_deleted") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
    }

    if !columns.iter().any(|c| c == "deleted_at") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN deleted_at TEXT",
            [],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (3)",
        [],
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v4(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(
            filename,
            city,
            country,
            media_type,
            content='media_files',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS media_fts_insert AFTER INSERT ON media_files BEGIN
            INSERT INTO media_fts(rowid, filename, city, country, media_type)
            VALUES (new.id, new.filename, new.city, new.country, new.media_type);
        END;

        CREATE TRIGGER IF NOT EXISTS media_fts_delete AFTER DELETE ON media_files BEGIN
            INSERT INTO media_fts(media_fts, rowid, filename, city, country, media_type)
            VALUES ('delete', old.id, old.filename, old.city, old.country, old.media_type);
        END;

        CREATE TRIGGER IF NOT EXISTS media_fts_update AFTER UPDATE ON media_files BEGIN
            INSERT INTO media_fts(media_fts, rowid, filename, city, country, media_type)
            VALUES ('delete', old.id, old.filename, old.city, old.country, old.media_type);
            INSERT INTO media_fts(rowid, filename, city, country, media_type)
            VALUES (new.id, new.filename, new.city, new.country, new.media_type);
        END;

        INSERT INTO media_fts(rowid, filename, city, country, media_type)
        SELECT id, filename, city, country, media_type FROM media_files;

        INSERT OR IGNORE INTO schema_version (version) VALUES (4);",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v5(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_not_deleted ON media_files(is_deleted)
            WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_type_active ON media_files(media_type)
            WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_deleted_at ON media_files(deleted_at)
            WHERE is_deleted = 1;

        INSERT OR IGNORE INTO schema_version (version) VALUES (5);",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

    Ok(())
}
