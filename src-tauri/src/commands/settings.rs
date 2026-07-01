use crate::scan;
use crate::state::AppState;
use crate::watcher;
use lightframe_core::config::AppConfig;
use lightframe_core::media::ThumbnailSize;
use lightframe_db::WatchedFolder;
use lightframe_thumbnail::thumb_path;
use serde::Serialize;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_log_directory() -> String {
    crate::logging::log_directory()
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn get_log_files() -> Vec<crate::logging::LogFileInfo> {
    crate::logging::list_log_files()
}

#[tauri::command]
pub fn cleanup_logs() -> Result<(), String> {
    let dir = crate::logging::log_directory();
    crate::logging::cleanup_logs(&dir);
    Ok(())
}

#[tauri::command]
pub fn get_log_config() -> lightframe_core::config::LogConfig {
    crate::logging::get_log_config()
}

#[tauri::command]
pub fn set_log_config(config: lightframe_core::config::LogConfig) -> Result<(), String> {
    crate::logging::set_log_config(config.clone());
    lightframe_core::config::update_log_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Clone, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let result = tokio::task::spawn_blocking(move || {
        let resp = ureq::get("https://api.github.com/repos/halft0n/LightFrame/releases/latest")
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "LightFrame")
            .call()
            .map_err(|e| format!("network error: {e}"))?;
        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("invalid response: {e}"))?;
        let tag = body["tag_name"]
            .as_str()
            .ok_or("missing tag_name")?
            .trim_start_matches('v')
            .to_string();
        let url = body["html_url"]
            .as_str()
            .unwrap_or("https://github.com/halft0n/LightFrame/releases")
            .to_string();
        Ok::<(String, String), String>((tag, url))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    let (latest, url) = result;
    let update_available = latest != current;
    Ok(UpdateCheckResult {
        current_version: current,
        latest_version: latest,
        update_available,
        release_url: url,
    })
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
    let canonical =
        std::fs::canonicalize(&path).map_err(|e| format!("cannot resolve path: {e}"))?;
    #[cfg(windows)]
    let canonical = crate::original_protocol::strip_extended_prefix(canonical);
    if !canonical.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let path_str = canonical
        .to_str()
        .ok_or_else(|| "path contains invalid unicode".to_string())?;

    let folder = state
        .db
        .add_watched_folder(path_str)
        .map_err(|e| e.to_string())?;

    scan::spawn_scan(app.clone(), &state, folder.id);
    if let Err(e) = watcher::start(&app, &state) {
        tracing::warn!(folder_id = folder.id, "failed to start file watcher: {e}");
    }
    Ok(folder)
}

#[tauri::command]
pub fn remove_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let hashes = state
        .db
        .list_media_hashes_by_folder(id)
        .map_err(|e| e.to_string())?;
    state
        .db
        .remove_watched_folder(id)
        .map_err(|e| e.to_string())?;
    for hash in hashes {
        if hash.len() >= 4 {
            for size in [
                ThumbnailSize::Micro,
                ThumbnailSize::Small,
                ThumbnailSize::Large,
            ] {
                let thumb = thumb_path(&hash, size);
                if thumb.exists()
                    && let Err(e) = std::fs::remove_file(&thumb)
                {
                    tracing::warn!(
                        "remove folder: failed to remove thumbnail {}: {e}",
                        thumb.display()
                    );
                }
            }
        }
    }
    watcher::start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_watched_folders(state: State<'_, AppState>) -> Result<Vec<WatchedFolder>, String> {
    state.db.list_watched_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_database(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Wait for any active scan to finish before wiping data
    let mut attempts = 0;
    while state.scan_queue.is_running() && attempts < 60 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        attempts += 1;
    }
    if state.scan_queue.is_running() {
        return Err(
            "Cannot reset: a scan is still in progress. Please wait for it to finish.".to_string(),
        );
    }

    state.db.reset_all_media_data().map_err(|e| e.to_string())?;

    // Delete thumbnail cache directory
    let thumb_dir = lightframe_core::config::thumb_cache_dir();
    if thumb_dir.exists() {
        let _ = std::fs::remove_dir_all(&thumb_dir);
        let _ = std::fs::create_dir_all(&thumb_dir);
    }

    // Re-scan all watched folders
    let folders = state.db.list_watched_folders().map_err(|e| e.to_string())?;
    for folder in folders {
        scan::spawn_scan(app.clone(), &state, folder.id);
    }

    Ok(())
}

#[derive(Serialize)]
pub struct MemoryBudgetInfo {
    pub total_mb: u64,
    pub available_mb: u64,
    pub micro_cap: usize,
    pub standard_cap: usize,
    pub micro_len: usize,
    pub standard_len: usize,
    pub under_pressure: bool,
}

#[tauri::command]
pub fn get_memory_budget(state: State<'_, AppState>) -> MemoryBudgetInfo {
    let (total_mb, available_mb) = crate::memory_budget::get_system_memory().unwrap_or((0, 0));
    let (micro_len, standard_len) = state.thumb_cache.len();
    let (micro_cap, standard_cap) = state.thumb_cache.capacity();
    let under_pressure = crate::memory_budget::is_under_pressure(available_mb, total_mb);
    MemoryBudgetInfo {
        total_mb,
        available_mb,
        micro_cap,
        standard_cap,
        micro_len,
        standard_len,
        under_pressure,
    }
}

#[tauri::command]
pub async fn rebuild_cache(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // Wait for any active scan to finish
    let mut attempts = 0;
    while state.scan_queue.is_running() && attempts < 60 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        attempts += 1;
    }
    if state.scan_queue.is_running() {
        return Err(
            "Cannot rebuild: a scan is still in progress. Please wait for it to finish."
                .to_string(),
        );
    }

    state.db.rebuild_cache().map_err(|e| e.to_string())?;

    // Delete thumbnail cache directory (stale thumbs)
    let thumb_dir = lightframe_core::config::thumb_cache_dir();
    if thumb_dir.exists() {
        let _ = std::fs::remove_dir_all(&thumb_dir);
        let _ = std::fs::create_dir_all(&thumb_dir);
    }

    // Trigger rescan for all watched folders
    let folders = state.db.list_watched_folders().map_err(|e| e.to_string())?;
    let folder_count = folders.len();
    for folder in &folders {
        scan::spawn_scan(app.clone(), &state, folder.id);
    }

    // Spawn background task to restore choices after all scans finish
    let db = std::sync::Arc::clone(&state.db);
    let scan_queue_running = state.scan_queue.running_flag();
    tokio::spawn(async move {
        // Wait for scans to complete (max 30 minutes)
        let mut wait_ms = 0u64;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            wait_ms += 2000;
            if !scan_queue_running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if wait_ms > 1_800_000 {
                tracing::error!("rebuild_cache: timed out waiting for rescan to complete");
                break;
            }
        }
        // Restore user choices
        match db.restore_rebuild_choices() {
            Ok((fav, edits, albums, faces)) => {
                tracing::info!(
                    "rebuild_cache: restored {fav} favorites, {edits} edit params, {albums} album items, {faces} manual faces"
                );
            }
            Err(e) => {
                tracing::error!("rebuild_cache: failed to restore choices: {e}");
            }
        }
    });

    Ok(format!(
        "Rebuild started. {} folder(s) queued for rescan. User data will be restored after scan completes.",
        folder_count
    ))
}

#[tauri::command]
pub fn get_pinned_items(
    state: State<'_, AppState>,
) -> Result<Vec<lightframe_db::PinnedItem>, String> {
    state.db.get_pinned_items().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_item(state: State<'_, AppState>, item_type: String, item_id: i64) -> Result<(), String> {
    state
        .db
        .pin_item(&item_type, item_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unpin_item(
    state: State<'_, AppState>,
    item_type: String,
    item_id: i64,
) -> Result<(), String> {
    state
        .db
        .unpin_item(&item_type, item_id)
        .map_err(|e| e.to_string())
}
