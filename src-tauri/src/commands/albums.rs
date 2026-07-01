use crate::state::AppState;
use lightframe_core::media::MediaFile;
use lightframe_db::{Album, SmartAlbum, SmartAlbumRule};
use tauri::State;

use super::check_batch_size;

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
    check_batch_size(&media_ids)?;
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
