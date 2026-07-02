use crate::protocol_utils::{cors_headers, error_response, ok_response, strip_scheme_path};
use crate::state::AppState;
use http::{StatusCode, header};
use lightframe_db::WatchedFolder;
use std::path::{Path, PathBuf};
use tauri::http::Response;

const MAX_IMAGE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for images
// Tauri 2 custom protocol handlers return `Response<Vec<u8>>`; the body is always buffered in memory.
// Video playback should use `convertFileSrc` (asset protocol), not original://.
const MAX_VIDEO_SIZE: u64 = 200 * 1024 * 1024; // 200MB for videos
const MAX_INLINE_READ_BYTES: u64 = 10 * 1024 * 1024; // warn above 10MB in-memory reads

pub fn handle(state: &AppState, request_path: &str) -> Response<Vec<u8>> {
    tracing::debug!("original protocol request: {request_path}");

    let raw = strip_scheme_path(request_path);

    let decoded = percent_decode(raw);
    let file_path = normalize_file_path(&decoded);

    if path_contains_parent_dir(&file_path) {
        tracing::warn!(path = %file_path.display(), "original protocol: path traversal rejected");
        return error_response(StatusCode::FORBIDDEN, "path not allowed");
    }

    if decoded.contains('\0') || path_contains_null_byte(&file_path) {
        tracing::warn!(path = %file_path.display(), "original protocol: null byte in path rejected");
        return error_response(StatusCode::BAD_REQUEST, "invalid path");
    }

    let watched_folders = state.watched_folders_cache.get(&state.db);

    let canonical = match std::fs::canonicalize(&file_path) {
        Ok(p) => strip_extended_prefix(p),
        Err(_) => {
            // File doesn't exist — distinguish 403 vs 404 by checking parent
            let parent_in_watched = file_path
                .parent()
                .and_then(|p| std::fs::canonicalize(p).ok())
                .map(strip_extended_prefix)
                .map(|cp| path_is_in_watched_folders(&cp, &watched_folders))
                .unwrap_or(false)
                || path_is_in_watched_folders(&file_path, &watched_folders);

            return if parent_in_watched {
                error_response(StatusCode::NOT_FOUND, "file not found")
            } else {
                error_response(StatusCode::FORBIDDEN, "path not allowed")
            };
        }
    };

    if !path_is_in_watched_folders(&canonical, &watched_folders) {
        tracing::warn!(
            "original protocol: canonical path {} escapes watched folders",
            canonical.display()
        );
        return error_response(StatusCode::FORBIDDEN, "path not allowed");
    }

    let metadata = match std::fs::metadata(&canonical) {
        Ok(m) => m,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "file not found"),
    };

    let mime = guess_mime(&canonical);
    let is_video = mime.starts_with("video/");
    let limit = if is_video {
        MAX_VIDEO_SIZE
    } else {
        MAX_IMAGE_SIZE
    };
    if metadata.len() > limit {
        return error_response(StatusCode::from_u16(413).unwrap(), "file too large");
    }
    if metadata.len() >= MAX_INLINE_READ_BYTES {
        tracing::warn!(
            size = metadata.len(),
            path = %canonical.display(),
            "serving very large file via in-memory read; consider convertFileSrc for video"
        );
    }

    if lightframe_core::decode::is_raw_path(&canonical) {
        match lightframe_core::decode::extract_raw_preview_bytes(&canonical) {
            Ok(jpeg) => {
                return ok_response(
                    cors_headers(
                        Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "image/jpeg")
                            .header(header::CACHE_CONTROL, "max-age=3600"),
                    ),
                    jpeg,
                );
            }
            Err(e) => {
                tracing::warn!(
                    path = %canonical.display(),
                    error = %e,
                    "RAW preview extraction failed, serving raw bytes"
                );
            }
        }
    }

    match std::fs::read(&canonical) {
        Ok(bytes) => ok_response(
            cors_headers(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CACHE_CONTROL, "max-age=3600"),
            ),
            bytes,
        ),
        Err(e) => {
            tracing::warn!(path = %canonical.display(), "original protocol read error: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "read failed")
        }
    }
}

pub fn path_is_in_watched_folders(path: &Path, folders: &[WatchedFolder]) -> bool {
    folders
        .iter()
        .any(|folder| path_is_under_folder(path, &folder.path))
}

fn path_is_under_folder(path: &Path, folder: &str) -> bool {
    let root = normalize_file_path(folder);
    if paths_equal(path, &root) {
        return true;
    }

    let mut root_components = root.components();
    for component in path.components() {
        match root_components.next() {
            Some(expected) if components_equal(expected, component) => continue,
            Some(_) => return false,
            None => return true,
        }
    }
    false
}

pub fn path_contains_parent_dir(path: &Path) -> bool {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return true;
    }
    path.to_string_lossy()
        .split(['/', '\\'])
        .any(|part| part == "..")
}

fn path_contains_null_byte(path: &Path) -> bool {
    path.as_os_str().as_encoded_bytes().contains(&0)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    #[cfg(windows)]
    {
        a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn components_equal(a: std::path::Component<'_>, b: std::path::Component<'_>) -> bool {
    if a == b {
        return true;
    }
    #[cfg(windows)]
    {
        use std::path::Component;
        match (a, b) {
            (Component::Prefix(pa), Component::Prefix(pb)) => {
                pa.as_os_str().eq_ignore_ascii_case(pb.as_os_str())
            }
            (Component::Normal(na), Component::Normal(nb)) => na.eq_ignore_ascii_case(nb),
            _ => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (a, b);
        false
    }
}

pub fn strip_extended_prefix(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(stripped) = s.strip_prefix("\\\\?\\") {
            return PathBuf::from(stripped);
        }
    }
    p
}

fn looks_like_windows_path(s: &str) -> bool {
    let s = s.trim_start_matches('/');
    if s.len() >= 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':' {
        return true;
    }
    s.starts_with("//") || s.starts_with("\\\\")
}

fn normalize_windows_path_str(decoded: &str) -> String {
    let mut s = decoded.replace('/', "\\");

    while s.starts_with('\\') {
        if s.starts_with("\\\\") {
            let after = &s[2..];
            if after.len() >= 2
                && after.as_bytes()[0].is_ascii_alphabetic()
                && after.as_bytes()[1] == b':'
            {
                s = after.to_string();
                continue;
            }
            break;
        }
        s = s[1..].to_string();
    }

    s
}

fn normalize_file_path(decoded: &str) -> PathBuf {
    let decoded = decoded.strip_prefix("\\\\?\\").unwrap_or(decoded);
    if looks_like_windows_path(decoded) || decoded.contains('\\') {
        PathBuf::from(normalize_windows_path_str(decoded))
    } else {
        PathBuf::from(decoded)
    }
}

fn guess_mime(path: &Path) -> &'static str {
    if lightframe_core::decode::is_raw_path(path) {
        return "application/x-raw";
    }

    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("tiff" | "tif") => "image/tiff",
        // SVG scripts don't execute when loaded via <img> src, which is how LightFrame renders them.
        // If embedding context changes (e.g. <object> or innerHTML), sanitize SVGs first.
        Some("svg") => "image/svg+xml",
        Some("heic" | "heif") => "image/heif",
        Some("avif") => "image/avif",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let mut result = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            result.push(val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use lightframe_core::config::AppConfig;
    use lightframe_db::{Database, WatchedFolder};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state_with_watched_dir(dir: &Path) -> AppState {
        let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let canonical_dir = strip_extended_prefix(canonical_dir);
        let db = Arc::new(Database::open(Path::new(":memory:")).expect("in-memory db"));
        db.add_watched_folder(canonical_dir.to_str().unwrap())
            .expect("add watched folder");
        AppState {
            db,
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            processing_budget: crate::state::ProcessingBudget::new(4),
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: std::sync::Arc::new(crate::thumb_cache::ThumbCache::new()),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: tempfile::tempdir().unwrap().keep(),
            watched_folders_cache: crate::state::WatchedFoldersCache::new(),
        }
    }

    fn encode_path_bytes(path: &Path) -> String {
        path.as_os_str()
            .as_encoded_bytes()
            .iter()
            .map(|b| match b {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'!'
                | b'~'
                | b'*'
                | b'\''
                | b'('
                | b')' => (*b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    }

    fn request_path_for_file(file: &Path) -> String {
        format!("/{}", encode_path_bytes(file))
    }

    #[test]
    fn percent_decode_empty_string() {
        assert_eq!(percent_decode(""), "");
    }

    #[test]
    fn percent_decode_plain_path() {
        assert_eq!(
            percent_decode("/home/user/photo.jpg"),
            "/home/user/photo.jpg"
        );
    }

    #[test]
    fn percent_decode_encoded_slashes_and_spaces() {
        assert_eq!(
            percent_decode("%2Fhome%2Fuser%2Fmy%20photo.jpg"),
            "/home/user/my photo.jpg"
        );
    }

    #[test]
    fn percent_decode_only_percent_chars() {
        assert_eq!(percent_decode("%%%"), "%%%");
    }

    #[test]
    fn percent_decode_malformed_sequences() {
        assert_eq!(percent_decode("%GG%2Z%"), "%GG%2Z%");
        assert_eq!(percent_decode("%2"), "%2");
    }

    #[test]
    fn guess_mime_common_formats() {
        assert_eq!(guess_mime(Path::new("a.jpg")), "image/jpeg");
        assert_eq!(guess_mime(Path::new("a.JPEG")), "image/jpeg");
        assert_eq!(guess_mime(Path::new("a.png")), "image/png");
        assert_eq!(guess_mime(Path::new("a.mp4")), "video/mp4");
        assert_eq!(guess_mime(Path::new("a.mov")), "video/quicktime");
        assert_eq!(guess_mime(Path::new("a.heic")), "image/heif");
        assert_eq!(
            guess_mime(Path::new("a.unknown")),
            "application/octet-stream"
        );
        assert_eq!(guess_mime(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn path_is_under_folder_rejects_prefix_collisions() {
        assert!(path_is_under_folder(
            Path::new("/photos/vacation.jpg"),
            "/photos"
        ));
        assert!(!path_is_under_folder(
            Path::new("/photos2/vacation.jpg"),
            "/photos"
        ));
    }

    fn watched_folder(path: &str) -> WatchedFolder {
        WatchedFolder {
            id: 1,
            path: path.to_string(),
            media_count: 0,
            last_scan: None,
            scan_status: "idle".to_string(),
        }
    }

    #[test]
    fn path_is_in_watched_folders_nested_and_exact() {
        let folders = vec![watched_folder("/photos")];
        assert!(path_is_in_watched_folders(
            Path::new("/photos/vacation.jpg"),
            &folders
        ));
        assert!(path_is_in_watched_folders(Path::new("/photos"), &folders));
        assert!(path_is_in_watched_folders(
            Path::new("/photos/2024/event/nested.jpg"),
            &folders
        ));
    }

    #[test]
    fn path_is_in_watched_folders_rejects_outside_and_prefix_collision() {
        let folders = vec![watched_folder("/photos")];
        assert!(!path_is_in_watched_folders(
            Path::new("/other/secret.jpg"),
            &folders
        ));
        assert!(!path_is_in_watched_folders(
            Path::new("/photos-backup/copy.jpg"),
            &folders
        ));
    }

    #[test]
    fn path_is_in_watched_folders_empty_list() {
        assert!(!path_is_in_watched_folders(
            Path::new("/photos/vacation.jpg"),
            &[]
        ));
    }

    #[test]
    #[cfg(windows)]
    fn strip_extended_prefix_removes_win32_prefix() {
        let stripped = strip_extended_prefix(PathBuf::from(r"\\?\C:\Users\photo.jpg"));
        assert_eq!(stripped.to_string_lossy(), r"C:\Users\photo.jpg");
    }

    #[test]
    fn strip_extended_prefix_leaves_clean_path() {
        let path = PathBuf::from("/home/user/photos/sample.jpg");
        assert_eq!(strip_extended_prefix(path.clone()), path);
    }

    #[test]
    #[cfg(unix)]
    fn strip_extended_prefix_unix_unchanged() {
        let path = PathBuf::from("/tmp/lightframe/test.jpg");
        assert_eq!(strip_extended_prefix(path.clone()), path);
    }

    #[test]
    fn normalize_windows_path_strips_uri_leading_slashes() {
        assert_eq!(
            normalize_windows_path_str("/C:/Users/王子胖/Pictures/photo.jpg"),
            "C:\\Users\\王子胖\\Pictures\\photo.jpg"
        );
        assert_eq!(
            normalize_windows_path_str("//C:/Users/photo.jpg"),
            "C:\\Users\\photo.jpg"
        );
        assert_eq!(
            normalize_windows_path_str("C:/Users/photo.jpg"),
            "C:\\Users\\photo.jpg"
        );
    }

    #[test]
    fn normalize_windows_path_preserves_unc() {
        assert_eq!(
            normalize_windows_path_str("//server/share/photo.jpg"),
            "\\\\server\\share\\photo.jpg"
        );
        assert_eq!(
            normalize_windows_path_str("\\\\server\\share\\photo.jpg"),
            "\\\\server\\share\\photo.jpg"
        );
    }

    #[test]
    fn normalize_file_path_handles_drive_letter() {
        let path = normalize_file_path("/C:/Users/test/photo.jpg");
        assert_eq!(path.to_string_lossy(), "C:\\Users\\test\\photo.jpg");
    }

    #[test]
    fn path_contains_parent_dir_detects_traversal() {
        assert!(path_contains_parent_dir(Path::new("/photos/../secret.jpg")));
        assert!(path_contains_parent_dir(Path::new(
            "C:\\photos\\..\\secret.jpg"
        )));
        assert!(!path_contains_parent_dir(Path::new("/photos/vacation.jpg")));
    }

    #[test]
    fn handle_path_traversal_returns_forbidden() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let traversal = dir.path().join("..").join("etc").join("passwd");
        let resp = handle(&state, &request_path_for_file(&traversal));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn percent_decode_chinese_utf8() {
        let encoded = "%E7%85%A7%E7%89%87.jpg";
        assert_eq!(percent_decode(encoded), "照片.jpg");
    }

    #[test]
    fn ok_and_error_responses_include_cors_header() {
        let err = error_response(StatusCode::NOT_FOUND, "missing");
        assert_eq!(
            err.headers()
                .get("Access-Control-Allow-Origin")
                .and_then(|v| v.to_str().ok()),
            Some("*")
        );
    }

    #[test]
    fn handle_missing_file_returns_404() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let missing = dir.path().join("missing.jpg");
        let resp = handle(&state, &request_path_for_file(&missing));
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_forbidden_outside_watched_folder() {
        let watched = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(watched.path());

        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join("secret.png");
        std::fs::write(&file, b"\x89PNG\r\n").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn handle_serves_existing_file_with_mime() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("sample.png");
        std::fs::write(&file, b"\x89PNG\r\n").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("image/png")
        );
        assert_eq!(*resp.body(), b"\x89PNG\r\n".to_vec());
    }

    #[test]
    fn handle_serves_raw_preview_as_jpeg() {
        use image::{ImageBuffer, Rgb};

        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("sample.cr2");

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 30) as u8, (y * 30) as u8, 128]));
        let mut jpeg = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .expect("encode jpeg");

        let mut data = b"fake-cr2-header".to_vec();
        data.extend_from_slice(&jpeg);
        std::fs::write(&file, &data).unwrap();

        let resp = handle(&state, &request_path_for_file(&file));

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("image/jpeg")
        );
        assert!(resp.body().starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn handle_strips_localhost_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("test.jpg");
        std::fs::write(&file, b"jpeg-data").unwrap();

        let encoded = encode_path_bytes(&file);
        let request_path = format!("/localhost/{encoded}");
        let resp = handle(&state, &request_path);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"jpeg-data".to_vec());
    }

    #[test]
    fn handle_chinese_characters_in_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("照片.jpg");
        std::fs::write(&file, b"chinese-photo").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"chinese-photo".to_vec());
    }

    #[test]
    fn handle_spaces_in_filename() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("my photo.jpg");
        std::fs::write(&file, b"spaced-name").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"spaced-name".to_vec());
    }

    #[test]
    fn handle_special_characters_in_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        // Avoid ? on Windows — it's illegal in NTFS filenames
        let name = if cfg!(windows) {
            "file#1_test&.jpg"
        } else {
            "file#1?test&.jpg"
        };
        let file = dir.path().join(name);
        std::fs::write(&file, b"special").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"special".to_vec());
    }

    #[test]
    fn handle_empty_watched_folders_list_returns_forbidden() {
        let db = Arc::new(Database::open(Path::new(":memory:")).expect("in-memory db"));
        let state = AppState {
            db,
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            processing_budget: crate::state::ProcessingBudget::new(4),
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: std::sync::Arc::new(crate::thumb_cache::ThumbCache::new()),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: tempfile::tempdir().unwrap().keep(),
            watched_folders_cache: crate::state::WatchedFoldersCache::new(),
        };

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("orphan.jpg");
        std::fs::write(&file, b"orphan").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn handle_long_filename_in_watched_folder() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let long_name = format!("{}.jpg", "a".repeat(200));
        let file = dir.path().join(&long_name);
        std::fs::write(&file, b"long-path").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"long-path".to_vec());
    }

    /// On Windows, junctions are handled by `canonicalize()` resolving the real path.
    /// The symlink escape test is Unix-only because junction creation requires elevated privileges.
    #[test]
    #[cfg(unix)]
    fn handle_symlink_escape_outside_watched_folder_returns_forbidden() {
        let watched = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(watched.path());

        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("outside.jpg");
        std::fs::write(&secret, b"outside").unwrap();

        let link = watched.path().join("link.jpg");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        let resp = handle(&state, &request_path_for_file(&link));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    #[cfg(unix)]
    fn handle_symlink_within_watched_folder_serves_target_content() {
        let watched = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(watched.path());

        let target = watched.path().join("real.jpg");
        std::fs::write(&target, b"inside").unwrap();

        let link = watched.path().join("link.jpg");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let resp = handle(&state, &request_path_for_file(&link));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"inside".to_vec());
    }

    #[test]
    fn handle_file_too_large_returns_413() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("huge.bin");

        let f = std::fs::File::create(&file).unwrap();
        f.set_len(MAX_IMAGE_SIZE + 1).unwrap();
        drop(f);

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn handle_empty_path_returns_not_found_or_forbidden() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let resp = handle(&state, "");
        assert!(
            resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::FORBIDDEN,
            "empty path should not succeed, got {}",
            resp.status()
        );
    }

    #[test]
    fn handle_url_encoded_path_segments() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let sub = dir.path().join("my photos");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("vacation pic.jpg");
        std::fs::write(&file, b"encoded-path").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"encoded-path".to_vec());
    }

    #[test]
    fn handle_very_long_path_in_watched_folder() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let nested = dir
            .path()
            .join("a".repeat(50))
            .join("b".repeat(50))
            .join("c".repeat(50));
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join(format!("{}.jpg", "x".repeat(100)));
        std::fs::write(&file, b"long-path").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"long-path".to_vec());
    }

    #[test]
    fn handle_null_byte_in_path_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("safe.jpg");
        std::fs::write(&file, b"safe").unwrap();

        let mut path_with_null = dir.path().to_string_lossy().to_string();
        path_with_null.push_str("/safe.jpg");
        path_with_null.insert(path_with_null.find("safe").unwrap(), '\0');

        let encoded: String = path_with_null
            .bytes()
            .map(|b| format!("%{b:02X}"))
            .collect();
        let resp = handle(&state, &format!("/{encoded}"));
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    #[cfg(unix)]
    fn handle_non_utf8_filename_in_watched_folder() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let name = OsStr::from_bytes(b"photo_\xFF\xFE.jpg");
        let file = dir.path().join(name);
        std::fs::write(&file, b"non-utf8").unwrap();

        let resp = handle(&state, &request_path_for_file(&file));
        assert!(
            resp.status() == StatusCode::OK || resp.status() == StatusCode::FORBIDDEN,
            "non-UTF8 path must not crash or leak; got {}",
            resp.status()
        );
    }

    #[test]
    fn handle_double_url_encoded_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("double.jpg");
        std::fs::write(&file, b"double-encoded").unwrap();

        let once = encode_path_bytes(&file);
        let twice: String = once
            .as_bytes()
            .iter()
            .map(|b| format!("%{b:02X}"))
            .collect();
        let resp = handle(&state, &format!("/{twice}"));
        assert!(
            resp.status() == StatusCode::OK
                || resp.status() == StatusCode::NOT_FOUND
                || resp.status() == StatusCode::FORBIDDEN,
            "double-encoded path should not crash, got {}",
            resp.status()
        );
    }

    #[test]
    fn handle_very_long_url_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("short.jpg");
        std::fs::write(&file, b"long-url").unwrap();

        let encoded = encode_path_bytes(&file);
        let long_request = format!("/{}{encoded}", "x".repeat(10_000));
        let resp = handle(&state, &long_request);
        assert!(
            resp.status() == StatusCode::OK
                || resp.status() == StatusCode::NOT_FOUND
                || resp.status() == StatusCode::FORBIDDEN
                || resp.status() == StatusCode::BAD_REQUEST,
            "very long URL should not crash, got {}",
            resp.status()
        );
    }

    #[test]
    fn handle_rapid_sequential_requests_do_not_crash() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("rapid.jpg");
        std::fs::write(&file, b"rapid-data").unwrap();
        let request = request_path_for_file(&file);

        for _ in 0..20 {
            let resp = handle(&state, &request);
            assert_eq!(resp.status(), StatusCode::OK);
            assert_eq!(*resp.body(), b"rapid-data".to_vec());
        }
    }
}
