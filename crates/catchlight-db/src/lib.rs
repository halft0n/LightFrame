pub mod migrations;
pub mod repo;

pub use repo::{
    Album, DuplicateGroup, DuplicateGroupDetail, DuplicateMember, FaceDetectionInput,
    FaceDetectionRecord, LocationGroup, LocationStats, MediaNeighbors, Memory, Person, SmartAlbum,
    SmartAlbumRule, TimelineGroup, WatchedFolder,
};

use catchlight_core::config;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Database {
    writer: Arc<Mutex<Connection>>,
    reader: Arc<Mutex<Connection>>,
}

fn is_memory_db(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s == ":memory:")
}

fn apply_writer_pragmas(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "PRAGMA page_size = 4096;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -64000;
         PRAGMA busy_timeout = 5000;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 268435456;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))
}

fn apply_reader_pragmas(conn: &Connection) -> catchlight_core::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA cache_size = -32000;
         PRAGMA mmap_size = 268435456;",
    )
    .map_err(|e| catchlight_core::Error::Database(e.to_string()))
}

impl Database {
    pub fn open(path: &Path) -> catchlight_core::Result<Self> {
        let writer =
            Connection::open(path).map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        apply_writer_pragmas(&writer)?;

        if is_memory_db(path) {
            // In-memory DBs cannot be opened twice; share one connection for reads and writes.
            let shared = Arc::new(Mutex::new(writer));
            let db = Self {
                writer: Arc::clone(&shared),
                reader: shared,
            };
            db.run_migrations()?;
            return Ok(db);
        }

        let reader = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| catchlight_core::Error::Database(e.to_string()))?;
        apply_reader_pragmas(&reader)?;

        let db = Self {
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
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

    /// Exclusive write connection for mutations.
    pub fn conn(&self) -> catchlight_core::Result<std::sync::MutexGuard<'_, Connection>> {
        self.writer
            .lock()
            .map_err(|e| catchlight_core::Error::Other(format!("writer mutex: {e}")))
    }

    /// Read-only connection for queries (does not block on writer lock).
    pub fn read_conn(&self) -> catchlight_core::Result<std::sync::MutexGuard<'_, Connection>> {
        self.reader
            .lock()
            .map_err(|e| catchlight_core::Error::Other(format!("reader mutex: {e}")))
    }

    fn run_migrations(&self) -> catchlight_core::Result<()> {
        let conn = self.conn()?;
        migrations::run(&conn)
    }
}
