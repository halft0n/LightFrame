pub mod scanner;
pub mod watcher;

#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod mft;

pub use watcher::{
    FolderWatcher, is_media_change_event, is_media_remove_event, is_media_rename_event,
};

use lightframe_core::Result;
use std::path::{Path, PathBuf};

const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif", "avif", "svg",
    "ico", "raw", "cr2", "cr3", "nef", "nrw", "arw", "dng", "orf", "rw2", "pef", "raf", "rwl",
    "3fr", "srw",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mts", "m2ts", "ts",
];

#[cfg(target_os = "windows")]
const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif", "avif", "svg",
    "ico", "raw", "cr2", "cr3", "nef", "nrw", "arw", "dng", "orf", "rw2", "pef", "raf", "rwl",
    "3fr", "srw", "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mts", "m2ts",
    "ts",
];

#[cfg(target_os = "windows")]
fn get_volume_letter(path: &Path) -> char {
    path.to_str()
        .and_then(|s| s.chars().next())
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('C')
}

pub fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            PHOTO_EXTENSIONS.contains(&lower.as_str()) || VIDEO_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

pub fn classify_extension(path: &Path) -> lightframe_core::media::MediaType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return lightframe_core::media::MediaType::Video;
    }

    let raw_exts = [
        "raw", "cr2", "cr3", "nef", "nrw", "arw", "dng", "orf", "rw2", "pef", "raf", "rwl", "3fr",
        "srw",
    ];
    if raw_exts.contains(&ext.as_str()) {
        return lightframe_core::media::MediaType::Raw;
    }

    if PHOTO_EXTENSIONS.contains(&ext.as_str()) {
        return lightframe_core::media::MediaType::Photo;
    }

    lightframe_core::media::MediaType::Unknown
}

pub async fn scan_folder(folder: &Path) -> Result<Vec<PathBuf>> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(scanner) = mft::MftScanner::new(get_volume_letter(folder))
            && let Ok(entries) = scanner.scan_media_files(MEDIA_EXTENSIONS)
            && !entries.is_empty()
        {
            return Ok(entries.into_iter().map(|e| e.path).collect());
        }
    }

    scanner::scan_with_walkdir(folder).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightframe_core::media::MediaType;

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
        assert_eq!(classify_extension(Path::new("e.nrw")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("f.rwl")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("g.3fr")), MediaType::Raw);
        assert_eq!(classify_extension(Path::new("h.srw")), MediaType::Raw);
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_extension(Path::new("a.txt")), MediaType::Unknown);
        assert_eq!(classify_extension(Path::new("noext")), MediaType::Unknown);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn get_volume_letter_extracts_drive() {
        assert_eq!(get_volume_letter(Path::new("/home/user/photos")), 'C');
        assert_eq!(get_volume_letter(Path::new("D:\\Photos\\vacation")), 'D');
    }

    #[tokio::test]
    async fn scan_folder_falls_back_to_walkdir() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("test.jpg");
        std::fs::write(&photo, b"fake jpeg").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();

        let results = scan_folder(dir.path()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], photo);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn mft_scanner_placeholder_returns_empty() {
        let scanner = mft::MftScanner::new('C').unwrap();
        let entries = scanner.scan_media_files(&["jpg"]).unwrap();
        assert!(entries.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn usn_journal_placeholder_returns_empty() {
        let journal = mft::UsnJournal::new('C').unwrap();
        let changes = journal.poll_changes().unwrap();
        assert!(changes.is_empty());
    }
}
