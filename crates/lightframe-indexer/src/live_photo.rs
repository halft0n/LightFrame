use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a detected Live Photo pair (still image + video companion).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePhotoPair {
    pub still_path: PathBuf,
    pub video_path: PathBuf,
}

/// Detect Live Photo pairs from a list of discovered file paths.
/// Pairs are matched by same directory + same stem (case-insensitive):
///   IMG_001.HEIC + IMG_001.MOV => pair
///   IMG_002.JPG + IMG_002.MOV => pair
///
/// Only `.mov` and `.mp4` are considered as video companions.
/// Only `.heic`, `.heif`, `.jpg`, `.jpeg` are considered as still images for pairing.
pub fn detect_live_photo_pairs(paths: &[PathBuf]) -> Vec<LivePhotoPair> {
    // Group files by (directory, lowercase stem)
    let mut groups: HashMap<(PathBuf, String), Vec<&Path>> = HashMap::new();

    for path in paths {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !is_live_video_extension(ext) && !is_live_still_extension(ext) {
            continue;
        }

        let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        groups.entry((dir, stem)).or_default().push(path);
    }

    let mut pairs = Vec::new();

    for files in groups.values() {
        let mut stills: Vec<&Path> = Vec::new();
        let mut videos: Vec<&Path> = Vec::new();

        for &file in files {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if is_live_video_extension(ext) {
                videos.push(file);
            } else if is_live_still_extension(ext) {
                stills.push(file);
            }
        }

        if stills.is_empty() || videos.is_empty() {
            continue;
        }

        // Pick best still: prefer HEIC/HEIF over JPG/JPEG
        stills.sort_by_key(|p| {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "heic" | "heif" => 0,
                _ => 1,
            }
        });

        let still = stills[0];
        let video = videos[0];

        pairs.push(LivePhotoPair {
            still_path: still.to_path_buf(),
            video_path: video.to_path_buf(),
        });
    }

    pairs
}

/// Check if a file extension qualifies as a Live Photo video companion.
pub fn is_live_video_extension(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "mov" | "mp4")
}

/// Check if a file extension qualifies as a Live Photo still image.
pub fn is_live_still_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "heic" | "heif" | "jpg" | "jpeg"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn same_name_heic_and_mov_are_paired() {
        let paths = vec![p("/photos/IMG_001.HEIC"), p("/photos/IMG_001.MOV")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].still_path, p("/photos/IMG_001.HEIC"));
        assert_eq!(pairs[0].video_path, p("/photos/IMG_001.MOV"));
    }

    #[test]
    fn same_name_jpg_and_mov_are_paired() {
        let paths = vec![p("/photos/IMG_002.JPG"), p("/photos/IMG_002.MOV")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].still_path, p("/photos/IMG_002.JPG"));
        assert_eq!(pairs[0].video_path, p("/photos/IMG_002.MOV"));
    }

    #[test]
    fn case_insensitive_matching() {
        let paths = vec![p("/photos/img_003.heic"), p("/photos/IMG_003.mov")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn no_pair_when_mov_has_no_matching_still() {
        let paths = vec![p("/photos/video_only.MOV"), p("/photos/unrelated.HEIC")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn no_pair_when_heic_has_no_matching_mov() {
        let paths = vec![p("/photos/IMG_004.HEIC"), p("/photos/IMG_004.PNG")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn different_directories_are_paired_independently() {
        let paths = vec![
            p("/dir_a/IMG_001.HEIC"),
            p("/dir_a/IMG_001.MOV"),
            p("/dir_b/IMG_001.HEIC"),
            p("/dir_b/IMG_001.MOV"),
        ];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn same_directory_same_name_different_still_extensions() {
        // If both JPG and HEIC exist with a MOV, prefer HEIC
        let paths = vec![
            p("/photos/IMG_005.JPG"),
            p("/photos/IMG_005.HEIC"),
            p("/photos/IMG_005.MOV"),
        ];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 1);
        // HEIC is preferred over JPG
        assert_eq!(pairs[0].still_path, p("/photos/IMG_005.HEIC"));
    }

    #[test]
    fn mp4_video_extension_is_also_paired() {
        let paths = vec![p("/photos/clip.HEIC"), p("/photos/clip.mp4")];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].video_path, p("/photos/clip.mp4"));
    }

    #[test]
    fn multiple_pairs_in_same_directory() {
        let paths = vec![
            p("/photos/A.HEIC"),
            p("/photos/A.MOV"),
            p("/photos/B.JPG"),
            p("/photos/B.MOV"),
            p("/photos/C.PNG"), // no pair
        ];
        let pairs = detect_live_photo_pairs(&paths);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn is_live_video_extension_works() {
        assert!(is_live_video_extension("mov"));
        assert!(is_live_video_extension("MOV"));
        assert!(is_live_video_extension("mp4"));
        assert!(is_live_video_extension("MP4"));
        assert!(!is_live_video_extension("avi"));
        assert!(!is_live_video_extension("mkv"));
    }

    #[test]
    fn is_live_still_extension_works() {
        assert!(is_live_still_extension("heic"));
        assert!(is_live_still_extension("HEIF"));
        assert!(is_live_still_extension("jpg"));
        assert!(is_live_still_extension("JPEG"));
        assert!(!is_live_still_extension("png"));
        assert!(!is_live_still_extension("webp"));
    }

    #[test]
    fn empty_input_returns_no_pairs() {
        let pairs = detect_live_photo_pairs(&[]);
        assert!(pairs.is_empty());
    }

    #[test]
    fn non_media_files_are_ignored() {
        let paths = vec![p("/photos/readme.txt"), p("/photos/data.json")];
        let pairs = detect_live_photo_pairs(&paths);
        assert!(pairs.is_empty());
    }
}
