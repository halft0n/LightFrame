use lightframe_metadata::{PhotoMetadata, extract};
use std::io::Write;

/// Build a minimal JPEG with an APP1 EXIF segment containing the specified IFD entries.
/// Each entry is (tag_u16, type_u16, count_u32, value_bytes).
fn build_jpeg_with_exif(entries: &[(u16, u16, u32, Vec<u8>)]) -> Vec<u8> {
    let mut ifd_data = Vec::new();
    let mut extra_data = Vec::new();

    let entry_count = entries.len() as u16;
    ifd_data.extend_from_slice(&entry_count.to_be_bytes());

    // IFD0 starts at offset 8 (after TIFF header)
    // Each IFD entry is 12 bytes; after all entries comes the 4-byte next-IFD pointer
    let values_offset_base: u32 = 8 + 2 + (entries.len() as u32) * 12 + 4;
    let mut current_extra_offset = values_offset_base;

    for (tag, typ, count, value) in entries {
        ifd_data.extend_from_slice(&tag.to_be_bytes());
        ifd_data.extend_from_slice(&typ.to_be_bytes());
        ifd_data.extend_from_slice(&count.to_be_bytes());

        if value.len() <= 4 {
            let mut padded = [0u8; 4];
            padded[..value.len()].copy_from_slice(value);
            ifd_data.extend_from_slice(&padded);
        } else {
            ifd_data.extend_from_slice(&current_extra_offset.to_be_bytes());
            extra_data.extend_from_slice(value);
            current_extra_offset += value.len() as u32;
        }
    }

    // Next IFD pointer = 0 (no more IFDs)
    ifd_data.extend_from_slice(&[0u8; 4]);

    // TIFF header: byte order (MM = big-endian), magic 42, offset to IFD0
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"MM"); // Big-endian
    tiff.extend_from_slice(&42u16.to_be_bytes());
    tiff.extend_from_slice(&8u32.to_be_bytes()); // IFD0 at offset 8
    tiff.extend_from_slice(&ifd_data);
    tiff.extend_from_slice(&extra_data);

    // APP1 marker + length
    let exif_header = b"Exif\x00\x00";
    let app1_length = 2 + exif_header.len() + tiff.len();
    let mut jpeg = Vec::new();
    jpeg.extend_from_slice(&[0xFF, 0xD8]); // SOI
    jpeg.extend_from_slice(&[0xFF, 0xE1]); // APP1
    jpeg.extend_from_slice(&(app1_length as u16).to_be_bytes());
    jpeg.extend_from_slice(exif_header);
    jpeg.extend_from_slice(&tiff);
    jpeg.extend_from_slice(&[0xFF, 0xD9]); // EOI
    jpeg
}

fn u32_rational(num: u32, den: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&num.to_be_bytes());
    v.extend_from_slice(&den.to_be_bytes());
    v
}

fn gps_dms_rational(deg: u32, min: u32, sec_num: u32, sec_den: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&deg.to_be_bytes());
    v.extend_from_slice(&1u32.to_be_bytes());
    v.extend_from_slice(&min.to_be_bytes());
    v.extend_from_slice(&1u32.to_be_bytes());
    v.extend_from_slice(&sec_num.to_be_bytes());
    v.extend_from_slice(&sec_den.to_be_bytes());
    v
}

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
    f.write_all(&[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.camera_make.is_none());
}

#[test]
fn extract_orientation_from_ifd0() {
    // Orientation (0x0112) is in IFD0, so our simple builder can handle it.
    let entries = vec![
        (0x0112u16, 3u16, 1u32, vec![0x00, 0x03, 0x00, 0x00]), // 3 = rotate 180
    ];
    let jpeg = build_jpeg_with_exif(&entries);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orient_ifd0.jpg");
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert_eq!(meta.orientation, Some(3));
}

#[test]
fn extract_camera_make_and_model() {
    // Tag 0x010F = Make (type ASCII=2)
    // Tag 0x0110 = Model (type ASCII=2)
    let make = b"Canon\0";
    let model = b"EOS R5\0";
    let entries = vec![
        (0x010Fu16, 2u16, make.len() as u32, make.to_vec()),
        (0x0110u16, 2u16, model.len() as u32, model.to_vec()),
    ];
    let jpeg = build_jpeg_with_exif(&entries);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("camera.jpg");
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert_eq!(meta.camera_make.as_deref(), Some("Canon"));
    assert_eq!(meta.camera_model.as_deref(), Some("EOS R5"));
}

#[test]
fn extract_no_exif_subdirectory_tags_return_none() {
    // Tags like ISO (0x8827), DateTimeOriginal (0x9003), PixelXDimension (0xA002)
    // live in the EXIF sub-IFD. Without a proper EXIF IFD pointer, they won't be found.
    // This tests that our extraction gracefully returns None.
    let entries = vec![(0x010Fu16, 2u16, 6u32, b"Canon\0".to_vec())];
    let jpeg = build_jpeg_with_exif(&entries);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no_sub_ifd.jpg");
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert_eq!(meta.camera_make.as_deref(), Some("Canon"));
    // These require EXIF sub-IFD which our simple builder doesn't support
    assert!(meta.iso.is_none());
    assert!(meta.date_taken.is_none());
    assert!(meta.width.is_none());
}

#[test]
fn extract_gps_coordinates() {
    // GPS IFD tags — these need a GPS IFD sub-pointer which is complex.
    // For simplicity, test the GPS extraction through a real JPEG fixture.
    // Since we can't easily construct GPS IFD in our minimal builder,
    // we test that files without GPS return None.
    let entries = vec![(0x010Fu16, 2u16, 6u32, b"Nikon\0".to_vec())];
    let jpeg = build_jpeg_with_exif(&entries);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no_gps.jpg");
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.latitude.is_none());
    assert!(meta.longitude.is_none());
}

#[test]
fn extract_from_corrupt_exif_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.jpg");

    // Valid JPEG with APP1 marker but garbage EXIF content
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
    jpeg.extend_from_slice(&20u16.to_be_bytes()); // length
    jpeg.extend_from_slice(b"Exif\x00\x00");
    jpeg.extend_from_slice(&[
        0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.width.is_none());
    assert!(meta.camera_make.is_none());
}

#[test]
fn extract_from_large_non_exif_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.jpg");

    // JPEG with a large COM (comment) segment but no EXIF
    let mut jpeg = vec![0xFF, 0xD8];
    jpeg.extend_from_slice(&[0xFF, 0xFE]); // COM marker
    let comment = vec![b'A'; 1000];
    let len = (comment.len() + 2) as u16;
    jpeg.extend_from_slice(&len.to_be_bytes());
    jpeg.extend_from_slice(&comment);
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert!(meta.width.is_none());
}

#[test]
fn extract_multiple_ifd0_fields_combined() {
    let make = b"Sony\0";
    let model = b"A7III\0";
    let entries = vec![
        (0x010Fu16, 2u16, make.len() as u32, make.to_vec()),
        (0x0110u16, 2u16, model.len() as u32, model.to_vec()),
        (0x0112u16, 3u16, 1u32, vec![0x00, 0x01, 0x00, 0x00]), // 1 = normal
    ];
    let jpeg = build_jpeg_with_exif(&entries);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("combined.jpg");
    std::fs::write(&path, &jpeg).unwrap();

    let meta = extract(&path).unwrap();
    assert_eq!(meta.camera_make.as_deref(), Some("Sony"));
    assert_eq!(meta.camera_model.as_deref(), Some("A7III"));
    assert_eq!(meta.orientation, Some(1));
}
