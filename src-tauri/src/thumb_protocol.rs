use crate::state::AppState;
use catchlight_core::media::ThumbnailSize;
use catchlight_thumbnail::thumb_path;
use http::{header, StatusCode};
use std::path::Path;
use tauri::http::Response;
use tauri::State;

pub fn handle(state: &State<'_, AppState>, request_path: &str) -> Response<Vec<u8>> {
    let path = request_path.trim_start_matches('/').trim_start_matches("localhost/");

    let Some((media_id_str, size_str)) = path.split_once('/') else {
        return error_response(StatusCode::BAD_REQUEST, "invalid thumb URL");
    };

    let Ok(media_id) = media_id_str.parse::<i64>() else {
        return error_response(StatusCode::BAD_REQUEST, "invalid media id");
    };

    let Some(size) = parse_size(size_str) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid size");
    };

    let media = match state.db.get_media_by_id(media_id) {
        Ok(Some(m)) => m,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "media not found"),
        Err(e) => {
            tracing::error!("thumb protocol db error: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    if matches!(size, ThumbnailSize::Micro) {
        if let Ok(Some(blob)) = state.db.get_micro_thumb(media_id) {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/jpeg")
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                .body(blob)
                .unwrap();
        }
    }

    let Some(hash) = media.blake3_hash else {
        return error_response(StatusCode::NOT_FOUND, "no hash for media");
    };

    let cache_path = thumb_path(&hash, size);
    if cache_path.exists() {
        return serve_file(&cache_path);
    }

    let src = Path::new(&media.path);
    if !src.exists() {
        return error_response(StatusCode::NOT_FOUND, "source file missing");
    }

    match catchlight_thumbnail::generate(src, &hash, size) {
        Ok(generated) => serve_file(&generated),
        Err(e) => {
            tracing::warn!(media_id, "thumbnail generation failed: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "generation failed")
        }
    }
}

fn parse_size(s: &str) -> Option<ThumbnailSize> {
    match s {
        "micro" => Some(ThumbnailSize::Micro),
        "small" => Some(ThumbnailSize::Small),
        "large" => Some(ThumbnailSize::Large),
        _ => None,
    }
}

fn serve_file(path: &Path) -> Response<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/webp")
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(bytes)
            .unwrap(),
        Err(_) => error_response(StatusCode::NOT_FOUND, "thumbnail not found"),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(message.as_bytes().to_vec())
        .unwrap()
}
