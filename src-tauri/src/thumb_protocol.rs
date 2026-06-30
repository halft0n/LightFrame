use crate::protocol_utils::{cors_headers, error_response, ok_response, strip_scheme_path};
use crate::state::AppState;
use http::{StatusCode, header};
use lightframe_core::media::ThumbnailSize;
use lightframe_thumbnail::thumb_path;
use std::path::Path;
use tauri::http::Response;

/// Minimal 1×1 transparent PNG returned when a thumbnail is not yet generated.
const PLACEHOLDER_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

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
        return thumb_ok_response(cached, content_type_for(size));
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
        return thumb_ok_response(blob, "image/jpeg");
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
                thumb_ok_response(bytes, "image/webp")
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

    // Thumbnails should be pre-generated during scan; on-demand generation blocks the UI thread.
    tracing::warn!(
        media_id,
        path = %media.path,
        ?size,
        "on-demand thumbnail generation — scan may not have completed for this file"
    );
    thumb_ok_response(PLACEHOLDER_PNG.to_vec(), "image/png")
}

fn thumb_ok_response(bytes: Vec<u8>, content_type: &str) -> Response<Vec<u8>> {
    ok_response(
        cors_headers(
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable"),
        ),
        bytes,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use lightframe_core::config::AppConfig;
    use lightframe_core::media::{MediaFile, MediaType};
    use lightframe_db::Database;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state() -> AppState {
        AppState {
            db: Arc::new(Database::open(std::path::Path::new(":memory:")).expect("in-memory db")),
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            download_cancel: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
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

    #[test]
    fn ok_response_includes_cors_header_for_webview2() {
        let resp = thumb_ok_response(vec![0xFF, 0xD8], "image/jpeg");
        assert_eq!(
            resp.headers()
                .get("Access-Control-Allow-Origin")
                .and_then(|v| v.to_str().ok()),
            Some("*")
        );
        assert_eq!(
            resp.headers()
                .get(http::header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some("max-age=31536000, immutable")
        );
    }

    #[test]
    fn error_response_includes_cors_header_for_webview2() {
        let resp = error_response(StatusCode::NOT_FOUND, "not found");
        assert_eq!(
            resp.headers()
                .get("Access-Control-Allow-Origin")
                .and_then(|v| v.to_str().ok()),
            Some("*")
        );
    }

    #[test]
    fn strip_scheme_path_handles_windows_style_localhost_prefix() {
        assert_eq!(strip_scheme_path("/localhost/42/small"), "42/small");
        assert_eq!(strip_scheme_path("//localhost//42//small"), "42//small");
    }

    #[test]
    fn handle_negative_media_id_not_found() {
        let state = test_state();
        let resp = handle(&state, "/-1/small");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_zero_media_id_not_found() {
        let state = test_state();
        let resp = handle(&state, "/0/small");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn handle_path_traversal_in_media_id_segment_returns_400() {
        let state = test_state();
        let resp = handle(&state, "/../1/small");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
