use http::{StatusCode, header};
use tauri::http::Response;

pub fn strip_scheme_path(request_path: &str) -> &str {
    let mut path = request_path.trim_start_matches('/');
    if let Some(rest) = path.strip_prefix("localhost/") {
        path = rest;
    } else if path == "localhost" {
        path = "";
    }
    path.trim_start_matches('/')
}

pub fn cors_headers(builder: http::response::Builder) -> http::response::Builder {
    builder.header("Access-Control-Allow-Origin", "*")
}

pub fn ok_response(builder: http::response::Builder, body: Vec<u8>) -> Response<Vec<u8>> {
    builder.body(body).unwrap_or_else(|_| {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(b"internal error".to_vec())
            .expect("hardcoded response must build")
    })
}

pub fn error_response(status: StatusCode, msg: &str) -> Response<Vec<u8>> {
    ok_response(
        cors_headers(
            Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, "text/plain"),
        ),
        msg.as_bytes().to_vec(),
    )
}
