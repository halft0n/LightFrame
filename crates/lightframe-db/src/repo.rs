use crate::Database;
use lightframe_core::media::{MediaFile, MediaType};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

type PerceptualCandidate = (i64, Option<u64>, Option<u64>);

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
        scan_status: row
            .get::<_, Option<String>>(4)?
            .unwrap_or_else(default_scan_status),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationGroup {
    pub country: String,
    pub city: Option<String>,
    pub count: i64,
    pub sample_media_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationStats {
    pub total_with_gps: i64,
    pub countries: i64,
    pub cities: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoCluster {
    pub latitude: f64,
    pub longitude: f64,
    pub count: i64,
    pub media_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub cover_media_id: Option<i64>,
    pub media_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn map_album_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Album> {
    Ok(Album {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        cover_media_id: row.get(3)?,
        media_count: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAlbum {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub rule_json: String,
    pub media_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAlbumRule {
    pub media_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub is_favorite: Option<bool>,
    pub min_size: Option<i64>,
    pub has_gps: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub cover_media_id: i64,
    pub media_count: i64,
    pub date_from: String,
    pub date_to: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: Option<String>,
    pub face_count: i64,
    pub cover_face_id: Option<i64>,
    pub sample_media_ids: Vec<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionRecord {
    pub id: i64,
    pub media_id: i64,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_w: f32,
    pub bbox_h: f32,
    pub confidence: f32,
    pub person_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FaceDetectionInput {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub embedding: Vec<f32>,
}

fn map_smart_album_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SmartAlbum> {
    Ok(SmartAlbum {
        id: row.get(0)?,
        name: row.get(1)?,
        icon: row.get(2)?,
        rule_json: row.get(3)?,
        media_count: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn map_memory_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        title: row.get(1)?,
        subtitle: row.get(2)?,
        cover_media_id: row.get(3)?,
        media_count: row.get(4)?,
        date_from: row.get(5)?,
        date_to: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn build_smart_album_filter(rule: &SmartAlbumRule) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut conditions = vec!["is_deleted = 0".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref media_type) = rule.media_type {
        conditions.push("media_type = ?".to_string());
        params.push(Box::new(media_type.clone()));
    }
    if let Some(ref date_from) = rule.date_from {
        conditions.push("date(COALESCE(created_at, modified_at)) >= date(?)".to_string());
        params.push(Box::new(date_from.clone()));
    }
    if let Some(ref date_to) = rule.date_to {
        conditions.push("date(COALESCE(created_at, modified_at)) <= date(?)".to_string());
        params.push(Box::new(date_to.clone()));
    }
    if let Some(ref country) = rule.country {
        conditions.push("country = ?".to_string());
        params.push(Box::new(country.clone()));
    }
    if let Some(ref city) = rule.city {
        conditions.push("city = ?".to_string());
        params.push(Box::new(city.clone()));
    }
    if let Some(is_favorite) = rule.is_favorite {
        conditions.push("is_favorite = ?".to_string());
        params.push(Box::new(if is_favorite { 1i64 } else { 0i64 }));
    }
    if let Some(min_size) = rule.min_size {
        conditions.push("size_bytes >= ?".to_string());
        params.push(Box::new(min_size));
    }
    if let Some(has_gps) = rule.has_gps {
        if has_gps {
            conditions.push("latitude IS NOT NULL AND longitude IS NOT NULL".to_string());
        } else {
            conditions.push("(latitude IS NULL OR longitude IS NULL)".to_string());
        }
    }

    (conditions.join(" AND "), params)
}

fn smart_album_media_count(db: &Database, rule: &SmartAlbumRule) -> lightframe_core::Result<i64> {
    let conn = db.read_conn()?;
    batch_smart_album_media_counts(&conn, std::slice::from_ref(rule))?
        .into_iter()
        .next()
        .ok_or_else(|| lightframe_core::Error::Other("batch count returned empty".to_string()))
}

fn batch_smart_album_media_counts(
    conn: &Connection,
    rules: &[SmartAlbumRule],
) -> lightframe_core::Result<Vec<i64>> {
    if rules.is_empty() {
        return Ok(Vec::new());
    }

    let mut subqueries = Vec::with_capacity(rules.len());
    let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    for rule in rules {
        let (where_clause, filter_params) = build_smart_album_filter(rule);
        subqueries.push(format!(
            "(SELECT COUNT(*) FROM media_files WHERE {where_clause})"
        ));
        all_params.extend(filter_params);
    }

    let sql = format!("SELECT {}", subqueries.join(", "));
    let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

    conn.query_row(&sql, param_refs.as_slice(), |row| {
        (0..rules.len())
            .map(|i| row.get::<_, i64>(i))
            .collect::<Result<Vec<_>, _>>()
    })
    .map_err(|e| lightframe_core::Error::Database(e.to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: i64,
    pub match_type: String,
    pub created_at: String,
    pub members: Vec<DuplicateMember>,
}

pub type DuplicateGroupDetail = DuplicateGroup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateMember {
    pub media_id: i64,
    pub similarity: f64,
    pub path: String,
    pub filename: String,
    pub size_bytes: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub created_at: Option<String>,
    pub modified_at: String,
}

/// Escape and tokenize user input for FTS5 MATCH queries.
fn sanitize_fts_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    trimmed
        .split_whitespace()
        .filter_map(|term| {
            let escaped = term.replace('"', "\"\"");
            if escaped.is_empty() {
                None
            } else {
                Some(format!("\"{escaped}\"*"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn perceptual_similarity(a: u64, b: u64) -> f64 {
    1.0 - (hamming_distance(a, b) as f64 / 64.0)
}

const PHASH_THRESHOLD: u32 = 10;

fn perceptual_pair_match(
    dhash_a: Option<u64>,
    dhash_b: Option<u64>,
    phash_a: Option<u64>,
    phash_b: Option<u64>,
    dhash_threshold: u32,
) -> Option<f64> {
    if let (Some(ha), Some(hb)) = (dhash_a, dhash_b) {
        let distance = hamming_distance(ha, hb);
        if distance <= dhash_threshold {
            return Some(perceptual_similarity(ha, hb));
        }
    }
    if let (Some(ha), Some(hb)) = (phash_a, phash_b) {
        let distance = hamming_distance(ha, hb);
        if distance <= PHASH_THRESHOLD {
            return Some(perceptual_similarity(ha, hb));
        }
    }
    None
}

impl Database {
    pub fn add_watched_folder(&self, path: &str) -> lightframe_core::Result<WatchedFolder> {
        let id = {
            let conn = self.conn()?;
            conn.execute(
                "INSERT OR IGNORE INTO watched_folders (path) VALUES (?1)",
                params![path],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            conn.query_row(
                "SELECT id FROM watched_folders WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        };

        self.get_watched_folder(id)?.ok_or_else(|| {
            lightframe_core::Error::Other(format!("folder {id} not found after insert"))
        })
    }

    pub fn upsert_media(&self, folder_id: i64, media: &MediaFile) -> lightframe_core::Result<i64> {
        let conn = self.conn()?;
        let media_type_str = format!("{:?}", media.media_type);

        conn.execute(
            "INSERT INTO media_files (folder_id, path, filename, media_type, size_bytes, width, height, created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(path) DO UPDATE SET
                media_type = excluded.media_type,
                size_bytes = excluded.size_bytes,
                width = COALESCE(excluded.width, width),
                height = COALESCE(excluded.height, height),
                created_at = COALESCE(excluded.created_at, created_at),
                modified_at = excluded.modified_at,
                blake3_hash = COALESCE(excluded.blake3_hash, blake3_hash),
                dhash = COALESCE(excluded.dhash, dhash),
                phash = COALESCE(excluded.phash, phash),
                latitude = COALESCE(excluded.latitude, latitude),
                longitude = COALESCE(excluded.longitude, longitude)",
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
                media.phash,
                media.latitude,
                media.longitude,
            ],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let id: i64 = conn
            .query_row(
                "SELECT id FROM media_files WHERE path = ?1",
                params![media.path],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(id)
    }

    pub fn set_micro_thumb(&self, media_id: i64, blob: &[u8]) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE media_files SET micro_thumb = ?1 WHERE id = ?2",
            params![blob, media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_micro_thumb(&self, media_id: i64) -> lightframe_core::Result<Option<Vec<u8>>> {
        let conn = self.read_conn()?;
        let result: Option<Option<Vec<u8>>> = conn
            .query_row(
                "SELECT micro_thumb FROM media_files WHERE id = ?1",
                params![media_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(result.flatten())
    }

    pub fn get_all_media(
        &self,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        // Legacy OFFSET pagination — prefer `get_media_page` for large datasets.
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

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
                    created_at: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| s.parse().ok()),
                    modified_at: row.get::<_, String>(8)?.parse().unwrap_or_default(),
                    blake3_hash: row.get(9)?,
                    dhash: row.get(10)?,
                    phash: row.get(11)?,
                    latitude: row.get(12)?,
                    longitude: row.get(13)?,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn get_media_page(
        &self,
        limit: i64,
        cursor: Option<(String, i64)>,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match cursor {
            None => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?1"
                    .to_string(),
                vec![Box::new(limit)],
            ),
            Some((created_at, id)) => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                   AND (created_at < ?1 OR (created_at = ?1 AND id < ?2))
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?3"
                    .to_string(),
                vec![Box::new(created_at), Box::new(id), Box::new(limit)],
            ),
        };

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(param_refs.as_slice(), Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_count(&self) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM media_files WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_by_folder(
        &self,
        folder_id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE folder_id = ?1 AND is_deleted = 0
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![folder_id, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn get_media_count_by_folder(&self, folder_id: i64) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM media_files WHERE folder_id = ?1 AND is_deleted = 0",
            params![folder_id],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_by_type(
        &self,
        media_type: &str,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE media_type = ?1 AND is_deleted = 0
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![media_type, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_count_by_type(&self, media_type: &str) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM media_files WHERE media_type = ?1 AND is_deleted = 0",
            params![media_type],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn set_screenshot_type(
        &self,
        media_id: i64,
        screenshot_type: &str,
    ) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE media_files SET screenshot_type = ?1 WHERE id = ?2",
            params![screenshot_type, media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_screenshots(
        &self,
        screenshot_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let (sql, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) = match screenshot_type {
            Some(st) => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE media_type = 'Screenshot' AND is_deleted = 0 AND screenshot_type = ?1
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?2 OFFSET ?3",
                vec![
                    Box::new(st.to_string()) as Box<dyn rusqlite::ToSql>,
                    Box::new(limit),
                    Box::new(offset),
                ],
            ),
            None => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE media_type = 'Screenshot' AND is_deleted = 0
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?1 OFFSET ?2",
                vec![Box::new(limit), Box::new(offset)],
            ),
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_screenshot_count(
        &self,
        screenshot_type: Option<&str>,
    ) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        match screenshot_type {
            Some(st) => conn.query_row(
                "SELECT COUNT(*) FROM media_files
                 WHERE media_type = 'Screenshot' AND is_deleted = 0 AND screenshot_type = ?1",
                params![st],
                |row| row.get(0),
            ),
            None => conn.query_row(
                "SELECT COUNT(*) FROM media_files
                 WHERE media_type = 'Screenshot' AND is_deleted = 0",
                [],
                |row| row.get(0),
            ),
        }
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn list_watched_folders(&self) -> lightframe_core::Result<Vec<WatchedFolder>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT w.id, w.path, COALESCE(COUNT(m.id), 0) as media_count, w.last_scan_at as last_scan, w.scan_status
                 FROM watched_folders w
                 LEFT JOIN media_files m ON m.folder_id = w.id AND m.is_deleted = 0
                 GROUP BY w.id
                 ORDER BY w.added_at",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_watched_folder_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_watched_folder(&self, id: i64) -> lightframe_core::Result<Option<WatchedFolder>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT w.id, w.path, COALESCE(COUNT(m.id), 0) as media_count, w.last_scan_at as last_scan, w.scan_status
             FROM watched_folders w
             LEFT JOIN media_files m ON m.folder_id = w.id AND m.is_deleted = 0
             WHERE w.id = ?1
             GROUP BY w.id",
            params![id],
            map_watched_folder_row,
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn remove_watched_folder(&self, id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM media_files WHERE folder_id = ?1", params![id])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        conn.execute("DELETE FROM watched_folders WHERE id = ?1", params![id])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_folder_scan_status(
        &self,
        folder_id: i64,
        status: &str,
    ) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE watched_folders SET scan_status = ?1 WHERE id = ?2",
            params![status, folder_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn update_last_scan_at(&self, folder_id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE watched_folders SET last_scan_at = datetime('now') WHERE id = ?1",
            params![folder_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
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
            phash: row.get(11)?,
            latitude: row.get(12)?,
            longitude: row.get(13)?,
        })
    }

    pub fn get_timeline_groups(
        &self,
        limit: i64,
        cursor: Option<(String, i64)>,
    ) -> lightframe_core::Result<Vec<TimelineGroup>> {
        let limit = limit.clamp(1, 500);
        let conn = self.read_conn()?;
        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match cursor {
            None => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude,
                        date(COALESCE(created_at, modified_at)) AS group_date
                 FROM media_files
                 WHERE is_deleted = 0
                 ORDER BY COALESCE(created_at, modified_at) DESC, id DESC
                 LIMIT ?1"
                    .to_string(),
                vec![Box::new(limit)],
            ),
            Some((timestamp, id)) => (
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude,
                        date(COALESCE(created_at, modified_at)) AS group_date
                 FROM media_files
                 WHERE is_deleted = 0
                   AND (
                     COALESCE(created_at, modified_at) < ?1
                     OR (COALESCE(created_at, modified_at) = ?1 AND id < ?2)
                   )
                 ORDER BY COALESCE(created_at, modified_at) DESC, id DESC
                 LIMIT ?3"
                    .to_string(),
                vec![Box::new(timestamp), Box::new(id), Box::new(limit)],
            ),
        };

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let group_date: String = row.get(14)?;
                let media = Self::map_media_row(row)?;
                Ok((group_date, media))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut groups: Vec<TimelineGroup> = Vec::new();
        for row in rows {
            let (date, media) = row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            if let Some(group) = groups.last_mut()
                && group.date == date
            {
                group.count += 1;
                group.media.push(media);
                continue;
            }
            groups.push(TimelineGroup {
                count: 1,
                date,
                media: vec![media],
            });
        }

        Ok(groups)
    }

    pub fn get_media_neighbors(&self, id: i64) -> lightframe_core::Result<MediaNeighbors> {
        let conn = self.read_conn()?;

        let prev_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM media_files
                 WHERE is_deleted = 0
                   AND COALESCE(created_at, modified_at) > (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1 AND is_deleted = 0
                 )
                 ORDER BY COALESCE(created_at, modified_at) ASC
                 LIMIT 1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let next_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM media_files
                 WHERE is_deleted = 0
                   AND COALESCE(created_at, modified_at) < (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1 AND is_deleted = 0
                 )
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT 1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(MediaNeighbors { prev_id, next_id })
    }

    pub fn get_media_window(
        &self,
        center_id: i64,
        radius: usize,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let radius = radius as i64;

        let center = self
            .get_media_by_id(center_id)?
            .ok_or_else(|| lightframe_core::Error::Other(format!("media {center_id} not found")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                   AND COALESCE(created_at, modified_at) > (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1 AND is_deleted = 0
                   )
                 ORDER BY COALESCE(created_at, modified_at) ASC
                 LIMIT ?2",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let newer_rows = stmt
            .query_map(params![center_id, radius], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut newer: Vec<MediaFile> = Vec::new();
        for row in newer_rows {
            newer.push(row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?);
        }
        newer.reverse();

        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                   AND COALESCE(created_at, modified_at) < (
                     SELECT COALESCE(created_at, modified_at) FROM media_files WHERE id = ?1 AND is_deleted = 0
                   )
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?2",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let older_rows = stmt
            .query_map(params![center_id, radius], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut older: Vec<MediaFile> = Vec::new();
        for row in older_rows {
            older.push(row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?);
        }

        let mut window = newer;
        window.push(center);
        window.extend(older);
        Ok(window)
    }

    pub fn get_media_by_id(&self, id: i64) -> lightframe_core::Result<Option<MediaFile>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT id, path, filename, media_type, size_bytes, width, height,
                    created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
             FROM media_files WHERE id = ?1 AND is_deleted = 0",
            params![id],
            Self::map_media_row,
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_by_ids(
        &self,
        ids: &[i64],
    ) -> lightframe_core::Result<std::collections::HashMap<i64, MediaFile>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let conn = self.read_conn()?;
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT id, path, filename, media_type, size_bytes, width, height,
                    created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
             FROM media_files
             WHERE is_deleted = 0 AND id IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                let media = Self::map_media_row(row)?;
                Ok((media.id, media))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<_, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_by_path(&self, path: &str) -> lightframe_core::Result<Option<MediaFile>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT id, path, filename, media_type, size_bytes, width, height,
                    created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
             FROM media_files WHERE path = ?1 AND is_deleted = 0",
            params![path],
            Self::map_media_row,
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_deletion_info(
        &self,
        media_id: i64,
    ) -> lightframe_core::Result<Option<(String, Option<String>, i64)>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT path, blake3_hash, is_deleted FROM media_files WHERE id = ?1",
            params![media_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn update_media_path(
        &self,
        old_path: &str,
        new_path: &str,
    ) -> lightframe_core::Result<bool> {
        let filename = std::path::Path::new(new_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(new_path);

        let conn = self.conn()?;
        #[cfg(windows)]
        let affected = conn
            .execute(
                "UPDATE media_files SET path = ?1, filename = ?2
                 WHERE lower(path) = lower(?3) AND is_deleted = 0",
                params![new_path, filename, old_path],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        #[cfg(not(windows))]
        let affected = conn
            .execute(
                "UPDATE media_files SET path = ?1, filename = ?2
                 WHERE path = ?3 AND is_deleted = 0",
                params![new_path, filename, old_path],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    pub fn soft_delete_by_path(&self, path: &str) -> lightframe_core::Result<bool> {
        let conn = self.conn()?;
        #[cfg(windows)]
        let affected = conn
            .execute(
                "UPDATE media_files SET is_deleted = 1, deleted_at = datetime('now')
                 WHERE lower(path) = lower(?1) AND is_deleted = 0",
                params![path],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        #[cfg(not(windows))]
        let affected = conn
            .execute(
                "UPDATE media_files SET is_deleted = 1, deleted_at = datetime('now')
                 WHERE path = ?1 AND is_deleted = 0",
                params![path],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    pub fn clear_duplicate_groups(&self) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM duplicate_groups", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn create_duplicate_group(
        &self,
        match_type: &str,
        media_ids: &[i64],
        similarities: &[f64],
    ) -> lightframe_core::Result<i64> {
        if media_ids.is_empty() {
            return Err(lightframe_core::Error::Other(
                "duplicate group requires at least one member".to_string(),
            ));
        }
        if media_ids.len() != similarities.len() {
            return Err(lightframe_core::Error::Other(
                "media_ids and similarities length mismatch".to_string(),
            ));
        }

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO duplicate_groups (match_type) VALUES (?1)",
            params![match_type],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let group_id = conn.last_insert_rowid();
        for (media_id, similarity) in media_ids.iter().zip(similarities.iter()) {
            conn.execute(
                "INSERT INTO duplicate_members (group_id, media_id, similarity) VALUES (?1, ?2, ?3)",
                params![group_id, media_id, similarity],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }

        Ok(group_id)
    }

    pub fn find_exact_duplicates(&self) -> lightframe_core::Result<Vec<DuplicateGroup>> {
        let hash_groups: Vec<Vec<i64>> = {
            let conn = self.conn()?;
            let mut stmt = conn
                .prepare(
                    "SELECT blake3_hash, GROUP_CONCAT(id) AS ids
                     FROM media_files
                     WHERE blake3_hash IS NOT NULL AND is_deleted = 0
                     GROUP BY blake3_hash
                     HAVING COUNT(*) > 1",
                )
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            let rows = stmt
                .query_map([], |row| {
                    let ids_str: String = row.get(1)?;
                    Ok(ids_str)
                })
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            let mut groups = Vec::new();
            for row in rows {
                let ids_str = row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
                let media_ids: Vec<i64> =
                    ids_str.split(',').filter_map(|s| s.parse().ok()).collect();
                if media_ids.len() >= 2 {
                    groups.push(media_ids);
                }
            }
            groups
        };

        let mut groups = Vec::new();
        for media_ids in hash_groups {
            let similarities = vec![1.0; media_ids.len()];
            let group_id = self.create_duplicate_group("exact", &media_ids, &similarities)?;
            groups.push(self.get_duplicate_group_by_id(group_id)?.ok_or_else(|| {
                lightframe_core::Error::Other(format!("group {group_id} not found"))
            })?);
        }

        Ok(groups)
    }

    pub fn find_perceptual_duplicates(
        &self,
        threshold: u32,
    ) -> lightframe_core::Result<Vec<DuplicateGroup>> {
        let exact_member_ids = self.exact_duplicate_member_ids()?;
        let candidates = self.load_perceptual_candidates(&exact_member_ids)?;

        if candidates.len() < 2 {
            return Ok(Vec::new());
        }

        let mut uf = UnionFind::new(candidates.iter().map(|(id, _, _)| *id));
        let mut pair_similarities: std::collections::HashMap<(i64, i64), f64> =
            std::collections::HashMap::new();
        let mut seen_pairs: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        // LSH bucketing by high 16 bits (with adjacent bucket for near-matches).
        let mut dhash_buckets: std::collections::HashMap<u16, Vec<usize>> =
            std::collections::HashMap::new();
        let mut phash_buckets: std::collections::HashMap<u16, Vec<usize>> =
            std::collections::HashMap::new();

        for (idx, (_, dhash, phash)) in candidates.iter().enumerate() {
            if let Some(dh) = dhash {
                let key = (*dh >> 48) as u16;
                dhash_buckets.entry(key).or_default().push(idx);
                dhash_buckets
                    .entry(key.wrapping_add(1))
                    .or_default()
                    .push(idx);
            }
            if let Some(ph) = phash {
                let key = (*ph >> 48) as u16;
                phash_buckets.entry(key).or_default().push(idx);
                phash_buckets
                    .entry(key.wrapping_add(1))
                    .or_default()
                    .push(idx);
            }
        }

        Self::compare_perceptual_bucket_pairs(
            &candidates,
            dhash_buckets.values(),
            threshold,
            &mut seen_pairs,
            &mut uf,
            &mut pair_similarities,
        );

        Self::compare_perceptual_bucket_pairs(
            &candidates,
            phash_buckets.values(),
            threshold,
            &mut seen_pairs,
            &mut uf,
            &mut pair_similarities,
        );

        let mut components: std::collections::HashMap<i64, Vec<i64>> =
            std::collections::HashMap::new();
        for (id, _, _) in &candidates {
            let root = uf.find(*id);
            components.entry(root).or_default().push(*id);
        }

        let mut groups = Vec::new();
        for members in components.values() {
            if members.len() < 2 {
                continue;
            }
            let mut members = members.clone();
            members.sort_unstable();

            let mut similarities = Vec::with_capacity(members.len());
            for &media_id in &members {
                let mut max_sim: f64 = 0.0;
                for &other_id in &members {
                    if media_id == other_id {
                        continue;
                    }
                    let key = if media_id <= other_id {
                        (media_id, other_id)
                    } else {
                        (other_id, media_id)
                    };
                    if let Some(&sim) = pair_similarities.get(&key) {
                        max_sim = max_sim.max(sim);
                    }
                }
                similarities.push(if max_sim > 0.0 { max_sim } else { 1.0 });
            }

            let group_id = self.create_duplicate_group("perceptual", &members, &similarities)?;
            groups.push(self.get_duplicate_group_by_id(group_id)?.ok_or_else(|| {
                lightframe_core::Error::Other(format!("group {group_id} not found"))
            })?);
        }

        Ok(groups)
    }

    fn compare_perceptual_bucket_pairs<'a>(
        candidates: &[PerceptualCandidate],
        bucket_groups: impl IntoIterator<Item = &'a Vec<usize>>,
        threshold: u32,
        seen_pairs: &mut std::collections::HashSet<(usize, usize)>,
        uf: &mut UnionFind,
        pair_similarities: &mut std::collections::HashMap<(i64, i64), f64>,
    ) {
        const MAX_BUCKET_SIZE: usize = 200;

        for indices in bucket_groups {
            let capped = if indices.len() > MAX_BUCKET_SIZE {
                &indices[..MAX_BUCKET_SIZE]
            } else {
                indices.as_slice()
            };

            for i in 0..capped.len() {
                for j in (i + 1)..capped.len() {
                    let a = capped[i];
                    let b = capped[j];
                    let pair = if a < b { (a, b) } else { (b, a) };
                    if !seen_pairs.insert(pair) {
                        continue;
                    }

                    let (id_a, dhash_a, phash_a) = candidates[a];
                    let (id_b, dhash_b, phash_b) = candidates[b];
                    if let Some(sim) =
                        perceptual_pair_match(dhash_a, dhash_b, phash_a, phash_b, threshold)
                    {
                        uf.union(id_a, id_b);
                        let key = if id_a <= id_b {
                            (id_a, id_b)
                        } else {
                            (id_b, id_a)
                        };
                        pair_similarities.insert(key, sim);
                    }
                }
            }
        }
    }

    fn exact_duplicate_member_ids(
        &self,
    ) -> lightframe_core::Result<std::collections::HashSet<i64>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT dm.media_id
                 FROM duplicate_members dm
                 JOIN duplicate_groups dg ON dg.id = dm.group_id
                 WHERE dg.match_type = 'exact'",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut ids = std::collections::HashSet::new();
        for row in rows {
            ids.insert(row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?);
        }
        Ok(ids)
    }

    fn load_perceptual_candidates(
        &self,
        exclude_ids: &std::collections::HashSet<i64>,
    ) -> lightframe_core::Result<Vec<PerceptualCandidate>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, dhash, phash FROM media_files
                 WHERE (dhash IS NOT NULL OR phash IS NOT NULL) AND is_deleted = 0",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<u64>>(1)?,
                    row.get::<_, Option<u64>>(2)?,
                ))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut candidates = Vec::new();
        for row in rows {
            let (id, dhash, phash) =
                row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            if !exclude_ids.contains(&id) {
                candidates.push((id, dhash, phash));
            }
        }
        Ok(candidates)
    }

    fn get_duplicate_group_by_id(
        &self,
        group_id: i64,
    ) -> lightframe_core::Result<Option<DuplicateGroup>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT dg.id, dg.match_type, dg.created_at,
                        dm.media_id, dm.similarity,
                        m.path, m.filename, m.size_bytes, m.width, m.height,
                        m.created_at, m.modified_at
                 FROM duplicate_groups dg
                 JOIN duplicate_members dm ON dm.group_id = dg.id
                 JOIN media_files m ON m.id = dm.media_id AND m.is_deleted = 0
                 WHERE dg.id = ?1
                 ORDER BY dm.media_id",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![group_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    DuplicateMember {
                        media_id: row.get(3)?,
                        similarity: row.get(4)?,
                        path: row.get(5)?,
                        filename: row.get(6)?,
                        size_bytes: row.get(7)?,
                        width: row.get(8)?,
                        height: row.get(9)?,
                        created_at: row.get(10)?,
                        modified_at: row.get(11)?,
                    },
                ))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut group: Option<DuplicateGroup> = None;
        for row in rows {
            let (id, match_type, created_at, member) =
                row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            match &mut group {
                Some(g) => g.members.push(member),
                None => {
                    group = Some(DuplicateGroup {
                        id,
                        match_type,
                        created_at,
                        members: vec![member],
                    });
                }
            }
        }

        Ok(group)
    }

    pub fn list_duplicate_groups(&self) -> lightframe_core::Result<Vec<DuplicateGroupDetail>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT dg.id, dg.match_type, dg.created_at,
                        dm.media_id, dm.similarity,
                        m.path, m.filename, m.size_bytes, m.width, m.height,
                        m.created_at, m.modified_at
                 FROM duplicate_groups dg
                 JOIN duplicate_members dm ON dm.group_id = dg.id
                 JOIN media_files m ON m.id = dm.media_id AND m.is_deleted = 0
                 ORDER BY dg.created_at DESC, dg.id, dm.media_id",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    DuplicateMember {
                        media_id: row.get(3)?,
                        similarity: row.get(4)?,
                        path: row.get(5)?,
                        filename: row.get(6)?,
                        size_bytes: row.get(7)?,
                        width: row.get(8)?,
                        height: row.get(9)?,
                        created_at: row.get(10)?,
                        modified_at: row.get(11)?,
                    },
                ))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut groups: Vec<DuplicateGroupDetail> = Vec::new();
        for row in rows {
            let (id, match_type, created_at, member) =
                row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            if let Some(group) = groups.last_mut()
                && group.id == id
            {
                group.members.push(member);
                continue;
            }
            groups.push(DuplicateGroupDetail {
                id,
                match_type,
                created_at,
                members: vec![member],
            });
        }

        Ok(groups)
    }

    pub fn delete_duplicate_group(&self, group_id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM duplicate_groups WHERE id = ?1",
            params![group_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn remove_from_duplicate_group(
        &self,
        group_id: i64,
        media_id: i64,
    ) -> lightframe_core::Result<()> {
        {
            let conn = self.conn()?;
            conn.execute(
                "DELETE FROM duplicate_members WHERE group_id = ?1 AND media_id = ?2",
                params![group_id, media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }

        let remaining: i64 = {
            let conn = self.conn()?;
            conn.query_row(
                "SELECT COUNT(*) FROM duplicate_members WHERE group_id = ?1",
                params![group_id],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
        };

        if remaining <= 1 {
            self.delete_duplicate_group(group_id)?;
        }

        Ok(())
    }

    pub fn get_duplicate_groups_count(&self) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row("SELECT COUNT(*) FROM duplicate_groups", [], |row| {
            row.get(0)
        })
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    /// Resolve a duplicate group by keeping the winner and optionally soft-deleting the others.
    /// When `soft_delete_others` is true, the remaining members are marked `is_deleted = 1`
    /// (moved to trash); files are NOT removed from disk.
    pub fn resolve_duplicate_group(
        &self,
        group_id: i64,
        keep_media_id: i64,
        soft_delete_others: bool,
    ) -> lightframe_core::Result<()> {
        let group = self
            .get_duplicate_group_by_id(group_id)?
            .ok_or_else(|| lightframe_core::Error::Other(format!("group {group_id} not found")))?;

        if !group.members.iter().any(|m| m.media_id == keep_media_id) {
            return Err(lightframe_core::Error::Other(format!(
                "media {keep_media_id} is not in group {group_id}"
            )));
        }

        for member in &group.members {
            if member.media_id != keep_media_id && soft_delete_others {
                self.set_deleted(member.media_id, true)?;
            }
        }

        self.delete_duplicate_group(group_id)
    }

    pub fn toggle_favorite(&self, media_id: i64) -> lightframe_core::Result<bool> {
        let conn = self.conn()?;
        let (current_favorite, is_deleted): (i64, i64) = conn
            .query_row(
                "SELECT is_favorite, is_deleted FROM media_files WHERE id = ?1",
                params![media_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if is_deleted != 0 {
            return Err(lightframe_core::Error::Other(format!(
                "cannot favorite deleted media {media_id}"
            )));
        }

        let new_value = if current_favorite == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE media_files SET is_favorite = ?1 WHERE id = ?2",
            params![new_value, media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(new_value == 1)
    }

    pub fn set_deleted(&self, media_id: i64, deleted: bool) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        if deleted {
            conn.execute(
                "UPDATE media_files SET is_deleted = 1, deleted_at = datetime('now') WHERE id = ?1",
                params![media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        } else {
            conn.execute(
                "UPDATE media_files SET is_deleted = 0, deleted_at = NULL WHERE id = ?1",
                params![media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }
        Ok(())
    }

    pub fn list_deleted_media(&self) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 1
                 ORDER BY deleted_at DESC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn permanently_delete_media(&self, media_id: i64) -> lightframe_core::Result<()> {
        let info = self
            .get_media_deletion_info(media_id)?
            .ok_or_else(|| lightframe_core::Error::Other(format!("media {media_id} not found")))?;

        let (_path, _hash, deleted) = info;
        if deleted == 0 {
            return Err(lightframe_core::Error::Other(format!(
                "media {media_id} is not in trash; soft-delete before permanent delete"
            )));
        }

        let conn = self.conn()?;
        conn.execute("DELETE FROM media_files WHERE id = ?1", params![media_id])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(())
    }

    pub fn batch_set_deleted(
        &self,
        media_ids: &[i64],
        deleted: bool,
    ) -> lightframe_core::Result<usize> {
        if media_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn()?;
        let placeholders: Vec<String> = media_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = if deleted {
            format!(
                "UPDATE media_files SET is_deleted = 1, deleted_at = datetime('now') WHERE id IN ({})",
                placeholders.join(", ")
            )
        } else {
            format!(
                "UPDATE media_files SET is_deleted = 0, deleted_at = NULL WHERE id IN ({})",
                placeholders.join(", ")
            )
        };
        let params: Vec<&dyn rusqlite::ToSql> = media_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();
        let affected = conn
            .execute(&sql, params.as_slice())
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(affected)
    }

    pub fn batch_set_favorite(
        &self,
        media_ids: &[i64],
        favorite: bool,
    ) -> lightframe_core::Result<usize> {
        if media_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn()?;
        let placeholders: Vec<String> = media_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect();
        let sql = format!(
            "UPDATE media_files SET is_favorite = ?1 WHERE is_deleted = 0 AND id IN ({})",
            placeholders.join(", ")
        );
        let fav_value: i64 = if favorite { 1 } else { 0 };
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fav_value)];
        for id in media_ids {
            params.push(Box::new(*id));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let affected = conn
            .execute(&sql, param_refs.as_slice())
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(affected)
    }

    pub fn batch_permanent_delete(&self, media_ids: &[i64]) -> lightframe_core::Result<usize> {
        if media_ids.is_empty() {
            return Ok(0);
        }

        let conn = self.conn()?;
        let placeholders: Vec<String> = media_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "DELETE FROM media_files WHERE is_deleted = 1 AND id IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::ToSql> = media_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();
        let affected = conn
            .execute(&sql, params.as_slice())
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(affected)
    }

    pub fn cleanup_deleted_older_than(&self, days: i64) -> lightframe_core::Result<usize> {
        if days <= 0 {
            return Err(lightframe_core::Error::Other(
                "cleanup days must be positive".to_string(),
            ));
        }
        let conn = self.conn()?;
        let deleted = conn
            .execute(
                "DELETE FROM media_files
                 WHERE is_deleted = 1
                   AND deleted_at IS NOT NULL
                   AND deleted_at < datetime('now', ?1)",
                params![format!("-{days} days")],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(deleted)
    }

    pub fn get_location_groups(&self) -> lightframe_core::Result<Vec<LocationGroup>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT country, city, COUNT(*) AS cnt, MIN(id) AS sample_id
                 FROM media_files
                 WHERE country IS NOT NULL AND is_deleted = 0
                 GROUP BY country, city
                 ORDER BY country, city",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(LocationGroup {
                    country: row.get(0)?,
                    city: row.get(1)?,
                    count: row.get(2)?,
                    sample_media_id: row.get(3)?,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_by_location(
        &self,
        country: &str,
        city: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE country = ?1 AND is_deleted = 0
                   AND ((?2 IS NULL AND city IS NULL) OR city = ?2)
                 ORDER BY COALESCE(created_at, modified_at) DESC
                 LIMIT ?3 OFFSET ?4",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![country, city, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn update_media_location(
        &self,
        media_id: i64,
        city: &str,
        country: &str,
    ) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE media_files SET city = ?1, country = ?2 WHERE id = ?3",
            params![city, country, media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn create_album(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> lightframe_core::Result<Album> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(lightframe_core::Error::Other(
                "album name cannot be empty".to_string(),
            ));
        }

        let id = {
            let conn = self.conn()?;
            conn.execute(
                "INSERT INTO albums (name, description) VALUES (?1, ?2)",
                params![trimmed, description],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            conn.last_insert_rowid()
        };
        self.get_album(id)?.ok_or_else(|| {
            lightframe_core::Error::Other(format!("album {id} not found after insert"))
        })
    }

    pub fn update_album(
        &self,
        id: i64,
        name: &str,
        description: Option<&str>,
    ) -> lightframe_core::Result<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(lightframe_core::Error::Other(
                "album name cannot be empty".to_string(),
            ));
        }

        let conn = self.conn()?;
        conn.execute(
            "UPDATE albums SET name = ?1, description = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![trimmed, description, id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_album(&self, id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM albums WHERE id = ?1", params![id])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_albums(&self) -> lightframe_core::Result<Vec<Album>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.name, a.description, a.cover_media_id,
                        COALESCE(COUNT(ai.media_id), 0) AS media_count,
                        a.created_at, a.updated_at
                 FROM albums a
                 LEFT JOIN album_items ai ON ai.album_id = a.id
                 GROUP BY a.id
                 ORDER BY a.updated_at DESC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_album_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_album(&self, id: i64) -> lightframe_core::Result<Option<Album>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT a.id, a.name, a.description, a.cover_media_id,
                    COALESCE(COUNT(ai.media_id), 0) AS media_count,
                    a.created_at, a.updated_at
             FROM albums a
             LEFT JOIN album_items ai ON ai.album_id = a.id
             WHERE a.id = ?1
             GROUP BY a.id",
            params![id],
            map_album_row,
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn add_to_album(&self, album_id: i64, media_ids: &[i64]) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        for media_id in media_ids {
            conn.execute(
                "INSERT OR IGNORE INTO album_items (album_id, media_id) VALUES (?1, ?2)",
                params![album_id, media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }
        conn.execute(
            "UPDATE albums SET updated_at = datetime('now') WHERE id = ?1",
            params![album_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn remove_from_album(&self, album_id: i64, media_id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM album_items WHERE album_id = ?1 AND media_id = ?2",
            params![album_id, media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        conn.execute(
            "UPDATE albums SET updated_at = datetime('now') WHERE id = ?1",
            params![album_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_album_media(
        &self,
        album_id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.path, m.filename, m.media_type, m.size_bytes, m.width, m.height,
                        m.created_at, m.modified_at, m.blake3_hash, m.dhash, m.phash, m.latitude, m.longitude
                 FROM album_items ai
                 JOIN media_files m ON m.id = ai.media_id
                 WHERE ai.album_id = ?1 AND m.is_deleted = 0
                 ORDER BY ai.sort_order, ai.added_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![album_id, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn set_album_cover(&self, album_id: i64, media_id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE albums SET cover_media_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![media_id, album_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn create_smart_album(
        &self,
        name: &str,
        icon: Option<&str>,
        rule: &SmartAlbumRule,
    ) -> lightframe_core::Result<SmartAlbum> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(lightframe_core::Error::Other(
                "smart album name cannot be empty".to_string(),
            ));
        }

        let rule_json = serde_json::to_string(rule)
            .map_err(|e| lightframe_core::Error::Other(format!("failed to serialize rule: {e}")))?;

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO smart_albums (name, icon, rule_json) VALUES (?1, ?2, ?3)",
            params![trimmed, icon, rule_json],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let id = conn.last_insert_rowid();
        let created_at = conn
            .query_row(
                "SELECT created_at FROM smart_albums WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        drop(conn);

        let media_count = smart_album_media_count(self, rule)?;

        Ok(SmartAlbum {
            id,
            name: trimmed.to_string(),
            icon: icon.map(str::to_string),
            rule_json,
            media_count,
            created_at,
        })
    }

    pub fn list_smart_albums(&self) -> lightframe_core::Result<Vec<SmartAlbum>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, icon, rule_json, 0 AS media_count, created_at
                 FROM smart_albums
                 ORDER BY created_at ASC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_smart_album_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut albums: Vec<SmartAlbum> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rules: Vec<SmartAlbumRule> = albums
            .iter()
            .map(|album| {
                serde_json::from_str(&album.rule_json).map_err(|e| {
                    lightframe_core::Error::Other(format!(
                        "invalid rule_json for album {}: {e}",
                        album.id
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let counts = batch_smart_album_media_counts(&conn, &rules)?;
        for (album, count) in albums.iter_mut().zip(counts) {
            album.media_count = count;
        }

        Ok(albums)
    }

    pub fn delete_smart_album(&self, id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM smart_albums WHERE id = ?1", params![id])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_smart_album_media(
        &self,
        id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let rule_json: String = conn
            .query_row(
                "SELECT rule_json FROM smart_albums WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rule: SmartAlbumRule = serde_json::from_str(&rule_json).map_err(|e| {
            lightframe_core::Error::Other(format!("invalid rule_json for smart album {id}: {e}"))
        })?;

        let (where_clause, filter_params) = build_smart_album_filter(&rule);
        let sql = format!(
            "SELECT id, path, filename, media_type, size_bytes, width, height,
                    created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
             FROM media_files
             WHERE {where_clause}
             ORDER BY COALESCE(created_at, modified_at) DESC
             LIMIT ? OFFSET ?"
        );

        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = filter_params;
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(param_refs.as_slice(), Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_on_this_day_media(&self, limit: i64) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM media_files
                 WHERE is_deleted = 0
                   AND media_type = 'Photo'
                   AND created_at IS NOT NULL
                   AND strftime('%m-%d', created_at) = strftime('%m-%d', 'now')
                   AND strftime('%Y', created_at) < strftime('%Y', 'now')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if count < 3 {
            return Ok(Vec::new());
        }

        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                   AND media_type = 'Photo'
                   AND created_at IS NOT NULL
                   AND strftime('%m-%d', created_at) = strftime('%m-%d', 'now')
                   AND strftime('%Y', created_at) < strftime('%Y', 'now')
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn generate_memories(&self) -> lightframe_core::Result<Vec<Memory>> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM memories", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut group_stmt = conn
            .prepare(
                "SELECT strftime('%Y', created_at) AS year,
                        strftime('%m', created_at) AS month,
                        COALESCE(country, '') AS country,
                        COUNT(*) AS cnt
                 FROM media_files
                 WHERE is_deleted = 0
                   AND media_type = 'Photo'
                   AND created_at IS NOT NULL
                 GROUP BY year, month, country
                 HAVING cnt >= 5",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let groups: Vec<(String, String, String, i64)> = group_stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        for (year, month, country, _count) in groups {
            let month_num: u32 = month.parse().unwrap_or(1);
            let year_num: i32 = year.parse().unwrap_or(0);

            let mut media_stmt = conn
                .prepare(
                    "SELECT id, city, created_at
                     FROM media_files
                     WHERE is_deleted = 0
                       AND media_type = 'Photo'
                       AND created_at IS NOT NULL
                       AND strftime('%Y', created_at) = ?1
                       AND strftime('%m', created_at) = ?2
                       AND COALESCE(country, '') = ?3
                     ORDER BY created_at ASC",
                )
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            let media_rows: Vec<(i64, Option<String>, String)> = media_stmt
                .query_map(params![year, month, country], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            if media_rows.len() < 5 {
                continue;
            }

            let cover_media_id = media_rows[0].0;
            let date_from = media_rows.first().map(|r| r.2.clone()).unwrap_or_default();
            let date_to = media_rows.last().map(|r| r.2.clone()).unwrap_or_default();

            let mut city_counts: std::collections::HashMap<String, i64> =
                std::collections::HashMap::new();
            for (_, city, _) in &media_rows {
                if let Some(c) = city
                    && !c.is_empty()
                {
                    *city_counts.entry(c.clone()).or_insert(0) += 1;
                }
            }
            let dominant_city = city_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(city, _)| city);

            let place = dominant_city
                .clone()
                .or_else(|| {
                    if country.is_empty() {
                        None
                    } else {
                        Some(country.clone())
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());

            let title = format!("{year_num}年{month_num}月 · {place}");
            let subtitle = dominant_city.map(|city| format!("{city}, {country}"));

            conn.execute(
                "INSERT INTO memories (title, subtitle, cover_media_id, date_from, date_to)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![title, subtitle, cover_media_id, date_from, date_to],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            let memory_id = conn.last_insert_rowid();
            for (media_id, _, _) in &media_rows {
                conn.execute(
                    "INSERT INTO memory_items (memory_id, media_id) VALUES (?1, ?2)",
                    params![memory_id, media_id],
                )
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            }
        }

        self.list_memories()
    }

    pub fn list_memories(&self) -> lightframe_core::Result<Vec<Memory>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.title, m.subtitle, m.cover_media_id,
                        COALESCE(COUNT(mi.media_id), 0) AS media_count,
                        m.date_from, m.date_to, m.created_at
                 FROM memories m
                 LEFT JOIN memory_items mi ON mi.memory_id = m.id
                 GROUP BY m.id
                 ORDER BY m.date_from DESC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_memory_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_memory_media(
        &self,
        memory_id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT mf.id, mf.path, mf.filename, mf.media_type, mf.size_bytes, mf.width, mf.height,
                        mf.created_at, mf.modified_at, mf.blake3_hash, mf.dhash, mf.phash, mf.latitude, mf.longitude
                 FROM memory_items mi
                 JOIN media_files mf ON mf.id = mi.media_id
                 WHERE mi.memory_id = ?1 AND mf.is_deleted = 0
                 ORDER BY COALESCE(mf.created_at, mf.modified_at) ASC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![memory_id, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_favorites(
        &self,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_favorite = 1 AND is_deleted = 0
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_favorites_count(&self) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM media_files WHERE is_favorite = 1 AND is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn is_favorite(&self, media_id: i64) -> lightframe_core::Result<bool> {
        let conn = self.read_conn()?;
        let result: i64 = conn
            .query_row(
                "SELECT is_favorite FROM media_files WHERE id = ?1 AND is_deleted = 0",
                params![media_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
            .unwrap_or(0);
        Ok(result != 0)
    }

    pub fn search_media(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.path, m.filename, m.media_type, m.size_bytes, m.width, m.height,
                        m.created_at, m.modified_at, m.blake3_hash, m.dhash, m.phash, m.latitude, m.longitude
                 FROM media_fts f
                 JOIN media_files m ON m.id = f.rowid
                 WHERE media_fts MATCH ?1 AND m.is_deleted = 0
                 ORDER BY rank
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![fts_query, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn search_media_count(&self, query: &str) -> lightframe_core::Result<i64> {
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(0);
        }

        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM media_fts f
             JOIN media_files m ON m.id = f.rowid
             WHERE media_fts MATCH ?1 AND m.is_deleted = 0",
            params![fts_query],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_location_stats(&self) -> lightframe_core::Result<LocationStats> {
        let conn = self.read_conn()?;

        let total_with_gps: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM media_files
                 WHERE latitude IS NOT NULL AND longitude IS NOT NULL AND is_deleted = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let countries: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT country) FROM media_files
                 WHERE country IS NOT NULL AND is_deleted = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let cities: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
                     SELECT DISTINCT country, city FROM media_files
                     WHERE country IS NOT NULL AND is_deleted = 0
                 )",
                [],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(LocationStats {
            total_with_gps,
            countries,
            cities,
        })
    }

    pub fn get_media_with_geo(
        &self,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, path, filename, media_type, size_bytes, width, height,
                        created_at, modified_at, blake3_hash, dhash, phash, latitude, longitude
                 FROM media_files
                 WHERE is_deleted = 0
                   AND latitude IS NOT NULL
                   AND longitude IS NOT NULL
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_geo_clusters(&self, grid_size: f64) -> lightframe_core::Result<Vec<GeoCluster>> {
        let grid = grid_size.max(0.001);
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT
                    (CAST(latitude / ?1 AS INTEGER) * ?1 + ?1 / 2.0) AS center_lat,
                    (CAST(longitude / ?1 AS INTEGER) * ?1 + ?1 / 2.0) AS center_lon,
                    GROUP_CONCAT(id) AS ids,
                    COUNT(*) AS cnt
                 FROM media_files
                 WHERE is_deleted = 0
                   AND latitude IS NOT NULL
                   AND longitude IS NOT NULL
                 GROUP BY CAST(latitude / ?1 AS INTEGER), CAST(longitude / ?1 AS INTEGER)
                 ORDER BY cnt DESC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![grid], |row| {
                let lat: f64 = row.get(0)?;
                let lon: f64 = row.get(1)?;
                let ids_str: String = row.get(2)?;
                let count: i64 = row.get(3)?;
                let media_ids: Vec<i64> =
                    ids_str.split(',').filter_map(|s| s.parse().ok()).collect();
                Ok(GeoCluster {
                    latitude: lat,
                    longitude: lon,
                    count,
                    media_ids,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    fn person_sample_media_ids(
        conn: &Connection,
        person_id: i64,
    ) -> lightframe_core::Result<Vec<i64>> {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT media_id FROM face_detections
                 WHERE person_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 4",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![person_id], |row| row.get(0))
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn list_persons(&self) -> lightframe_core::Result<Vec<Person>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, face_count, cover_face_id, created_at
                 FROM persons
                 ORDER BY face_count DESC, created_at DESC",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut persons = Vec::new();
        for row in rows {
            let (id, name, face_count, cover_face_id, created_at) =
                row.map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            let sample_media_ids = Self::person_sample_media_ids(&conn, id)?;
            persons.push(Person {
                id,
                name,
                face_count,
                cover_face_id,
                sample_media_ids,
                created_at,
            });
        }

        Ok(persons)
    }

    pub fn get_person_media(
        &self,
        person_id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<MediaFile>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT mf.id, mf.path, mf.filename, mf.media_type, mf.size_bytes,
                        mf.width, mf.height, mf.created_at, mf.modified_at, mf.blake3_hash,
                        mf.dhash, mf.phash, mf.latitude, mf.longitude
                 FROM face_detections fd
                 JOIN media_files mf ON mf.id = fd.media_id
                 WHERE fd.person_id = ?1 AND mf.is_deleted = 0
                 ORDER BY COALESCE(mf.created_at, mf.modified_at) DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![person_id, limit, offset], Self::map_media_row)
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn rename_person(&self, person_id: i64, name: &str) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        let updated = conn
            .execute(
                "UPDATE persons SET name = ?1 WHERE id = ?2",
                params![name, person_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if updated == 0 {
            return Err(lightframe_core::Error::Database(format!(
                "person {person_id} not found"
            )));
        }

        Ok(())
    }

    pub fn get_all_face_embeddings(&self) -> lightframe_core::Result<Vec<(i64, Vec<f32>)>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, face_embedding FROM face_detections
                 WHERE face_embedding IS NOT NULL",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob_to_f32_vec(&blob)))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(rows
            .filter_map(|r| r.ok())
            .filter(|(_, emb)| !emb.is_empty())
            .collect())
    }

    pub fn get_unassigned_face_embeddings(&self) -> lightframe_core::Result<Vec<(i64, Vec<f32>)>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT fd.id, fd.face_embedding FROM face_detections fd
                 LEFT JOIN persons p ON fd.person_id = p.id
                 WHERE fd.face_embedding IS NOT NULL
                   AND (fd.person_id IS NULL OR p.name IS NULL OR TRIM(p.name) = '')",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob_to_f32_vec(&blob)))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(rows
            .filter_map(|r| r.ok())
            .filter(|(_, emb)| !emb.is_empty())
            .collect())
    }

    /// Unassign faces from unnamed persons so they can be re-clustered.
    /// Does not delete person records — call [`Self::delete_empty_unnamed_persons`]
    /// after re-clustering to remove orphaned shells.
    pub fn unassign_faces_from_unnamed_persons(&self) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE face_detections SET person_id = NULL
             WHERE person_id IN (
                 SELECT id FROM persons WHERE name IS NULL OR TRIM(COALESCE(name, '')) = ''
             )",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Remove unnamed person records that no longer have any assigned faces.
    pub fn delete_empty_unnamed_persons(&self) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM persons
             WHERE (name IS NULL OR TRIM(COALESCE(name, '')) = '')
               AND NOT EXISTS (
                   SELECT 1 FROM face_detections WHERE person_id = persons.id
               )",
            [],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_person_face_count(&self, person_id: i64) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM face_detections WHERE person_id = ?1",
            params![person_id],
            |row| row.get(0),
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn clear_person_clusters(&self) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute("UPDATE face_detections SET person_id = NULL", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        conn.execute("DELETE FROM persons", [])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn merge_persons(&self, target_id: i64, source_ids: &[i64]) -> lightframe_core::Result<()> {
        let conn = self.conn()?;

        let target_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM persons WHERE id = ?1)",
                params![target_id],
                |row| row.get(0),
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        if !target_exists {
            return Err(lightframe_core::Error::Other(format!(
                "person {target_id} not found"
            )));
        }

        for source_id in source_ids {
            if *source_id == target_id {
                continue;
            }
            let source_exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM persons WHERE id = ?1)",
                    params![source_id],
                    |row| row.get(0),
                )
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            if !source_exists {
                return Err(lightframe_core::Error::Other(format!(
                    "person {source_id} not found"
                )));
            }
            conn.execute(
                "UPDATE face_detections SET person_id = ?1 WHERE person_id = ?2",
                params![target_id, source_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            conn.execute("DELETE FROM persons WHERE id = ?1", params![source_id])
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }

        conn.execute(
            "UPDATE persons SET face_count = (
                 SELECT COUNT(*) FROM face_detections WHERE person_id = ?1
             ) WHERE id = ?1",
            params![target_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_persons_count(&self) -> lightframe_core::Result<i64> {
        let conn = self.read_conn()?;
        conn.query_row("SELECT COUNT(*) FROM persons", [], |row| row.get(0))
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn store_clip_embedding(
        &self,
        media_id: i64,
        embedding: &[f32],
    ) -> lightframe_core::Result<()> {
        let blob = f32_slice_to_blob(embedding);
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO media_embeddings (media_id, clip_embedding, embedding_model)
             VALUES (?1, ?2, 'clip-vit-b32')
             ON CONFLICT(media_id) DO UPDATE SET
               clip_embedding = excluded.clip_embedding,
               embedding_model = excluded.embedding_model,
               created_at = datetime('now')",
            params![media_id, blob],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_clip_embedding(&self, media_id: i64) -> lightframe_core::Result<Option<Vec<f32>>> {
        let conn = self.read_conn()?;
        let blob: Option<Vec<u8>> = conn
            .query_row(
                "SELECT clip_embedding FROM media_embeddings WHERE media_id = ?1",
                params![media_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        Ok(blob.map(|b| blob_to_f32_vec(&b)).filter(|v| !v.is_empty()))
    }

    pub fn list_media_without_clip_embedding(
        &self,
        limit: i64,
    ) -> lightframe_core::Result<Vec<(i64, String)>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.path
                 FROM media_files m
                 LEFT JOIN media_embeddings e ON e.media_id = m.id
                 WHERE m.is_deleted = 0
                   AND m.media_type IN ('Photo', 'Screenshot', 'Raw')
                   AND (e.clip_embedding IS NULL)
                 ORDER BY m.id ASC
                 LIMIT ?1",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_all_clip_embeddings(&self) -> lightframe_core::Result<Vec<(i64, Vec<f32>)>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT media_id, clip_embedding FROM media_embeddings
                 WHERE clip_embedding IS NOT NULL",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob_to_f32_vec(&blob)))
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    /// Find media similar to the given media's CLIP embedding.
    ///
    /// NOTE: Current implementation is O(n) linear scan over all embeddings.
    /// For libraries >10K photos, consider using sqlite-vec or an external ANN index.
    pub fn find_similar_media(
        &self,
        media_id: i64,
        threshold: f32,
        limit: usize,
    ) -> lightframe_core::Result<Vec<(i64, f32)>> {
        let target = self.get_clip_embedding(media_id)?.ok_or_else(|| {
            lightframe_core::Error::Database(format!("no CLIP embedding for media {media_id}"))
        })?;

        let candidates: Vec<(i64, Vec<f32>)> = self
            .get_all_clip_embeddings()?
            .into_iter()
            .filter(|(id, _)| *id != media_id)
            .collect();

        Ok(find_similar_embeddings(
            &target,
            &candidates,
            threshold,
            limit,
        ))
    }

    /// Cosine-similarity search between a text/query embedding and stored media embeddings.
    pub fn semantic_search_by_embedding(
        &self,
        embedding: &[f32],
        threshold: f32,
        limit: usize,
    ) -> lightframe_core::Result<Vec<(MediaFile, f32)>> {
        let candidates = self.get_all_clip_embeddings()?;
        let scored = find_similar_embeddings(embedding, &candidates, threshold, limit);

        let ids: Vec<i64> = scored.iter().map(|(id, _)| *id).collect();
        let media_by_id = self.get_media_by_ids(&ids)?;

        let mut results = Vec::with_capacity(scored.len());
        for (id, score) in scored {
            if let Some(media) = media_by_id.get(&id) {
                results.push((media.clone(), score));
            }
        }
        Ok(results)
    }

    pub fn store_face_detections(
        &self,
        media_id: i64,
        faces: &[FaceDetectionInput],
    ) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM face_detections WHERE media_id = ?1",
            params![media_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        for face in faces {
            let blob = f32_slice_to_blob(&face.embedding);
            conn.execute(
                "INSERT INTO face_detections
                 (media_id, face_embedding, bbox_x, bbox_y, bbox_w, bbox_h, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    media_id,
                    blob,
                    face.bbox[0],
                    face.bbox[1],
                    face.bbox[2] - face.bbox[0],
                    face.bbox[3] - face.bbox[1],
                    face.confidence,
                ],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }

        Ok(())
    }

    pub fn get_faces_for_media(
        &self,
        media_id: i64,
    ) -> lightframe_core::Result<Vec<FaceDetectionRecord>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, media_id, bbox_x, bbox_y, bbox_w, bbox_h, confidence, person_id
                 FROM face_detections WHERE media_id = ?1",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![media_id], |row| {
                Ok(FaceDetectionRecord {
                    id: row.get(0)?,
                    media_id: row.get(1)?,
                    bbox_x: row.get(2)?,
                    bbox_y: row.get(3)?,
                    bbox_w: row.get(4)?,
                    bbox_h: row.get(5)?,
                    confidence: row.get(6)?,
                    person_id: row.get(7)?,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_face_by_id(
        &self,
        face_id: i64,
    ) -> lightframe_core::Result<Option<FaceDetectionRecord>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, media_id, bbox_x, bbox_y, bbox_w, bbox_h, confidence, person_id
                 FROM face_detections WHERE id = ?1",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![face_id], |row| {
                Ok(FaceDetectionRecord {
                    id: row.get(0)?,
                    media_id: row.get(1)?,
                    bbox_x: row.get(2)?,
                    bbox_y: row.get(3)?,
                    bbox_w: row.get(4)?,
                    bbox_h: row.get(5)?,
                    confidence: row.get(6)?,
                    person_id: row.get(7)?,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.next()
            .transpose()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_faces_for_person(
        &self,
        person_id: i64,
        limit: i64,
        offset: i64,
    ) -> lightframe_core::Result<Vec<FaceDetectionRecord>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, media_id, bbox_x, bbox_y, bbox_w, bbox_h, confidence, person_id
                 FROM face_detections
                 WHERE person_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![person_id, limit, offset], |row| {
                Ok(FaceDetectionRecord {
                    id: row.get(0)?,
                    media_id: row.get(1)?,
                    bbox_x: row.get(2)?,
                    bbox_y: row.get(3)?,
                    bbox_w: row.get(4)?,
                    bbox_h: row.get(5)?,
                    confidence: row.get(6)?,
                    person_id: row.get(7)?,
                })
            })
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn get_media_ids_without_faces(&self) -> lightframe_core::Result<Vec<i64>> {
        let conn = self.read_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT mf.id FROM media_files mf
                 WHERE mf.is_deleted = 0
                   AND mf.media_type IN ('Photo', 'Raw', 'LivePhoto', 'Screenshot')
                   AND NOT EXISTS (
                       SELECT 1 FROM face_detections fd WHERE fd.media_id = mf.id
                   )
                 ORDER BY mf.id",
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn create_person(&self, name: Option<&str>) -> lightframe_core::Result<i64> {
        let conn = self.conn()?;
        conn.execute("INSERT INTO persons (name) VALUES (?1)", params![name])
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn assign_face_to_person(
        &self,
        face_id: i64,
        person_id: i64,
    ) -> lightframe_core::Result<()> {
        let conn = self.conn()?;

        let old_person_id: Option<i64> = conn
            .query_row(
                "SELECT person_id FROM face_detections WHERE id = ?1",
                params![face_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
            .flatten();

        let updated = conn
            .execute(
                "UPDATE face_detections SET person_id = ?1 WHERE id = ?2",
                params![person_id, face_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if updated == 0 {
            return Err(lightframe_core::Error::Database(format!(
                "face {face_id} not found"
            )));
        }

        conn.execute(
            "UPDATE persons SET face_count = (
                 SELECT COUNT(*) FROM face_detections WHERE person_id = ?1
             ) WHERE id = ?1",
            params![person_id],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if let Some(old_id) = old_person_id
            && old_id != person_id
        {
            conn.execute(
                "UPDATE persons SET face_count = (
                     SELECT COUNT(*) FROM face_detections WHERE person_id = ?1
                 ) WHERE id = ?1",
                params![old_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            conn.execute(
                "DELETE FROM persons WHERE id = ?1 AND face_count = 0 AND (name IS NULL OR name = '')",
                params![old_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        }

        Ok(())
    }

    pub fn split_face_from_person(
        &self,
        face_id: i64,
        new_person_name: Option<&str>,
    ) -> lightframe_core::Result<i64> {
        let conn = self.conn()?;

        let old_person_id: Option<i64> = conn
            .query_row(
                "SELECT person_id FROM face_detections WHERE id = ?1",
                params![face_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
            .flatten();

        conn.execute(
            "INSERT INTO persons (name) VALUES (?1)",
            params![new_person_name],
        )
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
        let new_person_id = conn.last_insert_rowid();

        let updated = conn
            .execute(
                "UPDATE face_detections SET person_id = ?1 WHERE id = ?2",
                params![new_person_id, face_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if updated == 0 {
            conn.execute("DELETE FROM persons WHERE id = ?1", params![new_person_id])
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            return Err(lightframe_core::Error::Database(format!(
                "face {face_id} not found"
            )));
        }

        for person_id in [Some(new_person_id), old_person_id].into_iter().flatten() {
            conn.execute(
                "UPDATE persons SET face_count = (
                     SELECT COUNT(*) FROM face_detections WHERE person_id = ?1
                 ) WHERE id = ?1",
                params![person_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            let remaining: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM face_detections WHERE person_id = ?1",
                    params![person_id],
                    |row| row.get(0),
                )
                .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

            if remaining == 0 {
                conn.execute("DELETE FROM persons WHERE id = ?1", params![person_id])
                    .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;
            }
        }

        Ok(new_person_id)
    }

    pub fn save_edit_params(&self, media_id: i64, params: &str) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        let updated = conn
            .execute(
                "UPDATE media_files SET edit_params = ?1 WHERE id = ?2 AND is_deleted = 0",
                params![params, media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if updated == 0 {
            return Err(lightframe_core::Error::Database(format!(
                "media {media_id} not found or deleted"
            )));
        }

        Ok(())
    }

    pub fn get_edit_params(&self, media_id: i64) -> lightframe_core::Result<Option<String>> {
        let conn = self.read_conn()?;
        conn.query_row(
            "SELECT edit_params FROM media_files WHERE id = ?1",
            params![media_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| lightframe_core::Error::Database(e.to_string()))
    }

    pub fn clear_edit_params(&self, media_id: i64) -> lightframe_core::Result<()> {
        let conn = self.conn()?;
        let updated = conn
            .execute(
                "UPDATE media_files SET edit_params = NULL WHERE id = ?1",
                params![media_id],
            )
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?;

        if updated == 0 {
            return Err(lightframe_core::Error::Database(format!(
                "media {media_id} not found"
            )));
        }

        Ok(())
    }

    pub fn has_edits(&self, media_id: i64) -> lightframe_core::Result<bool> {
        let conn = self.read_conn()?;
        let value: Option<String> = conn
            .query_row(
                "SELECT edit_params FROM media_files WHERE id = ?1",
                params![media_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| lightframe_core::Error::Database(e.to_string()))?
            .flatten();

        Ok(value.as_ref().is_some_and(|s| !s.trim().is_empty()))
    }
}

fn f32_slice_to_blob(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>()
}

fn blob_to_f32_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn find_similar_embeddings(
    target: &[f32],
    candidates: &[(i64, Vec<f32>)],
    threshold: f32,
    limit: usize,
) -> Vec<(i64, f32)> {
    let mut scored: Vec<(i64, f32)> = candidates
        .iter()
        .filter_map(|(id, emb)| {
            let score = cosine_similarity(target, emb);
            (score >= threshold).then_some((*id, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

struct UnionFind {
    parent: std::collections::HashMap<i64, i64>,
}

impl UnionFind {
    fn new(ids: impl IntoIterator<Item = i64>) -> Self {
        let parent = ids.into_iter().map(|id| (id, id)).collect();
        Self { parent }
    }

    fn find(&mut self, id: i64) -> i64 {
        let parent = self.parent.get(&id).copied().unwrap_or(id);
        if parent != id {
            let root = self.find(parent);
            self.parent.insert(id, root);
            root
        } else {
            id
        }
    }

    fn union(&mut self, a: i64, b: i64) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_b, root_a);
        }
    }
}
