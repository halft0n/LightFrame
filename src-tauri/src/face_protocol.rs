use crate::state::AppState;
use http::{StatusCode, header};
use image::GenericImageView;
use std::io::Cursor;
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

    let src = std::path::Path::new(&media.path);
    if !src.is_file() {
        return error_response(StatusCode::NOT_FOUND, "source file missing");
    }

    let img = match image::open(src) {
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

    #[test]
    fn strip_scheme_path_variants() {
        assert_eq!(strip_scheme_path("/123"), "123");
        assert_eq!(strip_scheme_path("/localhost/456"), "456");
        assert_eq!(strip_scheme_path("localhost/789"), "789");
    }
}
