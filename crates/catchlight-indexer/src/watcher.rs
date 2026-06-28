use catchlight_core::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::debug;

pub struct FolderWatcher {
    _watcher: RecommendedWatcher,
    pub events: mpsc::UnboundedReceiver<Event>,
}

impl FolderWatcher {
    pub fn new(folder: &Path) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

        watcher
            .watch(folder, RecursiveMode::Recursive)
            .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

        debug!(path = %folder.display(), "watching folder");

        Ok(Self {
            _watcher: watcher,
            events: rx,
        })
    }
}
