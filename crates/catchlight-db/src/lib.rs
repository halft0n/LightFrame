pub mod migrations;
pub mod repo;

pub use repo::WatchedFolder;

use catchlight_core::config;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> catchlight_core::Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
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
