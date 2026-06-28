pub mod scanner;
pub mod watcher;

pub use watcher::{
    FolderWatcher, is_media_change_event, is_media_remove_event, is_media_rename_event,
};

use catchlight_core::Result;
use std::path::{Path, PathBuf};

const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif", "avif", "svg",
    "ico", "raw", "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "pef", "raf",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mts", "m2ts", "ts",
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

    let raw_exts = [
        "raw", "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "pef", "raf",
    ];
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

#[cfg(test)]
mod tests {
    use super::*;
    use catchlight_core::media::MediaType;

    #[test]
    fn detect_photo_extensions() {
        for ext in &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "avif"] {
            let name = format!("photo.{ext}");
            let path = Path::new(&name);
            assert!(is_media_file(path), "should detect .{ext} as media");
        }
    }

    #[test]
    fn detect_video_extensions() {
        for ext in &["mp4", "mov", "avi", "mkv", "webm"] {
            let name = format!("video.{ext}");
            let path = Path::new(&name);
            assert!(is_media_file(path), "should detect .{ext} as media");
        }
    }

    #[test]
    fn reject_non_media_extensions() {
        for ext in &["txt", "pdf", "doc", "exe", "zip", "rs", "toml"] {
            let name = format!("file.{ext}");
            let path = Path::new(&name);
            assert!(!is_media_file(path), ".{ext} should not be media");
        }
    }

    #[test]
    fn case_insensitive_detection() {
        assert!(is_media_file(Path::new("PHOTO.JPG")));
        assert!(is_media_file(Path::new("Video.MP4")));
        assert!(is_media_file(Path::new("image.Png")));
    }

    #[test]
    fn no_extension_is_not_media() {
        assert!(!is_media_file(Path::new("noextension")));
        assert!(!is_media_file(Path::new(".hidden")));
    }

    #[test]
    fn classify_photo() {
        assert_eq!(classify_extension(Path::new("a.jpg")), MediaType::Photo);
        assert_eq!(classify_extension(Path::new("b.png")), MediaType::Photo);
        assert_eq!(classify_extension(Path::new("c.webp")), MediaType::Photo);
    }

    #[test]
    fn classify_video() {
        assert_eq!(classify_extension(Path::new("a.mp4")), MediaType::Video);
        assert_eq!(classify_extension(Path::new("b.mov")), MediaType::Video);
        assert_eq!(classify_extension(Path::new("c.mkv")), MediaType::Video);
    }

    #[test]
    fn classify_raw() {
        assert_eq!(classify_extension(Path::new("a.cr2")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("b.nef")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("c.dng")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("d.arw")), MediaType::Raw);
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_extension(Path::new("a.txt")), MediaType::Unknown);
        assert_eq!(classify_extension(Path::new("noext")), MediaType::Unknown);
    }
}
