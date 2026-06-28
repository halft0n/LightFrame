use catchlight_core::config::{self, AppConfig};
use catchlight_db::Database;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub folder_id: i64,
    pub total: i64,
    pub processed: i64,
    pub phase: String,
}

#[derive(Clone)]
pub struct ScanStatus {
    folder_id: Arc<AtomicI64>,
    total: Arc<AtomicI64>,
    processed: Arc<AtomicI64>,
    phase: Arc<Mutex<String>>,
}

impl ScanStatus {
    pub fn new() -> Self {
        Self {
            folder_id: Arc::new(AtomicI64::new(0)),
            total: Arc::new(AtomicI64::new(0)),
            processed: Arc::new(AtomicI64::new(0)),
            phase: Arc::new(Mutex::new("idle".to_string())),
        }
    }

    pub fn reset(&self, folder_id: i64) {
        self.folder_id.store(folder_id, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.processed.store(0, Ordering::Relaxed);
        *self.phase.lock().expect("scan phase mutex poisoned") = "discovering".to_string();
    }

    pub fn set_total(&self, total: i64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn set_phase(&self, phase: &str) {
        *self.phase.lock().expect("scan phase mutex poisoned") = phase.to_string();
    }

    pub fn increment_processed(&self) -> i64 {
        self.processed.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn snapshot(&self) -> ScanProgress {
        ScanProgress {
            folder_id: self.folder_id.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            processed: self.processed.load(Ordering::Relaxed),
            phase: self
                .phase
                .lock()
                .expect("scan phase mutex poisoned")
                .clone(),
        }
    }
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AppState {
    pub db: Arc<Database>,
    pub config: AppConfig,
    pub scan_status: ScanStatus,
    pub scan_semaphore: Arc<Semaphore>,
    pub scanning: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> catchlight_core::Result<Self> {
        let db = Arc::new(Database::open_default()?);
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
            scan_semaphore: Arc::new(Semaphore::new(concurrency)),
            scanning: Arc::new(AtomicBool::new(false)),
        })
    }
}

fn load_config() -> AppConfig {
    let path = config::config_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
    }
    AppConfig::default()
}
