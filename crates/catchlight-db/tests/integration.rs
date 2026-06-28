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
    assert_eq!(
        folder.id, folder2.id,
        "duplicate folder should return same id"
    );
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

    db.upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.png"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/c.webp"))
        .unwrap();

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

    let folder = db
        .get_watched_folder(fid)
        .unwrap()
        .expect("folder should exist");
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

    db.upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
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

    let media = db
        .get_media_by_id(media_id)
        .unwrap()
        .expect("media should exist");
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

    db.upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();

    let folder = db
        .get_watched_folder(fid)
        .unwrap()
        .expect("folder should exist");
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
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
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

fn sample_media_with_hashes(path: &str, blake3: Option<&str>, dhash: Option<u64>) -> MediaFile {
    let mut media = sample_media(path);
    media.blake3_hash = blake3.map(str::to_string);
    media.dhash = dhash;
    media
}

#[test]
fn test_find_exact_duplicates() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/a.jpg", Some("hash_same"), Some(1)),
        )
        .unwrap();
    let id2 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/b.jpg", Some("hash_same"), Some(2)),
        )
        .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/c.jpg", Some("hash_unique"), Some(3)),
    )
    .unwrap();

    let groups = db.find_exact_duplicates().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].match_type, "exact");
    assert_eq!(groups[0].members.len(), 2);

    let member_ids: Vec<i64> = groups[0].members.iter().map(|m| m.media_id).collect();
    assert!(member_ids.contains(&id1));
    assert!(member_ids.contains(&id2));
    assert!(
        groups[0]
            .members
            .iter()
            .all(|m| (m.similarity - 1.0).abs() < f64::EPSILON)
    );
}

#[test]
fn test_find_perceptual_duplicates() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    // Three hashes forming a chain: A~B, B~C (connected component)
    let hash_a = 0u64;
    let hash_b = 1u64; // distance 1 from A
    let hash_c = 3u64; // distance 1 from B, distance 2 from A
    let hash_d = 0xFFFF_FFFFu64; // 32 bits different from hash_a

    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/a.jpg", Some("unique_a"), Some(hash_a)),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/b.jpg", Some("unique_b"), Some(hash_b)),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/c.jpg", Some("unique_c"), Some(hash_c)),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/d.jpg", Some("unique_d"), Some(hash_d)),
    )
    .unwrap();

    let groups = db.find_perceptual_duplicates(10).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].match_type, "perceptual");
    assert_eq!(groups[0].members.len(), 3);
}

#[test]
fn test_find_perceptual_skips_exact_group_members() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/a.jpg", Some("same_hash"), Some(0)),
        )
        .unwrap();
    let id2 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/b.jpg", Some("same_hash"), Some(1)),
        )
        .unwrap();
    db.find_exact_duplicates().unwrap();

    let id3 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/c.jpg", Some("other"), Some(2)),
        )
        .unwrap();

    let groups = db.find_perceptual_duplicates(10).unwrap();
    let perceptual_member_ids: Vec<i64> = groups
        .iter()
        .flat_map(|g| g.members.iter().map(|m| m.media_id))
        .collect();
    assert!(!perceptual_member_ids.contains(&id1));
    assert!(!perceptual_member_ids.contains(&id2));
    assert!(!perceptual_member_ids.contains(&id3));
}

#[test]
fn test_create_and_list_duplicate_groups() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    let id2 = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();

    let group_id = db
        .create_duplicate_group("exact", &[id1, id2], &[1.0, 1.0])
        .unwrap();
    assert!(group_id > 0);

    let groups = db.list_duplicate_groups().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, group_id);
    assert_eq!(groups[0].members.len(), 2);
    assert_eq!(groups[0].members[0].filename, "a.jpg");

    assert_eq!(db.get_duplicate_groups_count().unwrap(), 1);

    db.remove_from_duplicate_group(group_id, id1).unwrap();
    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
}

#[test]
fn test_resolve_duplicate() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(fid, &sample_media("/photos/keep.jpg"))
        .unwrap();
    let id2 = db
        .upsert_media(fid, &sample_media("/photos/remove.jpg"))
        .unwrap();
    let group_id = db
        .create_duplicate_group("exact", &[id1, id2], &[1.0, 1.0])
        .unwrap();

    db.resolve_duplicate_group(group_id, id1, true).unwrap();

    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
    assert!(db.get_media_by_id(id1).unwrap().is_some());

    let deleted = db.list_deleted_media().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, id2);
}

#[test]
fn test_dismiss_duplicate_group() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    let id2 = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
    let group_id = db
        .create_duplicate_group("exact", &[id1, id2], &[1.0, 1.0])
        .unwrap();

    db.delete_duplicate_group(group_id).unwrap();
    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
    assert_eq!(db.list_duplicate_groups().unwrap().len(), 0);
}

#[test]
fn test_toggle_favorite() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    let favorited = db.toggle_favorite(media_id).unwrap();
    assert!(favorited);

    let unfavorited = db.toggle_favorite(media_id).unwrap();
    assert!(!unfavorited);
}

#[test]
fn test_is_favorite() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    assert!(!db.is_favorite(media_id).unwrap());

    assert!(db.toggle_favorite(media_id).unwrap());
    assert!(db.is_favorite(media_id).unwrap());

    assert!(!db.toggle_favorite(media_id).unwrap());
    assert!(!db.is_favorite(media_id).unwrap());

    assert!(!db.is_favorite(9999).unwrap());

    let deleted_id = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
    db.toggle_favorite(deleted_id).unwrap();
    db.set_deleted(deleted_id, true).unwrap();
    assert!(!db.is_favorite(deleted_id).unwrap());
}

#[test]
fn test_soft_delete_and_cleanup() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    db.set_deleted(media_id, true).unwrap();
    let deleted = db.list_deleted_media().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, media_id);

    // Set deleted_at to 40 days ago for cleanup test
    {
        let conn = db.conn();
        conn.execute(
            "UPDATE media_files SET deleted_at = datetime('now', '-40 days') WHERE id = ?1",
            rusqlite::params![media_id],
        )
        .unwrap();
    }

    let cleaned = db.cleanup_deleted_older_than(30).unwrap();
    assert_eq!(cleaned, 1);
    assert!(db.get_media_by_id(media_id).unwrap().is_none());

    let media_id2 = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
    db.set_deleted(media_id2, true).unwrap();
    db.set_deleted(media_id2, false).unwrap();
    assert!(db.list_deleted_media().unwrap().is_empty());

    let media_id3 = db
        .upsert_media(fid, &sample_media("/photos/c.jpg"))
        .unwrap();
    db.set_deleted(media_id3, true).unwrap();
    db.permanently_delete_media(media_id3).unwrap();
    assert!(db.get_media_by_id(media_id3).unwrap().is_none());
}
