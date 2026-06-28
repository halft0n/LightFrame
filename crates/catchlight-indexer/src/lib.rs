pub mod scanner;
pub mod watcher;

use catchlight_core::Result;
use std::path::{Path, PathBuf};

const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif",
    "avif", "svg", "ico", "raw", "cr2", "cr3", "nef", "arw", "dng", "orf",
    "rw2", "pef", "raf",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mts",
    "m2ts", "ts",
];

pub fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            PHOTO_EXTENSIONS.contains(&lower.as_str()) || VIDEO_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

pub fn classify_extension(path: &Path) -> catchlight_core::media::MediaType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return catchlight_core::media::MediaType::Video;
    }

    let raw_exts = ["raw", "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "pef", "raf"];
    if raw_exts.contains(&ext.as_str()) {
        return catchlight_core::media::MediaType::Raw;
    }

    if PHOTO_EXTENSIONS.contains(&ext.as_str()) {
        return catchlight_core::media::MediaType::Photo;
    }

    catchlight_core::media::MediaType::Unknown
}

pub async fn scan_folder(folder: &Path) -> Result<Vec<PathBuf>> {
    scanner::scan(folder).await
}
