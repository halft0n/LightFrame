use crate::state::AppState;
use catchlight_indexer::{FolderWatcher, is_media_change_event};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tracing::{debug, warn};

const DEBOUNCE: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, serde::Serialize)]
pub struct FolderChangedPayload {
    pub folder_id: i64,
}

pub struct WatchManager {
    active: Arc<AtomicBool>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl WatchManager {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            task: Mutex::new(None),
        }
    }

    pub fn active_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.active)
    }
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn start(app: &AppHandle, state: &AppState) -> catchlight_core::Result<()> {
    stop(state)?;

    let folders = state.db.list_watched_folders()?;
    if folders.is_empty() {
        debug!("no watched folders, skipping folder watcher");
        return Ok(());
    }

    let mut watchers = Vec::new();
    for folder in &folders {
        let path = Path::new(&folder.path);
        if !path.is_dir() {
            warn!(
                path = %folder.path,
                "watched folder path is not a directory, skipping watch"
            );
            continue;
        }
        match FolderWatcher::new(path) {
            Ok(watcher) => watchers.push((folder.id, watcher)),
            Err(e) => warn!(path = %folder.path, "failed to watch folder: {e}"),
        }
    }

    if watchers.is_empty() {
        return Ok(());
    }

    let active = state.watch_manager.active_flag();
    active.store(true, Ordering::SeqCst);

    let app = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        watch_loop(app, watchers, active).await;
    });

    *state
        .watch_manager
        .task
        .lock()
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))? = Some(handle);

    debug!(count = folders.len(), "folder watcher started");
    Ok(())
}

pub fn stop(state: &AppState) -> catchlight_core::Result<()> {
    state.watch_manager.active.store(false, Ordering::SeqCst);

    let handle = state
        .watch_manager
        .task
        .lock()
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?
        .take();

    if let Some(handle) = handle {
        handle.abort();
    }

    Ok(())
}

async fn watch_loop(
    app: AppHandle,
    mut watchers: Vec<(i64, FolderWatcher)>,
    active: Arc<AtomicBool>,
) {
    let mut pending: HashMap<i64, Instant> = HashMap::new();

    while active.load(Ordering::SeqCst) {
        for (folder_id, watcher) in &mut watchers {
            while let Some(event) = watcher.try_recv() {
                if is_media_change_event(&event) {
                    pending.insert(*folder_id, Instant::now() + DEBOUNCE);
                }
            }
        }

        let now = Instant::now();
        let ready: Vec<i64> = pending
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(id, _)| *id)
            .collect();

        for folder_id in ready {
            pending.remove(&folder_id);
            let payload = FolderChangedPayload { folder_id };
            if let Err(e) = app.emit("folder-changed", &payload) {
                warn!(folder_id, "failed to emit folder-changed: {e}");
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
