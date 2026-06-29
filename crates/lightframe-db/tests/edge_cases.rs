//! Database edge-case tests: concurrency, empty queries, unicode, pagination boundaries.

use chrono::NaiveDateTime;
use lightframe_core::media::{MediaFile, MediaType};
use lightframe_db::{Database, FaceDetectionInput};
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

#[test]
fn get_media_by_ids_empty_array_returns_empty_map() {
    let db = create_test_db();
    assert!(db.get_media_by_ids(&[]).unwrap().is_empty());
}

#[test]
fn get_media_by_ids_mixed_existing_and_missing() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let id1 = db
        .upsert_media(fid, &sample_media("/photos/exists.jpg"))
        .unwrap();

    let map = db.get_media_by_ids(&[id1, 999_999, -1]).unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key(&id1));
    assert!(!map.contains_key(&999_999));
}

#[test]
fn batch_set_favorite_empty_array_is_no_op() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    assert_eq!(db.batch_set_favorite(&[], true).unwrap(), 0);
    assert!(!db.is_favorite(media_id).unwrap());
}

#[test]
fn batch_set_deleted_empty_array_is_no_op() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    assert_eq!(db.batch_set_deleted(&[], true).unwrap(), 0);
    assert!(db.get_media_by_id(media_id).unwrap().is_some());
    assert!(db.list_deleted_media().unwrap().is_empty());
}

#[test]
fn create_album_with_very_long_name() {
    let db = create_test_db();
    let long_name = format!("Album {}", "x".repeat(1000));
    let album = db.create_album(&long_name, None).unwrap();
    let stored = db.get_album(album.id).unwrap().unwrap();
    assert_eq!(stored.name, long_name);
}

#[test]
fn create_album_with_unicode_emoji_name() {
    let db = create_test_db();
    let album = db.create_album("旅行 📸✨", Some("2024 🎉")).unwrap();
    let stored = db.get_album(album.id).unwrap().unwrap();
    assert_eq!(stored.name, "旅行 📸✨");
    assert_eq!(stored.description.as_deref(), Some("2024 🎉"));
}

#[test]
fn fts_search_empty_string_returns_empty() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/vacation.jpg"))
        .unwrap();

    assert!(db.search_media("", 10, 0).unwrap().is_empty());
    assert!(db.search_media("   ", 10, 0).unwrap().is_empty());
    assert_eq!(db.search_media_count("").unwrap(), 0);
    assert_eq!(db.get_media_count().unwrap(), 1);
}

#[test]
fn fts_search_very_long_query_does_not_error() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/vacation.jpg"))
        .unwrap();

    let long_query = "keyword ".repeat(1500);
    assert!(db.search_media(&long_query, 10, 0).is_ok());
    assert!(db.search_media_count(&long_query).is_ok());
    assert_eq!(db.get_media_count().unwrap(), 1);
}

#[test]
fn duplicate_group_operations_on_nonexistent_groups() {
    let db = create_test_db();

    assert!(db.resolve_duplicate_group(999_999, 1, true).is_err());
    db.delete_duplicate_group(999_999).unwrap();
    db.remove_from_duplicate_group(999_999, 1).unwrap();
    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
}

#[test]
fn face_store_accepts_nan_and_infinity_embeddings() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/face.jpg"))
        .unwrap();

    let faces = vec![
        FaceDetectionInput {
            bbox: [0.1, 0.2, 0.3, 0.4],
            confidence: 0.95,
            embedding: vec![f32::NAN, 1.0, f32::INFINITY],
        },
        FaceDetectionInput {
            bbox: [0.5, 0.5, 0.2, 0.2],
            confidence: 0.88,
            embedding: vec![f32::NEG_INFINITY, 0.0, -1.0],
        },
    ];
    db.store_face_detections(media_id, &faces).unwrap();

    let stored = db.get_faces_for_media(media_id).unwrap();
    assert_eq!(stored.len(), 2);
}
