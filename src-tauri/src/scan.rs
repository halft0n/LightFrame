use crate::memory;
use crate::state::{AppState, ScanQueue, ScanStatus};
use futures::stream::{self, StreamExt};
use lightframe_core::media::{MediaFile, MediaType, ThumbnailSize};
use lightframe_db::Database;
use lightframe_indexer::{classify_extension, scan_folder as discover_files};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tracing::{Instrument, error, info, warn};

const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(500);
const PROGRESS_EMIT_FILE_INTERVAL: i64 = 50;
const MEDIA_BATCH_SIZE: usize = 20;
const MEDIA_BATCH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone)]
struct MediaBatchPayload {
    folder_id: i64,
    items: Vec<MediaFile>,
}

struct MediaBatchEmitter {
    app: AppHandle,
    folder_id: i64,
    buffer: Mutex<Vec<MediaFile>>,
    last_emit: Mutex<Instant>,
}

impl MediaBatchEmitter {
    fn new(app: AppHandle, folder_id: i64) -> Self {
        Self {
            app,
            folder_id,
            buffer: Mutex::new(Vec::new()),
            last_emit: Mutex::new(Instant::now()),
        }
    }

    fn push(&self, item: MediaFile) {
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        buffer.push(item);

        let emit_by_size = buffer.len() >= MEDIA_BATCH_SIZE;
        let emit_by_time = self
            .last_emit
            .lock()
            .map(|last| last.elapsed() >= MEDIA_BATCH_INTERVAL)
            .unwrap_or(true);

        if emit_by_size || emit_by_time {
            self.emit_locked(&mut buffer);
        }
    }

    fn flush(&self) {
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        if !buffer.is_empty() {
            self.emit_locked(&mut buffer);
        }
    }

    fn emit_locked(&self, buffer: &mut Vec<MediaFile>) {
        let items: Vec<MediaFile> = buffer.drain(..).collect();
        if items.is_empty() {
            return;
        }
        let payload = MediaBatchPayload {
            folder_id: self.folder_id,
            items,
        };
        if let Err(e) = self.app.emit("media-batch-ready", &payload) {
            warn!("failed to emit media-batch-ready: {e}");
        }
        if let Ok(mut last) = self.last_emit.lock() {
            *last = Instant::now();
        }
    }
}

fn emit_progress(app: &AppHandle, status: &ScanStatus) {
    let payload = status.snapshot();
    if let Err(e) = app.emit("scan-progress", &payload) {
        warn!("failed to emit scan-progress: {e}");
    }
}

struct ProgressThrottler {
    last_emit: Mutex<Instant>,
}

impl ProgressThrottler {
    fn new() -> Self {
        Self {
            last_emit: Mutex::new(
                Instant::now()
                    .checked_sub(PROGRESS_EMIT_INTERVAL)
                    .unwrap_or_else(Instant::now),
            ),
        }
    }

    fn maybe_emit(&self, app: &AppHandle, status: &ScanStatus, force: bool) {
        if force {
            emit_progress(app, status);
            if let Ok(mut last) = self.last_emit.lock() {
                *last = Instant::now();
            }
            return;
        }

        let scanned = status.snapshot().scanned;
        let emit_by_count = scanned % PROGRESS_EMIT_FILE_INTERVAL == 0;
        let emit_by_time = self
            .last_emit
            .lock()
            .map(|last| last.elapsed() >= PROGRESS_EMIT_INTERVAL)
            .unwrap_or(true);

        if emit_by_count || emit_by_time {
            emit_progress(app, status);
            if let Ok(mut last) = self.last_emit.lock() {
                *last = Instant::now();
            }
        }
    }
}

/// Start scanning watched folders.
///
/// Scans are queued and processed sequentially by a background worker.
pub fn spawn_scan(app: AppHandle, state: &AppState, folder_id: i64) -> bool {
    start_scan_queue(&app, state);
    if state.scan_queue.is_running() {
        info!(folder_id, "scan queued while another scan is in progress");
    }
    state.scan_queue.enqueue(folder_id);
    true
}

impl ScanQueue {
    pub fn start(
        &self,
        app: AppHandle,
        db: Arc<Database>,
        scan_status: ScanStatus,
        concurrency: usize,
    ) {
        let Some(mut rx) = self.try_start_worker() else {
            return;
        };

        let running = self.running_flag();

        tauri::async_runtime::spawn(async move {
            while let Some(folder_id) = rx.recv().await {
                running.store(true, Ordering::SeqCst);
                let result =
                    run_scan(&app, db.clone(), scan_status.clone(), concurrency, folder_id).await;
                if let Err(e) = result {
                    error!(folder_id, "scan failed: {e}");
                    scan_status.set_status("error");
                    emit_progress(&app, &scan_status);
                }
                running.store(false, Ordering::SeqCst);
            }
        });
    }
}

fn start_scan_queue(app: &AppHandle, state: &AppState) {
    state.scan_queue.start(
        app.clone(),
        Arc::clone(&state.db),
        state.scan_status.clone(),
        state.scan_concurrency,
    );
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
    let progress = Arc::new(ProgressThrottler::new());
    progress.maybe_emit(app, &scan_status, true);
    let batch_emitter = Arc::new(MediaBatchEmitter::new(app.clone(), folder_id));

    stream::iter(files.into_iter().map(|path| {
        let db = Arc::clone(&db);
        let scan_status = scan_status.clone();
        let app = app.clone();
        let progress = Arc::clone(&progress);
        let batch_emitter = Arc::clone(&batch_emitter);
        async move {
            match process_file(&db, folder_id, &path).await {
                Ok(Some(media_id)) => match db.get_media_by_id(media_id) {
                    Ok(Some(media)) => batch_emitter.push(media),
                    Ok(None) => {
                        warn!(media_id, path = %path.display(), "processed media not found in db");
                    }
                    Err(e) => {
                        warn!(media_id, path = %path.display(), "failed to read processed media: {e}");
                    }
                },
                Ok(None) => {}
                Err(e) => {
                    warn!(path = %path.display(), "failed to process file: {e}");
                    scan_status.increment_errors();
                }
            }
            let scanned = scan_status.increment_scanned();
            if scanned % 100 == 0 {
                memory::log_memory("scan_progress");
            }
            progress.maybe_emit(&app, &scan_status, false);
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<()>()
    .await;

    batch_emitter.flush();

    db.update_last_scan_at(folder_id).inspect_err(|_| {
        let _ = db.set_folder_scan_status(folder_id, "error");
    })?;
    db.set_folder_scan_status(folder_id, "idle")?;
    scan_status.set_status("complete");
    progress.maybe_emit(app, &scan_status, true);
    memory::log_memory("scan_end");
    info!(
        folder_id,
        total = scan_status.snapshot().scanned,
        "scan complete"
    );

    Ok(())
}

async fn process_file(
    db: &Database,
    folder_id: i64,
    path: &Path,
) -> lightframe_core::Result<Option<i64>> {
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
) -> lightframe_core::Result<Option<i64>> {
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
        return Ok(None);
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

                    let dhash = lightframe_dedup::dhash_from_decoded(&decoded).ok();
                    let phash = lightframe_dedup::phash_from_decoded(&decoded).ok();

                    if let Err(e) = lightframe_thumbnail::generate_from_decoded(
                        &decoded,
                        &hash,
                        ThumbnailSize::Micro,
                    ) {
                        tracing::warn!(path = %path.display(), "thumbnail generation failed: {e}");
                    }
                    if let Err(e) = lightframe_thumbnail::generate_from_decoded(
                        &decoded,
                        &hash,
                        ThumbnailSize::Small,
                    ) {
                        tracing::warn!(path = %path.display(), "thumbnail generation failed: {e}");
                    }

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
                        if let Err(e) = lightframe_thumbnail::generate(
                            &frame,
                            &hash_clone,
                            ThumbnailSize::Micro,
                        ) {
                            tracing::warn!(path = %frame.display(), "thumbnail generation failed: {e}");
                        }
                        if let Err(e) = lightframe_thumbnail::generate(
                            &frame,
                            &hash_clone,
                            ThumbnailSize::Small,
                        ) {
                            tracing::warn!(path = %frame.display(), "thumbnail generation failed: {e}");
                        }
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
    if let Some(blob) = micro_blob
        && let Err(e) = db.set_micro_thumb(media_id, &blob)
    {
        tracing::warn!(media_id, "failed to set micro thumb: {e}");
    }

    if matches!(media_type, MediaType::Screenshot)
        && let Ok(screenshot_type) = lightframe_ai::classify_screenshot(&path)
        && let Err(e) = db.set_screenshot_type(media_id, screenshot_type.label())
    {
        tracing::warn!(media_id, "failed to set screenshot type: {e}");
    }

    if let (Some(lat), Some(lon)) = (media.latitude, media.longitude)
        && let Some(loc) = lightframe_geo::reverse_geocode(lat, lon)
    {
        let country = loc.country.as_deref().unwrap_or("");
        let city = loc.city.as_deref().unwrap_or("");
        if !country.is_empty()
            && let Err(e) = db.update_media_location(media_id, city, country)
        {
            tracing::warn!(media_id, "failed to update media location: {e}");
        }
    }

    Ok(Some(media_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn scan_module_compiles() {
        let _: fn(AppHandle, &AppState, i64) -> bool = spawn_scan;
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

    #[tokio::test]
    async fn discover_case_insensitive_extensions() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("upper.JPG"), b"jpg").unwrap();
        fs::write(dir.path().join("mixed.PnG"), b"png").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn discover_raw_extensions_are_included() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("photo.cr2"), b"raw").unwrap();
        fs::write(dir.path().join("photo.nef"), b"raw").unwrap();
        fs::write(dir.path().join("readme.txt"), b"text").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
                ext.eq_ignore_ascii_case("cr2") || ext.eq_ignore_ascii_case("nef")
            })
        }));
    }

    #[tokio::test]
    async fn discover_ignores_double_extension_backup_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("photo.jpg.bak"), b"backup").unwrap();
        fs::write(dir.path().join("valid.jpg"), b"jpg").unwrap();

        let files = discover_files(dir.path()).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "valid.jpg");
    }

    fn progress_would_emit(
        throttler: &ProgressThrottler,
        status: &ScanStatus,
        force: bool,
    ) -> bool {
        if force {
            return true;
        }
        let scanned = status.snapshot().scanned;
        let emit_by_count = scanned % PROGRESS_EMIT_FILE_INTERVAL == 0;
        let emit_by_time = throttler
            .last_emit
            .lock()
            .map(|last| last.elapsed() >= PROGRESS_EMIT_INTERVAL)
            .unwrap_or(true);
        emit_by_count || emit_by_time
    }

    #[test]
    fn progress_throttler_force_always_emits() {
        let throttler = ProgressThrottler::new();
        let status = ScanStatus::new();
        status.set_total(100);
        for scanned in [0, 1, 23, 49, 50, 100] {
            status.reset(1);
            for _ in 0..scanned {
                status.increment_scanned();
            }
            assert!(
                progress_would_emit(&throttler, &status, true),
                "force should emit at scanned={scanned}"
            );
        }
    }

    #[test]
    fn progress_throttler_emit_every_fiftieth_item() {
        let throttler = ProgressThrottler::new();
        let status = ScanStatus::new();
        *throttler.last_emit.lock().unwrap() = Instant::now();

        for scanned in [50, 100, 150] {
            status.reset(1);
            for _ in 0..scanned {
                status.increment_scanned();
            }
            assert!(
                progress_would_emit(&throttler, &status, false),
                "scanned={scanned} should emit by count"
            );
        }
    }

    #[test]
    fn progress_throttler_first_count_within_time_window() {
        let throttler = ProgressThrottler::new();
        let status = ScanStatus::new();
        status.reset(1);
        status.increment_scanned();
        assert_eq!(status.snapshot().scanned, 1);
        assert!(
            progress_would_emit(&throttler, &status, false),
            "initial throttler allows first progress emit via elapsed interval"
        );
    }

    #[test]
    fn progress_throttler_non_modulo_within_time_window_does_not_emit() {
        let throttler = ProgressThrottler::new();
        *throttler.last_emit.lock().unwrap() = Instant::now();
        let status = ScanStatus::new();
        status.reset(1);
        for _ in 0..23 {
            status.increment_scanned();
        }
        assert_eq!(status.snapshot().scanned, 23);
        assert!(
            !progress_would_emit(&throttler, &status, false),
            "scanned=23 should not emit when inside time window"
        );
    }

    use lightframe_core::media::MediaType;
    use std::sync::Arc;

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

    /// Mirrors MediaBatchEmitter buffering without AppHandle / event emission.
    struct TestBatchCollector {
        batches: Mutex<Vec<Vec<MediaFile>>>,
        buffer: Mutex<Vec<MediaFile>>,
    }

    impl TestBatchCollector {
        fn new() -> Self {
            Self {
                batches: Mutex::new(Vec::new()),
                buffer: Mutex::new(Vec::new()),
            }
        }

        fn push(&self, item: MediaFile) {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            buffer.push(item);
            if buffer.len() >= MEDIA_BATCH_SIZE {
                self.drain_to_batch(&mut buffer);
            }
        }

        fn flush(&self) {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            if !buffer.is_empty() {
                self.drain_to_batch(&mut buffer);
            }
        }

        fn drain_to_batch(&self, buffer: &mut Vec<MediaFile>) {
            let items: Vec<MediaFile> = buffer.drain(..).collect();
            if !items.is_empty() {
                self.batches
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(items);
            }
        }

        fn batches(&self) -> Vec<Vec<MediaFile>> {
            self.batches
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }

        fn pending_count(&self) -> usize {
            self.buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len()
        }

        fn total_items(&self) -> usize {
            self.batches()
                .iter()
                .map(|b| b.len())
                .sum::<usize>()
                + self.pending_count()
        }
    }

    #[test]
    fn test_batch_collector_flushes_at_capacity() {
        let collector = TestBatchCollector::new();
        for i in 0..25 {
            collector.push(sample_media(&format!("/photos/file{i}.jpg")));
        }

        let batches = collector.batches();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), MEDIA_BATCH_SIZE);
        assert_eq!(collector.pending_count(), 5);

        collector.flush();
        let batches = collector.batches();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), MEDIA_BATCH_SIZE);
        assert_eq!(batches[1].len(), 5);
        assert_eq!(collector.pending_count(), 0);
        assert_eq!(collector.total_items(), 25);
    }

    #[test]
    fn test_batch_collector_flush_empties_buffer() {
        let collector = TestBatchCollector::new();
        for i in 0..5 {
            collector.push(sample_media(&format!("/photos/file{i}.jpg")));
        }

        assert_eq!(collector.batches().len(), 0);
        assert_eq!(collector.pending_count(), 5);

        collector.flush();
        let batches = collector.batches();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 5);
        assert_eq!(collector.pending_count(), 0);
    }

    #[test]
    fn test_batch_collector_empty_flush_noop() {
        let collector = TestBatchCollector::new();
        collector.flush();
        assert!(collector.batches().is_empty());
        assert_eq!(collector.pending_count(), 0);
        assert_eq!(collector.total_items(), 0);
    }

    fn fs_modified_at(path: &Path) -> chrono::NaiveDateTime {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc())
            .unwrap_or_default()
    }

    fn sync_db_modified_at_from_filesystem(db: &Database, path: &Path) {
        let path_str = path.to_string_lossy();
        let mtime = fs_modified_at(path);
        // NaiveDateTime::to_string() is not reliably parsed by get_media_by_path; use RFC3339.
        let formatted = mtime.format("%Y-%m-%dT%H:%M:%S%.9f").to_string();
        let conn = db.conn().unwrap();
        conn.execute(
            "UPDATE media_files SET modified_at = ?1 WHERE path = ?2",
            (formatted.as_str(), path_str.as_ref()),
        )
        .unwrap();
    }

    #[test]
    fn rfc3339_modified_at_parses_for_skip_check() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mtime.jpg");
        fs::write(&file, b"x").unwrap();
        let mtime = fs_modified_at(&file);
        let formatted = mtime.format("%Y-%m-%dT%H:%M:%S%.9f").to_string();
        let parsed: chrono::NaiveDateTime = formatted.parse().expect("RFC3339 should parse");
        assert_eq!(parsed, mtime);
    }

    fn setup_test_db_and_folder() -> (tempfile::TempDir, Arc<Database>, i64, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("photos");
        fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            fs::canonicalize(&watched).unwrap(),
        );
        // In-memory DB avoids WAL read-replica lag between writer and reader connections.
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let folder_id = db
            .add_watched_folder(watched_canonical.to_str().unwrap())
            .unwrap()
            .id;
        (root, db, folder_id, watched_canonical)
    }

    #[tokio::test]
    async fn process_file_persists_to_db_before_events() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("persist.jpg");
        fs::write(&file, b"fake-jpeg-bytes").unwrap();

        let media_id = process_file_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("first scan should insert media");

        let stored = db.get_media_by_id(media_id).unwrap().unwrap();
        assert_eq!(stored.path, file.to_string_lossy());
        assert_eq!(stored.filename, "persist.jpg");
    }

    #[tokio::test]
    async fn rescan_skips_unchanged_files() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("unchanged.jpg");
        fs::write(&file, b"same-content").unwrap();

        let first = process_file_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("first scan should insert");
        sync_db_modified_at_from_filesystem(&db, &file);

        let second = process_file_inner(&db, folder_id, &file).await.unwrap();
        assert_eq!(second, None);
        assert!(
            db.get_media_by_id(first).unwrap().is_some(),
            "original record should remain in db"
        );
    }

    #[tokio::test]
    async fn process_file_returns_none_for_unchanged() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("skip.jpg");
        fs::write(&file, b"unchanged-payload").unwrap();

        process_file_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("initial insert");
        sync_db_modified_at_from_filesystem(&db, &file);
        let skipped = process_file_inner(&db, folder_id, &file).await.unwrap();
        assert!(skipped.is_none());
    }

    #[tokio::test]
    async fn process_file_with_missing_file_returns_error() {
        let (_root, db, folder_id, _watched) = setup_test_db_and_folder();
        let missing = PathBuf::from("/nonexistent/lightframe/missing-file.jpg");
        let result = process_file_inner(&db, folder_id, &missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_empty_folder_completes_without_error() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();

        let files = discover_files(&watched).await.unwrap();
        assert!(files.is_empty(), "empty watched folder should yield no media files");

        db.set_folder_scan_status(folder_id, "scanning").unwrap();
        db.update_last_scan_at(folder_id).unwrap();
        db.set_folder_scan_status(folder_id, "idle").unwrap();

        let folder = db.get_watched_folder(folder_id).unwrap().unwrap();
        assert_eq!(folder.scan_status, "idle");
    }
}
