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
