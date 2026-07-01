use crate::scan;
use crate::state::{AppState, ScanProgress};
use crate::watcher;
use tauri::{AppHandle, State};

use super::db_err;

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
pub async fn regenerate_thumbnails(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::thumb_regen::ThumbnailRegenResult, String> {
    crate::thumb_regen::regenerate_all_thumbnails(app, &state).await
}

#[tauri::command]
pub async fn regenerate_thumbnail_single(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<bool, String> {
    let db = state.db.clone();
    let regenerated = tokio::task::spawn_blocking(move || {
        crate::thumb_regen::regenerate_thumbnails_for_media_db(&db, media_id)
    })
    .await
    .map_err(|e| e.to_string())??;

    if regenerated {
        state.thumb_cache.invalidate_media(media_id);
        crate::face_protocol::invalidate_face_cache_for_media(&state, media_id);
    }
    Ok(regenerated)
}
