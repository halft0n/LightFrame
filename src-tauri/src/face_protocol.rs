use crate::original_protocol::{path_is_in_watched_folders, strip_extended_prefix};
use crate::protocol_utils::{cors_headers, error_response, ok_response, strip_scheme_path};
use crate::state::AppState;
use http::{StatusCode, header};
use image::GenericImageView;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tauri::http::Response;

fn face_cache_path(state: &AppState, face_id: i64) -> PathBuf {
    state.face_cache_dir.join(format!("{face_id}.jpg"))
}

fn jpeg_response(body: Vec<u8>) -> Response<Vec<u8>> {
    ok_response(
        cors_headers(
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/jpeg")
                .header(header::CACHE_CONTROL, "max-age=86400"),
        ),
        body,
    )
}

fn read_face_cache(state: &AppState, face_id: i64) -> Option<Vec<u8>> {
    let path = face_cache_path(state, face_id);
    match std::fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() => Some(bytes),
        _ => None,
    }
}

fn write_face_cache(state: &AppState, face_id: i64, jpeg: &[u8]) {
    let path = face_cache_path(state, face_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, jpeg);
}

/// Remove cached face crops for all faces of a given media.
/// Call this when media is deleted, moved, or re-scanned.
pub fn invalidate_face_cache_for_media(state: &AppState, media_id: i64) {
    let faces = match state.db.get_faces_for_media(media_id) {
        Ok(f) => f,
        Err(_) => return,
    };
    for face in faces {
        let path = face_cache_path(state, face.id);
        let _ = std::fs::remove_file(path);
    }
}

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

    // Quick watched-folder check using the DB path (not canonicalized yet)
    let path_plausible = file_path
        .parent()
        .map(|p| path_is_in_watched_folders(p, &watched_folders))
        .unwrap_or(false)
        || path_is_in_watched_folders(file_path, &watched_folders);

    if !path_plausible {
        return error_response(StatusCode::FORBIDDEN, "path not allowed");
    }

    // Serve from disk cache after basic security check
    if let Some(cached) = read_face_cache(state, face_id) {
        // Even on cache hit, verify the source media is still in watched folders
        let canonical = match std::fs::canonicalize(file_path) {
            Ok(p) => strip_extended_prefix(p),
            Err(_) => {
                return error_response(StatusCode::FORBIDDEN, "path not allowed");
            }
        };
        if !path_is_in_watched_folders(&canonical, &watched_folders) {
            tracing::warn!(
                "face protocol: cached face canonical path {} escapes watched folders",
                canonical.display()
            );
            return error_response(StatusCode::FORBIDDEN, "path not allowed");
        }
        return jpeg_response(cached);
    }

    let canonical = match std::fs::canonicalize(file_path) {
        Ok(p) => strip_extended_prefix(p),
        Err(_) => {
            return error_response(StatusCode::NOT_FOUND, "source file missing");
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

    const MAX_FACE_SOURCE_SIZE: u64 = 100 * 1024 * 1024;
    match std::fs::metadata(&canonical) {
        Ok(m) if m.len() > MAX_FACE_SOURCE_SIZE => {
            return error_response(StatusCode::from_u16(413).unwrap(), "source image too large");
        }
        Err(_) => {
            return error_response(StatusCode::NOT_FOUND, "file not found");
        }
        _ => {}
    }

    let img = match lightframe_core::decode::decode_image(&canonical) {
        Ok(decoded) => match decoded.to_dynamic_image() {
            Ok(img) => img,
            Err(e) => {
                tracing::warn!(path = %media.path, "failed to decode image for face crop: {e}");
                return error_response(StatusCode::NOT_FOUND, "failed to decode image");
            }
        },
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

    let jpeg = buf.into_inner();
    write_face_cache(state, face_id, &jpeg);
    jpeg_response(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol_utils::{error_response, strip_scheme_path};
    use http::StatusCode;
    use lightframe_core::config::AppConfig;
    use lightframe_core::media::{MediaFile, MediaType};
    use lightframe_db::{Database, FaceDetectionInput};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_state() -> AppState {
        let face_cache = tempfile::tempdir().unwrap();
        AppState {
            db: Arc::new(Database::open(std::path::Path::new(":memory:")).expect("in-memory db")),
            config: AppConfig::default(),
            scan_status: crate::state::ScanStatus::new(),
            scan_concurrency: 2,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: std::sync::Arc::new(crate::thumb_cache::ThumbCache::new()),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: face_cache.into_path(),
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
        let canonical_dir =
            strip_extended_prefix(std::fs::canonicalize(dir).expect("canonicalize dir"));
        let folder_id = state
            .db
            .add_watched_folder(canonical_dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let file = canonical_dir.join("face_source.jpg");
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
        let canonical_dir = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
        let folder_id = state
            .db
            .add_watched_folder(canonical_dir.to_str().unwrap())
            .unwrap()
            .id;
        let missing_path = canonical_dir.join("gone.jpg");
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
    fn handle_valid_face_crop_uses_disk_cache_on_second_request() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [20.0, 30.0, 80.0, 90.0]);

        let cache_path = face_cache_path(&state, face_id);
        let _ = std::fs::remove_file(&cache_path);

        let first = handle(&state, &format!("/{face_id}"));
        assert_eq!(first.status(), StatusCode::OK);
        assert!(cache_path.exists(), "first request should write face cache");

        let second = handle(&state, &format!("/{face_id}"));
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(first.body(), second.body());

        let _ = std::fs::remove_file(&cache_path);
    }

    #[test]
    fn handle_cache_hit_rejects_when_source_removed() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [20.0, 30.0, 80.0, 90.0]);

        let cache_path = face_cache_path(&state, face_id);
        let _ = std::fs::remove_file(&cache_path);

        let first = handle(&state, &format!("/{face_id}"));
        assert_eq!(first.status(), StatusCode::OK);
        assert!(cache_path.exists());

        std::fs::remove_file(dir.path().join("face_source.jpg")).unwrap();

        let second = handle(&state, &format!("/{face_id}"));
        assert_eq!(second.status(), StatusCode::FORBIDDEN);
        assert_eq!(String::from_utf8_lossy(second.body()), "path not allowed");

        let _ = std::fs::remove_file(&cache_path);
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

    fn insert_face_for_media_path(
        state: &AppState,
        watched_dir: &std::path::Path,
        media_path: &std::path::Path,
        bbox: [f32; 4],
    ) -> i64 {
        let canonical_dir =
            strip_extended_prefix(std::fs::canonicalize(watched_dir).expect("canonicalize dir"));
        let folder_id = state
            .db
            .add_watched_folder(canonical_dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let media = MediaFile {
            id: 0,
            path: media_path.to_string_lossy().to_string(),
            filename: media_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
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
        state.db.get_faces_for_media(media_id).expect("get faces")[0].id
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

        let ok = ok_response(
            cors_headers(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(http::header::CONTENT_TYPE, "image/jpeg")
                    .header(http::header::CACHE_CONTROL, "max-age=86400"),
            ),
            b"jpeg".to_vec(),
        );
        assert_eq!(
            ok.headers()
                .get("Access-Control-Allow-Origin")
                .and_then(|v| v.to_str().ok()),
            Some("*")
        );
    }

    #[test]
    fn handle_path_traversal_in_url_returns_bad_request() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [10.0, 10.0, 50.0, 50.0]);

        for path in [
            "/../../etc/passwd",
            &format!("/{face_id}/../../../etc/passwd"),
            "/%2e%2e%2f%2e%2e%2fetc/passwd",
            "/..%2f..%2f123",
            "/123/extra/segments",
        ] {
            let resp = handle(&state, path);
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn handle_null_byte_in_face_id_path_is_rejected() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [10.0, 10.0, 50.0, 50.0]);

        let mut path = face_id.to_string();
        path.insert(1, '\0');
        let resp = handle(&state, &format!("/{path}"));
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handle_malformed_and_overflow_face_ids_return_bad_request() {
        let state = test_state();

        for path in [
            "/",
            "",
            "/not-a-number",
            "/12.34",
            "/0x10",
            "/9223372036854775808",
            &format!("/{}", "9".repeat(500)),
            &format!("/{}", "1".repeat(100)),
        ] {
            let resp = handle(&state, path);
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn handle_forbidden_outside_watched_folder() {
        let state = test_state();
        let watched = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join("secret.jpg");
        write_test_jpeg(&file, 100, 100);

        let face_id =
            insert_face_for_media_path(&state, watched.path(), &file, [0.0, 0.0, 50.0, 50.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert_eq!(String::from_utf8_lossy(resp.body()), "path not allowed");
    }

    #[test]
    #[cfg(unix)]
    fn handle_symlink_escape_outside_watched_folder_returns_forbidden() {
        let state = test_state();
        let watched = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("outside.jpg");
        write_test_jpeg(&secret, 100, 100);

        let link = watched.path().join("link.jpg");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        let face_id =
            insert_face_for_media_path(&state, watched.path(), &link, [0.0, 0.0, 50.0, 50.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn handle_negative_bbox_is_clamped_to_valid_crop() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [-50.0, -30.0, 80.0, 90.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.body().starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn handle_zero_area_bbox_still_returns_jpeg() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [50.0, 50.0, 0.0, 0.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.body().starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn handle_valid_face_crop_includes_cors_header() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let (face_id, _) = insert_face_media(&state, dir.path(), [20.0, 30.0, 80.0, 90.0]);

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("Access-Control-Allow-Origin")
                .and_then(|v| v.to_str().ok()),
            Some("*")
        );
    }

    #[test]
    fn handle_oversized_source_file_returns_413() {
        let state = test_state();
        let dir = tempfile::tempdir().unwrap();
        let canonical_dir =
            strip_extended_prefix(std::fs::canonicalize(dir.path()).expect("canonicalize dir"));
        let folder_id = state
            .db
            .add_watched_folder(canonical_dir.to_str().unwrap())
            .expect("add folder")
            .id;
        let file = canonical_dir.join("huge.jpg");
        write_test_jpeg(&file, 100, 100);
        let f = std::fs::OpenOptions::new().write(true).open(&file).unwrap();
        f.set_len(100 * 1024 * 1024 + 1).unwrap();
        drop(f);

        let media = MediaFile {
            id: 0,
            path: file.to_string_lossy().to_string(),
            filename: "huge.jpg".to_string(),
            media_type: MediaType::Photo,
            size_bytes: 100 * 1024 * 1024 + 1,
            width: Some(100),
            height: Some(100),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: Some("hugehash".to_string()),
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
                    confidence: 0.99,
                    embedding: vec![1.0, 0.0],
                }],
            )
            .unwrap();
        let face_id = state.db.get_faces_for_media(media_id).unwrap()[0].id;

        let resp = handle(&state, &format!("/{face_id}"));
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            String::from_utf8_lossy(resp.body()),
            "source image too large"
        );
    }
}
