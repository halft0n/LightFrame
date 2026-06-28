use crate::scan;
use crate::state::{AppState, ScanProgress};
use catchlight_core::media::MediaFile;
use catchlight_db::{MediaNeighbors, TimelineGroup, WatchedFolder};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome to CatchLight, {}!", name)
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn add_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<i64, String> {
    let folder_path = std::path::Path::new(&path);
    if !folder_path.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let folder_id = state
        .db
        .add_watched_folder(&path)
        .map_err(|e| e.to_string())?;

    scan::spawn_scan(app, &state, folder_id);
    Ok(folder_id)
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
pub fn get_timeline_groups(state: State<'_, AppState>) -> Result<Vec<TimelineGroup>, String> {
    state
        .db
        .get_timeline_groups(5000)
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
