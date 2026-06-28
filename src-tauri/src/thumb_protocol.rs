use crate::state::AppState;
use catchlight_core::media::ThumbnailSize;
use catchlight_thumbnail::thumb_path;
use http::{StatusCode, header};
use std::path::Path;
use tauri::http::Response;

pub fn handle(state: &AppState, request_path: &str) -> Response<Vec<u8>> {
    tracing::debug!("thumb protocol request: {request_path}");

    let path = strip_scheme_path(request_path);

    let Some((media_id_str, size_str)) = path.split_once('/') else {
        tracing::warn!(request_path, normalized = path, "invalid thumb URL path");
        return error_response(StatusCode::BAD_REQUEST, "invalid thumb URL");
    };

    let Ok(media_id) = media_id_str.parse::<i64>() else {
        tracing::warn!(media_id_str, "invalid media id in thumb URL");
        return error_response(StatusCode::BAD_REQUEST, "invalid media id");
    };

    let Some(size) = parse_size(size_str) else {
        tracing::warn!(size_str, "invalid thumbnail size in thumb URL");
        return error_response(StatusCode::BAD_REQUEST, "invalid size");
    };

    if let Some(cached) = state.thumb_cache.get(media_id, size) {
        return ok_response(cached, content_type_for(size));
    }

    let media = match state.db.get_media_by_id(media_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            tracing::debug!(media_id, "thumb protocol: media not found");
            return error_response(StatusCode::NOT_FOUND, "media not found");
        }
        Err(e) => {
            tracing::error!(media_id, "thumb protocol db error: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    if matches!(size, ThumbnailSize::Micro)
        && let Ok(Some(blob)) = state.db.get_micro_thumb(media_id)
    {
        state.thumb_cache.insert(media_id, size, blob.clone());
        return ok_response(blob, "image/jpeg");
    }

    let Some(hash) = media.blake3_hash else {
        tracing::warn!(media_id, path = %media.path, "no hash for media");
        return error_response(StatusCode::NOT_FOUND, "no hash for media");
    };

    let cache_path = thumb_path(&hash, size);
    if cache_path.exists() {
        return match std::fs::read(&cache_path) {
            Ok(bytes) => {
                state.thumb_cache.insert(media_id, size, bytes.clone());
                ok_response(bytes, "image/webp")
            }
            Err(e) => {
                tracing::warn!(
                    ?cache_path,
                    media_id,
                    "failed to read cached thumbnail: {e}"
                );
                error_response(StatusCode::NOT_FOUND, "thumbnail not found")
            }
        };
    }

    let src = Path::new(&media.path);
    if !src.exists() {
        tracing::warn!(media_id, path = %media.path, "source file missing for thumbnail");
        return error_response(StatusCode::NOT_FOUND, "source file missing");
    }

    match catchlight_thumbnail::generate(src, &hash, size) {
        Ok(generated) => match std::fs::read(&generated) {
            Ok(bytes) => {
                state.thumb_cache.insert(media_id, size, bytes.clone());
                ok_response(bytes, "image/webp")
            }
            Err(e) => {
                tracing::warn!(
                    ?generated,
                    media_id,
                    "failed to read generated thumbnail: {e}"
                );
                error_response(StatusCode::NOT_FOUND, "thumbnail not found")
            }
        },
        Err(e) => {
            tracing::warn!(media_id, path = %media.path, ?size, "thumbnail generation failed: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "generation failed")
        }
    }
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

fn parse_size(s: &str) -> Option<ThumbnailSize> {
    match s {
        "micro" => Some(ThumbnailSize::Micro),
        "small" => Some(ThumbnailSize::Small),
        "large" => Some(ThumbnailSize::Large),
        _ => None,
    }
}

fn content_type_for(size: ThumbnailSize) -> &'static str {
    match size {
        ThumbnailSize::Micro => "image/jpeg",
        ThumbnailSize::Small | ThumbnailSize::Large => "image/webp",
    }
}

fn ok_response(bytes: Vec<u8>, content_type: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .unwrap()
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
    use catchlight_core::media::{MediaFile, MediaType};
    use catchlight_db::Database;
    use http::StatusCode;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state() -> AppState {
        AppState {
            db: Arc::new(Database::open(std::path::Path::new(":memory:")).expect("in-memory db")),
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            scanning: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(catchlight_ai::AiDispatcher::new())),
        }
    }

    fn insert_media(state: &AppState, dir: &std::path::Path, hash: Option<&str>) -> i64 {
        let folder_id = state
            .db
            .add_watched_folder(dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let file = dir.join("photo.jpg");
        std::fs::write(&file, b"fake-jpeg").expect("write source");
        let media = MediaFile {
            id: 0,
            path: file.to_string_lossy().to_string(),
            filename: "photo.jpg".to_string(),
            media_type: MediaType::Photo,
            size_bytes: 9,
            width: Some(100),
            height: Some(100),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: hash.map(str::to_string),
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        };
        state
            .db
            .upsert_media(folder_id, &media)
            .expect("upsert media")
    }

    #[test]
    fn strip_scheme_path_variants() {
        assert_eq!(strip_scheme_path("/123/small"), "123/small");
        assert_eq!(strip_scheme_path("/localhost/123/small"), "123/small");
        assert_eq!(strip_scheme_path("localhost/123/small"), "123/small");
        assert_eq!(strip_scheme_path("//localhost//123/small"), "123/small");
        assert_eq!(strip_scheme_path("/localhost"), "");
    }

    #[test]
    fn parse_size_accepts_known_values() {
        assert_eq!(parse_size("micro"), Some(ThumbnailSize::Micro));
        assert_eq!(parse_size("small"), Some(ThumbnailSize::Small));
        assert_eq!(parse_size("large"), Some(ThumbnailSize::Large));
        assert_eq!(parse_size("xlarge"), None);
    }

    #[test]
    fn handle_invalid_media_id_returns_400() {
        let state = test_state();
        let resp = handle(&state, "/not-a-number/small");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(String::from_utf8_lossy(resp.body()), "invalid media id");
    }

    #[test]
    fn handle_invalid_size_returns_400() {
        let state = test_state();
        let resp = handle(&state, "/42/huge");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(String::from_utf8_lossy(resp.body()), "invalid size");
    }

    #[test]
    fn handle_missing_path_segment_returns_400() {
        let state = test_state();
        let resp = handle(&state, "/42");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(String::from_utf8_lossy(resp.body()), "invalid thumb URL");
    }

    #[test]
    fn handle_empty_request_path_returns_400() {
        let state = test_state();
        let resp = handle(&state, "");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handle_extra_slashes_still_parses() {
        let state = test_state();
        let resp = handle(&state, "///localhost///999/small");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_very_large_media_id_not_found() {
        let state = test_state();
        let resp = handle(&state, &format!("/{}/small", i64::MAX));
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(String::from_utf8_lossy(resp.body()), "media not found");
    }

    #[test]
    fn handle_media_not_found_returns_404() {
        let state = test_state();
        let resp = handle(&state, "/404/small");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_missing_blake3_hash_returns_404() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state();
        let media_id = insert_media(&state, dir.path(), None);
        let resp = handle(&state, &format!("/{media_id}/small"));
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(String::from_utf8_lossy(resp.body()), "no hash for media");
    }

    #[test]
    fn handle_cache_hit_returns_cached_bytes() {
        let state = test_state();
        let cached = vec![0xFF, 0xD8, 0xFF, 0xD9];
        state
            .thumb_cache
            .insert(7, ThumbnailSize::Small, cached.clone());
        let resp = handle(&state, "/7/small");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), cached);
        assert_eq!(
            resp.headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("image/webp")
        );
    }

    #[test]
    fn handle_micro_cache_hit_uses_jpeg_content_type() {
        let state = test_state();
        let cached = vec![0xFF, 0xD8, 0xFF, 0xD9];
        state
            .thumb_cache
            .insert(8, ThumbnailSize::Micro, cached.clone());
        let resp = handle(&state, "/8/micro");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("image/jpeg")
        );
    }

    #[test]
    fn handle_micro_db_blob_before_disk_cache() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state();
        let media_id = insert_media(&state, dir.path(), Some("abc123"));
        let blob = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x01];
        state.db.set_micro_thumb(media_id, &blob).unwrap();
        let resp = handle(&state, &format!("/{media_id}/micro"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(*resp.body(), blob);
    }
}
