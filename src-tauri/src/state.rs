use lightframe_ai::AiDispatcher;
use lightframe_core::config::{self, AppConfig};
use lightframe_core::media::ThumbnailSize;
use lightframe_db::Database;
use lightframe_thumbnail::thumb_path;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub folder_id: i64,
    pub total: i64,
    pub scanned: i64,
    pub errors: i64,
    pub status: String,
}

#[derive(Clone)]
pub struct ScanStatus {
    folder_id: Arc<AtomicI64>,
    total: Arc<AtomicI64>,
    scanned: Arc<AtomicI64>,
    errors: Arc<AtomicI64>,
    status: Arc<Mutex<String>>,
}

impl ScanStatus {
    pub fn new() -> Self {
        Self {
            folder_id: Arc::new(AtomicI64::new(0)),
            total: Arc::new(AtomicI64::new(0)),
            scanned: Arc::new(AtomicI64::new(0)),
            errors: Arc::new(AtomicI64::new(0)),
            status: Arc::new(Mutex::new("idle".to_string())),
        }
    }

    pub fn reset(&self, folder_id: i64) {
        self.folder_id.store(folder_id, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.scanned.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = "scanning".to_string();
    }

    pub fn set_total(&self, total: i64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn set_status(&self, status: &str) {
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status.to_string();
    }

    pub fn increment_scanned(&self) -> i64 {
        self.scanned.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn increment_errors(&self) -> i64 {
        self.errors.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn snapshot(&self) -> ScanProgress {
        ScanProgress {
            folder_id: self.folder_id.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            scanned: self.scanned.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            status: self
                .status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        }
    }
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self::new()
    }
}

use crate::thumb_cache::ThumbCache;
use crate::watcher::WatchManager;
use std::sync::Mutex as StdMutex;
use tokio::sync::mpsc;

pub struct ScanQueue {
    tx: mpsc::UnboundedSender<i64>,
    rx: StdMutex<Option<mpsc::UnboundedReceiver<i64>>>,
    running: Arc<AtomicBool>,
    worker_started: AtomicBool,
}

impl ScanQueue {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: StdMutex::new(Some(rx)),
            running: Arc::new(AtomicBool::new(false)),
            worker_started: AtomicBool::new(false),
        }
    }

    pub fn enqueue(&self, folder_id: i64) {
        if let Err(e) = self.tx.send(folder_id) {
            tracing::warn!(folder_id, "failed to enqueue scan: {e}");
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub(crate) fn try_start_worker(&self) -> Option<mpsc::UnboundedReceiver<i64>> {
        if self
            .worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }
        self.rx.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub(crate) fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
}

impl Default for ScanQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AppState {
    pub db: Arc<Database>,
    pub config: AppConfig,
    pub scan_status: ScanStatus,
    pub scan_concurrency: usize,
    pub scan_queue: ScanQueue,
    pub face_detecting: Arc<AtomicBool>,
    pub dedup_scanning: Arc<AtomicBool>,
    pub thumb_regenerating: Arc<AtomicBool>,
    pub watch_manager: WatchManager,
    pub thumb_cache: ThumbCache,
    pub ai: Arc<tokio::sync::Mutex<AiDispatcher>>,
}

const TRASH_RETENTION_DAYS: i64 = 30;

pub(crate) fn purge_expired_trash(db: &Database) {
    let watched_folders = match db.list_watched_folders() {
        Ok(folders) => folders,
        Err(e) => {
            tracing::warn!("startup: failed to list watched folders for trash cleanup: {e}");
            return;
        }
    };

    let expired_items = match db.list_expired_deleted_media(TRASH_RETENTION_DAYS) {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!("startup: failed to list expired deleted media: {e}");
            return;
        }
    };

    for (path, hash) in expired_items {
        let file_path = std::path::Path::new(&path);
        let check_path = match file_path.canonicalize() {
            Ok(raw) => crate::original_protocol::strip_extended_prefix(raw),
            Err(_) => file_path.to_path_buf(),
        };
        if !crate::original_protocol::path_is_in_watched_folders(&check_path, &watched_folders) {
            tracing::warn!("trash cleanup: skipping {path} (outside watched folders)");
            continue;
        }
        if file_path.is_file()
            && let Err(e) = std::fs::remove_file(file_path)
        {
            tracing::warn!("trash cleanup: failed to remove {path}: {e}");
        }
        if let Some(hash) = hash.filter(|h| h.len() >= 4) {
            for size in [
                ThumbnailSize::Micro,
                ThumbnailSize::Small,
                ThumbnailSize::Large,
            ] {
                let thumb = thumb_path(&hash, size);
                let _ = std::fs::remove_file(&thumb);
            }
        }
    }

    if let Err(e) = db.cleanup_deleted_older_than(TRASH_RETENTION_DAYS) {
        tracing::warn!("startup: failed to cleanup deleted media: {e}");
    }
}

impl AppState {
    pub fn new() -> lightframe_core::Result<Self> {
        let db = Arc::new(Database::open_default()?);
        {
            let db_clone = Arc::clone(&db);
            std::thread::spawn(move || purge_expired_trash(&db_clone));
        }
        let config = config::load_config();
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let concurrency = ((cpus as f64) * 0.7).ceil() as usize;
        let concurrency = concurrency.clamp(2, 16);

        Ok(Self {
            db,
            config,
            scan_status: ScanStatus::new(),
            scan_concurrency: concurrency,
            scan_queue: ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            watch_manager: WatchManager::new(),
            thumb_cache: ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(AiDispatcher::new())),
        })
    }
}

#[cfg(test)]
pub(crate) fn load_config_from_path(path: &std::path::Path) -> AppConfig {
    config::load_config_from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightframe_core::media::{MediaFile, MediaType};

    fn sample_media(path: &str) -> MediaFile {
        MediaFile {
            id: 0,
            path: path.to_string(),
            filename: path.rsplit('/').next().unwrap_or(path).to_string(),
            media_type: MediaType::Photo,
            size_bytes: 1024,
            width: Some(100),
            height: Some(100),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: Some("abcd1234".to_string()),
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        }
    }

    fn set_deleted_at_days_ago(db: &Database, media_id: i64, days_ago: i64) {
        let conn = db.conn().unwrap();
        let sql = format!(
            "UPDATE media_files SET deleted_at = datetime('now', '-{days_ago} days') WHERE id = {media_id}"
        );
        conn.execute_batch(&sql).unwrap();
    }

    #[test]
    fn scan_queue_starts_not_running() {
        let queue = ScanQueue::new();
        assert!(!queue.is_running());
    }

    #[test]
    fn scan_queue_is_running_default_false() {
        let queue = ScanQueue::new();
        assert!(!queue.is_running());
    }

    #[test]
    fn scan_queue_enqueue_multiple_folders() {
        let queue = ScanQueue::new();
        queue.enqueue(10);
        queue.enqueue(20);
        queue.enqueue(30);

        let mut rx = queue.try_start_worker().expect("worker should start once");
        assert_eq!(rx.try_recv().unwrap(), 10);
        assert_eq!(rx.try_recv().unwrap(), 20);
        assert_eq!(rx.try_recv().unwrap(), 30);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn scan_queue_try_start_only_once() {
        let queue = ScanQueue::new();
        assert!(queue.try_start_worker().is_some());
        assert!(queue.try_start_worker().is_none());
    }

    #[test]
    fn scan_queue_running_flag_shared() {
        use std::sync::atomic::Ordering;

        let queue = ScanQueue::new();
        assert!(!queue.is_running());

        let flag = queue.running_flag();
        flag.store(true, Ordering::SeqCst);
        assert!(queue.is_running());

        flag.store(false, Ordering::SeqCst);
        assert!(!queue.is_running());
    }

    #[test]
    fn scan_status_tracks_progress_and_errors() {
        let status = ScanStatus::new();
        status.reset(42);
        status.set_total(10);
        status.increment_scanned();
        status.increment_errors();
        status.set_status("complete");

        let snap = status.snapshot();
        assert_eq!(snap.folder_id, 42);
        assert_eq!(snap.total, 10);
        assert_eq!(snap.scanned, 1);
        assert_eq!(snap.errors, 1);
        assert_eq!(snap.status, "complete");
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-config.json");
        assert!(!missing.exists());

        let cfg = load_config_from_path(&missing);
        assert_eq!(cfg.locale, "zh-CN");
        assert_eq!(cfg.thumbnail_quality, 85);
        assert!(!cfg.ai_enabled);
    }

    #[test]
    fn load_config_invalid_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{not valid json").unwrap();

        let cfg = load_config_from_path(&path);
        assert_eq!(cfg.locale, "zh-CN");
        assert_eq!(cfg.log.level, "info");
    }

    #[test]
    fn load_config_valid_json_loads_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{"locale":"en-US","thumbnail_quality":92,"ai_enabled":true,"python_path":null,"log":{"level":"debug","retention_days":21,"max_size_mb":250}}"#,
        )
        .unwrap();

        let cfg = load_config_from_path(&path);
        assert_eq!(cfg.locale, "en-US");
        assert_eq!(cfg.thumbnail_quality, 92);
        assert!(cfg.ai_enabled);
        assert_eq!(cfg.log.level, "debug");
        assert_eq!(cfg.log.retention_days, 21);
        assert_eq!(cfg.log.max_size_mb, 250);
    }

    #[test]
    fn load_config_empty_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "").unwrap();

        let cfg = load_config_from_path(&path);
        assert_eq!(cfg.locale, "zh-CN");
    }

    #[test]
    fn purge_expired_trash_removes_files_inside_watched_folders() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        std::fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        );
        let watched_str = watched_canonical.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Database::open(&db_path).unwrap();
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;

        let inside_path = watched_canonical.join("expired.jpg");
        std::fs::write(&inside_path, b"jpeg-data").unwrap();
        let inside_str = inside_path.to_str().unwrap().to_string();

        let media_id = db
            .upsert_media(folder_id, &sample_media(&inside_str))
            .unwrap();
        db.set_deleted(media_id, true).unwrap();
        set_deleted_at_days_ago(&db, media_id, 40);

        assert!(inside_path.is_file());
        purge_expired_trash(&db);
        assert!(
            !inside_path.exists(),
            "expired file inside watched folder should be removed"
        );
    }

    #[test]
    fn purge_expired_trash_skips_files_outside_watched_folders() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&watched).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let watched_str = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        )
        .to_str()
        .unwrap()
        .to_string();
        let outside_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&outside).unwrap(),
        );
        let outside_path = outside_canonical.join("expired.jpg");
        std::fs::write(&outside_path, b"jpeg-data").unwrap();
        let outside_str = outside_path.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Database::open(&db_path).unwrap();
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;

        let media_id = db
            .upsert_media(folder_id, &sample_media(&outside_str))
            .unwrap();
        db.set_deleted(media_id, true).unwrap();
        set_deleted_at_days_ago(&db, media_id, 40);

        purge_expired_trash(&db);
        assert!(
            outside_path.is_file(),
            "files outside watched folders must not be deleted during trash cleanup"
        );
    }

    #[test]
    fn purge_expired_trash_cleans_db_records() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        std::fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        );
        let watched_str = watched_canonical.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Database::open(&db_path).unwrap();
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;

        let file_path = watched_canonical.join("old.jpg");
        std::fs::write(&file_path, b"jpeg").unwrap();
        let media_id = db
            .upsert_media(folder_id, &sample_media(&file_path.to_string_lossy()))
            .unwrap();
        db.set_deleted(media_id, true).unwrap();
        set_deleted_at_days_ago(&db, media_id, 45);

        purge_expired_trash(&db);
        assert!(db.get_media_by_id(media_id).unwrap().is_none());
    }

    #[test]
    fn purge_expired_trash_leaves_recently_deleted_files() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        std::fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        );
        let watched_str = watched_canonical.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Database::open(&db_path).unwrap();
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;

        let file_path = watched_canonical.join("recent.jpg");
        std::fs::write(&file_path, b"jpeg").unwrap();
        let media_id = db
            .upsert_media(folder_id, &sample_media(&file_path.to_string_lossy()))
            .unwrap();
        db.set_deleted(media_id, true).unwrap();

        purge_expired_trash(&db);
        assert!(
            file_path.is_file(),
            "recently deleted file should not be removed from disk"
        );
        let conn = db.conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM media_files WHERE id = ?1",
                [media_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "recently deleted record should still exist in DB");
    }
}
