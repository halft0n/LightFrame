use crate::state::AppState;
use lightframe_core::media::MediaFile;
use lightframe_db::{GeoCluster, LocationGroup, LocationStats, MediaNeighbors, TimelineGroup};
use tauri::State;

use super::{check_batch_size, db_err, remove_media_from_disk, validate_media_path};

#[tauri::command]
pub fn get_media_list(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_all_media(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_media_page(
    state: State<'_, AppState>,
    limit: i64,
    cursor: Option<(String, i64)>,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    state
        .db
        .get_media_page(limit, cursor)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count(state: State<'_, AppState>) -> Result<i64, String> {
    state.db.get_media_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_folder(
    state: State<'_, AppState>,
    folder_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_folder(folder_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count_by_folder(
    state: State<'_, AppState>,
    folder_id: i64,
) -> Result<i64, String> {
    state
        .db
        .get_media_count_by_folder(folder_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_export(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
    output_dir: String,
) -> Result<usize, String> {
    batch_export_impl(&state, media_ids, output_dir)
}

pub(crate) fn batch_export_impl(
    state: &AppState,
    media_ids: Vec<i64>,
    output_dir: String,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    let output = std::path::Path::new(&output_dir);
    if !output.is_dir() {
        return Err(format!("not a directory: {output_dir}"));
    }

    let mut exported = 0usize;
    let mut copy_attempts = 0usize;
    for media_id in media_ids {
        let Some(media) = state
            .db
            .get_media_by_id(media_id)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if let Err(e) = validate_media_path(&state.db, &media.path) {
            tracing::warn!("batch export: skipping media {}: {e}", media_id);
            continue;
        }

        let source = std::path::Path::new(&media.path);
        if !source.is_file() {
            tracing::warn!("batch export: source file not found: {}", media.path);
            continue;
        }

        let sanitized_name = sanitize_filename(&media.filename);
        let dest = unique_export_path(output, &sanitized_name);
        if !dest.starts_with(output) {
            return Err("invalid export path".to_string());
        }
        copy_attempts += 1;
        if let Err(e) = std::fs::copy(source, &dest) {
            tracing::warn!(
                "batch export: failed to copy {} to {}: {e}",
                media.path,
                dest.display()
            );
            continue;
        }
        exported += 1;
    }

    if copy_attempts > 0 && exported == 0 {
        return Err("batch export failed for all items".to_string());
    }

    Ok(exported)
}

fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("unnamed");
    let base = if base.is_empty() || base == "." || base == ".." {
        "unnamed"
    } else {
        base
    };
    base.to_string()
}

fn unique_export_path(dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let filename = sanitize_filename(filename);
    let mut dest = dir.join(&filename);
    if !dest.exists() {
        return dest;
    }

    let path = std::path::Path::new(&filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename);
    let ext = path.extension().and_then(|s| s.to_str());

    for i in 1..10000 {
        let candidate_name = match ext {
            Some(ext) => format!("{stem}_{i}.{ext}"),
            None => format!("{stem}_{i}"),
        };
        dest = dir.join(&candidate_name);
        if !dest.exists() {
            return dest;
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let fallback = match ext {
        Some(ext) => format!("{stem}_{ts}.{ext}"),
        None => format!("{stem}_{ts}"),
    };
    dir.join(fallback)
}
#[tauri::command]
pub fn get_media_by_id(state: State<'_, AppState>, id: i64) -> Result<Option<MediaFile>, String> {
    state
        .db
        .get_media_by_id(id)
        .map_err(|e| db_err("get_media_by_id", &id.to_string(), e))
}
#[tauri::command]
pub fn get_timeline_groups(
    state: State<'_, AppState>,
    limit: Option<i64>,
    cursor_created_at: Option<String>,
    cursor_id: Option<i64>,
) -> Result<Vec<TimelineGroup>, String> {
    let limit = limit.unwrap_or(200).clamp(1, 500);
    let cursor = match (cursor_created_at, cursor_id) {
        (Some(ts), Some(id)) => Some((ts, id)),
        _ => None,
    };
    state
        .db
        .get_timeline_groups(limit, cursor)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_neighbors(state: State<'_, AppState>, id: i64) -> Result<MediaNeighbors, String> {
    state
        .db
        .get_media_neighbors(id)
        .map_err(|e| db_err("get_media_neighbors", &id.to_string(), e))
}

#[tauri::command]
pub async fn get_media_window(
    state: State<'_, AppState>,
    media_id: i64,
    radius: usize,
) -> Result<Vec<MediaFile>, String> {
    let radius = radius.min(50);
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.get_media_window(media_id, radius)
            .map_err(|e| db_err("get_media_window", &media_id.to_string(), e))
    })
    .await
    .map_err(|e| format!("get_media_window({media_id}): {e}"))?
}

#[tauri::command]
pub fn get_location_groups(state: State<'_, AppState>) -> Result<Vec<LocationGroup>, String> {
    state.db.get_location_groups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_location(
    state: State<'_, AppState>,
    country: String,
    city: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_location(&country, city.as_deref(), limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_location_stats(state: State<'_, AppState>) -> Result<LocationStats, String> {
    state.db.get_location_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_type(
    state: State<'_, AppState>,
    media_type: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_type(&media_type, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count_by_type(
    state: State<'_, AppState>,
    media_type: String,
) -> Result<i64, String> {
    state
        .db
        .get_media_count_by_type(&media_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_screenshots(
    state: State<'_, AppState>,
    screenshot_type: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_screenshots(screenshot_type.as_deref(), limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_screenshot_count(
    state: State<'_, AppState>,
    screenshot_type: Option<String>,
) -> Result<i64, String> {
    state
        .db
        .get_screenshot_count(screenshot_type.as_deref())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn toggle_favorite(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state
        .db
        .toggle_favorite(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorites(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_favorites(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorites_count(state: State<'_, AppState>) -> Result<i64, String> {
    state.db.get_favorites_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_favorite(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state
        .db
        .is_favorite(media_id)
        .map_err(|e| db_err("is_favorite", &media_id.to_string(), e))
}

#[tauri::command]
pub fn delete_media(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .set_deleted(media_id, true)
        .map_err(|e| db_err("delete_media", &media_id.to_string(), e))
}

#[tauri::command]
pub fn get_deleted_media(state: State<'_, AppState>) -> Result<Vec<MediaFile>, String> {
    state.db.list_deleted_media().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_media(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .set_deleted(media_id, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn permanently_delete(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    let Some((path, hash, deleted)) = state
        .db
        .get_media_deletion_info(media_id)
        .map_err(|e| db_err("permanently_delete", &media_id.to_string(), e))?
    else {
        return Err(format!("permanently_delete({media_id}): media not found"));
    };
    if deleted == 0 {
        return Err(format!(
            "permanently_delete({media_id}): media is not in trash; soft-delete before permanent delete"
        ));
    }

    state
        .db
        .permanently_delete_media(media_id)
        .map_err(|e| db_err("permanently_delete", &media_id.to_string(), e))?;
    remove_media_from_disk(&path, hash.as_deref(), &state.db);
    state.thumb_cache.invalidate_media(media_id);
    crate::face_protocol::invalidate_face_cache_for_media(&state, media_id);
    Ok(())
}

#[tauri::command]
pub fn batch_delete_media(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .batch_set_deleted(&media_ids, true)
        .map_err(|e| db_err("batch_delete_media", &format!("{} ids", media_ids.len()), e))
}

#[tauri::command]
pub fn batch_add_to_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_ids: Vec<i64>,
) -> Result<(), String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .add_to_album(album_id, &media_ids)
        .map_err(|e| db_err("batch_add_to_album", &format!("album {album_id}"), e))
}

#[tauri::command]
pub fn batch_toggle_favorite(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
    favorite: bool,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .batch_set_favorite(&media_ids, favorite)
        .map_err(|e| {
            db_err(
                "batch_toggle_favorite",
                &format!("{} ids", media_ids.len()),
                e,
            )
        })
}

#[tauri::command]
pub fn batch_restore_media(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state.db.batch_set_deleted(&media_ids, false).map_err(|e| {
        db_err(
            "batch_restore_media",
            &format!("{} ids", media_ids.len()),
            e,
        )
    })
}

#[tauri::command]
pub fn batch_permanent_delete(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    let mut to_remove = Vec::new();
    for media_id in &media_ids {
        if let Some((path, hash, deleted)) = state
            .db
            .get_media_deletion_info(*media_id)
            .map_err(|e| e.to_string())?
            && deleted != 0
        {
            to_remove.push((path, hash));
        }
    }

    let affected = state.db.batch_permanent_delete(&media_ids).map_err(|e| {
        db_err(
            "batch_permanent_delete",
            &format!("{} ids", media_ids.len()),
            e,
        )
    })?;

    for (path, hash) in &to_remove {
        remove_media_from_disk(path, hash.as_deref(), &state.db);
    }

    for media_id in &media_ids {
        state.thumb_cache.invalidate_media(*media_id);
        crate::face_protocol::invalidate_face_cache_for_media(&state, *media_id);
    }

    Ok(affected)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn search_media(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .search_media(&query, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_media_count(state: State<'_, AppState>, query: String) -> Result<i64, String> {
    state
        .db
        .search_media_count(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_edit(state: State<'_, AppState>, media_id: i64, params: String) -> Result<(), String> {
    const MAX_EDIT_PARAMS_SIZE: usize = 64 * 1024; // 64KB
    if params.len() > MAX_EDIT_PARAMS_SIZE {
        return Err("edit parameters too large".to_string());
    }
    crate::image_edit::parse_edit_params(&params)?;
    state
        .db
        .save_edit_params(media_id, &params)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_edit(state: State<'_, AppState>, media_id: i64) -> Result<Option<String>, String> {
    state
        .db
        .get_edit_params(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn revert_edit(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .clear_edit_params(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_edits(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state.db.has_edits(media_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_edited(
    state: State<'_, AppState>,
    media_id: i64,
    output_path: String,
    quality: u8,
) -> Result<(), String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let params = state
        .db
        .get_edit_params(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no edit params saved for this media".to_string())?;

    validate_media_path(&state.db, &media.path)?;

    let quality = quality.clamp(1, 100);
    let out = std::path::Path::new(&output_path);
    if crate::original_protocol::path_contains_parent_dir(out) {
        return Err("output path contains path traversal".to_string());
    }
    if let Some(parent) = out.parent()
        && !parent.exists()
    {
        return Err("output directory does not exist".to_string());
    }
    crate::image_edit::export_edited_image(std::path::Path::new(&media.path), out, &params, quality)
}

#[tauri::command]
pub fn get_media_with_geo(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 5000);
    let offset = offset.max(0);
    state
        .db
        .get_media_with_geo(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_geo_clusters(
    state: State<'_, AppState>,
    grid_size: f64,
) -> Result<Vec<GeoCluster>, String> {
    state
        .db
        .get_geo_clusters(grid_size)
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn sanitize_filename_strips_path_traversal() {
        assert_eq!(sanitize_filename("../../outside.jpg"), "outside.jpg");
        assert_eq!(sanitize_filename("normal.jpg"), "normal.jpg");
        assert_eq!(sanitize_filename("sub/file.jpg"), "file.jpg");
        assert_eq!(sanitize_filename("..\\..\\evil.png"), "evil.png");
        assert_eq!(sanitize_filename(""), "unnamed");
        assert_eq!(sanitize_filename(".."), "unnamed");
    }

    #[test]
    fn unique_export_path_stays_within_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dest = unique_export_path(dir.path(), "../../outside.jpg");
        assert!(dest.starts_with(dir.path()));
        assert_eq!(
            dest.file_name().and_then(|n| n.to_str()),
            Some("outside.jpg")
        );
    }

    #[test]
    fn unique_export_path_sanitizes_separators_in_name() {
        let dir = tempfile::tempdir().unwrap();
        let dest = unique_export_path(dir.path(), "foo/bar.jpg");
        assert_eq!(dest, dir.path().join("bar.jpg"));
    }

    #[test]
    fn unique_export_path_adds_numbered_suffix_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("photo.jpg"), b"original").unwrap();

        let dest = unique_export_path(dir.path(), "photo.jpg");
        assert_eq!(dest, dir.path().join("photo_1.jpg"));
        assert!(!dest.exists());
    }

    #[test]
    fn unique_export_path_increments_suffix_until_free() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("photo.jpg"), b"1").unwrap();
        std::fs::write(dir.path().join("photo_1.jpg"), b"2").unwrap();
        std::fs::write(dir.path().join("photo_2.jpg"), b"3").unwrap();

        let dest = unique_export_path(dir.path(), "photo.jpg");
        assert_eq!(dest, dir.path().join("photo_3.jpg"));
    }

    #[test]
    fn unique_export_path_handles_extensionless_collision() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README"), b"v1").unwrap();

        let dest = unique_export_path(dir.path(), "README");
        assert_eq!(dest, dir.path().join("README_1"));
    }

    #[test]
    fn sanitize_filename_dot_segments_become_unnamed() {
        assert_eq!(sanitize_filename("   "), "   ");
        assert_eq!(sanitize_filename(""), "unnamed");
        assert_eq!(sanitize_filename("///"), "unnamed");
        assert_eq!(sanitize_filename("."), "unnamed");
    }
}

#[cfg(test)]
mod batch_export_tests {
    use super::*;
    use crate::state::ScanStatus;
    use lightframe_core::media::{MediaFile, MediaType};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn sample_media(path: &str) -> MediaFile {
        MediaFile {
            id: 0,
            path: path.to_string(),
            filename: path.rsplit('/').next().unwrap_or(path).to_string(),
            media_type: MediaType::Photo,
            size_bytes: 512,
            width: Some(64),
            height: Some(64),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        }
    }

    fn test_app_state(db: Arc<lightframe_db::Database>) -> AppState {
        AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            downloading: Arc::new(AtomicBool::new(false)),
            download_cancel: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: tempfile::tempdir().unwrap().into_path(),
        }
    }

    fn setup_watched_env() -> (tempfile::TempDir, AppState, i64, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("photos");
        std::fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        );
        let watched_str = watched_canonical.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Arc::new(lightframe_db::Database::open(&db_path).unwrap());
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;
        let state = test_app_state(db);
        (root, state, folder_id, watched_canonical)
    }

    #[test]
    fn batch_export_empty_batch_returns_zero() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let db = Arc::new(
            lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap(),
        );
        let state = test_app_state(db);

        let exported =
            batch_export_impl(&state, vec![], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
    }

    #[test]
    fn batch_export_all_invalid_ids_returns_zero() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db);

        let exported =
            batch_export_impl(&state, vec![999, 1000], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
        assert!(std::fs::read_dir(&out).unwrap().next().is_none());
    }

    #[test]
    fn batch_export_mixed_valid_and_invalid_exports_only_valid() {
        let (_root, state, folder_id, watched) = setup_watched_env();
        let out = _root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();

        let valid_path = watched.join("good.jpg");
        std::fs::write(&valid_path, b"jpeg-bytes").unwrap();
        let valid_id = state
            .db
            .upsert_media(folder_id, &sample_media(valid_path.to_str().unwrap()))
            .unwrap();

        let exported = batch_export_impl(
            &state,
            vec![valid_id, 99999],
            out.to_string_lossy().to_string(),
        )
        .unwrap();
        assert_eq!(exported, 1);
        assert!(out.join("good.jpg").is_file());
    }

    #[test]
    fn batch_export_rejects_non_directory_output() {
        let (_root, state, folder_id, watched) = setup_watched_env();
        let file_path = watched.join("photo.jpg");
        std::fs::write(&file_path, b"jpeg").unwrap();
        let media_id = state
            .db
            .upsert_media(folder_id, &sample_media(file_path.to_str().unwrap()))
            .unwrap();

        let err = batch_export_impl(
            &state,
            vec![media_id],
            file_path.to_string_lossy().to_string(),
        )
        .unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn batch_export_skips_path_outside_watched_folders() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&watched).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let watched_str = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        )
        .to_str()
        .unwrap()
        .to_string();

        let db_path = root.path().join("library.db");
        let db = Arc::new(lightframe_db::Database::open(&db_path).unwrap());
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;
        let state = test_app_state(db);

        let outside_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&outside).unwrap(),
        );
        let outside_file = outside_canonical.join("secret.jpg");
        std::fs::write(&outside_file, b"jpeg").unwrap();
        let media_id = state
            .db
            .upsert_media(folder_id, &sample_media(outside_file.to_str().unwrap()))
            .unwrap();

        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let exported =
            batch_export_impl(&state, vec![media_id], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
    }

    #[test]
    fn face_detecting_guard_releases_on_drop() {
        let flag = Arc::new(AtomicBool::new(true));
        struct FaceDetectingGuard(Arc<AtomicBool>);
        impl Drop for FaceDetectingGuard {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
            }
        }
        {
            let _guard = FaceDetectingGuard(Arc::clone(&flag));
            assert!(flag.load(Ordering::SeqCst));
        }
        assert!(!flag.load(Ordering::SeqCst));
    }
}
#[cfg(test)]
mod edit_persistence_tests {
    use lightframe_core::media::{MediaFile, MediaType};

    fn sample_media(path: &str) -> MediaFile {
        MediaFile {
            id: 0,
            path: path.to_string(),
            filename: path.rsplit('/').next().unwrap_or(path).to_string(),
            media_type: MediaType::Photo,
            size_bytes: 512,
            width: Some(64),
            height: Some(64),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        }
    }

    #[test]
    fn edit_save_load_revert_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let db = lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap();
        let folder_id = db.add_watched_folder("/photos").unwrap().id;
        let media_id = db
            .upsert_media(folder_id, &sample_media("/photos/edit.jpg"))
            .unwrap();

        let params = r#"{"brightness":12.0,"contrast":-5.0}"#;
        crate::image_edit::parse_edit_params(params).expect("valid params");
        db.save_edit_params(media_id, params).unwrap();
        assert!(db.has_edits(media_id).unwrap());

        let loaded = db.get_edit_params(media_id).unwrap().expect("saved params");
        assert_eq!(loaded, params);

        db.clear_edit_params(media_id).unwrap();
        assert!(!db.has_edits(media_id).unwrap());
        assert!(db.get_edit_params(media_id).unwrap().is_none());
    }

    #[test]
    fn edit_save_rejects_invalid_json_via_parse() {
        let invalid = r#"{"brightness":"not-a-number"}"#;
        assert!(crate::image_edit::parse_edit_params(invalid).is_err());
    }
}
