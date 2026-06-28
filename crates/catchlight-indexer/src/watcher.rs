use catchlight_core::Result;
use notify::event::EventKind;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::debug;

pub fn is_media_change_event(event: &Event) -> bool {
    use crate::is_media_file;

    if !event.paths.iter().any(|p| is_media_file(p)) {
        return false;
    }

    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

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

    pub fn try_recv(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use std::path::PathBuf;

    #[test]
    fn media_create_event_is_relevant() {
        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![PathBuf::from("/photos/vacation.jpg")],
            attrs: notify::event::EventAttributes::default(),
        };
        assert!(is_media_change_event(&event));
    }

    #[test]
    fn non_media_event_is_ignored() {
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Any)),
            paths: vec![PathBuf::from("/photos/readme.txt")],
            attrs: notify::event::EventAttributes::default(),
        };
        assert!(!is_media_change_event(&event));
    }

    #[test]
    fn media_remove_event_is_relevant() {
        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![PathBuf::from("/photos/old.png")],
            attrs: notify::event::EventAttributes::default(),
        };
        assert!(is_media_change_event(&event));
    }
}
