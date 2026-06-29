use crate::state::AppState;
use catchlight_db::WatchedFolder;
use http::{StatusCode, header};
use std::path::{Path, PathBuf};
use tauri::http::Response;

const MAX_SERVE_SIZE: u64 = 500 * 1024 * 1024; // 500MB

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

    let canonical = match std::fs::canonicalize(&file_path) {
        Ok(p) => strip_extended_prefix(p),
        Err(e) => {
            tracing::warn!("original protocol: canonicalize failed: {e}");
            return error_response(StatusCode::NOT_FOUND, "file not found");
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

    if metadata.len() > MAX_SERVE_SIZE {
        return error_response(StatusCode::from_u16(413).unwrap(), "file too large");
    }

    match std::fs::read(&canonical) {
        Ok(bytes) => {
            let mime = guess_mime(&canonical);
            finish_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CACHE_CONTROL, "max-age=3600")
                    .header("Access-Control-Allow-Origin", "*"),
                bytes,
            )
        }
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

fn path_contains_parent_dir(path: &Path) -> bool {
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
                pa.as_os_str().to_ascii_lowercase() == pb.as_os_str().to_ascii_lowercase()
            }
            (Component::Normal(na), Component::Normal(nb)) => {
                na.to_ascii_lowercase() == nb.to_ascii_lowercase()
            }
            _ => false,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (a, b);
        false
    }
}

fn strip_extended_prefix(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(stripped) = s.strip_prefix("\\\\?\\") {
            return PathBuf::from(stripped);
        }
    }
    p
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
    if looks_like_windows_path(decoded) || decoded.contains('\\') {
        PathBuf::from(normalize_windows_path_str(decoded))
    } else {
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

fn finish_response(builder: http::response::Builder, body: Vec<u8>) -> Response<Vec<u8>> {
    builder.body(body).unwrap_or_else(|_| {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(b"internal error".to_vec())
            .expect("hardcoded response must build")
    })
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    finish_response(
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/plain")
            .header("Access-Control-Allow-Origin", "*"),
        message.as_bytes().to_vec(),
    )
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
            scanning: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(catchlight_ai::AiDispatcher::new())),
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
        f.set_len(MAX_SERVE_SIZE + 1).unwrap();
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
}
