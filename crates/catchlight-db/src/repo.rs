use crate::Database;
use catchlight_core::media::{MediaFile, MediaType};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolder {
    pub id: i64,
    pub path: String,
    pub media_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan: Option<String>,
    #[serde(default = "default_scan_status")]
    pub scan_status: String,
}

fn default_scan_status() -> String {
    "idle".to_string()
}

fn map_watched_folder_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WatchedFolder> {
    Ok(WatchedFolder {
        id: row.get(0)?,
        path: row.get(1)?,
        media_count: row.get(2)?,
        last_scan: row.get(3)?,
        scan_status: row.get::<_, Option<String>>(4)?.unwrap_or_else(default_scan_status),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineGroup {
    pub date: String,
    pub count: i64,
    pub media: Vec<MediaFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaNeighbors {
    pub prev_id: Option<i64>,
    pub next_id: Option<i64>,
}

impl Database {
    pub fn add_watched_folder(&self, path: &str) -> catchlight_core::Result<WatchedFolder> {
        let id = {
            let conn = self.conn();
            conn.execute(
                "INSERT OR IGNORE INTO watched_folders (path) VALUES (?1)",
                params![path],
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

            conn.query_row(
                "SELECT id FROM watched_folders WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?
        };

        self.get_watched_folder(id)?
            .ok_or_else(|| catchlight_core::Error::Other(format!("folder {id} not found after insert")))
    }

    pub fn upsert_media(&self, folder_id: i64, media: &MediaFile) -> catchlight_core::Result<i64> {
        let conn = self.conn();
        let media_type_str = format!("{:?}", media.media_type);

        conn.execute(
            "INSERT INTO media_files (folder_id, path, filename, media_type, size_bytes, width, height, created_at, modified_at, blake3_hash, dhash, latitude, longitude)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(path) DO UPDATE SET
                media_type = excluded.media_type,
                size_bytes = excluded.size_bytes,
                width = COALESCE(excluded.width, width),
                height = COALESCE(excluded.height, height),
                created_at = COALESCE(excluded.created_at, created_at),
                modified_at = excluded.modified_at,
                blake3_hash = COALESCE(excluded.blake3_hash, blake3_hash),
                dhash = COALESCE(excluded.dhash, dhash)",
            params![
                folder_id,
                media.path,
                media.filename,
                media_type_str,
                media.size_bytes,
                media.width,
                media.height,
                media.created_at.map(|d| d.to_string()),
                media.modified_at.to_string(),
                media.blake3_hash,
                media.dhash,
                media.latitude,
                media.longitude,
            ],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let id: i64 = conn
            .query_row(
                "SELECT id FROM media_files WHERE path = ?1",
                params![media.path],
                |row| row.get(0),
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        Ok(id)
    }

    pub fn set_micro_thumb(&self, media_id: i64, blob: &[u8]) -> catchlight_core::Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE media_files SET micro_thumb = ?1 WHERE id = ?2",
            params![blob, media_id],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_micro_thumb(&self, media_id: i64) -> catchlight_core::Result<Option<Vec<u8>>> {
        let conn = self.conn();
        let result: Option<Option<Vec<u8>>> = conn
            .query_row(
                "SELECT micro_thumb FROM media_files WHERE id = ?1",
                params![media_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        Ok(result.flatten())
    }

    pub fn get_all_media(&self, limit: i64, offset: i64) -> catchlight_core::Result<Vec<MediaFile>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, latitude, longitude
                 FROM media_files
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit, offset], |row| {
                let media_type_str: String = row.get(3)?;
                let media_type = match media_type_str.as_str() {
                    "Photo" => MediaType::Photo,
                    "Video" => MediaType::Video,
                    "Screenshot" => MediaType::Screenshot,
                    "Raw" => MediaType::Raw,
                    "LivePhoto" => MediaType::LivePhoto,
                    _ => MediaType::Unknown,
                };

                Ok(MediaFile {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    filename: row.get(2)?,
                    media_type,
                    size_bytes: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    created_at: row.get::<_, Option<String>>(7)?
                        .and_then(|s| s.parse().ok()),
                    modified_at: row.get::<_, String>(8)?
                        .parse()
                        .unwrap_or_default(),
                    blake3_hash: row.get(9)?,
                    dhash: row.get(10)?,
                    latitude: row.get(11)?,
                    longitude: row.get(12)?,
                })
            })
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| catchlight_core::Error::Database(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn get_media_count(&self) -> catchlight_core::Result<i64> {
        let conn = self.conn();
        conn.query_row("SELECT COUNT(*) FROM media_files", [], |row| row.get(0))
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))
    }

    pub fn list_watched_folders(&self) -> catchlight_core::Result<Vec<WatchedFolder>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT w.id, w.path, COALESCE(COUNT(m.id), 0) as media_count, w.last_scan_at as last_scan, w.scan_status
                 FROM watched_folders w
                 LEFT JOIN media_files m ON m.folder_id = w.id
                 GROUP BY w.id
                 ORDER BY w.added_at",
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_watched_folder_row)
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))
    }

    pub fn get_watched_folder(&self, id: i64) -> catchlight_core::Result<Option<WatchedFolder>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT w.id, w.path, COALESCE(COUNT(m.id), 0) as media_count, w.last_scan_at as last_scan, w.scan_status
             FROM watched_folders w
             LEFT JOIN media_files m ON m.folder_id = w.id
             WHERE w.id = ?1
             GROUP BY w.id",
            params![id],
            map_watched_folder_row,
        )
        .optional()
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))
    }

    pub fn remove_watched_folder(&self, id: i64) -> catchlight_core::Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM media_files WHERE folder_id = ?1",
            params![id],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        conn.execute("DELETE FROM watched_folders WHERE id = ?1", params![id])
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_folder_scan_status(&self, folder_id: i64, status: &str) -> catchlight_core::Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE watched_folders SET scan_status = ?1 WHERE id = ?2",
            params![status, folder_id],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn update_last_scan_at(&self, folder_id: i64) -> catchlight_core::Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE watched_folders SET last_scan_at = datetime('now') WHERE id = ?1",
            params![folder_id],
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    fn map_media_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaFile> {
        let media_type_str: String = row.get(3)?;
        let media_type = match media_type_str.as_str() {
            "Photo" => MediaType::Photo,
            "Video" => MediaType::Video,
            "Screenshot" => MediaType::Screenshot,
            "Raw" => MediaType::Raw,
            "LivePhoto" => MediaType::LivePhoto,
            _ => MediaType::Unknown,
        };

        Ok(MediaFile {
            id: row.get(0)?,
            path: row.get(1)?,
            filename: row.get(2)?,
            media_type,
            size_bytes: row.get(4)?,
            width: row.get(5)?,
            height: row.get(6)?,
            created_at: row
                .get::<_, Option<String>>(7)?
                .and_then(|s| s.parse().ok()),
            modified_at: row.get::<_, String>(8)?.parse().unwrap_or_default(),
            blake3_hash: row.get(9)?,
            dhash: row.get(10)?,
            latitude: row.get(11)?,
            longitude: row.get(12)?,
        })
    }

    pub fn get_timeline_groups(
        &self,
        limit: i64,
        offset: i64,
    ) -> catchlight_core::Result<Vec<TimelineGroup>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, latitude, longitude,
                        date(COALESCE(created_at, modified_at)) AS group_date
                 FROM media_files
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit, offset], |row| {
                let group_date: String = row.get(13)?;
                let media = Self::map_media_row(row)?;
                Ok((group_date, media))
            })
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let mut groups: Vec<TimelineGroup> = Vec::new();
        for row in rows {
            let (date, media) = row.map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
            if let Some(group) = groups.last_mut() {
                if group.date == date {
                    group.count += 1;
                    group.media.push(media);
                    continue;
                }
            }
            groups.push(TimelineGroup {
                count: 1,
                date,
                media: vec![media],
            });
        }

        Ok(groups)
    }

    pub fn get_media_neighbors(&self, id: i64) -> catchlight_core::Result<MediaNeighbors> {
        let conn = self.conn();

        let prev_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM media_files
                 WHERE COALESCE(created_at, modified_at) > (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1
                 )
                 ORDER BY COALESCE(created_at, modified_at) ASC
                 LIMIT 1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let next_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM media_files
                 WHERE COALESCE(created_at, modified_at) < (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1
                 )
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT 1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        Ok(MediaNeighbors { prev_id, next_id })
    }

    pub fn get_media_by_id(&self, id: i64) -> catchlight_core::Result<Option<MediaFile>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, path, filename, media_type, size_bytes, width, height,
                    created_at, modified_at, blake3_hash, dhash, latitude, longitude
             FROM media_files WHERE id = ?1",
            params![id],
            Self::map_media_row,
        )
        .optional()
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))
    }
}
