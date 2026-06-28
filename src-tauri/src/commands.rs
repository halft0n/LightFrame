use crate::scan;
use crate::state::{AppState, ScanProgress};
use catchlight_core::config::AppConfig;
use catchlight_core::media::MediaFile;
use catchlight_db::{MediaNeighbors, TimelineGroup, WatchedFolder};
use tauri::{AppHandle, State};

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

    scan::spawn_scan(app, &state, folder.id);
    Ok(folder)
}

#[tauri::command]
pub fn remove_watched_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state
        .db
        .remove_watched_folder(id)
        .map_err(|e| e.to_string())
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
