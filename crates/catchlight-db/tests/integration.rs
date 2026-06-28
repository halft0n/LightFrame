use catchlight_core::media::{MediaFile, MediaType};
use catchlight_db::Database;
use std::path::Path;

fn create_test_db() -> Database {
    Database::open(Path::new(":memory:")).expect("in-memory DB should open")
}

fn sample_media(path: &str) -> MediaFile {
    MediaFile {
        id: 0,
        path: path.to_string(),
        filename: path.rsplit('/').next().unwrap_or(path).to_string(),
        media_type: MediaType::Photo,
        size_bytes: 2048,
        width: Some(1920),
        height: Some(1080),
        created_at: None,
        modified_at: chrono::NaiveDateTime::default(),
        blake3_hash: Some("abcdef1234567890".to_string()),
        dhash: Some(0xDEADBEEF),
        latitude: None,
        longitude: None,
    }
}

#[test]
fn open_and_migrate() {
    let _db = create_test_db();
}

#[test]
fn add_watched_folder() {
    let db = create_test_db();
    let id = db.add_watched_folder("/photos").unwrap();
    assert!(id > 0);

    let id2 = db.add_watched_folder("/photos").unwrap();
    assert_eq!(id, id2, "duplicate folder should return same id");
}

#[test]
fn add_different_folders() {
    let db = create_test_db();
    let id1 = db.add_watched_folder("/photos").unwrap();
    let id2 = db.add_watched_folder("/videos").unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn upsert_and_get_media() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap();

    let media = sample_media("/photos/sunset.jpg");
    let media_id = db.upsert_media(folder_id, &media).unwrap();
    assert!(media_id > 0);

    let results = db.get_all_media(100, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].filename, "sunset.jpg");
    assert_eq!(results[0].media_type, MediaType::Photo);
    assert_eq!(results[0].size_bytes, 2048);
}

#[test]
fn upsert_idempotent() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap();

    let media = sample_media("/photos/sunset.jpg");
    db.upsert_media(folder_id, &media).unwrap();
    db.upsert_media(folder_id, &media).unwrap();

    let count = db.get_media_count().unwrap();
    assert_eq!(count, 1, "duplicate upsert should not create new row");
}

#[test]
fn get_media_count() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap();

    assert_eq!(db.get_media_count().unwrap(), 0);

    db.upsert_media(folder_id, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(folder_id, &sample_media("/photos/b.png")).unwrap();
    db.upsert_media(folder_id, &sample_media("/photos/c.webp")).unwrap();

    assert_eq!(db.get_media_count().unwrap(), 3);
}

#[test]
fn get_media_pagination() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap();

    for i in 0..10 {
        db.upsert_media(folder_id, &sample_media(&format!("/photos/{i}.jpg")))
            .unwrap();
    }

    let page1 = db.get_all_media(3, 0).unwrap();
    assert_eq!(page1.len(), 3);

    let page2 = db.get_all_media(3, 3).unwrap();
    assert_eq!(page2.len(), 3);

    let all = db.get_all_media(100, 0).unwrap();
    assert_eq!(all.len(), 10);
}

#[test]
fn multiple_folders_media() {
    let db = create_test_db();
    let f1 = db.add_watched_folder("/photos").unwrap();
    let f2 = db.add_watched_folder("/backup").unwrap();

    db.upsert_media(f1, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(f2, &sample_media("/backup/b.jpg")).unwrap();

    assert_eq!(db.get_media_count().unwrap(), 2);
}
