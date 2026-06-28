use catchlight_indexer::scan_folder;
use std::fs;

#[tokio::test]
async fn scan_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let results = scan_folder(dir.path()).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn scan_finds_photos() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("photo1.jpg"), b"fake jpg").unwrap();
    fs::write(dir.path().join("photo2.png"), b"fake png").unwrap();
    fs::write(dir.path().join("readme.txt"), b"not a photo").unwrap();

    let results = scan_folder(dir.path()).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn scan_finds_videos() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("clip.mp4"), b"fake mp4").unwrap();
    fs::write(dir.path().join("clip.mov"), b"fake mov").unwrap();

    let results = scan_folder(dir.path()).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn scan_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("vacation").join("day1");
    fs::create_dir_all(&sub).unwrap();

    fs::write(dir.path().join("root.jpg"), b"root").unwrap();
    fs::write(sub.join("nested.png"), b"nested").unwrap();

    let results = scan_folder(dir.path()).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn scan_ignores_non_media() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("doc.pdf"), b"pdf").unwrap();
    fs::write(dir.path().join("code.rs"), b"code").unwrap();
    fs::write(dir.path().join("data.json"), b"json").unwrap();

    let results = scan_folder(dir.path()).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn scan_nonexistent_folder() {
    let result = scan_folder(std::path::Path::new("/nonexistent/path/12345")).await;
    assert!(result.is_ok(), "walkdir silently returns empty for nonexistent paths");
}

#[tokio::test]
async fn scan_mixed_case_extensions() {
    let dir = tempfile::tempdir().unwrap();

    fs::write(dir.path().join("photo.JPG"), b"upper").unwrap();
    fs::write(dir.path().join("video.Mp4"), b"mixed").unwrap();

    let results = scan_folder(dir.path()).await.unwrap();
    assert_eq!(results.len(), 2);
}
