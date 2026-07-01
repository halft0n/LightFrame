use crate::state::AppState;
use lightframe_core::media::{MediaType, ThumbnailSize};
use lightframe_db::Database;
use lightframe_thumbnail::{thumb_file_needs_regeneration, thumb_path};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

const PAGE_SIZE: i64 = 200;

pub fn media_needs_thumbnail_regeneration(db: &Database, media_id: i64, hash: &str) -> bool {
    if thumb_file_needs_regeneration(&thumb_path(hash, ThumbnailSize::Micro))
        || thumb_file_needs_regeneration(&thumb_path(hash, ThumbnailSize::Small))
    {
        return true;
    }
    db.get_micro_thumb(media_id)
        .ok()
        .flatten()
        .is_none_or(|blob| blob.is_empty())
}

pub fn regenerate_thumbnails_for_media(state: &AppState, media_id: i64) -> Result<bool, String> {
    let result = regenerate_thumbnails_for_media_db(&state.db, media_id)?;
    if result {
        state.thumb_cache.invalidate_media(media_id);
        crate::face_protocol::invalidate_face_cache_for_media(state, media_id);
    }
    Ok(result)
}

pub fn regenerate_thumbnails_for_media_db(
    db: &Arc<Database>,
    media_id: i64,
) -> Result<bool, String> {
    let media = db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let Some(hash) = media.blake3_hash.clone() else {
        return Ok(false);
    };

    if !media_needs_thumbnail_regeneration(db, media_id, &hash) {
        return Ok(false);
    }

    let path = Path::new(&media.path);
    if !path.is_file() {
        tracing::warn!(media_id, path = %media.path, "source missing, skipping thumbnail regen");
        return Ok(false);
    }

    let is_image = matches!(
        media.media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    );

    if is_image {
        let db_ref = Arc::clone(db);
        let hash_clone = hash.clone();
        let path_buf = path.to_path_buf();
        let regenerated = std::thread::spawn(move || -> Result<bool, String> {
            let decoded = lightframe_core::decode::decode_image(&path_buf)
                .map_err(|e| format!("decode failed: {e}"))?;

            lightframe_thumbnail::regenerate_from_decoded(
                &decoded,
                &hash_clone,
                ThumbnailSize::Micro,
            )
            .map_err(|e| e.to_string())?;
            lightframe_thumbnail::regenerate_from_decoded(
                &decoded,
                &hash_clone,
                ThumbnailSize::Small,
            )
            .map_err(|e| e.to_string())?;

            if let Ok(micro) = lightframe_thumbnail::micro_blob_from_decoded(&decoded) {
                let _ = db_ref.set_micro_thumb(media_id, &micro);
            }
            Ok(true)
        })
        .join()
        .map_err(|_| "thumbnail regeneration thread panicked".to_string())??;

        return Ok(regenerated);
    }

    if matches!(media.media_type, MediaType::Video) {
        if lightframe_video::find_ffmpeg().is_none() {
            return Ok(false);
        }

        let temp_frame = thumb_path(&hash, ThumbnailSize::Small).with_extension("frame.jpg");
        if let Some(parent) = temp_frame.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let path_owned = path.to_path_buf();
        let frame_clone = temp_frame.clone();
        let extraction_result = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("failed to create runtime: {e}"))?;
            rt.block_on(lightframe_video::extract_frame(
                &path_owned,
                &frame_clone,
                1.0,
            ))
            .map_err(|e| format!("frame extraction failed: {e}"))
        })
        .join()
        .map_err(|_| "video frame extraction thread panicked".to_string())?;

        if extraction_result.is_err() || !temp_frame.exists() {
            return Ok(false);
        }

        let hash_clone = hash.clone();
        let frame = temp_frame.clone();
        let regenerated = std::thread::spawn(move || -> Result<bool, String> {
            lightframe_thumbnail::regenerate(&frame, &hash_clone, ThumbnailSize::Micro)
                .map_err(|e| e.to_string())?;
            lightframe_thumbnail::regenerate(&frame, &hash_clone, ThumbnailSize::Small)
                .map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&frame);
            Ok(true)
        })
        .join()
        .map_err(|_| "video thumbnail regeneration thread panicked".to_string())??;

        if regenerated {
            let db_ref = Arc::clone(db);
            let hash_for_micro = hash.clone();
            if let Ok(micro) = std::thread::spawn(move || {
                let small = thumb_path(&hash_for_micro, ThumbnailSize::Small);
                lightframe_thumbnail::generate_micro_blob(&small)
            })
            .join()
                && let Ok(blob) = micro
            {
                let _ = db_ref.set_micro_thumb(media_id, &blob);
            }
        }

        return Ok(regenerated);
    }

    Ok(false)
}

#[derive(serde::Serialize, Clone)]
pub struct ThumbnailRegenProgress {
    pub processed: i64,
    pub total: i64,
    pub regenerated: i64,
    pub status: String,
}

#[derive(serde::Serialize)]
pub struct ThumbnailRegenResult {
    pub regenerated: i64,
}

pub async fn regenerate_all_thumbnails(
    app: AppHandle,
    state: &AppState,
) -> Result<ThumbnailRegenResult, String> {
    if state
        .thumb_regenerating
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("thumbnail regeneration already in progress".to_string());
    }

    struct ThumbRegeneratingGuard(Arc<AtomicBool>);
    impl Drop for ThumbRegeneratingGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = ThumbRegeneratingGuard(Arc::clone(&state.thumb_regenerating));

    let total = state.db.get_media_count().map_err(|e| e.to_string())?;
    let mut processed = 0i64;
    let mut regenerated = 0i64;
    let mut offset = 0i64;

    let emit = |processed: i64, regenerated: i64, status: &str| {
        let payload = ThumbnailRegenProgress {
            processed,
            total,
            regenerated,
            status: status.to_string(),
        };
        if let Err(e) = app.emit("thumbnail-regen-progress", &payload) {
            tracing::warn!("failed to emit thumbnail-regen-progress: {e}");
        }
    };

    emit(0, 0, "running");

    while offset < total {
        let batch = state
            .db
            .get_all_media(PAGE_SIZE, offset)
            .map_err(|e| e.to_string())?;

        if batch.is_empty() {
            break;
        }

        for media in batch {
            processed += 1;
            match regenerate_thumbnails_for_media(state, media.id) {
                Ok(true) => regenerated += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(media_id = media.id, "thumbnail regen failed: {e}"),
            }
            emit(processed, regenerated, "running");
        }

        offset += PAGE_SIZE;
        tokio::task::yield_now().await;
    }

    emit(processed, regenerated, "complete");

    Ok(ThumbnailRegenResult { regenerated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use image::{ImageBuffer, Rgb};
    use lightframe_core::config::AppConfig;
    use lightframe_core::media::{MediaFile, MediaType};
    use lightframe_db::Database;
    use lightframe_thumbnail::thumb_path;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    static DATA_DIR_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn new(key: &'static str, value: &Path) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: tests hold DATA_DIR_LOCK so env vars are not read concurrently.
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: tests hold DATA_DIR_LOCK so env vars are not read concurrently.
            unsafe {
                match &self.previous {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    struct TestHarness {
        _data_dir: TempDir,
        _data_dir_lock: std::sync::MutexGuard<'static, ()>,
        _env_guard: EnvVarGuard,
        pub state: AppState,
        pub media_dir: PathBuf,
    }

    impl TestHarness {
        fn new() -> Self {
            let lock = DATA_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let data_dir = TempDir::new().expect("temp data dir");
            let media_dir = data_dir.path().join("photos");
            std::fs::create_dir_all(&media_dir).expect("create media dir");

            #[cfg(windows)]
            let env_key = "LOCALAPPDATA";
            #[cfg(not(windows))]
            let env_key = "XDG_DATA_HOME";

            let env_guard = EnvVarGuard::new(env_key, data_dir.path());

            let state = AppState {
                db: Arc::new(Database::open(Path::new(":memory:")).expect("in-memory db")),
                config: AppConfig::default(),
                scan_status: crate::state::ScanStatus::new(),
                scan_concurrency: 2,
                scan_queue: crate::state::ScanQueue::new(),
                face_detecting: Arc::new(AtomicBool::new(false)),
                dedup_scanning: Arc::new(AtomicBool::new(false)),
                thumb_regenerating: Arc::new(AtomicBool::new(false)),
                active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
                watch_manager: crate::watcher::WatchManager::new(),
                thumb_cache: crate::thumb_cache::ThumbCache::new(),
                ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
                face_cache_dir: tempfile::tempdir().unwrap().into_path(),
            };

            Self {
                _data_dir: data_dir,
                _data_dir_lock: lock,
                _env_guard: env_guard,
                state,
                media_dir,
            }
        }
    }

    fn write_test_png(path: &Path) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(64, 64, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 64]));
        img.save(path).expect("write png");
    }

    fn write_thumb_file(hash: &str, size: ThumbnailSize, contents: &[u8]) {
        let path = thumb_path(hash, size);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create thumb parent");
        }
        std::fs::write(path, contents).expect("write thumb");
    }

    fn insert_media(
        harness: &TestHarness,
        filename: &str,
        hash: Option<&str>,
        path_override: Option<&Path>,
    ) -> i64 {
        let folder_id = harness
            .state
            .db
            .add_watched_folder(harness.media_dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let file_path = path_override
            .map(Path::to_path_buf)
            .unwrap_or_else(|| harness.media_dir.join(filename));
        if path_override.is_none() {
            write_test_png(&file_path);
        }
        let media = MediaFile {
            id: 0,
            path: file_path.to_string_lossy().to_string(),
            filename: filename.to_string(),
            media_type: MediaType::Photo,
            size_bytes: 64 * 64 * 3,
            width: Some(64),
            height: Some(64),
            created_at: None,
            modified_at: NaiveDateTime::default(),
            blake3_hash: hash.map(str::to_string),
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        };
        harness
            .state
            .db
            .upsert_media(folder_id, &media)
            .expect("upsert media")
    }

    #[test]
    fn media_needs_regeneration_when_micro_thumb_file_missing() {
        let harness = TestHarness::new();
        let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write_thumb_file(hash, ThumbnailSize::Small, b"small");
        harness
            .state
            .db
            .set_micro_thumb(1, b"jpeg")
            .expect("set micro");

        assert!(media_needs_thumbnail_regeneration(
            &harness.state.db,
            1,
            hash
        ));
    }

    #[test]
    fn media_needs_regeneration_when_small_thumb_file_missing() {
        let harness = TestHarness::new();
        let hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        write_thumb_file(hash, ThumbnailSize::Micro, b"micro");
        harness
            .state
            .db
            .set_micro_thumb(1, b"jpeg")
            .expect("set micro");

        assert!(media_needs_thumbnail_regeneration(
            &harness.state.db,
            1,
            hash
        ));
    }

    #[test]
    fn media_needs_regeneration_when_db_micro_thumb_is_none() {
        let harness = TestHarness::new();
        let hash = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        write_thumb_file(hash, ThumbnailSize::Micro, b"micro");
        write_thumb_file(hash, ThumbnailSize::Small, b"small");

        assert!(media_needs_thumbnail_regeneration(
            &harness.state.db,
            1,
            hash
        ));
    }

    #[test]
    fn media_needs_regeneration_when_db_micro_thumb_is_empty() {
        let harness = TestHarness::new();
        let hash = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        write_thumb_file(hash, ThumbnailSize::Micro, b"micro");
        write_thumb_file(hash, ThumbnailSize::Small, b"small");
        harness
            .state
            .db
            .set_micro_thumb(1, &[])
            .expect("set empty micro");

        assert!(media_needs_thumbnail_regeneration(
            &harness.state.db,
            1,
            hash
        ));
    }

    #[test]
    fn media_needs_regeneration_false_when_all_thumbnails_exist() {
        let harness = TestHarness::new();
        let hash = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        write_thumb_file(hash, ThumbnailSize::Micro, b"micro");
        write_thumb_file(hash, ThumbnailSize::Small, b"small");
        let media_id = insert_media(&harness, "complete.png", Some(hash), None);
        harness
            .state
            .db
            .set_micro_thumb(media_id, b"jpeg-bytes")
            .expect("set micro");

        assert!(!media_needs_thumbnail_regeneration(
            &harness.state.db,
            media_id,
            hash
        ));
    }

    #[test]
    fn regenerate_returns_error_for_nonexistent_media() {
        let harness = TestHarness::new();
        let result = regenerate_thumbnails_for_media(&harness.state, 999_999);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("media 999999 not found"));
    }

    #[test]
    fn regenerate_returns_false_for_media_without_hash() {
        let harness = TestHarness::new();
        let media_id = insert_media(&harness, "no-hash.png", None, None);
        let result = regenerate_thumbnails_for_media(&harness.state, media_id).unwrap();
        assert!(!result);
    }

    #[test]
    fn regenerate_returns_false_for_missing_source_file() {
        let harness = TestHarness::new();
        let hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let missing = harness.media_dir.join("gone.png");
        let media_id = insert_media(&harness, "gone.png", Some(hash), Some(&missing));

        assert!(media_needs_thumbnail_regeneration(
            &harness.state.db,
            media_id,
            hash
        ));
        let result = regenerate_thumbnails_for_media(&harness.state, media_id).unwrap();
        assert!(!result);
    }

    #[test]
    fn thumbnail_regen_progress_serializes() {
        let progress = ThumbnailRegenProgress {
            processed: 10,
            total: 100,
            regenerated: 3,
            status: "running".to_string(),
        };
        let json = serde_json::to_string(&progress).expect("serialize progress");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(value["processed"], 10);
        assert_eq!(value["total"], 100);
        assert_eq!(value["regenerated"], 3);
        assert_eq!(value["status"], "running");
    }

    #[test]
    fn thumbnail_regen_result_serializes() {
        let result = ThumbnailRegenResult { regenerated: 42 };
        let json = serde_json::to_string(&result).expect("serialize result");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(value["regenerated"], 42);
    }

    #[test]
    fn regenerate_skips_when_all_thumbnails_already_exist() {
        let harness = TestHarness::new();
        let hash = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        write_thumb_file(hash, ThumbnailSize::Micro, b"micro");
        write_thumb_file(hash, ThumbnailSize::Small, b"small");
        let media_id = insert_media(&harness, "complete.png", Some(hash), None);
        harness
            .state
            .db
            .set_micro_thumb(media_id, b"jpeg-bytes")
            .expect("set micro");

        let result = regenerate_thumbnails_for_media(&harness.state, media_id).unwrap();
        assert!(!result);
    }

    #[test]
    fn media_needs_regeneration_when_hash_is_empty_string() {
        let harness = TestHarness::new();
        assert!(media_needs_thumbnail_regeneration(&harness.state.db, 1, ""));
    }
}
