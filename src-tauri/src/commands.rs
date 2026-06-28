use crate::scan;
use crate::state::{AppState, ScanProgress};
use crate::watcher;
use catchlight_core::config::AppConfig;
use catchlight_core::media::MediaFile;
use catchlight_ai::AiStatus;
use catchlight_db::{
    Album, DuplicateGroupDetail, LocationGroup, LocationStats, MediaNeighbors, Memory, Person,
    SmartAlbum, SmartAlbumRule, TimelineGroup, WatchedFolder,
};
use serde::Serialize;
use tauri::{AppHandle, State};

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
    state
        .db
        .list_watched_folders()
        .map_err(|e| e.to_string())
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
pub fn get_media_count(state: State<'_, AppState>) -> Result<i64, String> {
    state.db.get_media_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_id(state: State<'_, AppState>, id: i64) -> Result<Option<MediaFile>, String> {
    state.db.get_media_by_id(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_folder(app: AppHandle, state: State<'_, AppState>, folder_id: i64) -> Result<(), String> {
    state
        .db
        .get_watched_folder(folder_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("folder {folder_id} not found"))?;

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
pub fn get_media_neighbors(
    state: State<'_, AppState>,
    id: i64,
) -> Result<MediaNeighbors, String> {
    state
        .db
        .get_media_neighbors(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_dedup_scan(state: State<'_, AppState>) -> Result<DedupScanResult, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.clear_duplicate_groups()
            .map_err(|e| e.to_string())?;

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
    state
        .db
        .get_location_groups()
        .map_err(|e| e.to_string())
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
    state
        .db
        .get_location_stats()
        .map_err(|e| e.to_string())
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
pub fn delete_media(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .set_deleted(media_id, true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_deleted_media(state: State<'_, AppState>) -> Result<Vec<MediaFile>, String> {
    state
        .db
        .list_deleted_media()
        .map_err(|e| e.to_string())
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
    state
        .db
        .permanently_delete_media(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_delete_media(state: State<'_, AppState>, media_ids: Vec<i64>) -> Result<usize, String> {
    state
        .db
        .batch_set_deleted(&media_ids, true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_add_to_album(
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
pub fn batch_toggle_favorite(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
    favorite: bool,
) -> Result<usize, String> {
    state
        .db
        .batch_set_favorite(&media_ids, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_restore_media(state: State<'_, AppState>, media_ids: Vec<i64>) -> Result<usize, String> {
    state
        .db
        .batch_set_deleted(&media_ids, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_permanent_delete(state: State<'_, AppState>, media_ids: Vec<i64>) -> Result<usize, String> {
    state
        .db
        .batch_permanent_delete(&media_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
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
    state
        .db
        .list_smart_albums()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_smart_album(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state
        .db
        .delete_smart_album(id)
        .map_err(|e| e.to_string())
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

#[tauri::command]
pub fn save_edit(
    state: State<'_, AppState>,
    media_id: i64,
    params: String,
) -> Result<(), String> {
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
