use crate::scan;
use crate::state::{AppState, ScanProgress};
use crate::watcher;
use catchlight_ai::AiStatus;
use catchlight_core::config::AppConfig;
use catchlight_core::media::{MediaFile, ThumbnailSize};
use catchlight_db::{
    Album, DuplicateGroupDetail, LocationGroup, LocationStats, MediaNeighbors, Memory, Person,
    SmartAlbum, SmartAlbumRule, TimelineGroup, WatchedFolder,
};
use catchlight_thumbnail::thumb_path;
use serde::Serialize;
use tauri::{AppHandle, State};

const MAX_BATCH_SIZE: usize = 1000;

fn check_batch_size(media_ids: &[i64]) -> Result<(), String> {
    if media_ids.len() > MAX_BATCH_SIZE {
        return Err(format!(
            "batch size {} exceeds maximum {}",
            media_ids.len(),
            MAX_BATCH_SIZE
        ));
    }
    Ok(())
}

fn db_err(cmd: &str, context: &str, e: impl std::fmt::Display) -> String {
    format!("{cmd}({context}): {e}")
}

#[derive(Serialize)]
pub struct DedupScanResult {
    pub exact_groups: usize,
    pub perceptual_groups: usize,
    pub total_duplicates: usize,
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.clone()
}

#[tauri::command]
pub fn add_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<WatchedFolder, String> {
    let folder_path = std::path::Path::new(&path);
    if !folder_path.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let folder = state
        .db
        .add_watched_folder(&path)
        .map_err(|e| e.to_string())?;

    scan::spawn_scan(app.clone(), &state, folder.id);
    let _ = watcher::start(&app, &state);
    Ok(folder)
}

#[tauri::command]
pub fn remove_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state
        .db
        .remove_watched_folder(id)
        .map_err(|e| e.to_string())?;
    watcher::start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_watched_folders(state: State<'_, AppState>) -> Result<Vec<WatchedFolder>, String> {
    state.db.list_watched_folders().map_err(|e| e.to_string())
}

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
    check_batch_size(&media_ids)?;
    let output = std::path::Path::new(&output_dir);
    if !output.is_dir() {
        return Err(format!("not a directory: {output_dir}"));
    }

    let mut exported = 0usize;
    for media_id in media_ids {
        let Some(media) = state
            .db
            .get_media_by_id(media_id)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

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
        std::fs::copy(source, &dest)
            .map_err(|e| format!("failed to copy {} to {}: {e}", media.path, dest.display()))?;
        exported += 1;
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

    for i in 1..1000 {
        let candidate_name = match ext {
            Some(ext) => format!("{stem}_{i}.{ext}"),
            None => format!("{stem}_{i}"),
        };
        dest = dir.join(&candidate_name);
        if !dest.exists() {
            return dest;
        }
    }

    dir.join(&filename)
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
}

#[tauri::command]
pub fn get_media_by_id(state: State<'_, AppState>, id: i64) -> Result<Option<MediaFile>, String> {
    state
        .db
        .get_media_by_id(id)
        .map_err(|e| db_err("get_media_by_id", &id.to_string(), e))
}

#[tauri::command]
#[tracing::instrument(skip(app, state))]
pub fn scan_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_id: i64,
) -> Result<(), String> {
    state
        .db
        .get_watched_folder(folder_id)
        .map_err(|e| db_err("scan_folder", &folder_id.to_string(), e))?
        .ok_or_else(|| format!("scan_folder({folder_id}): folder not found"))?;

    scan::spawn_scan(app, &state, folder_id);
    Ok(())
}

#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> ScanProgress {
    state.scan_status.snapshot()
}

#[tauri::command]
pub fn start_watching(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    watcher::start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_watching(state: State<'_, AppState>) -> Result<(), String> {
    watcher::stop(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_timeline_groups(
    state: State<'_, AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<TimelineGroup>, String> {
    let limit = limit.unwrap_or(200).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);
    state
        .db
        .get_timeline_groups(limit, offset)
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
pub async fn run_dedup_scan(state: State<'_, AppState>) -> Result<DedupScanResult, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.clear_duplicate_groups().map_err(|e| e.to_string())?;

        let exact_groups = db.find_exact_duplicates().map_err(|e| e.to_string())?;
        let perceptual_groups = db
            .find_perceptual_duplicates(10)
            .map_err(|e| e.to_string())?;

        let total_duplicates = exact_groups
            .iter()
            .chain(perceptual_groups.iter())
            .map(|g| g.members.len())
            .sum();

        Ok(DedupScanResult {
            exact_groups: exact_groups.len(),
            perceptual_groups: perceptual_groups.len(),
            total_duplicates,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_groups(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroupDetail>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.list_duplicate_groups().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_duplicate_groups_count().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_duplicate(
    state: State<'_, AppState>,
    group_id: i64,
    keep_media_id: i64,
    delete_files: bool,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.resolve_duplicate_group(group_id, keep_media_id, delete_files)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn dismiss_duplicate_group(
    state: State<'_, AppState>,
    group_id: i64,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.delete_duplicate_group(group_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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

#[derive(Serialize)]
pub struct ModelStatus {
    pub models_dir: String,
    pub clip_available: bool,
    pub face_available: bool,
}

#[tauri::command]
pub async fn get_model_status() -> Result<ModelStatus, String> {
    Ok(ModelStatus {
        models_dir: catchlight_ai::models::models_dir()
            .to_string_lossy()
            .to_string(),
        clip_available: catchlight_ai::models::clip_model_available(),
        face_available: catchlight_ai::models::face_model_available(),
    })
}

#[tauri::command]
pub fn open_models_dir() -> Result<(), String> {
    catchlight_ai::models::ensure_models_dir().map_err(|e| e.to_string())?;
    let path = catchlight_ai::models::models_dir();
    open_in_file_manager(&path)
}

fn open_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn create_album(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Album, String> {
    if name.trim().is_empty() {
        return Err("album name cannot be empty".to_string());
    }
    state
        .db
        .create_album(&name, description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_album(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_album(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_album(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("album name cannot be empty".to_string());
    }
    state
        .db
        .update_album(id, &name, description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_album_cover(
    state: State<'_, AppState>,
    album_id: i64,
    media_id: i64,
) -> Result<(), String> {
    state
        .db
        .set_album_cover(album_id, media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_albums(state: State<'_, AppState>) -> Result<Vec<Album>, String> {
    state.db.list_albums().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_ids: Vec<i64>,
) -> Result<(), String> {
    state
        .db
        .add_to_album(album_id, &media_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_id: i64,
) -> Result<(), String> {
    state
        .db
        .remove_from_album(album_id, media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_album_media(
    state: State<'_, AppState>,
    album_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_album_media(album_id, limit, offset)
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
    state.thumb_cache.invalidate_media(media_id);
    remove_media_from_disk(&path, hash.as_deref());
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

    for media_id in &media_ids {
        state.thumb_cache.invalidate_media(*media_id);
    }

    for (path, hash) in to_remove {
        remove_media_from_disk(&path, hash.as_deref());
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

#[derive(Serialize)]
pub struct SearchResult {
    pub media_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub relevance: f32,
}

/// Ranked text search placeholder for future CLIP vector search.
///
/// The command name `semantic_search` is kept for frontend compatibility. Until CLIP text
/// encoder and embedding index are available, this uses SQLite FTS5 keyword matching and
/// assigns a descending pseudo-relevance score by result order.
#[tauri::command]
pub fn semantic_search(
    state: State<'_, AppState>,
    query_text: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(50).clamp(1, 500) as i64;
    let trimmed = query_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let media = state
        .db
        .search_media(trimmed, limit, 0)
        .map_err(|e| db_err("semantic_search", trimmed, e))?;

    Ok(media
        .into_iter()
        .enumerate()
        .map(|(i, m)| SearchResult {
            media_id: m.id,
            file_name: m.filename,
            file_path: m.path,
            relevance: 1.0 - (i as f32 * 0.01).min(0.9),
        })
        .collect())
}

#[tauri::command]
pub fn create_smart_album(
    state: State<'_, AppState>,
    name: String,
    icon: Option<String>,
    rule: SmartAlbumRule,
) -> Result<SmartAlbum, String> {
    if name.trim().is_empty() {
        return Err("smart album name cannot be empty".to_string());
    }
    state
        .db
        .create_smart_album(&name, icon.as_deref(), &rule)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_smart_albums(state: State<'_, AppState>) -> Result<Vec<SmartAlbum>, String> {
    state.db.list_smart_albums().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_smart_album(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_smart_album(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_smart_album_media(
    state: State<'_, AppState>,
    id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_smart_album_media(id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_on_this_day(state: State<'_, AppState>, limit: i64) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    state
        .db
        .get_on_this_day_media(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.generate_memories().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    state.db.list_memories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memory_media(
    state: State<'_, AppState>,
    memory_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_memory_media(memory_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ai_status() -> AiStatus {
    catchlight_ai::check_ai_status()
}

#[derive(Serialize)]
pub struct SimilarPhoto {
    pub media_id: i64,
    pub similarity: f32,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct FaceInfo {
    pub id: i64,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub person_id: Option<i64>,
}

const SIMILAR_THRESHOLD: f32 = 0.65;

#[tauri::command]
pub async fn compute_clip_embedding(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<(), String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let mut ai = state.ai.lock().await;
    ai.ensure_python().await.map_err(|e| e.to_string())?;

    let embedding = ai
        .compute_embedding(path)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "CLIP embedding unavailable — place a CLIP ONNX model in the data/models directory or install the Python AI extension".to_string()
        })?;

    state
        .db
        .store_clip_embedding(media_id, &embedding)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_similar_photos(
    state: State<'_, AppState>,
    media_id: i64,
    limit: Option<usize>,
) -> Result<Vec<SimilarPhoto>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 100);

    if state
        .db
        .get_clip_embedding(media_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        compute_clip_embedding(state.clone(), media_id).await?;
    }

    let similar = state
        .db
        .find_similar_media(media_id, SIMILAR_THRESHOLD, limit)
        .map_err(|e| e.to_string())?;

    let ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();
    let media_map = state.db.get_media_by_ids(&ids).map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(similar.len());
    for (id, score) in similar {
        let media = media_map
            .get(&id)
            .ok_or_else(|| format!("media {id} not found"))?;
        results.push(SimilarPhoto {
            media_id: id,
            similarity: score,
            file_name: media.filename.clone(),
            file_path: media.path.clone(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn detect_faces(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<Vec<FaceInfo>, String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let mut ai = state.ai.lock().await;
    ai.ensure_python().await.map_err(|e| e.to_string())?;

    let faces = ai
        .detect_faces_in_image(path)
        .await
        .map_err(|e| e.to_string())?;

    if !faces.is_empty() {
        let inputs: Vec<catchlight_db::FaceDetectionInput> = faces
            .iter()
            .map(|f| catchlight_db::FaceDetectionInput {
                bbox: f.bbox,
                confidence: f.confidence,
                embedding: f.embedding.clone(),
            })
            .collect();
        state
            .db
            .store_face_detections(media_id, &inputs)
            .map_err(|e| e.to_string())?;
    }

    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|r| FaceInfo {
            id: r.id,
            bbox: [r.bbox_x, r.bbox_y, r.bbox_x + r.bbox_w, r.bbox_y + r.bbox_h],
            confidence: r.confidence,
            person_id: r.person_id,
        })
        .collect())
}

#[tauri::command]
pub fn get_faces(state: State<'_, AppState>, media_id: i64) -> Result<Vec<FaceInfo>, String> {
    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|r| FaceInfo {
            id: r.id,
            bbox: [r.bbox_x, r.bbox_y, r.bbox_x + r.bbox_w, r.bbox_y + r.bbox_h],
            confidence: r.confidence,
            person_id: r.person_id,
        })
        .collect())
}

#[tauri::command]
pub fn list_persons(state: State<'_, AppState>) -> Result<Vec<Person>, String> {
    state.db.list_persons().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_person_media(
    state: State<'_, AppState>,
    person_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_person_media(person_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_person(
    state: State<'_, AppState>,
    person_id: i64,
    name: String,
) -> Result<(), String> {
    state
        .db
        .rename_person(person_id, &name)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct PersonClusterInfo {
    pub person_id: i64,
    pub name: Option<String>,
    pub face_count: i64,
}

const DEFAULT_FACE_CLUSTER_THRESHOLD: f32 = 0.45;

#[tauri::command]
pub async fn cluster_faces(
    state: State<'_, AppState>,
    threshold: Option<f32>,
) -> Result<Vec<PersonClusterInfo>, String> {
    let threshold = threshold.unwrap_or(DEFAULT_FACE_CLUSTER_THRESHOLD);
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let faces = db
            .get_unassigned_face_embeddings()
            .map_err(|e| e.to_string())?;
        if faces.is_empty() {
            return Ok(Vec::new());
        }

        db.clear_unnamed_person_clusters()
            .map_err(|e| e.to_string())?;

        let clusters = catchlight_ai::cluster_face_embeddings(&faces, threshold);
        let mut results = Vec::with_capacity(clusters.len());

        for cluster in clusters {
            let person_id = db.create_person(None).map_err(|e| e.to_string())?;
            for face_id in cluster.face_ids {
                db.assign_face_to_person(face_id, person_id)
                    .map_err(|e| e.to_string())?;
            }
            let face_count = db
                .get_person_face_count(person_id)
                .map_err(|e| e.to_string())?;
            results.push(PersonClusterInfo {
                person_id,
                name: None,
                face_count,
            });
        }

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn merge_persons(state: State<'_, AppState>, person_ids: Vec<i64>) -> Result<(), String> {
    if person_ids.len() < 2 {
        return Err("need at least two persons to merge".to_string());
    }
    let target_id = person_ids[0];
    let source_ids: Vec<i64> = person_ids[1..].to_vec();
    state
        .db
        .merge_persons(target_id, &source_ids)
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

    let quality = quality.clamp(1, 100);
    crate::image_edit::export_edited_image(
        std::path::Path::new(&media.path),
        std::path::Path::new(&output_path),
        &params,
        quality,
    )
}

fn remove_media_from_disk(path: &str, hash: Option<&str>) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("permanent delete: file not found on disk: {path}");
        }
        Err(e) => tracing::warn!("permanent delete: failed to remove file {path}: {e}"),
    }

    if let Some(hash) = hash.filter(|h| h.len() >= 4) {
        for size in [
            ThumbnailSize::Micro,
            ThumbnailSize::Small,
            ThumbnailSize::Large,
        ] {
            let thumb = thumb_path(hash, size);
            if thumb.exists()
                && let Err(e) = std::fs::remove_file(&thumb)
            {
                tracing::warn!(
                    "permanent delete: failed to remove thumbnail {}: {e}",
                    thumb.display()
                );
            }
        }
    }
}
