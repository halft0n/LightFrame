use crate::memory;
use crate::state::{AppState, ScanQueue, ScanStatus};
use chrono::Timelike;
use futures::stream::{self, StreamExt};
use lightframe_core::media::{MediaFile, MediaType, ThumbnailSize};
use lightframe_db::Database;
use lightframe_indexer::{classify_extension, scan_folder_streaming};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tracing::{Instrument, error, info, warn};

const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(500);
const PROGRESS_EMIT_FILE_INTERVAL: i64 = 50;
const MEDIA_BATCH_SIZE: usize = 20;
const MEDIA_BATCH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone)]
struct EnrichmentProgress {
    folder_id: i64,
    total: i64,
    processed: i64,
    status: String,
}

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
        let items: Vec<MediaFile> = std::mem::take(buffer);
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
                let result = run_scan(
                    &app,
                    db.clone(),
                    scan_status.clone(),
                    concurrency,
                    folder_id,
                )
                .await;
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

    // Phase 1: Streaming discovery + fast index (metadata + DB only, no heavy work)
    let rx = scan_folder_streaming(&root);
    let discovered = Arc::new(AtomicI64::new(0));
    let discovered_for_stream = Arc::clone(&discovered);

    let file_stream = futures::stream::unfold(rx, move |mut rx| {
        let discovered = Arc::clone(&discovered_for_stream);
        async move {
            let path = rx.recv().await?;
            let count = discovered.fetch_add(1, Ordering::Relaxed) + 1;
            Some(((path, count), rx))
        }
    });

    scan_status.set_status("scanning");
    let progress = Arc::new(ProgressThrottler::new());
    progress.maybe_emit(app, &scan_status, true);
    let batch_emitter = Arc::new(MediaBatchEmitter::new(app.clone(), folder_id));
    let media_ids_to_enrich: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));

    file_stream
        .map(|(path, _discovered_count)| {
            let db = Arc::clone(&db);
            let scan_status = scan_status.clone();
            let app = app.clone();
            let progress = Arc::clone(&progress);
            let batch_emitter = Arc::clone(&batch_emitter);
            let discovered = Arc::clone(&discovered);
            let enrich_list = Arc::clone(&media_ids_to_enrich);
            async move {
                scan_status.set_total(discovered.load(Ordering::Relaxed));

                match quick_index_file(&db, folder_id, &path).await {
                    Ok(Some((media_id, media))) => {
                        batch_emitter.push(media);
                        enrich_list
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push(media_id);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(path = %path.display(), "failed to index file: {e}");
                        scan_status.increment_errors();
                    }
                }
                let scanned = scan_status.increment_scanned();
                if scanned % 500 == 0 {
                    memory::log_memory("scan_progress");
                }
                progress.maybe_emit(&app, &scan_status, false);
            }
        })
        .buffer_unordered(concurrency)
        .collect::<()>()
        .await;

    let total_discovered = discovered.load(Ordering::Relaxed);
    scan_status.set_total(total_discovered);
    batch_emitter.flush();

    db.update_last_scan_at(folder_id).inspect_err(|_| {
        let _ = db.set_folder_scan_status(folder_id, "error");
    })?;
    db.set_folder_scan_status(folder_id, "indexed")?;
    scan_status.set_status("indexed");
    progress.maybe_emit(app, &scan_status, true);

    info!(
        folder_id,
        total = total_discovered,
        "phase 1 (index) complete"
    );

    // Phase 1.5: Live Photo pairing
    if let Err(e) = pair_live_photos(&db, folder_id) {
        warn!(folder_id, "live photo pairing failed: {e}");
    }

    // Phase 2: Background enrichment (thumbnails, hashes, etc.)
    let mut ids = std::mem::take(
        &mut *media_ids_to_enrich
            .lock()
            .unwrap_or_else(|e| e.into_inner()),
    );
    if let Ok(unenriched) = db.get_unenriched_media_ids(folder_id) {
        let existing: std::collections::HashSet<i64> = ids.iter().copied().collect();
        let new_count = unenriched
            .into_iter()
            .filter(|id| !existing.contains(id))
            .inspect(|id| ids.push(*id))
            .count();
        if new_count > 0 {
            info!(
                folder_id,
                count = new_count,
                "re-enriching previously skipped files"
            );
        }
    }
    if !ids.is_empty() {
        info!(
            folder_id,
            count = ids.len(),
            "starting phase 2 (enrichment)"
        );
        // I/O-bound work (file reading, hashing, thumbnail gen): scale with CPU cores
        let enrich_concurrency = concurrency;
        let enrich_total = ids.len() as i64;
        let enrich_processed = Arc::new(AtomicI64::new(0));
        let enrich_last_emit = Arc::new(Mutex::new(Instant::now()));

        let _ = app.emit(
            "enrichment-progress",
            EnrichmentProgress {
                folder_id,
                total: enrich_total,
                processed: 0,
                status: "running".to_string(),
            },
        );

        // Channel for batch DB writes
        let (hash_tx, mut hash_rx) =
            tokio::sync::mpsc::channel::<(i64, String, Option<u64>, Option<u64>)>(128);
        let (micro_tx, mut micro_rx) = tokio::sync::mpsc::channel::<(i64, Vec<u8>)>(128);

        let db_batch = Arc::clone(&db);
        let batch_writer = tokio::spawn(async move {
            let mut hash_buf: Vec<(i64, String, Option<u64>, Option<u64>)> = Vec::with_capacity(50);
            let mut micro_buf: Vec<(i64, Vec<u8>)> = Vec::with_capacity(50);
            let mut hash_open = true;
            let mut micro_open = true;

            while hash_open || micro_open || !hash_buf.is_empty() || !micro_buf.is_empty() {
                tokio::select! {
                    msg = hash_rx.recv(), if hash_open => {
                        match msg {
                            Some(item) => {
                                hash_buf.push(item);
                                if hash_buf.len() >= 50 {
                                    if let Err(e) = db_batch.batch_update_media_hashes(&hash_buf) {
                                        warn!("batch hash write failed ({} items): {e}", hash_buf.len());
                                    }
                                    hash_buf.clear();
                                }
                            }
                            None => {
                                hash_open = false;
                                if !hash_buf.is_empty() {
                                    if let Err(e) = db_batch.batch_update_media_hashes(&hash_buf) {
                                        warn!("final batch hash write failed ({} items): {e}", hash_buf.len());
                                    }
                                    hash_buf.clear();
                                }
                            }
                        }
                    }
                    msg = micro_rx.recv(), if micro_open => {
                        match msg {
                            Some((id, blob)) => {
                                micro_buf.push((id, blob));
                                if micro_buf.len() >= 50 {
                                    if let Err(e) = db_batch.batch_set_micro_thumbs(&micro_buf) {
                                        warn!("batch micro thumb write failed ({} items): {e}", micro_buf.len());
                                    }
                                    micro_buf.clear();
                                }
                            }
                            None => {
                                micro_open = false;
                                if !micro_buf.is_empty() {
                                    if let Err(e) = db_batch.batch_set_micro_thumbs(&micro_buf) {
                                        warn!("final batch micro thumb write failed ({} items): {e}", micro_buf.len());
                                    }
                                    micro_buf.clear();
                                }
                            }
                        }
                    }
                }
            }
        });

        stream::iter(ids.into_iter().map(|media_id| {
            let db = Arc::clone(&db);
            let processed = Arc::clone(&enrich_processed);
            let last_emit = Arc::clone(&enrich_last_emit);
            let app = app.clone();
            let hash_tx = hash_tx.clone();
            let micro_tx = micro_tx.clone();
            async move {
                match enrich_media_batch(&db, media_id).await {
                    Ok(result) => {
                        let _ = hash_tx
                            .send((media_id, result.blake3_hash, result.dhash, result.phash))
                            .await;
                        if let Some(blob) = result.micro_blob {
                            let _ = micro_tx.send((media_id, blob)).await;
                        }
                    }
                    Err(e) => {
                        warn!(media_id, "enrichment failed: {e}");
                    }
                }
                let done = processed.fetch_add(1, Ordering::Relaxed) + 1;
                let should_emit = done == enrich_total
                    || done % 10 == 0
                    || last_emit
                        .lock()
                        .map(|last| last.elapsed() >= Duration::from_millis(500))
                        .unwrap_or(true);
                if should_emit {
                    if let Ok(mut last) = last_emit.lock() {
                        *last = Instant::now();
                    }
                    let _ = app.emit(
                        "enrichment-progress",
                        EnrichmentProgress {
                            folder_id,
                            total: enrich_total,
                            processed: done,
                            status: "running".to_string(),
                        },
                    );
                }
            }
        }))
        .buffer_unordered(enrich_concurrency)
        .collect::<()>()
        .await;

        // Drop senders to signal batch writer to flush
        drop(hash_tx);
        drop(micro_tx);
        let _ = batch_writer.await;

        let _ = app.emit(
            "enrichment-progress",
            EnrichmentProgress {
                folder_id,
                total: enrich_total,
                processed: enrich_total,
                status: "complete".to_string(),
            },
        );
        info!(folder_id, "phase 2 (enrichment) complete");
    }

    db.set_folder_scan_status(folder_id, "idle")?;
    scan_status.set_status("complete");
    progress.maybe_emit(app, &scan_status, true);

    memory::log_memory("scan_end");
    Ok(())
}

/// Phase 1: Fast index — stat, skip check, EXIF metadata, DB upsert.
/// Returns `(media_id, MediaFile)` for newly indexed files, `None` for skipped.
async fn quick_index_file(
    db: &Database,
    folder_id: i64,
    path: &Path,
) -> lightframe_core::Result<Option<(i64, MediaFile)>> {
    let path_str = path.to_string_lossy().to_string();
    let span = tracing::info_span!("quick_index", path = %path_str);
    quick_index_inner(db, folder_id, path)
        .instrument(span)
        .await
}

async fn quick_index_inner(
    db: &Database,
    folder_id: i64,
    path: &Path,
) -> lightframe_core::Result<Option<(i64, MediaFile)>> {
    let path = crate::original_protocol::strip_extended_prefix(path.to_path_buf());
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
        .map(|t| {
            let dt = chrono::DateTime::<chrono::Utc>::from(t).naive_utc();
            dt.with_nanosecond(0).unwrap_or(dt)
        })
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
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || lightframe_metadata::extract(&path)
        })
        .await
        .map_err(|e| lightframe_core::Error::Other(e.to_string()))??
    } else {
        lightframe_metadata::PhotoMetadata::default()
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
        blake3_hash: None,
        dhash: None,
        phash: None,
        latitude: meta.latitude,
        longitude: meta.longitude,
    };

    let media_id = db.upsert_media(folder_id, &media)?;
    let mut stored = media;
    stored.id = media_id;

    if let (Some(lat), Some(lon)) = (stored.latitude, stored.longitude) {
        let geo_result =
            tokio::task::spawn_blocking(move || lightframe_geo::reverse_geocode(lat, lon))
                .await
                .map_err(|e| lightframe_core::Error::Other(e.to_string()))?;

        if let Some(loc) = geo_result {
            let country = loc.country.as_deref().unwrap_or("");
            let city = loc.city.as_deref().unwrap_or("");
            if !country.is_empty() {
                let _ = db.update_media_location(media_id, city, country);
            }
        }
    }

    Ok(Some((media_id, stored)))
}

struct EnrichResult {
    blake3_hash: String,
    dhash: Option<u64>,
    phash: Option<u64>,
    micro_blob: Option<Vec<u8>>,
}

/// Phase 2 enrichment that returns results without writing to DB.
/// DB writes are batched by the caller.
async fn enrich_media_batch(db: &Database, media_id: i64) -> lightframe_core::Result<EnrichResult> {
    let media = db
        .get_media_by_id(media_id)?
        .ok_or_else(|| lightframe_core::Error::Other(format!("media {media_id} not found")))?;

    let path = PathBuf::from(&media.path);
    let media_type = media.media_type;

    let blake3_hash = tokio::task::spawn_blocking({
        let path = path.clone();
        move || lightframe_dedup::file_hash(&path)
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))??;

    let is_image = matches!(
        media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    );

    let (dhash, phash, micro_blob) = if is_image {
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
        .map_err(|e| lightframe_core::Error::Other(e.to_string()))?
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
        micro_blob
    };

    // Screenshot detection still writes directly (infrequent)
    if matches!(media_type, MediaType::Screenshot) || matches!(media_type, MediaType::Photo) {
        let mt = media_type;
        if matches!(mt, MediaType::Photo) {
            if let (Some(w), Some(h)) = (media.width, media.height)
                && lightframe_ai::detect_screenshot(&path, w, h)
                    .map(|s| s.is_likely_screenshot())
                    .unwrap_or(false)
            {
                let _ = db.set_media_type(media_id, "Screenshot");
                if let Ok(st) = lightframe_ai::classify_screenshot(&path) {
                    let _ = db.set_screenshot_type(media_id, st.label());
                }
            }
        } else if let Ok(st) = lightframe_ai::classify_screenshot(&path) {
            let _ = db.set_screenshot_type(media_id, st.label());
        }
    }

    Ok(EnrichResult {
        blake3_hash,
        dhash,
        phash,
        micro_blob,
    })
}

fn pair_live_photos(db: &Database, folder_id: i64) -> lightframe_core::Result<()> {
    use lightframe_indexer::detect_live_photo_pairs;

    let all_media = db.get_media_by_folder(folder_id, 100_000, 0)?;
    if all_media.is_empty() {
        return Ok(());
    }

    let paths: Vec<PathBuf> = all_media.iter().map(|m| PathBuf::from(&m.path)).collect();
    let pairs = detect_live_photo_pairs(&paths);

    if pairs.is_empty() {
        return Ok(());
    }

    // Build path -> media_id lookup
    let path_to_id: std::collections::HashMap<&str, i64> =
        all_media.iter().map(|m| (m.path.as_str(), m.id)).collect();

    for pair in &pairs {
        let still_path = pair.still_path.to_string_lossy();
        let video_path = pair.video_path.to_string_lossy();

        let Some(&sid) = path_to_id.get(still_path.as_ref()) else {
            continue;
        };
        let Some(&vid) = path_to_id.get(video_path.as_ref()) else {
            continue;
        };

        if let Err(e) = db.set_live_pair(sid, vid) {
            warn!(
                still_id = sid,
                video_id = vid,
                "failed to set live pair: {e}"
            );
        }
    }

    info!(folder_id, count = pairs.len(), "live photo pairs detected");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    async fn discover_files(path: &Path) -> lightframe_core::Result<Vec<PathBuf>> {
        lightframe_indexer::scan_folder(path).await
    }

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
            let items: Vec<MediaFile> = std::mem::take(buffer);
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
            self.buffer.lock().unwrap_or_else(|e| e.into_inner()).len()
        }

        fn total_items(&self) -> usize {
            self.batches().iter().map(|b| b.len()).sum::<usize>() + self.pending_count()
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
            .map(|t| {
                let dt = chrono::DateTime::<chrono::Utc>::from(t).naive_utc();
                dt.with_nanosecond(0).unwrap_or(dt)
            })
            .unwrap_or_default()
    }

    fn sync_db_modified_at_from_filesystem(db: &Database, path: &Path) {
        let path_str = path.to_string_lossy();
        let mtime = fs_modified_at(path);
        let formatted = mtime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let conn = db.conn().unwrap();
        conn.execute(
            "UPDATE media_files SET modified_at = ?1 WHERE path = ?2",
            (formatted.as_str(), path_str.as_ref()),
        )
        .unwrap();
    }

    #[test]
    fn modified_at_truncated_to_seconds_parses_for_skip_check() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mtime.jpg");
        fs::write(&file, b"x").unwrap();
        let mtime = fs_modified_at(&file);
        assert_eq!(
            mtime.nanosecond(),
            0,
            "fs_modified_at should truncate to seconds"
        );
        let formatted = mtime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let parsed: chrono::NaiveDateTime = formatted.parse().expect("should parse");
        assert_eq!(parsed, mtime);
    }

    fn setup_test_db_and_folder() -> (tempfile::TempDir, Arc<Database>, i64, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("photos");
        fs::create_dir_all(&watched).unwrap();
        let watched_canonical =
            crate::original_protocol::strip_extended_prefix(fs::canonicalize(&watched).unwrap());
        // In-memory DB avoids WAL read-replica lag between writer and reader connections.
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let folder_id = db
            .add_watched_folder(watched_canonical.to_str().unwrap())
            .unwrap()
            .id;
        (root, db, folder_id, watched_canonical)
    }

    #[tokio::test]
    async fn quick_index_persists_to_db() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("persist.jpg");
        fs::write(&file, b"fake-jpeg-bytes").unwrap();

        let (media_id, media) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("first scan should insert media");

        let stored = db.get_media_by_id(media_id).unwrap().unwrap();
        assert_eq!(stored.path, file.to_string_lossy());
        assert_eq!(stored.filename, "persist.jpg");
        assert_eq!(media.filename, "persist.jpg");
    }

    #[tokio::test]
    async fn rescan_skips_unchanged_files() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("unchanged.jpg");
        fs::write(&file, b"same-content").unwrap();

        let (first, _) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("first scan should insert");
        sync_db_modified_at_from_filesystem(&db, &file);

        let second = quick_index_inner(&db, folder_id, &file).await.unwrap();
        assert!(second.is_none());
        assert!(
            db.get_media_by_id(first).unwrap().is_some(),
            "original record should remain in db"
        );
    }

    #[tokio::test]
    async fn quick_index_returns_none_for_unchanged() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("skip.jpg");
        fs::write(&file, b"unchanged-payload").unwrap();

        quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("initial insert");
        sync_db_modified_at_from_filesystem(&db, &file);
        let skipped = quick_index_inner(&db, folder_id, &file).await.unwrap();
        assert!(skipped.is_none());
    }

    #[tokio::test]
    async fn quick_index_with_missing_file_returns_error() {
        let (_root, db, folder_id, _watched) = setup_test_db_and_folder();
        let missing = PathBuf::from("/nonexistent/lightframe/missing-file.jpg");
        let result = quick_index_inner(&db, folder_id, &missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_empty_folder_completes_without_error() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();

        let files = lightframe_indexer::scan_folder(&watched).await.unwrap();
        assert!(
            files.is_empty(),
            "empty watched folder should yield no media files"
        );

        db.set_folder_scan_status(folder_id, "scanning").unwrap();
        db.update_last_scan_at(folder_id).unwrap();
        db.set_folder_scan_status(folder_id, "idle").unwrap();

        let folder = db.get_watched_folder(folder_id).unwrap().unwrap();
        assert_eq!(folder.scan_status, "idle");
    }

    #[tokio::test]
    async fn update_media_hashes_persists() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("hash-test.jpg");
        fs::write(&file, b"test-content").unwrap();

        let (media_id, _) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("should index");

        let before = db.get_media_by_id(media_id).unwrap().unwrap();
        assert!(before.blake3_hash.is_none());

        db.update_media_hashes(media_id, "abc123", Some(42), Some(99))
            .unwrap();
        let after = db.get_media_by_id(media_id).unwrap().unwrap();
        assert_eq!(after.blake3_hash.as_deref(), Some("abc123"));
        assert_eq!(after.dhash, Some(42));
        assert_eq!(after.phash, Some(99));
    }

    #[tokio::test]
    async fn streaming_discovery_yields_all_media_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"img-a").unwrap();
        fs::write(dir.path().join("b.png"), b"img-b").unwrap();
        fs::write(dir.path().join("readme.txt"), b"not media").unwrap();

        let mut rx = lightframe_indexer::scan_folder_streaming(dir.path());
        let mut paths = Vec::new();
        while let Some(p) = rx.recv().await {
            paths.push(p);
        }
        assert_eq!(paths.len(), 2);
        let names: Vec<_> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.jpg".to_string()));
        assert!(names.contains(&"b.png".to_string()));
    }

    #[tokio::test]
    async fn streaming_discovery_empty_dir_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut rx = lightframe_indexer::scan_folder_streaming(dir.path());
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn quick_index_reindexes_after_mtime_change() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("reindex.jpg");
        fs::write(&file, b"first-version").unwrap();

        let (first_id, _) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("initial insert");
        sync_db_modified_at_from_filesystem(&db, &file);

        let skipped = quick_index_inner(&db, folder_id, &file).await.unwrap();
        assert!(skipped.is_none(), "unchanged file should be skipped");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&file, b"second-version-longer").unwrap();

        let result = quick_index_inner(&db, folder_id, &file).await.unwrap();
        assert!(result.is_some(), "modified file should be reindexed");
        let (reindex_id, _) = result.unwrap();
        assert_eq!(reindex_id, first_id, "reindex should upsert same record");
    }

    #[tokio::test]
    async fn set_media_type_persists() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("screenshot-test.jpg");
        fs::write(&file, b"fake-jpg").unwrap();

        let (media_id, media) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("should index");
        assert!(
            matches!(media.media_type, MediaType::Photo),
            "initial type should be Photo"
        );

        db.set_media_type(media_id, "Screenshot").unwrap();
        let updated = db.get_media_by_id(media_id).unwrap().unwrap();
        assert!(
            matches!(updated.media_type, MediaType::Screenshot),
            "type should be Screenshot after update"
        );
    }

    #[tokio::test]
    async fn quick_index_multiple_files_assigns_unique_ids() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let f1 = watched.join("multi1.jpg");
        let f2 = watched.join("multi2.png");
        let f3 = watched.join("multi3.jpeg");
        fs::write(&f1, b"data1").unwrap();
        fs::write(&f2, b"data2").unwrap();
        fs::write(&f3, b"data3").unwrap();

        let (id1, _) = quick_index_inner(&db, folder_id, &f1)
            .await
            .unwrap()
            .expect("f1");
        let (id2, _) = quick_index_inner(&db, folder_id, &f2)
            .await
            .unwrap()
            .expect("f2");
        let (id3, _) = quick_index_inner(&db, folder_id, &f3)
            .await
            .unwrap()
            .expect("f3");

        let mut ids = vec![id1, id2, id3];
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "all media IDs should be unique");
    }

    #[test]
    fn batch_collector_auto_flushes_multiple_batches() {
        let collector = TestBatchCollector::new();
        for i in 0..55 {
            collector.push(sample_media(&format!("/photos/batch{i}.jpg")));
        }
        let batches = collector.batches();
        assert_eq!(
            batches.len(),
            2,
            "55 items = 2 full batches ({0}+{0}) + 15 pending",
            MEDIA_BATCH_SIZE
        );
        assert_eq!(batches[0].len(), MEDIA_BATCH_SIZE);
        assert_eq!(batches[1].len(), MEDIA_BATCH_SIZE);
        assert_eq!(collector.pending_count(), 55 - 2 * MEDIA_BATCH_SIZE);

        collector.flush();
        let batches = collector.batches();
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[2].len(), 55 - 2 * MEDIA_BATCH_SIZE);
        assert_eq!(collector.pending_count(), 0);
    }

    #[test]
    fn batch_collector_total_items_across_batches_and_pending() {
        let collector = TestBatchCollector::new();
        for i in 0..33 {
            collector.push(sample_media(&format!("/photos/total{i}.jpg")));
        }
        assert_eq!(collector.total_items(), 33);
    }

    #[tokio::test]
    async fn quick_index_stores_file_size() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("size-test.jpg");
        let content = vec![0u8; 1234];
        fs::write(&file, &content).unwrap();

        let (media_id, media) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("should index");
        assert_eq!(media.size_bytes, 1234);

        let stored = db.get_media_by_id(media_id).unwrap().unwrap();
        assert_eq!(stored.size_bytes, 1234);
    }

    #[tokio::test]
    async fn quick_index_blake3_hash_is_none_in_phase1() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("no-hash.jpg");
        fs::write(&file, b"test-data").unwrap();

        let (media_id, media) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("should index");
        assert!(
            media.blake3_hash.is_none(),
            "phase 1 should NOT compute blake3_hash"
        );

        let stored = db.get_media_by_id(media_id).unwrap().unwrap();
        assert!(
            stored.blake3_hash.is_none(),
            "stored blake3_hash should also be None after phase 1"
        );
    }

    #[test]
    fn modified_at_truncation_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("idempotent.jpg");
        fs::write(&file, b"x").unwrap();
        let m1 = fs_modified_at(&file);
        let m2 = fs_modified_at(&file);
        assert_eq!(m1, m2, "fs_modified_at should be deterministic");
        assert_eq!(m1.nanosecond(), 0);
    }

    #[tokio::test]
    async fn reindex_clears_stale_hashes() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let file = watched.join("hash-stale.jpg");
        fs::write(&file, b"original-content").unwrap();

        let (media_id, _) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("initial index");

        db.update_media_hashes(media_id, "oldhash", Some(111), Some(222))
            .unwrap();
        let enriched = db.get_media_by_id(media_id).unwrap().unwrap();
        assert_eq!(enriched.blake3_hash.as_deref(), Some("oldhash"));

        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&file, b"modified-content-longer").unwrap();

        let (reindex_id, _) = quick_index_inner(&db, folder_id, &file)
            .await
            .unwrap()
            .expect("reindex after modification");
        assert_eq!(reindex_id, media_id);

        let after_reindex = db.get_media_by_id(media_id).unwrap().unwrap();
        assert!(
            after_reindex.blake3_hash.is_none(),
            "reindex should clear stale blake3_hash"
        );
        assert!(
            after_reindex.dhash.is_none(),
            "reindex should clear stale dhash"
        );
        assert!(
            after_reindex.phash.is_none(),
            "reindex should clear stale phash"
        );
    }

    #[tokio::test]
    async fn get_unenriched_media_ids_finds_null_hash_records() {
        let (_root, db, folder_id, watched) = setup_test_db_and_folder();
        let f1 = watched.join("enriched.jpg");
        let f2 = watched.join("unenriched.jpg");
        fs::write(&f1, b"data-e").unwrap();
        fs::write(&f2, b"data-u").unwrap();

        let (id1, _) = quick_index_inner(&db, folder_id, &f1)
            .await
            .unwrap()
            .expect("f1");
        let (id2, _) = quick_index_inner(&db, folder_id, &f2)
            .await
            .unwrap()
            .expect("f2");

        db.update_media_hashes(id1, "hash1", Some(1), Some(1))
            .unwrap();

        let unenriched = db.get_unenriched_media_ids(folder_id).unwrap();
        assert_eq!(unenriched.len(), 1);
        assert_eq!(unenriched[0], id2);
    }

    #[test]
    fn pair_live_photos_detects_and_links_pairs() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db").as_path()).unwrap();
        let folder_id = db
            .add_watched_folder(dir.path().to_str().unwrap())
            .unwrap()
            .id;

        // Insert a HEIC and its companion MOV
        let heic_path = dir.path().join("IMG_001.HEIC");
        let mov_path = dir.path().join("IMG_001.MOV");
        fs::write(&heic_path, b"fake heic").unwrap();
        fs::write(&mov_path, b"fake mov").unwrap();

        let heic_media = lightframe_core::media::MediaFile {
            id: 0,
            path: heic_path.to_string_lossy().to_string(),
            filename: "IMG_001.HEIC".into(),
            media_type: lightframe_core::media::MediaType::Photo,
            size_bytes: 100,
            width: None,
            height: None,
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        };
        let mov_media = lightframe_core::media::MediaFile {
            id: 0,
            path: mov_path.to_string_lossy().to_string(),
            filename: "IMG_001.MOV".into(),
            media_type: lightframe_core::media::MediaType::Video,
            ..heic_media.clone()
        };

        let heic_id = db.upsert_media(folder_id, &heic_media).unwrap();
        let mov_id = db.upsert_media(folder_id, &mov_media).unwrap();

        // Run pair_live_photos
        pair_live_photos(&db, folder_id).unwrap();

        // Verify pairing
        let pair = db.get_live_pair(heic_id).unwrap();
        assert_eq!(pair, Some(mov_id));

        // Verify MOV is hidden from media page
        let page = db.get_media_page(100, None).unwrap();
        let ids: Vec<i64> = page.iter().map(|m| m.id).collect();
        assert!(ids.contains(&heic_id));
        assert!(!ids.contains(&mov_id));

        // Verify media type is LivePhoto
        let live_photo = page.iter().find(|m| m.id == heic_id).unwrap();
        assert_eq!(
            live_photo.media_type,
            lightframe_core::media::MediaType::LivePhoto
        );
    }
}
