use crate::original_protocol::{path_is_in_watched_folders, strip_extended_prefix};
use crate::state::AppState;
use http::{StatusCode, header};
use image::GenericImageView;
use std::io::Cursor;
use std::path::Path;
use tauri::http::Response;

pub fn handle(state: &AppState, request_path: &str) -> Response<Vec<u8>> {
    tracing::debug!("face protocol request: {request_path}");

    let path = strip_scheme_path(request_path);
    let Ok(face_id) = path.parse::<i64>() else {
        return error_response(StatusCode::BAD_REQUEST, "invalid face id");
    };

    let face = match state.db.get_face_by_id(face_id) {
        Ok(Some(f)) => f,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "face not found"),
        Err(e) => {
            tracing::error!(face_id, "face protocol db error: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    let media = match state.db.get_media_by_id(face.media_id) {
        Ok(Some(m)) => m,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "media not found"),
        Err(e) => {
            tracing::error!(media_id = face.media_id, "face protocol db error: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    let watched_folders = match state.db.list_watched_folders() {
        Ok(folders) => folders,
        Err(e) => {
            tracing::error!("face protocol: failed to list watched folders: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error");
        }
    };

    let file_path = Path::new(&media.path);
    let canonical = match std::fs::canonicalize(file_path) {
        Ok(p) => strip_extended_prefix(p),
        Err(_) => {
            let parent_in_watched = file_path
                .parent()
                .and_then(|p| std::fs::canonicalize(p).ok())
                .map(strip_extended_prefix)
                .map(|cp| path_is_in_watched_folders(&cp, &watched_folders))
                .unwrap_or(false)
                || path_is_in_watched_folders(file_path, &watched_folders);

            return if parent_in_watched {
                error_response(StatusCode::NOT_FOUND, "source file missing")
            } else {
                error_response(StatusCode::FORBIDDEN, "path not allowed")
            };
        }
    };

    if !path_is_in_watched_folders(&canonical, &watched_folders) {
        tracing::warn!(
            "face protocol: canonical path {} escapes watched folders",
            canonical.display()
        );
        return error_response(StatusCode::FORBIDDEN, "path not allowed");
    }

    if !canonical.is_file() {
        return error_response(StatusCode::NOT_FOUND, "source file missing");
    }

    let img = match image::open(&canonical) {
        Ok(img) => img,
        Err(e) => {
            tracing::warn!(path = %media.path, "failed to open image for face crop: {e}");
            return error_response(StatusCode::NOT_FOUND, "failed to open image");
        }
    };

    let (img_w, img_h) = img.dimensions();
    let x = face.bbox_x.max(0.0).floor() as u32;
    let y = face.bbox_y.max(0.0).floor() as u32;
    let w = face.bbox_w.max(1.0).ceil() as u32;
    let h = face.bbox_h.max(1.0).ceil() as u32;
    let x = x.min(img_w.saturating_sub(1));
    let y = y.min(img_h.saturating_sub(1));
    let w = w.min(img_w.saturating_sub(x)).max(1);
    let h = h.min(img_h.saturating_sub(y)).max(1);

    let crop = img.crop_imm(x, y, w, h);
    let mut buf = Cursor::new(Vec::new());
    if crop.write_to(&mut buf, image::ImageFormat::Jpeg).is_err() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to encode crop");
    }

    ok_response(buf.into_inner())
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

fn ok_response(bytes: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "max-age=86400")
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(b"internal error".to_vec())
                .expect("hardcoded response must build")
        })
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .header("Access-Control-Allow-Origin", "*")
        .body(message.as_bytes().to_vec())
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(b"internal error".to_vec())
                .expect("hardcoded response must build")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use lightframe_core::config::AppConfig;
    use lightframe_core::media::{MediaFile, MediaType};
    use lightframe_db::{Database, FaceDetectionInput};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state() -> AppState {
        AppState {
            db: Arc::new(Database::open(std::path::Path::new(":memory:")).expect("in-memory db")),
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            scanning: Arc::new(AtomicBool::new(false)),
            face_detecting: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
        }
    }

    fn write_test_jpeg(path: &std::path::Path, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgb([200, 100, 50])
            } else {
                image::Rgb([30, 60, 90])
            }
        });
        img.save(path).expect("write jpeg");
    }

    fn insert_face_media(state: &AppState, dir: &std::path::Path, bbox: [f32; 4]) -> (i64, i64) {
        let folder_id = state
            .db
            .add_watched_folder(dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let file = dir.join("face_source.jpg");
        write_test_jpeg(&file, 200, 200);
        let media = MediaFile {
            id: 0,
            path: file.to_string_lossy().to_string(),
            filename: "face_source.jpg".to_string(),
            media_type: MediaType::Photo,
            size_bytes: 4096,
            width: Some(200),
            height: Some(200),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: Some("facehash".to_string()),
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        };
        let media_id = state
            .db
            .upsert_media(folder_id, &media)
            .expect("upsert media");
        state
            .db
            .store_face_detections(
                media_id,
                &[FaceDetectionInput {
                    bbox,
                    confidence: 0.99,
                    embedding: vec![1.0, 0.0, 0.5],
                }],
            )
            .expect("store face");
        let face_id = state.db.get_faces_for_media(media_id).expect("get faces")[0].id;
        (face_id, media_id)
    }

    #[test]
    fn strip_scheme_path_variants() {
        assert_eq!(strip_scheme_path("/123"), "123");
        assert_eq!(strip_scheme_path("/localhost/456"), "456");
        assert_eq!(strip_scheme_path("localhost/789"), "789");
    }

    #[test]
    fn handle_invalid_face_id_returns_400() {
        let state = test_state();
        let resp = handle(&state, "/not-a-face");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(String::from_utf8_lossy(resp.body()), "invalid face id");
    }

    #[test]
    fn handle_missing_face_returns_404() {
        let state = test_state();
        let resp = handle(&state, "/424242");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(String::from_utf8_lossy(resp.body()), "face not found");
    }

    #[test]
    fn handle_missing_media_file_returns_404() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let folder_id = state
            .db
            .add_watched_folder(dir.path().to_str().unwrap())
            .unwrap()
            .id;
        let missing_path = dir.path().join("gone.jpg");
        let media = MediaFile {
            id: 0,
            path: missing_path.to_string_lossy().to_string(),
            filename: "gone.jpg".to_string(),
            media_type: MediaType::Photo,
            size_bytes: 100,
            width: Some(100),
            height: Some(100),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        };
        let media_id = state.db.upsert_media(folder_id, &media).unwrap();
        state
            .db
            .store_face_detections(
                media_id,
                &[FaceDetectionInput {
                    bbox: [0.0, 0.0, 50.0, 50.0],
                    confidence: 0.9,
                    embedding: vec![1.0],
                }],
            )
            .unwrap();
        let face_id = state.db.get_faces_for_media(media_id).unwrap()[0].id;

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(String::from_utf8_lossy(resp.body()), "source file missing");
    }

    #[test]
    fn handle_valid_face_crop_returns_jpeg() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [20.0, 30.0, 80.0, 90.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("image/jpeg")
        );
        assert!(resp.body().starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn handle_out_of_bounds_bbox_is_clamped() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [150.0, 150.0, 400.0, 400.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!resp.body().is_empty());
    }
}
