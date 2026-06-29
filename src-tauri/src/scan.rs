use crate::memory;
use crate::state::{AppState, ScanStatus};
use futures::stream::{self, StreamExt};
use lightframe_core::media::{MediaFile, MediaType, ThumbnailSize};
use lightframe_db::Database;
use lightframe_indexer::{classify_extension, scan_folder as discover_files};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter};
use tracing::{Instrument, error, info, warn};

fn emit_progress(app: &AppHandle, status: &ScanStatus) {
    let payload = status.snapshot();
    if let Err(e) = app.emit("scan-progress", &payload) {
        warn!("failed to emit scan-progress: {e}");
    }
}

/// Start scanning watched folders.
///
/// Only one scan runs at a time. If a scan is already in progress,
/// new scan requests are silently ignored. This prevents resource
/// contention on disk I/O-heavy operations.
///
/// TODO: Implement per-folder scan queue for v0.2.0
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
) -> lightframe_core::Result<()> {
    let folder = db
        .get_watched_folder(folder_id)?
        .ok_or_else(|| lightframe_core::Error::Other(format!("folder {folder_id} not found")))?;

    scan_status.reset(folder_id);
    db.set_folder_scan_status(folder_id, "scanning")?;
    emit_progress(app, &scan_status);
    memory::log_memory("scan_start");

    let root = PathBuf::from(&folder.path);
    let files = async {
        discover_files(&root).await.inspect_err(|_| {
            let _ = db.set_folder_scan_status(folder_id, "error");
        })
    }
    .instrument(tracing::info_span!("file_discovery", folder_id))
    .await?;
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
            let scanned = scan_status.increment_scanned();
            if scanned % 100 == 0 {
                memory::log_memory("scan_progress");
            }
            emit_progress(&app, &scan_status);
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<()>()
    .await;

    db.update_last_scan_at(folder_id).inspect_err(|_| {
        let _ = db.set_folder_scan_status(folder_id, "error");
    })?;
    db.set_folder_scan_status(folder_id, "idle")?;
    scan_status.set_status("complete");
    emit_progress(app, &scan_status);
    memory::log_memory("scan_end");
    info!(
        folder_id,
        total = scan_status.snapshot().scanned,
        "scan complete"
    );

    Ok(())
}

async fn process_file(db: &Database, folder_id: i64, path: &Path) -> lightframe_core::Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let span = tracing::info_span!("process_file", path = %path_str);
    process_file_inner(db, folder_id, path)
        .instrument(span)
        .await
}

async fn process_file_inner(
    db: &Database,
    folder_id: i64,
    path: &Path,
) -> lightframe_core::Result<()> {
    let path = path.to_path_buf();
    let media_type = classify_extension(&path);

    let fs_meta = tokio::task::spawn_blocking({
        let path = path.clone();
        move || std::fs::metadata(&path)
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))??;

    let modified_at = fs_meta
        .modified()
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc())
        .unwrap_or_default();

    let path_str = path.to_string_lossy();
    if let Ok(Some(existing)) = db.get_media_by_path(&path_str)
        && existing.size_bytes == fs_meta.len()
        && existing.modified_at == modified_at
    {
        tracing::debug!("skipping unchanged file: {path_str}");
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let meta = if matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    ) {
        async {
            tokio::task::spawn_blocking({
                let path = path.clone();
                move || lightframe_metadata::extract(&path)
            })
            .await
            .map_err(|e| lightframe_core::Error::Other(e.to_string()))?
        }
        .instrument(tracing::info_span!("metadata_extraction"))
        .await?
    } else {
        lightframe_metadata::PhotoMetadata::default()
    };

    let blake3_hash = async {
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || lightframe_dedup::file_hash(&path)
        })
        .await
        .map_err(|e| lightframe_core::Error::Other(e.to_string()))?
    }
    .instrument(tracing::info_span!("blake3_hashing"))
    .await?;

    let is_image = matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    );

    let (dhash, phash, image_micro_blob) = if is_image {
        async {
            tokio::task::spawn_blocking({
                let path = path.clone();
                let hash = blake3_hash.clone();
                move || -> (Option<u64>, Option<u64>, Option<Vec<u8>>) {
                    let decoded = match lightframe_core::decode::decode_image(&path) {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::warn!(path = %path.display(), "decode failed: {e}");
                            return (None, None, None);
                        }
                    };

                    let dhash = Some(lightframe_dedup::dhash_from_decoded(&decoded));
                    let phash = Some(lightframe_dedup::phash_from_decoded(&decoded));

                    let _ = lightframe_thumbnail::generate_from_decoded(
                        &decoded,
                        &hash,
                        ThumbnailSize::Micro,
                    );
                    let _ = lightframe_thumbnail::generate_from_decoded(
                        &decoded,
                        &hash,
                        ThumbnailSize::Small,
                    );

                    let micro = lightframe_thumbnail::micro_blob_from_decoded(&decoded).ok();

                    (dhash, phash, micro)
                }
            })
            .await
            .map_err(|e| lightframe_core::Error::Other(e.to_string()))
        }
        .instrument(tracing::info_span!("thumbnail_generation"))
        .await?
    } else {
        (None, None, None)
    };

    if matches!(media_type, MediaType::Video) {
        let hash = blake3_hash.clone();
        let vid_path = path.clone();
        let temp_frame = lightframe_thumbnail::thumb_path(&hash, ThumbnailSize::Small)
            .with_extension("frame.jpg");

        if lightframe_video::find_ffmpeg().is_some() {
            if let Some(parent) = temp_frame.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match lightframe_video::extract_frame(&vid_path, &temp_frame, 1.0).await {
                Ok(()) if temp_frame.exists() => {
                    let frame = temp_frame.clone();
                    let hash_clone = hash.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        let _ = lightframe_thumbnail::generate(
                            &frame,
                            &hash_clone,
                            ThumbnailSize::Micro,
                        );
                        let _ = lightframe_thumbnail::generate(
                            &frame,
                            &hash_clone,
                            ThumbnailSize::Small,
                        );
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
            if lightframe_ai::detect_screenshot(&path, w, h)
                .map(|score| score.is_likely_screenshot())
                .unwrap_or(false)
            {
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

    let micro_blob = if matches!(media_type, MediaType::Video) {
        let hash = blake3_hash.clone();
        tokio::task::spawn_blocking(move || {
            let small_thumb = lightframe_thumbnail::thumb_path(&hash, ThumbnailSize::Small);
            if small_thumb.exists() {
                lightframe_thumbnail::generate_micro_blob(&small_thumb).ok()
            } else {
                None
            }
        })
        .await
        .map_err(|e| lightframe_core::Error::Other(e.to_string()))?
    } else {
        image_micro_blob
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
        phash,
        latitude: meta.latitude,
        longitude: meta.longitude,
    };

    let media_id = {
        let _span = tracing::info_span!("db_upsert").entered();
        db.upsert_media(folder_id, &media)?
    };
    if let Some(blob) = micro_blob {
        let _ = db.set_micro_thumb(media_id, &blob);
    }

    if matches!(media_type, MediaType::Screenshot)
        && let Ok(screenshot_type) = lightframe_ai::classify_screenshot(&path)
    {
        let _ = db.set_screenshot_type(media_id, screenshot_type.label());
    }

    if let (Some(lat), Some(lon)) = (media.latitude, media.longitude)
        && let Some(loc) = lightframe_geo::reverse_geocode(lat, lon)
    {
        let country = loc.country.as_deref().unwrap_or("");
        let city = loc.city.as_deref().unwrap_or("");
        if !country.is_empty() {
            let _ = db.update_media_location(media_id, city, country);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn scan_module_compiles() {
        let _: fn(AppHandle, &AppState, i64) = spawn_scan;
    }

    #[tokio::test]
    async fn discover_valid_images_in_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("photo1.jpg"), b"fake jpg").unwrap();
        fs::write(dir.path().join("photo2.png"), b"fake png").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn discover_nonexistent_directory_returns_empty() {
        let files = discover_files(Path::new("/nonexistent/lightframe/scan-test"))
            .await
            .unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn discover_regular_file_path_yields_that_file_if_media() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("solo.jpg");
        fs::write(&file, b"solo").unwrap();

        let files = discover_files(&file).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file);
    }

    #[tokio::test]
    async fn discover_regular_file_non_media_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("readme.txt");
        fs::write(&file, b"text").unwrap();

        let files = discover_files(&file).await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn discover_empty_directory_returns_no_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = discover_files(dir.path()).await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn discover_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(dir.path().join("root.jpg"), b"root").unwrap();
        fs::write(nested.join("deep.png"), b"deep").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn discover_files_without_extension_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("noext"), b"unknown").unwrap();
        fs::write(dir.path().join("valid.jpg"), b"jpg").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].extension().and_then(|e| e.to_str()) == Some("jpg"));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn discover_symlink_to_media_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.jpg");
        fs::write(&target, b"real").unwrap();
        let link = dir.path().join("link.jpg");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert!(
            files.iter().any(|p| p.file_name().unwrap() == "real.jpg"),
            "real file should be discovered"
        );
        assert!(
            !files.iter().any(|p| p.file_name().unwrap() == "link.jpg"),
            "symlinks are not indexed (follow_links=false)"
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn discover_unreadable_subdirectory_is_skipped_gracefully() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("visible.jpg"), b"ok").unwrap();

        let restricted = dir.path().join("restricted");
        fs::create_dir(&restricted).unwrap();
        fs::write(restricted.join("hidden.jpg"), b"hidden").unwrap();
        let mut perms = fs::metadata(&restricted).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&restricted, perms).unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert!(
            files
                .iter()
                .any(|p| p.file_name().unwrap() == "visible.jpg"),
            "should still discover files in readable directories"
        );
        assert!(
            !files.iter().any(|p| p.file_name().unwrap() == "hidden.jpg"),
            "unreadable subdirectory should be skipped"
        );

        let mut restore = fs::metadata(&restricted).unwrap().permissions();
        restore.set_mode(0o755);
        let _ = fs::set_permissions(&restricted, restore);
    }
}
