use rusqlite::Connection;

pub fn run(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

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

    if current < 6 {
        v6(conn)?;
    }

    if current < 7 {
        v7(conn)?;
    }

    if current < 8 {
        v8(conn)?;
    }

    if current < 9 {
        v9(conn)?;
    }

    if current < 10 {
        v10(conn)?;
    }

    if current < 11 {
        v11(conn)?;
    }

    if current < 12 {
        v12(conn)?;
    }

    Ok(())
}

fn v1(conn: &Connection) -> lightframe_core::Result<()> {
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
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v2(conn: &Connection) -> lightframe_core::Result<()> {
    let has_scan_status: bool = conn
        .prepare("PRAGMA table_info(watched_folders)")
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .any(|name| name == "scan_status");

    if !has_scan_status {
        conn.execute(
            "ALTER TABLE watched_folders ADD COLUMN scan_status TEXT NOT NULL DEFAULT 'idle'",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (2)",
        [],
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v3(conn: &Connection) -> lightframe_core::Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(media_files)")
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "is_favorite") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    if !columns.iter().any(|c| c == "is_deleted") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    if !columns.iter().any(|c| c == "deleted_at") {
        conn.execute("ALTER TABLE media_files ADD COLUMN deleted_at TEXT", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (3)",
        [],
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v4(conn: &Connection) -> lightframe_core::Result<()> {
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
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v5(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_not_deleted ON media_files(is_deleted)
            WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_type_active ON media_files(media_type)
            WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_deleted_at ON media_files(deleted_at)
            WHERE is_deleted = 1;

        INSERT OR IGNORE INTO schema_version (version) VALUES (5);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v6(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS smart_albums (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            icon TEXT,
            rule_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            subtitle TEXT,
            cover_media_id INTEGER REFERENCES media_files(id),
            date_from TEXT NOT NULL,
            date_to TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS memory_items (
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            media_id INTEGER NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
            PRIMARY KEY (memory_id, media_id)
        );

        INSERT INTO smart_albums (name, icon, rule_json)
        SELECT 'All Videos', '🎬', '{\"media_type\":\"Video\"}'
        WHERE NOT EXISTS (SELECT 1 FROM smart_albums WHERE name = 'All Videos');

        INSERT INTO smart_albums (name, icon, rule_json)
        SELECT 'All Screenshots', '📱', '{\"media_type\":\"Screenshot\"}'
        WHERE NOT EXISTS (SELECT 1 FROM smart_albums WHERE name = 'All Screenshots');

        INSERT INTO smart_albums (name, icon, rule_json)
        SELECT 'With GPS', '📍', '{\"has_gps\":true}'
        WHERE NOT EXISTS (SELECT 1 FROM smart_albums WHERE name = 'With GPS');

        INSERT OR IGNORE INTO schema_version (version) VALUES (6);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v7(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS media_embeddings (
            media_id INTEGER PRIMARY KEY REFERENCES media_files(id) ON DELETE CASCADE,
            clip_embedding BLOB,
            embedding_model TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS face_detections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            media_id INTEGER NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
            face_embedding BLOB,
            bbox_x REAL NOT NULL,
            bbox_y REAL NOT NULL,
            bbox_w REAL NOT NULL,
            bbox_h REAL NOT NULL,
            confidence REAL NOT NULL,
            person_id INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS persons (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            face_count INTEGER NOT NULL DEFAULT 0,
            cover_face_id INTEGER REFERENCES face_detections(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_faces_media ON face_detections(media_id);
        CREATE INDEX IF NOT EXISTS idx_faces_person ON face_detections(person_id);

        INSERT OR IGNORE INTO schema_version (version) VALUES (7);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v8(conn: &Connection) -> lightframe_core::Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(media_files)")
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "edit_params") {
        conn.execute("ALTER TABLE media_files ADD COLUMN edit_params TEXT", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (8)",
        [],
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v9(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_files_created_at ON media_files(created_at) WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_files_media_type ON media_files(media_type) WHERE is_deleted = 0;
        CREATE INDEX IF NOT EXISTS idx_media_files_favorite ON media_files(is_favorite) WHERE is_deleted = 0 AND is_favorite = 1;
        CREATE INDEX IF NOT EXISTS idx_media_files_deleted ON media_files(is_deleted, deleted_at) WHERE is_deleted = 1;
        CREATE INDEX IF NOT EXISTS idx_media_files_country_city ON media_files(country, city) WHERE is_deleted = 0 AND country IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_media_files_blake3_hash ON media_files(blake3_hash) WHERE is_deleted = 0 AND blake3_hash IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_media_files_folder_id ON media_files(folder_id);
        CREATE INDEX IF NOT EXISTS idx_media_files_path ON media_files(path);
        CREATE INDEX IF NOT EXISTS idx_album_items_album_id ON album_items(album_id);
        CREATE INDEX IF NOT EXISTS idx_album_items_media_id ON album_items(media_id);
        CREATE INDEX IF NOT EXISTS idx_duplicate_members_group_id ON duplicate_members(group_id);
        CREATE INDEX IF NOT EXISTS idx_duplicate_members_media_id ON duplicate_members(media_id);
        CREATE INDEX IF NOT EXISTS idx_face_detections_person_id ON face_detections(person_id);
        CREATE INDEX IF NOT EXISTS idx_face_detections_media_id ON face_detections(media_id);
        CREATE INDEX IF NOT EXISTS idx_memory_items_memory_id ON memory_items(memory_id);

        INSERT OR IGNORE INTO schema_version (version) VALUES (9);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v10(conn: &Connection) -> lightframe_core::Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(media_files)")
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.iter().any(|c| c == "screenshot_type") {
        conn.execute(
            "ALTER TABLE media_files ADD COLUMN screenshot_type TEXT",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_screenshot_type ON media_files(screenshot_type)
            WHERE is_deleted = 0 AND media_type = 'Screenshot'",
        [],
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (10)",
        [],
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v11(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "INSERT INTO smart_albums (name, icon, rule_json)
        SELECT 'RAW Photos', '📷', '{\"media_type\":\"Raw\"}'
        WHERE NOT EXISTS (SELECT 1 FROM smart_albums WHERE name = 'RAW Photos');

        INSERT OR IGNORE INTO schema_version (version) VALUES (11);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}

fn v12(conn: &Connection) -> lightframe_core::Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_files_geo
            ON media_files(latitude, longitude)
            WHERE is_deleted = 0 AND latitude IS NOT NULL AND longitude IS NOT NULL;

        CREATE INDEX IF NOT EXISTS idx_media_files_created_id_desc
            ON media_files(created_at DESC, id DESC)
            WHERE is_deleted = 0;

        INSERT OR IGNORE INTO schema_version (version) VALUES (12);",
    )
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

    Ok(())
}
