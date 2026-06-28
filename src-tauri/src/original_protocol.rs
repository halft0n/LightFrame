use http::{header, StatusCode};
use std::path::Path;
use tauri::http::Response;

pub fn handle(request_path: &str) -> Response<Vec<u8>> {
    let raw = request_path
        .trim_start_matches('/')
        .trim_start_matches("localhost/");

    let decoded = percent_decode(raw);
    let file_path = Path::new(&decoded);

    if !file_path.exists() {
        return error_response(StatusCode::NOT_FOUND, "file not found");
    }

    match std::fs::read(file_path) {
        Ok(bytes) => {
            let mime = guess_mime(file_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "max-age=3600")
                .body(bytes)
                .unwrap()
        }
        Err(e) => {
            tracing::warn!("original protocol read error: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "read failed")
        }
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
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(val);
                i += 3;
                continue;
            }
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
        .body(message.as_bytes().to_vec())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    fn encode_uri_component(path: &str) -> String {
        path.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
                | b'*' | b'\'' | b'(' | b')' => (b as char).to_string(),
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
        assert_eq!(percent_decode("/home/user/photo.jpg"), "/home/user/photo.jpg");
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
        assert_eq!(guess_mime(Path::new("a.unknown")), "application/octet-stream");
        assert_eq!(guess_mime(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn handle_missing_file_returns_404() {
        let resp = handle("/nonexistent/path/photo.jpg");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_serves_existing_file_with_mime() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sample.png");
        std::fs::write(&file, b"\x89PNG\r\n").unwrap();

        let resp = handle(&request_path_for_file(&file));

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
        let file = dir.path().join("test.jpg");
        std::fs::write(&file, b"jpeg-data").unwrap();

        let encoded = encode_uri_component(file.to_str().unwrap());
        let request_path = format!("/localhost/{encoded}");
        let resp = handle(&request_path);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), b"jpeg-data".to_vec());
    }
}
