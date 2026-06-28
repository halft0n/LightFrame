use catchlight_core::media::{MediaFile, MediaType};
use catchlight_db::Database;
use chrono::NaiveDateTime;
use std::path::Path;

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
    let folder = db.add_watched_folder("/photos").unwrap();
    assert!(folder.id > 0);
    assert_eq!(folder.media_count, 0);
    assert_eq!(folder.scan_status, "idle");

    let folder2 = db.add_watched_folder("/photos").unwrap();
    assert_eq!(folder.id, folder2.id, "duplicate folder should return same id");
}

#[test]
fn add_different_folders() {
    let db = create_test_db();
    let id1 = insert_folder_id(&db, "/photos");
    let id2 = insert_folder_id(&db, "/videos");
    assert_ne!(id1, id2);
}

#[test]
fn upsert_and_get_media() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let media = sample_media("/photos/sunset.jpg");
    let media_id = db.upsert_media(fid, &media).unwrap();
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
    let fid = insert_folder_id(&db, "/photos");

    let media = sample_media("/photos/sunset.jpg");
    db.upsert_media(fid, &media).unwrap();
    db.upsert_media(fid, &media).unwrap();

    let count = db.get_media_count().unwrap();
    assert_eq!(count, 1, "duplicate upsert should not create new row");
}

#[test]
fn get_media_count() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    assert_eq!(db.get_media_count().unwrap(), 0);

    db.upsert_media(fid, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.png")).unwrap();
    db.upsert_media(fid, &sample_media("/photos/c.webp")).unwrap();

    assert_eq!(db.get_media_count().unwrap(), 3);
}

#[test]
fn get_media_pagination() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    for i in 0..10 {
        db.upsert_media(fid, &sample_media(&format!("/photos/{i}.jpg")))
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
    let f1 = insert_folder_id(&db, "/photos");
    let f2 = insert_folder_id(&db, "/backup");

    db.upsert_media(f1, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(f2, &sample_media("/backup/b.jpg")).unwrap();

    assert_eq!(db.get_media_count().unwrap(), 2);
}

#[test]
fn list_watched_folders_returns_all_added() {
    let db = create_test_db();
    db.add_watched_folder("/photos").unwrap();
    db.add_watched_folder("/videos").unwrap();
    db.add_watched_folder("/backup").unwrap();

    let folders = db.list_watched_folders().unwrap();
    assert_eq!(folders.len(), 3);

    let paths: Vec<&str> = folders.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"/photos"));
    assert!(paths.contains(&"/videos"));
    assert!(paths.contains(&"/backup"));
}

#[test]
fn get_watched_folder_by_id() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let folder = db.get_watched_folder(fid).unwrap().expect("folder should exist");
    assert_eq!(folder.id, fid);
    assert_eq!(folder.path, "/photos");
    assert_eq!(folder.media_count, 0);

    let missing = db.get_watched_folder(9999).unwrap();
    assert!(missing.is_none());
}

#[test]
fn remove_watched_folder_deletes_folder_and_media() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(fid, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.jpg")).unwrap();
    assert_eq!(db.get_media_count().unwrap(), 2);

    db.remove_watched_folder(fid).unwrap();

    assert!(db.get_watched_folder(fid).unwrap().is_none());
    assert_eq!(db.get_media_count().unwrap(), 0);
    assert!(db.list_watched_folders().unwrap().is_empty());
}

#[test]
fn get_media_by_id() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/sunset.jpg"))
        .unwrap();

    let media = db.get_media_by_id(media_id).unwrap().expect("media should exist");
    assert_eq!(media.id, media_id);
    assert_eq!(media.filename, "sunset.jpg");
    assert_eq!(media.path, "/photos/sunset.jpg");

    let missing = db.get_media_by_id(9999).unwrap();
    assert!(missing.is_none());
}

#[test]
fn update_last_scan_at() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let before = db
        .get_watched_folder(fid)
        .unwrap()
        .expect("folder should exist");
    assert!(before.last_scan.is_none());

    db.update_last_scan_at(fid).unwrap();

    let after = db
        .get_watched_folder(fid)
        .unwrap()
        .expect("folder should exist");
    assert!(after.last_scan.is_some());
}

#[test]
fn watched_folder_media_count() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(fid, &sample_media("/photos/a.jpg")).unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.jpg")).unwrap();

    let folder = db.get_watched_folder(fid).unwrap().expect("folder should exist");
    assert_eq!(folder.media_count, 2);
}

#[test]
fn get_timeline_groups_returns_grouped_media() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;

    let mut media1 = sample_media("/photos/a.jpg");
    media1.created_at =
        Some(NaiveDateTime::parse_from_str("2024-06-15 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap());
    db.upsert_media(folder_id, &media1).unwrap();

    let mut media2 = sample_media("/photos/b.jpg");
    media2.created_at =
        Some(NaiveDateTime::parse_from_str("2024-06-15 14:00:00", "%Y-%m-%d %H:%M:%S").unwrap());
    db.upsert_media(folder_id, &media2).unwrap();

    let mut media3 = sample_media("/photos/c.jpg");
    media3.created_at =
        Some(NaiveDateTime::parse_from_str("2024-06-14 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap());
    db.upsert_media(folder_id, &media3).unwrap();

    let groups = db.get_timeline_groups(100, 0).unwrap();
    assert_eq!(groups.len(), 2, "should have 2 date groups");
    assert_eq!(groups[0].date, "2024-06-15");
    assert_eq!(groups[0].count, 2);
    assert_eq!(groups[0].media.len(), 2);
    assert_eq!(groups[1].date, "2024-06-14");
    assert_eq!(groups[1].count, 1);
}

#[test]
fn get_timeline_groups_respects_limit_and_offset() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;

    for i in 0..10 {
        let mut media = sample_media(&format!("/photos/img_{i}.jpg"));
        media.created_at = Some(
            NaiveDateTime::parse_from_str(
                &format!("2024-06-{:02} 10:00:00", 10 + i),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
        );
        db.upsert_media(folder_id, &media).unwrap();
    }

    let groups = db.get_timeline_groups(5, 0).unwrap();
    assert!(groups.iter().map(|g| g.count).sum::<i64>() <= 5);

    let all = db.get_timeline_groups(100, 0).unwrap();
    assert_eq!(all.iter().map(|g| g.count).sum::<i64>(), 10);
}

#[test]
fn get_media_neighbors_returns_adjacent_ids() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;

    let dates = [
        "2024-06-10 10:00:00",
        "2024-06-11 10:00:00",
        "2024-06-12 10:00:00",
    ];
    let mut ids = Vec::new();

    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/img_{i}.jpg"));
        media.created_at =
            Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        let id = db.upsert_media(folder_id, &media).unwrap();
        ids.push(id);
    }

    // Middle photo should have neighbors
    let nb = db.get_media_neighbors(ids[1]).unwrap();
    assert_eq!(nb.prev_id, Some(ids[2])); // newer photo (2024-06-12)
    assert_eq!(nb.next_id, Some(ids[0])); // older photo (2024-06-10)

    // First photo (oldest) has no next
    let nb = db.get_media_neighbors(ids[0]).unwrap();
    assert!(nb.next_id.is_none());

    // Last photo (newest) has no prev
    let nb = db.get_media_neighbors(ids[2]).unwrap();
    assert!(nb.prev_id.is_none());
}

#[test]
fn get_media_neighbors_returns_none_for_nonexistent() {
    let db = create_test_db();
    let nb = db.get_media_neighbors(9999).unwrap();
    assert!(nb.prev_id.is_none());
    assert!(nb.next_id.is_none());
}

#[test]
fn get_timeline_groups_empty_database() {
    let db = create_test_db();
    let groups = db.get_timeline_groups(100, 0).unwrap();
    assert!(groups.is_empty());
}

#[test]
fn set_and_get_micro_thumb() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;
    let media = sample_media("/photos/test_thumb.jpg");
    let media_id = db.upsert_media(folder_id, &media).unwrap();

    let thumb = db.get_micro_thumb(media_id).unwrap();
    assert!(thumb.is_none());

    let blob = vec![0xFF, 0xD8, 0xFF, 0xE0];
    db.set_micro_thumb(media_id, &blob).unwrap();

    let retrieved = db.get_micro_thumb(media_id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), blob);
}
