pub mod migrations;
pub mod repo;

pub use repo::{
    Album, DuplicateGroup, DuplicateGroupDetail, DuplicateMember, FaceDetectionInput,
    FaceDetectionRecord, LocationGroup, LocationStats, MediaNeighbors, Memory, Person, SmartAlbum,
    SmartAlbumRule, TimelineGroup, WatchedFolder,
};

use catchlight_core::config;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> catchlight_core::Result<Self> {
        let conn =
            Connection::open(path).map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_default() -> catchlight_core::Result<Self> {
        let path = config::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::open(&path)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }

    fn run_migrations(&self) -> catchlight_core::Result<()> {
        migrations::run(&self.conn())
    }
}
