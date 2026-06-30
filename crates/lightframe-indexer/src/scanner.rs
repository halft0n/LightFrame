use crate::is_media_file;
use lightframe_core::Result;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::info;

pub async fn scan_with_walkdir(root: &Path) -> Result<Vec<PathBuf>> {
    scan(root).await
}

pub async fn scan(root: &Path) -> Result<Vec<PathBuf>> {
    let root = root.to_path_buf();

    let files = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && is_media_file(entry.path()) {
                results.push(entry.into_path());
            }
        }
        results
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))?;

    info!(count = files.len(), "scan complete");
    Ok(files)
}

/// Stream-based discovery: yields file paths as they are found instead of
/// waiting for the entire directory tree to be enumerated.
pub fn scan_streaming(root: &Path) -> mpsc::Receiver<PathBuf> {
    let (tx, rx) = mpsc::channel(512);
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && is_media_file(entry.path())
                && tx.blocking_send(entry.into_path()).is_err()
            {
                break;
            }
        }
    });
    rx
}
