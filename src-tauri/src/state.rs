use lightframe_ai::AiDispatcher;
use lightframe_core::config::{self, AppConfig};
use lightframe_db::Database;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub folder_id: i64,
    pub total: i64,
    pub scanned: i64,
    pub status: String,
}

#[derive(Clone)]
pub struct ScanStatus {
    folder_id: Arc<AtomicI64>,
    total: Arc<AtomicI64>,
    scanned: Arc<AtomicI64>,
    status: Arc<Mutex<String>>,
}

impl ScanStatus {
    pub fn new() -> Self {
        Self {
            folder_id: Arc::new(AtomicI64::new(0)),
            total: Arc::new(AtomicI64::new(0)),
            scanned: Arc::new(AtomicI64::new(0)),
            status: Arc::new(Mutex::new("idle".to_string())),
        }
    }

    pub fn reset(&self, folder_id: i64) {
        self.folder_id.store(folder_id, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.scanned.store(0, Ordering::Relaxed);
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

    pub fn snapshot(&self) -> ScanProgress {
        ScanProgress {
            folder_id: self.folder_id.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            scanned: self.scanned.load(Ordering::Relaxed),
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

pub struct AppState {
    pub db: Arc<Database>,
    pub config: AppConfig,
    pub scan_status: ScanStatus,
    pub scan_concurrency: usize,
    pub scanning: Arc<AtomicBool>,
    pub face_detecting: Arc<AtomicBool>,
    pub watch_manager: WatchManager,
    pub thumb_cache: ThumbCache,
    pub ai: Arc<tokio::sync::Mutex<AiDispatcher>>,
}

impl AppState {
    pub fn new() -> lightframe_core::Result<Self> {
        let db = Arc::new(Database::open_default()?);
        let _ = db.cleanup_deleted_older_than(30);
        let config = load_config();
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
            scanning: Arc::new(AtomicBool::new(false)),
            face_detecting: Arc::new(AtomicBool::new(false)),
            watch_manager: WatchManager::new(),
            thumb_cache: ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(AiDispatcher::new())),
        })
    }
}

fn load_config() -> AppConfig {
    let path = config::config_path();
    if path.exists()
        && let Ok(data) = std::fs::read_to_string(&path)
        && let Ok(cfg) = serde_json::from_str(&data)
    {
        return cfg;
    }
    AppConfig::default()
}
