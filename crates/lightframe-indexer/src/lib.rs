pub mod scanner;
pub mod watcher;

#[cfg(target_os = "windows")]
mod mft;

pub use watcher::{
    FolderWatcher, is_media_change_event, is_media_remove_event, is_media_rename_event,
};

use lightframe_core::Result;
use std::path::{Path, PathBuf};

const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "heic", "heif", "avif", "svg", "ico",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "mts", "m2ts", "ts",
];

#[cfg(target_os = "windows")]
fn windows_media_extensions() -> Vec<&'static str> {
    PHOTO_EXTENSIONS
        .iter()
        .chain(lightframe_core::decode::RAW_EXTENSIONS.iter())
        .chain(VIDEO_EXTENSIONS.iter())
        .copied()
        .collect()
}

#[cfg(target_os = "windows")]
fn get_volume_letter(path: &Path) -> char {
    path.to_str()
        .and_then(|s| s.chars().next())
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('C')
}

pub fn is_media_file(path: &Path) -> bool {
    if lightframe_core::decode::is_raw_path(path) {
        return true;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            PHOTO_EXTENSIONS.contains(&lower.as_str()) || VIDEO_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

pub fn classify_extension(path: &Path) -> lightframe_core::media::MediaType {
    if lightframe_core::decode::is_raw_path(path) {
        return lightframe_core::media::MediaType::Raw;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return lightframe_core::media::MediaType::Video;
    }

    if PHOTO_EXTENSIONS.contains(&ext.as_str()) {
        return lightframe_core::media::MediaType::Photo;
    }

    lightframe_core::media::MediaType::Unknown
}

pub async fn scan_folder(folder: &Path) -> Result<Vec<PathBuf>> {
    #[cfg(target_os = "windows")]
    {
        let extensions = windows_media_extensions();
        if let Ok(scanner) = mft::MftScanner::new(get_volume_letter(folder))
            && let Ok(entries) = scanner.scan_media_files(&extensions)
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
        use lightframe_core::decode::RAW_EXTENSIONS;

        for ext in RAW_EXTENSIONS {
            let name = format!("sample.{ext}");
            assert_eq!(
                classify_extension(Path::new(&name)),
                MediaType::Raw,
                ".{ext} should classify as Raw"
            );
        }
    }

    #[test]
    fn is_media_file_detects_all_raw_extensions() {
        use lightframe_core::decode::RAW_EXTENSIONS;

        for ext in RAW_EXTENSIONS {
            let name = format!("photo.{ext}");
            assert!(
                is_media_file(Path::new(&name)),
                ".{ext} should be detected as media"
            );
        }
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

    #[tokio::test]
    async fn scan_folder_with_only_non_media_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("notes.pdf"), b"%PDF").unwrap();
        std::fs::write(dir.path().join("archive.zip"), b"PK").unwrap();

        let results = scan_folder(dir.path()).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn scan_folder_with_deeply_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let mut nested = dir.path().to_path_buf();
        for i in 0..20 {
            nested = nested.join(format!("level-{i}"));
            std::fs::create_dir_all(&nested).unwrap();
        }
        let photo = nested.join("deep.jpg");
        std::fs::write(&photo, b"fake jpeg").unwrap();

        let results = scan_folder(dir.path()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], photo);
    }

    #[test]
    fn is_media_file_rejects_double_extension_backup() {
        assert!(!is_media_file(Path::new("photo.jpg.bak")));
        assert!(!is_media_file(Path::new("clip.mp4.old")));
    }

    #[test]
    fn is_media_file_detects_hidden_prefixed_media_names() {
        assert!(is_media_file(Path::new(".hidden.jpg")));
        assert!(is_media_file(Path::new(".secret.png")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn mft_scanner_placeholder_returns_empty() {
        let scanner = mft::MftScanner::new('C').unwrap();
        let entries = scanner.scan_media_files(&["jpg"]).unwrap();
        assert!(entries.is_empty());
    }
}
