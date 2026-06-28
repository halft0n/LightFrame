use crate::is_media_file;
use catchlight_core::Result;
use std::path::{Path, PathBuf};
use tracing::info;

pub async fn scan(root: &Path) -> Result<Vec<PathBuf>> {
    let root = root.to_path_buf();

    let files = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(true)
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
    .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

    info!(count = files.len(), "scan complete");
    Ok(files)
}
