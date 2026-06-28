use crate::state::{AppState, ScanStatus};
use catchlight_core::media::{MediaFile, MediaType, ThumbnailSize};
use catchlight_db::Database;
use catchlight_indexer::{classify_extension, scan_folder as discover_files};
use futures::stream::{self, StreamExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::{error, warn};

fn emit_progress(app: &AppHandle, status: &ScanStatus) {
    let payload = status.snapshot();
    if let Err(e) = app.emit("scan-progress", &payload) {
        warn!("failed to emit scan-progress: {e}");
    }
}

pub fn spawn_scan(app: AppHandle, state: &AppState, folder_id: i64) {
    if state
        .scanning
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        warn!(folder_id, "scan already in progress, skipping");
        return;
    }

    let app = app.clone();
    let db = Arc::clone(&state.db);
    let scan_status = state.scan_status.clone();
    let concurrency = state.scan_concurrency;
    let scanning = Arc::clone(&state.scanning);

    tauri::async_runtime::spawn(async move {
        let result = run_scan(&app, db, scan_status.clone(), concurrency, folder_id).await;
        if let Err(e) = result {
            error!(folder_id, "scan failed: {e}");
            scan_status.set_status("error");
            emit_progress(&app, &scan_status);
        }
        scanning.store(false, Ordering::SeqCst);
    });
}

pub async fn run_scan(
    app: &AppHandle,
    db: Arc<Database>,
    scan_status: ScanStatus,
    concurrency: usize,
    folder_id: i64,
) -> catchlight_core::Result<()> {
    let folder = db
        .get_watched_folder(folder_id)?
        .ok_or_else(|| catchlight_core::Error::Other(format!("folder {folder_id} not found")))?;

    scan_status.reset(folder_id);
    db.set_folder_scan_status(folder_id, "scanning")?;
    emit_progress(app, &scan_status);

    let root = PathBuf::from(&folder.path);
    let files = discover_files(&root).await.map_err(|e| {
        let _ = db.set_folder_scan_status(folder_id, "error");
        e
    })?;
    scan_status.set_total(files.len() as i64);
    scan_status.set_status("scanning");
    emit_progress(app, &scan_status);

    stream::iter(files.into_iter().map(|path| {
        let db = Arc::clone(&db);
        let scan_status = scan_status.clone();
        let app = app.clone();
        async move {
            if let Err(e) = process_file(&db, folder_id, &path).await {
                warn!(path = %path.display(), "failed to process file: {e}");
            }
            scan_status.increment_scanned();
            emit_progress(&app, &scan_status);
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<()>()
    .await;

    db.update_last_scan_at(folder_id).map_err(|e| {
        let _ = db.set_folder_scan_status(folder_id, "error");
        e
    })?;
    db.set_folder_scan_status(folder_id, "idle")?;
    scan_status.set_status("complete");
    emit_progress(app, &scan_status);

    Ok(())
}

async fn process_file(
    db: &Database,
    folder_id: i64,
    path: &Path,
) -> catchlight_core::Result<()> {
    let path = path.to_path_buf();
    let media_type = classify_extension(&path);

    let fs_meta = tokio::task::spawn_blocking({
        let path = path.clone();
        move || std::fs::metadata(&path)
    })
    .await
    .map_err(|e| catchlight_core::Error::Other(e.to_string()))??;

    let modified_at = fs_meta
        .modified()
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc())
        .unwrap_or_default();

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let meta = if matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    ) {
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || catchlight_metadata::extract(&path)
        })
        .await
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))??
    } else {
        catchlight_metadata::PhotoMetadata::default()
    };

    let blake3_hash = tokio::task::spawn_blocking({
        let path = path.clone();
        move || catchlight_dedup::file_hash(&path)
    })
    .await
    .map_err(|e| catchlight_core::Error::Other(e.to_string()))??;

    let dhash = if matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    ) {
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || catchlight_dedup::dhash(&path)
        })
        .await
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?
        .ok()
    } else {
        None
    };

    if matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    ) {
        let hash = blake3_hash.clone();
        let thumb_path = path.clone();
        tokio::task::spawn_blocking(move || {
            let _ = catchlight_thumbnail::generate(&thumb_path, &hash, ThumbnailSize::Micro);
            let _ = catchlight_thumbnail::generate(&thumb_path, &hash, ThumbnailSize::Small);
        })
        .await
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;
    }

    if matches!(media_type, MediaType::Video) {
        let hash = blake3_hash.clone();
        let vid_path = path.clone();
        let temp_frame = catchlight_thumbnail::thumb_path(&hash, ThumbnailSize::Small)
            .with_extension("frame.jpg");

        if catchlight_video::find_ffmpeg().is_some() {
            if let Some(parent) = temp_frame.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match catchlight_video::extract_frame(&vid_path, &temp_frame, 1.0).await {
                Ok(()) if temp_frame.exists() => {
                    let frame = temp_frame.clone();
                    let hash_clone = hash.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        let _ = catchlight_thumbnail::generate(&frame, &hash_clone, ThumbnailSize::Micro);
                        let _ = catchlight_thumbnail::generate(&frame, &hash_clone, ThumbnailSize::Small);
                        let _ = std::fs::remove_file(&frame);
                    })
                    .await;
                }
                _ => {
                    tracing::debug!(path = %vid_path.display(), "video frame extraction failed");
                }
            }
        }
    }

    let media_type = if matches!(media_type, MediaType::Photo) {
        if let (Some(w), Some(h)) = (meta.width, meta.height) {
            if catchlight_ai::detect_screenshot(&path, w, h).unwrap_or(false) {
                MediaType::Screenshot
            } else {
                media_type
            }
        } else {
            media_type
        }
    } else {
        media_type
    };

    let micro_blob = if matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    ) {
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || catchlight_thumbnail::generate_micro_blob(&path)
        })
        .await
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?
        .ok()
    } else if matches!(media_type, MediaType::Video) {
        let hash = blake3_hash.clone();
        tokio::task::spawn_blocking(move || {
            let small_thumb = catchlight_thumbnail::thumb_path(&hash, ThumbnailSize::Small);
            if small_thumb.exists() {
                catchlight_thumbnail::generate_micro_blob(&small_thumb).ok()
            } else {
                None
            }
        })
        .await
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?
    } else {
        None
    };

    let media = MediaFile {
        id: 0,
        path: path.to_string_lossy().to_string(),
        filename,
        media_type,
        size_bytes: fs_meta.len(),
        width: meta.width,
        height: meta.height,
        created_at: meta.date_taken,
        modified_at,
        blake3_hash: Some(blake3_hash),
        dhash,
        latitude: meta.latitude,
        longitude: meta.longitude,
    };

    let media_id = db.upsert_media(folder_id, &media)?;
    if let Some(blob) = micro_blob {
        let _ = db.set_micro_thumb(media_id, &blob);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_module_compiles() {
        let _: fn(AppHandle, &AppState, i64) = spawn_scan;
    }
}
