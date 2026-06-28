use catchlight_metadata::{PhotoMetadata, extract};
use std::io::Write;

#[test]
fn extract_from_non_image_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.txt");
    std::fs::write(&path, b"not an image").unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.width.is_none());
    assert!(meta.height.is_none());
    assert!(meta.date_taken.is_none());
    assert!(meta.camera_make.is_none());
    assert!(meta.latitude.is_none());
}

#[test]
fn extract_from_nonexistent_file_errors() {
    let result = extract(std::path::Path::new("/nonexistent/photo.jpg"));
    assert!(result.is_err());
}

#[test]
fn extract_from_empty_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.jpg");
    std::fs::File::create(&path).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.width.is_none());
}

#[test]
fn photo_metadata_default_is_all_none() {
    let meta = PhotoMetadata::default();
    assert!(meta.width.is_none());
    assert!(meta.height.is_none());
    assert!(meta.date_taken.is_none());
    assert!(meta.camera_make.is_none());
    assert!(meta.camera_model.is_none());
    assert!(meta.focal_length.is_none());
    assert!(meta.aperture.is_none());
    assert!(meta.iso.is_none());
    assert!(meta.shutter_speed.is_none());
    assert!(meta.latitude.is_none());
    assert!(meta.longitude.is_none());
    assert!(meta.orientation.is_none());
}

#[test]
fn photo_metadata_serde_roundtrip() {
    let meta = PhotoMetadata {
        width: Some(4032),
        height: Some(3024),
        camera_make: Some("Canon".to_string()),
        camera_model: Some("EOS R5".to_string()),
        iso: Some(100),
        latitude: Some(39.9042),
        longitude: Some(116.4074),
        ..Default::default()
    };

    let json = serde_json::to_string(&meta).unwrap();
    let back: PhotoMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(back.width, Some(4032));
    assert_eq!(back.camera_make.as_deref(), Some("Canon"));
    assert_eq!(back.iso, Some(100));
}

#[test]
fn minimal_valid_jpeg_returns_no_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("minimal.jpg");

    let mut f = std::fs::File::create(&path).unwrap();
    // Write minimal JPEG header (SOI + EOI markers)
    f.write_all(&[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.camera_make.is_none());
}
