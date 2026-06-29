//! Database edge-case tests: concurrency, empty queries, unicode, pagination boundaries.

use catchlight_core::media::{MediaFile, MediaType};
use catchlight_db::Database;
use chrono::NaiveDateTime;
use std::path::Path;
use std::sync::Arc;
use std::thread;

fn create_test_db() -> Database {
    Database::open(Path::new(":memory:")).expect("in-memory DB should open")
}

fn insert_folder_id(db: &Database, path: &str) -> i64 {
    db.add_watched_folder(path).unwrap().id
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
        modified_at: NaiveDateTime::default(),
        blake3_hash: Some("abcdef1234567890".to_string()),
        dhash: Some(0xDEADBEEF),
        phash: None,
        latitude: None,
        longitude: None,
    }
}

#[test]
fn empty_database_queries_return_empty_results() {
    let db = create_test_db();

    assert!(db.get_all_media(100, 0).unwrap().is_empty());
    assert_eq!(db.get_media_count().unwrap(), 0);
    assert!(db.list_albums().unwrap().is_empty());
    assert!(db.list_watched_folders().unwrap().is_empty());
    assert!(db.search_media("anything", 10, 0).unwrap().is_empty());
    assert_eq!(db.search_media_count("anything").unwrap(), 0);
    assert!(db.get_favorites(10, 0).unwrap().is_empty());
    assert_eq!(db.get_favorites_count().unwrap(), 0);
    assert!(db.list_deleted_media().unwrap().is_empty());
    assert!(db.list_duplicate_groups().unwrap().is_empty());
}

#[test]
fn pagination_zero_limit_returns_empty() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    assert!(db.get_all_media(0, 0).unwrap().is_empty());
    assert_eq!(db.get_media_count().unwrap(), 1);
}

#[test]
fn pagination_very_large_offset_returns_empty() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    for i in 0..5 {
        db.upsert_media(fid, &sample_media(&format!("/photos/{i}.jpg")))
            .unwrap();
    }

    let page = db.get_all_media(10, 1_000_000).unwrap();
    assert!(page.is_empty());
}

#[test]
fn very_long_filename_and_path_roundtrip() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let long_name = format!("{}.jpg", "x".repeat(400));
    let long_path = format!("/photos/nested/{}", long_name);
    let mut media = sample_media(&long_path);
    media.filename = long_name.clone();
    media.path = long_path.clone();

    let media_id = db.upsert_media(fid, &media).unwrap();
    let stored = db.get_media_by_id(media_id).unwrap().unwrap();
    assert_eq!(stored.filename, long_name);
    assert_eq!(stored.path, long_path);
}

#[test]
fn unicode_filename_and_album_name() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/照片");

    let mut media = sample_media("/照片/日落🌅.jpg");
    media.filename = "日落🌅.jpg".to_string();
    let media_id = db.upsert_media(fid, &media).unwrap();

    let album = db.create_album("日本旅行 🇯🇵", Some("2024年夏")).unwrap();
    db.add_to_album(album.id, &[media_id]).unwrap();

    let results = db.search_media("日落", 10, 0).unwrap();
    assert_eq!(results.len(), 1);

    let albums = db.list_albums().unwrap();
    assert!(albums.iter().any(|a| a.name.contains('🇯')));
}

#[test]
fn search_sql_injection_patterns_do_not_corrupt_database() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/vacation.jpg"))
        .unwrap();

    let injections = [
        "' OR '1'='1",
        "\"; DROP TABLE media_files;--",
        "1; DELETE FROM media_files",
        "test' UNION SELECT * FROM media_files--",
        "%' OR filename LIKE '%",
    ];

    for query in injections {
        assert!(
            db.search_media(query, 10, 0).is_ok(),
            "search should not error on injection pattern: {query}"
        );
    }

    assert_eq!(db.get_media_count().unwrap(), 1);
}

#[test]
fn concurrent_reads_while_writes_complete() {
    let db = Arc::new(create_test_db());
    let fid = insert_folder_id(&db, "/photos");

    let writer = {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            for i in 0..30 {
                db.upsert_media(fid, &sample_media(&format!("/photos/concurrent_{i}.jpg")))
                    .unwrap();
            }
        })
    };

    let readers: Vec<_> = (0..6)
        .map(|_| {
            let db = Arc::clone(&db);
            thread::spawn(move || {
                for _ in 0..25 {
                    let _ = db.get_all_media(5, 0);
                    let _ = db.get_media_count();
                    let _ = db.list_albums();
                }
            })
        })
        .collect();

    writer.join().expect("writer thread panicked");
    for handle in readers {
        handle.join().expect("reader thread panicked");
    }

    assert_eq!(db.get_media_count().unwrap(), 30);
}

#[test]
fn concurrent_read_conn_and_write_conn_both_succeed() {
    let db = Arc::new(create_test_db());
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(fid, &sample_media("/photos/seed.jpg"))
        .unwrap();

    let read_handle = {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            for _ in 0..50 {
                let count = db.get_media_count().unwrap();
                assert!(count <= 11);
            }
        })
    };

    let write_handle = {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            for i in 0..10 {
                db.upsert_media(fid, &sample_media(&format!("/photos/extra_{i}.jpg")))
                    .unwrap();
            }
        })
    };

    read_handle.join().unwrap();
    write_handle.join().unwrap();
    assert_eq!(db.get_media_count().unwrap(), 11);
}

#[test]
fn get_media_by_id_invalid_ids_return_none() {
    let db = create_test_db();

    assert!(db.get_media_by_id(0).unwrap().is_none());
    assert!(db.get_media_by_id(-1).unwrap().is_none());
    assert!(db.get_media_by_id(999_999).unwrap().is_none());
}

#[test]
fn toggle_favorite_on_nonexistent_media_errors() {
    let db = create_test_db();
    assert!(db.toggle_favorite(0).is_err());
    assert!(db.toggle_favorite(-42).is_err());
    assert!(db.toggle_favorite(999_999).is_err());
}
