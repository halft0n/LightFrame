use catchlight_core::media::ThumbnailSize;
use catchlight_thumbnail::{generate, generate_micro_blob, thumb_path};

fn create_test_image(dir: &std::path::Path, name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let path = dir.join(name);
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    img.save(&path).unwrap();
    path
}

#[test]
fn thumb_path_structure() {
    let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let path = thumb_path(hash, ThumbnailSize::Small);

    let path_str = path.to_string_lossy();
    assert!(
        path_str.contains("ab"),
        "should use first 2 chars as prefix"
    );
    assert!(
        path_str.contains("cd"),
        "should use chars 2-4 as sub-prefix"
    );
    assert!(path_str.ends_with("_small.webp"));
}

#[test]
fn thumb_path_micro() {
    let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let path = thumb_path(hash, ThumbnailSize::Micro);
    assert!(path.to_string_lossy().ends_with("_micro.webp"));
}

#[test]
fn thumb_path_large() {
    let hash = "ffee0011223344556677889900aabbccddeeff0011223344556677889900aabb";
    let path = thumb_path(hash, ThumbnailSize::Large);
    assert!(path.to_string_lossy().ends_with("_large.webp"));
}

#[test]
fn generate_thumbnail_creates_file() {
    let src_dir = tempfile::tempdir().unwrap();
    let src = create_test_image(src_dir.path(), "test.png", 800, 600);

    let hash = "aabbccdd11223344aabbccdd11223344aabbccdd11223344aabbccdd11223344";

    let result = generate(&src, hash, ThumbnailSize::Small);
    assert!(result.is_ok());
    let out_path = result.unwrap();
    assert!(out_path.exists());
}

#[test]
fn generate_idempotent() {
    let src_dir = tempfile::tempdir().unwrap();
    let src = create_test_image(src_dir.path(), "test.png", 400, 300);
    let hash = "eeff0011223344556677889900aabbcceeff0011223344556677889900aabbcc";

    let p1 = generate(&src, hash, ThumbnailSize::Small).unwrap();
    let p2 = generate(&src, hash, ThumbnailSize::Small).unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn generate_micro_blob_returns_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let src = create_test_image(dir.path(), "test.png", 200, 200);

    let blob = generate_micro_blob(&src).unwrap();
    assert!(!blob.is_empty());
    assert_eq!(blob[0], 0xFF, "JPEG starts with 0xFF");
    assert_eq!(blob[1], 0xD8, "JPEG starts with 0xFFD8");
}

#[test]
fn generate_from_nonexistent_errors() {
    let result = generate(
        std::path::Path::new("/nonexistent.jpg"),
        "0000000000000000000000000000000000000000000000000000000000000000",
        ThumbnailSize::Small,
    );
    assert!(result.is_err());
}

#[test]
fn thumb_path_sizes_are_distinct() {
    let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let micro = thumb_path(hash, ThumbnailSize::Micro);
    let small = thumb_path(hash, ThumbnailSize::Small);
    let large = thumb_path(hash, ThumbnailSize::Large);

    assert_ne!(micro, small);
    assert_ne!(small, large);
    assert_ne!(micro, large);
}

#[test]
fn thumbnail_size_enum_pixel_values() {
    assert_eq!(ThumbnailSize::Micro.pixels(), 64);
    assert_eq!(ThumbnailSize::Small.pixels(), 256);
    assert_eq!(ThumbnailSize::Large.pixels(), 1024);
}

#[test]
fn thumb_path_uses_hash_prefix_directories() {
    let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let path = thumb_path(hash, ThumbnailSize::Small);
    let parts: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    assert!(parts.iter().any(|p| p == "ab"));
    assert!(parts.iter().any(|p| p == "cd"));
    assert!(parts.last().is_some_and(|p| p.ends_with("_small.webp")));
}

#[test]
fn micro_blob_from_nonexistent_errors() {
    let result = generate_micro_blob(std::path::Path::new("/nonexistent.jpg"));
    assert!(result.is_err());
}
