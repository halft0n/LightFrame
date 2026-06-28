use crate::state::AppState;
use catchlight_db::WatchedFolder;
use http::{StatusCode, header};
use std::path::{Path, PathBuf};
use tauri::http::Response;

pub fn handle(state: &AppState, request_path: &str) -> Response<Vec<u8>> {
    tracing::debug!("original protocol request: {request_path}");

    let raw = strip_scheme_path(request_path);

    let decoded = percent_decode(raw);
    let file_path = normalize_file_path(&decoded);

    let watched_folders = match state.db.list_watched_folders() {
        Ok(folders) => folders,
        Err(e) => {
            tracing::error!("original protocol: failed to list watched folders: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    if !path_is_in_watched_folders(&file_path, &watched_folders) {
        return error_response(StatusCode::FORBIDDEN, "path not allowed");
    }

    if !file_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "file not found");
    }

    match std::fs::read(&file_path) {
        Ok(bytes) => {
            let mime = guess_mime(&file_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "max-age=3600")
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes)
                .unwrap()
        }
        Err(e) => {
            tracing::warn!(path = %file_path.display(), "original protocol read error: {e}");
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
    let root = Path::new(folder);
    if path == root {
        return true;
    }

    let mut root_components = root.components();
    for component in path.components() {
        match root_components.next() {
            Some(expected) if expected == component => continue,
            Some(_) => return false,
            None => return true,
        }
    }
    false
}

fn strip_scheme_path(request_path: &str) -> &str {
    let mut path = request_path.trim_start_matches('/');
    if let Some(rest) = path.strip_prefix("localhost/") {
        path = rest;
    } else if path == "localhost" {
        path = "";
    }
    path.trim_start_matches('/')
}

fn normalize_file_path(decoded: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let normalized = decoded.replace('/', "\\");
        let trimmed = normalized.trim_start_matches('\\');
        if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
            PathBuf::from(trimmed)
        } else {
            PathBuf::from(normalized)
        }
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(decoded)
    }
}

fn guess_mime(path: &Path) -> &'static str {
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

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .header("Access-Control-Allow-Origin", "*")
        .body(message.as_bytes().to_vec())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use catchlight_core::config::AppConfig;
    use catchlight_db::Database;
    use http::StatusCode;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state_with_watched_dir(dir: &Path) -> AppState {
        let db = Arc::new(Database::open(Path::new(":memory:")).expect("in-memory db"));
        db.add_watched_folder(dir.to_str().unwrap())
            .expect("add watched folder");
        AppState {
            db,
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            scanning: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(catchlight_ai::AiDispatcher::new())),
        }
    }

    fn encode_uri_component(path: &str) -> String {
        path.bytes()
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
                | b')' => (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    }

    fn request_path_for_file(file: &Path) -> String {
        format!("/{}", encode_uri_component(file.to_str().unwrap()))
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
    fn handle_strips_localhost_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(dir.path());
        let file = dir.path().join("test.jpg");
        std::fs::write(&file, b"jpeg-data").unwrap();

        let encoded = encode_uri_component(file.to_str().unwrap());
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
        let file = dir.path().join("file#1?test&.jpg");
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
            scanning: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(catchlight_ai::AiDispatcher::new())),
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

    #[test]
    #[cfg(unix)]
    fn handle_symlink_inside_watched_folder_serves_target_content() {
        let watched = tempfile::tempdir().unwrap();
        let state = test_state_with_watched_dir(watched.path());

        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("outside.jpg");
        std::fs::write(&secret, b"outside").unwrap();

        let link = watched.path().join("link.jpg");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        let resp = handle(&state, &request_path_for_file(&link));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"outside".to_vec());
    }
}
