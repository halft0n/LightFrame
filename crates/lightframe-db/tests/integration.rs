use chrono::NaiveDateTime;
use lightframe_core::media::{MediaFile, MediaType};
use lightframe_db::{Database, FaceDetectionInput};
use std::path::Path;

fn create_test_db() -> Database {
    Database::open(Path::new(":memory:")).expect("in-memory DB should open")
}

fn create_file_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");
    let db = Database::open(&path).expect("file DB should open");
    (db, dir)
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
        phash: None,
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

    db.set_deleted(media_id, true).unwrap();
    assert!(db.get_media_by_id(media_id).unwrap().is_none());
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

    let groups = db.get_timeline_groups(100, None).unwrap();
    assert_eq!(groups.len(), 2, "should have 2 date groups");
    assert_eq!(groups[0].date, "2024-06-15");
    assert_eq!(groups[0].count, 2);
    assert_eq!(groups[0].media.len(), 2);
    assert_eq!(groups[1].date, "2024-06-14");
    assert_eq!(groups[1].count, 1);
}

#[test]
fn get_timeline_groups_respects_limit_and_cursor() {
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

    let groups = db.get_timeline_groups(5, None).unwrap();
    assert!(groups.iter().map(|g| g.count).sum::<i64>() <= 5);

    let all = db.get_timeline_groups(100, None).unwrap();
    assert_eq!(all.iter().map(|g| g.count).sum::<i64>(), 10);

    let last_media = all.last().and_then(|g| g.media.last()).expect("last media");
    let ts = last_media
        .created_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| {
            last_media
                .modified_at
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        });
    let page2 = db
        .get_timeline_groups(5, Some((ts, last_media.id)))
        .unwrap();
    assert!(page2.iter().map(|g| g.count).sum::<i64>() <= 5);
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
    let groups = db.get_timeline_groups(100, None).unwrap();
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

fn sample_media_with_hashes(
    path: &str,
    blake3: Option<&str>,
    dhash: Option<u64>,
    phash: Option<u64>,
) -> MediaFile {
    let mut media = sample_media(path);
    media.blake3_hash = blake3.map(str::to_string);
    media.dhash = dhash;
    media.phash = phash;
    media
}

#[test]
fn test_find_exact_duplicates() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/a.jpg", Some("hash_same"), Some(1), None),
        )
        .unwrap();
    let id2 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/b.jpg", Some("hash_same"), Some(2), None),
        )
        .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/c.jpg", Some("hash_unique"), Some(3), None),
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
        &sample_media_with_hashes("/photos/a.jpg", Some("unique_a"), Some(hash_a), None),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/b.jpg", Some("unique_b"), Some(hash_b), None),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/c.jpg", Some("unique_c"), Some(hash_c), None),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/d.jpg", Some("unique_d"), Some(hash_d), None),
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
            &sample_media_with_hashes("/photos/a.jpg", Some("same_hash"), Some(0), None),
        )
        .unwrap();
    let id2 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/b.jpg", Some("same_hash"), Some(1), None),
        )
        .unwrap();
    db.find_exact_duplicates().unwrap();

    let id3 = db
        .upsert_media(
            fid,
            &sample_media_with_hashes("/photos/c.jpg", Some("other"), Some(2), None),
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
        let conn = db.conn().unwrap();
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

#[test]
fn test_find_perceptual_duplicates_via_phash() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let dhash_far = 0x7FFF_FFFF_FFFF_FFFFu64;
    let phash_a = 0u64;
    let phash_b = 1u64;

    db.upsert_media(
        fid,
        &sample_media_with_hashes(
            "/photos/a.jpg",
            Some("unique_a"),
            Some(dhash_far),
            Some(phash_a),
        ),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/b.jpg", Some("unique_b"), Some(0), Some(phash_b)),
    )
    .unwrap();

    let groups = db.find_perceptual_duplicates(5).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].members.len(), 2);
}

#[test]
fn test_get_on_this_day_media() {
    use chrono::{Datelike, NaiveDate};

    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let today = chrono::Local::now().date_naive();
    let month = today.month();
    let day = today.day();
    let current_year = today.year();

    let photo_on = |year: i32, suffix: &str| -> MediaFile {
        let mut media = sample_media(&format!("/photos/{year}-{suffix}.jpg"));
        media.created_at =
            NaiveDate::from_ymd_opt(year, month, day).and_then(|d| d.and_hms_opt(12, 0, 0));
        media
    };

    db.upsert_media(fid, &photo_on(current_year - 1, "a"))
        .unwrap();
    db.upsert_media(fid, &photo_on(current_year - 2, "b"))
        .unwrap();

    assert!(db.get_on_this_day_media(10).unwrap().is_empty());

    db.upsert_media(fid, &photo_on(current_year - 3, "c"))
        .unwrap();

    let results = db.get_on_this_day_media(10).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(
        results[0].path,
        format!("/photos/{}-a.jpg", current_year - 1)
    );
    assert_eq!(
        results[1].path,
        format!("/photos/{}-b.jpg", current_year - 2)
    );
    assert_eq!(
        results[2].path,
        format!("/photos/{}-c.jpg", current_year - 3)
    );
}

#[test]
fn test_on_this_day_no_past_photos_returns_empty() {
    use chrono::{Datelike, NaiveDate};

    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let today = chrono::Local::now().date_naive();
    let month = today.month();
    let day = today.day();
    let current_year = today.year();

    let mut current_year_photo = sample_media("/photos/this-year.jpg");
    current_year_photo.created_at =
        NaiveDate::from_ymd_opt(current_year, month, day).and_then(|d| d.and_hms_opt(12, 0, 0));
    db.upsert_media(fid, &current_year_photo).unwrap();

    assert!(db.get_on_this_day_media(10).unwrap().is_empty());
}

#[test]
fn test_on_this_day_fewer_than_three_past_photos_returns_empty() {
    use chrono::{Datelike, NaiveDate};

    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let today = chrono::Local::now().date_naive();
    let month = today.month();
    let day = today.day();
    let current_year = today.year();

    let photo_on = |year: i32, suffix: &str| -> MediaFile {
        let mut media = sample_media(&format!("/photos/{year}-{suffix}.jpg"));
        media.created_at =
            NaiveDate::from_ymd_opt(year, month, day).and_then(|d| d.and_hms_opt(12, 0, 0));
        media
    };

    db.upsert_media(fid, &photo_on(current_year - 1, "a"))
        .unwrap();
    db.upsert_media(fid, &photo_on(current_year - 2, "b"))
        .unwrap();

    assert!(db.get_on_this_day_media(10).unwrap().is_empty());
}

#[test]
fn test_update_album_rename() {
    let db = create_test_db();
    let album = db.create_album("Vacation", None).unwrap();

    db.update_album(album.id, "Summer Trip", Some("2024 photos"))
        .unwrap();

    let updated = db.get_album(album.id).unwrap().expect("album should exist");
    assert_eq!(updated.name, "Summer Trip");
    assert_eq!(updated.description.as_deref(), Some("2024 photos"));
}

#[test]
fn test_update_album_empty_name_fails() {
    let db = create_test_db();
    let album = db.create_album("Test Album", None).unwrap();

    let result = db.update_album(album.id, "   ", None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("album name cannot be empty")
    );

    let unchanged = db.get_album(album.id).unwrap().expect("album should exist");
    assert_eq!(unchanged.name, "Test Album");
}

#[test]
fn test_set_album_cover() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/cover.jpg"))
        .unwrap();
    let album = db.create_album("My Album", None).unwrap();

    db.add_to_album(album.id, &[media_id]).unwrap();
    db.set_album_cover(album.id, media_id).unwrap();

    let updated = db.get_album(album.id).unwrap().expect("album should exist");
    assert_eq!(updated.cover_media_id, Some(media_id));
}

#[test]
fn get_media_page_keyset_pagination_with_cursors() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let dates = [
        "2024-06-15 10:00:00",
        "2024-06-14 10:00:00",
        "2024-06-13 10:00:00",
        "2024-06-12 10:00:00",
        "2024-06-11 10:00:00",
    ];

    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/img_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        db.upsert_media(fid, &media).unwrap();
    }

    let page1 = db.get_media_page(2, None).unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].filename, "img_0.jpg");
    assert_eq!(page1[1].filename, "img_1.jpg");

    let cursor = (dates[1].to_string(), page1[1].id);
    let page2 = db.get_media_page(2, Some(cursor)).unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].filename, "img_2.jpg");
    assert_eq!(page2[1].filename, "img_3.jpg");

    let cursor2 = (dates[3].to_string(), page2[1].id);
    let page3 = db.get_media_page(2, Some(cursor2)).unwrap();
    assert_eq!(page3.len(), 1);
    assert_eq!(page3[0].filename, "img_4.jpg");
}

#[test]
fn batch_operations_with_empty_inputs_are_no_ops() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();

    assert_eq!(db.batch_set_deleted(&[], true).unwrap(), 0);
    assert_eq!(db.batch_set_favorite(&[], true).unwrap(), 0);
    assert_eq!(db.batch_permanent_delete(&[]).unwrap(), 0);

    assert!(db.get_media_by_id(media_id).unwrap().is_some());
    assert!(!db.is_favorite(media_id).unwrap());
}

#[test]
fn smart_album_media_count_batch_query() {
    use lightframe_db::SmartAlbumRule;

    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let mut photo = sample_media("/photos/photo.jpg");
    photo.media_type = MediaType::Photo;
    db.upsert_media(fid, &photo).unwrap();

    let mut video = sample_media("/photos/clip.mp4");
    video.media_type = MediaType::Video;
    video.filename = "clip.mp4".to_string();
    video.path = "/photos/clip.mp4".to_string();
    db.upsert_media(fid, &video).unwrap();

    let photo_rule = SmartAlbumRule {
        media_type: Some("Photo".to_string()),
        date_from: None,
        date_to: None,
        country: None,
        city: None,
        is_favorite: None,
        min_size: None,
        has_gps: None,
    };
    let video_rule = SmartAlbumRule {
        media_type: Some("Video".to_string()),
        date_from: None,
        date_to: None,
        country: None,
        city: None,
        is_favorite: None,
        min_size: None,
        has_gps: None,
    };

    let photo_album = db
        .create_smart_album("Photos Only", None, &photo_rule)
        .unwrap();
    let video_album = db
        .create_smart_album("Videos Only", None, &video_rule)
        .unwrap();

    assert_eq!(photo_album.media_count, 1);
    assert_eq!(video_album.media_count, 1);

    let listed = db.list_smart_albums().unwrap();
    let photo_entry = listed
        .iter()
        .find(|a| a.name == "Photos Only")
        .expect("photo smart album");
    let video_entry = listed
        .iter()
        .find(|a| a.name == "Videos Only")
        .expect("video smart album");
    assert_eq!(photo_entry.media_count, 1);
    assert_eq!(video_entry.media_count, 1);
}

#[test]
fn timeline_groups_ordering_uses_id_tiebreaker() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;
    let same_time =
        NaiveDateTime::parse_from_str("2024-06-15 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..3 {
        let mut media = sample_media(&format!("/photos/tie_{i}.jpg"));
        media.created_at = Some(same_time);
        db.upsert_media(folder_id, &media).unwrap();
    }

    let groups = db.get_timeline_groups(100, None).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 3);

    let ids: Vec<i64> = groups[0].media.iter().map(|m| m.id).collect();
    assert!(
        ids[0] > ids[1] && ids[1] > ids[2],
        "same timestamp should order by id DESC: {ids:?}"
    );
}

#[test]
fn concurrent_read_access_from_multiple_threads() {
    use std::sync::Arc;
    use std::thread;

    let db = Arc::new(create_test_db());
    let fid = insert_folder_id(&db, "/photos");

    for i in 0..20 {
        db.upsert_media(fid, &sample_media(&format!("/photos/img_{i}.jpg")))
            .unwrap();
    }

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let db = Arc::clone(&db);
            thread::spawn(move || {
                let items = db.get_all_media(100, 0).unwrap();
                assert_eq!(items.len(), 20);
                db.get_media_count().unwrap()
            })
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().expect("thread panicked"), 20);
    }
}

#[test]
fn clip_embeddings_store_retrieve_and_overwrite() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/clip.jpg"))
        .unwrap();

    let embedding_v1 = vec![0.1, 0.2, 0.3];
    db.store_clip_embedding(media_id, &embedding_v1).unwrap();
    assert_eq!(
        db.get_clip_embedding(media_id).unwrap(),
        Some(embedding_v1.clone())
    );

    let embedding_v2 = vec![0.9, 0.8, 0.7];
    db.store_clip_embedding(media_id, &embedding_v2).unwrap();
    assert_eq!(db.get_clip_embedding(media_id).unwrap(), Some(embedding_v2));
}

#[test]
fn clip_embedding_missing_media_returns_none() {
    let db = create_test_db();
    assert!(db.get_clip_embedding(9999).unwrap().is_none());
}

#[test]
fn semantic_search_by_embedding_ranks_by_cosine_similarity() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id_a = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    let id_b = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
    let id_c = db
        .upsert_media(fid, &sample_media("/photos/c.jpg"))
        .unwrap();

    db.store_clip_embedding(id_a, &[1.0, 0.0, 0.0]).unwrap();
    db.store_clip_embedding(id_b, &[0.9, 0.1, 0.0]).unwrap();
    db.store_clip_embedding(id_c, &[0.0, 1.0, 0.0]).unwrap();

    let query = [1.0, 0.0, 0.0];
    let results = db.semantic_search_by_embedding(&query, 0.5, 10).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0.id, id_a);
    assert!(results[0].1 > results[1].1);
    assert_eq!(results[1].0.id, id_b);
}

#[test]
fn face_detections_store_and_retrieve() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/face.jpg"))
        .unwrap();

    let faces = vec![FaceDetectionInput {
        bbox: [10.0, 20.0, 110.0, 120.0],
        confidence: 0.95,
        embedding: vec![1.0, 0.0, 0.5],
    }];
    db.store_face_detections(media_id, &faces).unwrap();

    let stored = db.get_faces_for_media(media_id).unwrap();
    assert_eq!(stored.len(), 1);
    assert!((stored[0].confidence - 0.95).abs() < f32::EPSILON);
}

#[test]
fn face_detections_empty_media_returns_empty() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/noface.jpg"))
        .unwrap();
    assert!(db.get_faces_for_media(media_id).unwrap().is_empty());
}

#[test]
fn person_create_rename_and_merge() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/person.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.99,
            embedding: vec![1.0, 0.0],
        }],
    )
    .unwrap();
    let faces = db.get_faces_for_media(media_id).unwrap();
    let face_id = faces[0].id;

    let person_a = db.create_person(Some("Alice")).unwrap();
    let person_b = db.create_person(Some("Bob")).unwrap();
    db.assign_face_to_person(face_id, person_a).unwrap();

    db.rename_person(person_a, "Alicia").unwrap();
    let persons = db.list_persons().unwrap();
    assert_eq!(
        persons
            .iter()
            .find(|p| p.id == person_a)
            .and_then(|p| p.name.as_deref()),
        Some("Alicia")
    );

    db.merge_persons(person_a, &[person_b]).unwrap();
    assert_eq!(db.get_persons_count().unwrap(), 1);
}

#[test]
fn person_rename_empty_string_allowed() {
    let db = create_test_db();
    let person_id = db.create_person(Some("Named")).unwrap();
    db.rename_person(person_id, "").unwrap();
    let persons = db.list_persons().unwrap();
    assert_eq!(persons[0].name.as_deref(), Some(""));
}

#[test]
fn merge_single_person_is_no_op() {
    let db = create_test_db();
    let person_id = db.create_person(Some("Solo")).unwrap();
    db.merge_persons(person_id, &[person_id]).unwrap();
    assert_eq!(db.get_persons_count().unwrap(), 1);
}

#[test]
fn album_very_long_name_and_duplicate_names() {
    let db = create_test_db();
    let long_name = "A".repeat(500);
    let album1 = db.create_album(&long_name, None).unwrap();
    assert_eq!(album1.name, long_name);

    let album2 = db.create_album("Vacation", None).unwrap();
    let album3 = db.create_album("Vacation", None).unwrap();
    assert_ne!(album2.id, album3.id);
}

#[test]
fn album_add_same_media_twice_is_idempotent() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    let album = db.create_album("Dup Items", None).unwrap();

    db.add_to_album(album.id, &[media_id]).unwrap();
    db.add_to_album(album.id, &[media_id]).unwrap();

    let updated = db.get_album(album.id).unwrap().expect("album exists");
    assert_eq!(updated.media_count, 1);
}

#[test]
fn search_fts5_chinese_characters() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let mut media = sample_media("/photos/日落.jpg");
    media.filename = "日落海滩.jpg".to_string();
    db.upsert_media(fid, &media).unwrap();

    let results = db.search_media("日落", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].filename.contains('日'));
}

#[test]
fn search_special_sql_characters_do_not_break_query() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/100_percent.jpg"))
        .unwrap();

    assert!(db.search_media("100%", 10, 0).is_ok());
    assert!(db.search_media("test_query", 10, 0).is_ok());
    assert!(db.search_media("it's fine", 10, 0).is_ok());
}

#[test]
fn batch_delete_with_many_items() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let mut ids = Vec::new();
    for i in 0..1001 {
        let id = db
            .upsert_media(fid, &sample_media(&format!("/photos/batch_{i}.jpg")))
            .unwrap();
        ids.push(id);
    }

    let deleted = db.batch_set_deleted(&ids, true).unwrap();
    assert_eq!(deleted, 1001);
    assert_eq!(db.get_media_count().unwrap(), 0);
}

#[test]
fn get_media_page_cursor_at_end_returns_empty() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let dates: Vec<String> = (0..3)
        .map(|i| format!("2024-07-{:02} 10:00:00", 10 + i))
        .collect();
    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/end_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        db.upsert_media(fid, &media).unwrap();
    }

    let page = db.get_media_page(10, None).unwrap();
    assert_eq!(page.len(), 3);
    let last = page.last().unwrap();
    let cursor = (dates[0].clone(), last.id);
    let beyond = db.get_media_page(10, Some(cursor)).unwrap();
    assert!(beyond.is_empty());
}

#[test]
fn get_media_page_cursor_after_deleted_item_still_pages() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let dates: Vec<String> = (0..4)
        .map(|i| format!("2024-08-{:02} 10:00:00", 10 + i))
        .collect();
    let mut ids = Vec::new();
    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/del_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        ids.push(db.upsert_media(fid, &media).unwrap());
    }

    db.set_deleted(ids[1], true).unwrap();
    let page1 = db.get_media_page(2, None).unwrap();
    assert_eq!(page1.len(), 2);

    let cursor = (dates[2].clone(), page1[1].id);
    let page2 = db.get_media_page(2, Some(cursor)).unwrap();
    assert!(!page2.is_empty());
}

#[test]
fn timeline_groups_all_same_date() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;
    let same_time =
        NaiveDateTime::parse_from_str("2024-06-15 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..5 {
        let mut media = sample_media(&format!("/photos/same_{i}.jpg"));
        media.created_at = Some(same_time);
        db.upsert_media(folder_id, &media).unwrap();
    }

    let groups = db.get_timeline_groups(100, None).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 5);
}

#[test]
fn timeline_groups_spanning_years() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;

    let dates = [
        "2022-01-01 12:00:00",
        "2023-06-15 12:00:00",
        "2024-12-31 12:00:00",
    ];
    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/year_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        db.upsert_media(folder_id, &media).unwrap();
    }

    let groups = db.get_timeline_groups(100, None).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].date, "2024-12-31");
    assert_eq!(groups[2].date, "2022-01-01");
}

#[test]
fn semantic_search_by_embedding_empty_database() {
    let db = create_test_db();
    let results = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.5, 10)
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn semantic_search_by_embedding_filters_below_threshold() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let id = db
        .upsert_media(fid, &sample_media("/photos/dissimilar.jpg"))
        .unwrap();
    db.store_clip_embedding(id, &[0.0, 1.0, 0.0]).unwrap();

    let results = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.9, 10)
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn semantic_search_by_embedding_respects_limit() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    for i in 0..5 {
        let id = db
            .upsert_media(fid, &sample_media(&format!("/photos/limit_{i}.jpg")))
            .unwrap();
        db.store_clip_embedding(id, &[1.0, 0.0, 0.0]).unwrap();
    }

    let results = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.5, 2)
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn semantic_search_by_embedding_sorted_by_score_descending() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id_exact = db
        .upsert_media(fid, &sample_media("/photos/exact.jpg"))
        .unwrap();
    let id_close = db
        .upsert_media(fid, &sample_media("/photos/close.jpg"))
        .unwrap();
    let id_far = db
        .upsert_media(fid, &sample_media("/photos/far.jpg"))
        .unwrap();

    db.store_clip_embedding(id_exact, &[1.0, 0.0, 0.0]).unwrap();
    db.store_clip_embedding(id_close, &[0.8, 0.2, 0.0]).unwrap();
    db.store_clip_embedding(id_far, &[0.5, 0.5, 0.0]).unwrap();

    let results = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.4, 10)
        .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].1 >= results[1].1);
    assert!(results[1].1 >= results[2].1);
    assert_eq!(results[0].0.id, id_exact);
}

#[test]
fn get_media_ids_without_faces_excludes_media_with_detections() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let with_face = db
        .upsert_media(fid, &sample_media("/photos/with_face.jpg"))
        .unwrap();
    let without_face = db
        .upsert_media(fid, &sample_media("/photos/without_face.jpg"))
        .unwrap();

    db.store_face_detections(
        with_face,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.99,
            embedding: vec![1.0, 0.0],
        }],
    )
    .unwrap();

    let ids = db.get_media_ids_without_faces(500).unwrap();
    assert!(ids.contains(&without_face));
    assert!(!ids.contains(&with_face));
}

#[test]
fn get_faces_for_person_returns_assigned_faces() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/person_faces.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[
            FaceDetectionInput {
                bbox: [0.0, 0.0, 10.0, 10.0],
                confidence: 0.95,
                embedding: vec![1.0, 0.0],
            },
            FaceDetectionInput {
                bbox: [20.0, 20.0, 40.0, 40.0],
                confidence: 0.90,
                embedding: vec![0.0, 1.0],
            },
        ],
    )
    .unwrap();

    let faces = db.get_faces_for_media(media_id).unwrap();
    let person_a = db.create_person(Some("Alice")).unwrap();
    let person_b = db.create_person(Some("Bob")).unwrap();
    db.assign_face_to_person(faces[0].id, person_a).unwrap();
    db.assign_face_to_person(faces[1].id, person_b).unwrap();

    let alice_faces = db.get_faces_for_person(person_a, 10, 0).unwrap();
    assert_eq!(alice_faces.len(), 1);
    assert_eq!(alice_faces[0].id, faces[0].id);
    assert_eq!(alice_faces[0].person_id, Some(person_a));

    let paged = db.get_faces_for_person(person_a, 1, 0).unwrap();
    assert_eq!(paged.len(), 1);
}

#[test]
fn get_face_by_id_returns_record_or_none() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/face_lookup.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[FaceDetectionInput {
            bbox: [5.0, 5.0, 25.0, 25.0],
            confidence: 0.88,
            embedding: vec![0.5, 0.5],
        }],
    )
    .unwrap();
    let face_id = db.get_faces_for_media(media_id).unwrap()[0].id;

    let found = db.get_face_by_id(face_id).unwrap();
    assert!(found.is_some());
    let face = found.unwrap();
    assert_eq!(face.media_id, media_id);
    assert!((face.confidence - 0.88).abs() < f32::EPSILON);

    assert!(db.get_face_by_id(999_999).unwrap().is_none());
}

#[test]
fn get_timeline_groups_first_page_without_cursor() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;

    for i in 0..3 {
        let mut media = sample_media(&format!("/photos/page1_{i}.jpg"));
        media.created_at = Some(
            NaiveDateTime::parse_from_str(
                &format!("2024-07-{:02} 12:00:00", 10 - i),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
        );
        db.upsert_media(folder_id, &media).unwrap();
    }

    let page1 = db.get_timeline_groups(2, None).unwrap();
    assert_eq!(page1.iter().map(|g| g.count).sum::<i64>(), 2);
}

#[test]
fn get_timeline_groups_second_page_with_cursor() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;
    let dates: Vec<String> = (0..6)
        .map(|i| format!("2024-06-{:02} 10:00:00", 10 + i))
        .collect();

    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/page2_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        db.upsert_media(folder_id, &media).unwrap();
    }

    let page1 = db.get_timeline_groups(3, None).unwrap();
    assert_eq!(page1.iter().map(|g| g.count).sum::<i64>(), 3);

    let page1_ids: Vec<i64> = page1
        .iter()
        .flat_map(|g| g.media.iter().map(|m| m.id))
        .collect();
    let last_id = *page1_ids.last().expect("page1 item");
    let cursor_ts = dates[3].clone();

    let page2 = db
        .get_timeline_groups(3, Some((cursor_ts, last_id)))
        .unwrap();
    let page2_ids: Vec<i64> = page2
        .iter()
        .flat_map(|g| g.media.iter().map(|m| m.id))
        .collect();

    assert_eq!(page2.iter().map(|g| g.count).sum::<i64>(), 3);
    for id in &page2_ids {
        assert!(!page1_ids.contains(id));
    }
    assert_eq!(page1_ids.len() + page2_ids.len(), 6);
}

#[test]
fn get_timeline_groups_single_date_spans_pages() {
    let db = create_test_db();
    let folder_id = db.add_watched_folder("/photos").unwrap().id;
    let same_time = "2024-09-15 10:00:00".to_string();
    let parsed = NaiveDateTime::parse_from_str(&same_time, "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..8 {
        let mut media = sample_media(&format!("/photos/same_day_{i}.jpg"));
        media.created_at = Some(parsed);
        db.upsert_media(folder_id, &media).unwrap();
    }

    let page1 = db.get_timeline_groups(3, None).unwrap();
    assert_eq!(page1.len(), 1);
    assert_eq!(page1[0].date, "2024-09-15");
    assert_eq!(page1[0].count, 3);

    let last_id = page1[0].media.last().expect("media").id;
    let page2 = db
        .get_timeline_groups(3, Some((same_time.clone(), last_id)))
        .unwrap();

    assert_eq!(page2.len(), 1);
    assert_eq!(page2[0].date, "2024-09-15");
    assert_eq!(page2[0].count, 3);

    let page1_ids: Vec<i64> = page1[0].media.iter().map(|m| m.id).collect();
    let page2_ids: Vec<i64> = page2[0].media.iter().map(|m| m.id).collect();
    for id in &page2_ids {
        assert!(!page1_ids.contains(id));
    }
}

fn sample_screenshot(path: &str) -> MediaFile {
    let mut media = sample_media(path);
    media.media_type = MediaType::Screenshot;
    media
}

#[allow(clippy::too_many_arguments)]
fn insert_photo_with_location(
    db: &Database,
    fid: i64,
    path: &str,
    created_at: Option<NaiveDateTime>,
    city: Option<&str>,
    country: &str,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> i64 {
    let mut media = sample_media(path);
    media.created_at = created_at;
    media.latitude = latitude;
    media.longitude = longitude;
    let id = db.upsert_media(fid, &media).unwrap();
    if let Some(c) = city {
        db.update_media_location(id, c, country).unwrap();
    } else {
        db.update_media_location(id, "", country).unwrap();
        let conn = db.conn().unwrap();
        conn.execute(
            "UPDATE media_files SET city = NULL WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();
    }
    id
}

#[test]
fn location_groups_and_media_by_location() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let tokyo_id = insert_photo_with_location(
        &db,
        fid,
        "/photos/tokyo.jpg",
        None,
        Some("东京"),
        "日本",
        Some(35.68),
        Some(139.69),
    );
    insert_photo_with_location(
        &db,
        fid,
        "/photos/tokyo2.jpg",
        None,
        Some("东京"),
        "日本",
        Some(35.69),
        Some(139.70),
    );
    insert_photo_with_location(
        &db,
        fid,
        "/photos/osaka.jpg",
        None,
        Some("大阪"),
        "日本",
        Some(34.69),
        Some(135.50),
    );
    insert_photo_with_location(
        &db,
        fid,
        "/photos/usa.jpg",
        None,
        None,
        "USA",
        Some(40.71),
        Some(-74.01),
    );

    let groups = db.get_location_groups().unwrap();
    assert_eq!(groups.len(), 3);
    let tokyo_group = groups
        .iter()
        .find(|g| g.city.as_deref() == Some("东京"))
        .expect("tokyo group");
    assert_eq!(tokyo_group.country, "日本");
    assert_eq!(tokyo_group.count, 2);
    assert!(tokyo_group.sample_media_id == tokyo_id || tokyo_group.sample_media_id > 0);

    let tokyo_media = db
        .get_media_by_location("日本", Some("东京"), 10, 0)
        .unwrap();
    assert_eq!(tokyo_media.len(), 2);

    let usa_no_city = db.get_media_by_location("USA", None, 10, 0).unwrap();
    assert_eq!(usa_no_city.len(), 1);

    assert!(
        db.get_media_by_location("日本", Some("京都"), 10, 0)
            .unwrap()
            .is_empty()
    );

    db.update_media_location(tokyo_id, "横滨", "日本").unwrap();
    let tokyo_after = db
        .get_media_by_location("日本", Some("东京"), 10, 0)
        .unwrap();
    assert_eq!(tokyo_after.len(), 1);
    assert_eq!(
        db.get_media_by_location("日本", Some("横滨"), 10, 0)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn location_groups_empty_database() {
    let db = create_test_db();
    assert!(db.get_location_groups().unwrap().is_empty());
}

#[test]
fn location_stats_and_geo_queries() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let stats_empty = db.get_location_stats().unwrap();
    assert_eq!(stats_empty.total_with_gps, 0);
    assert_eq!(stats_empty.countries, 0);
    assert_eq!(stats_empty.cities, 0);

    insert_photo_with_location(
        &db,
        fid,
        "/photos/gps1.jpg",
        None,
        Some("Paris"),
        "France",
        Some(48.8566),
        Some(2.3522),
    );
    insert_photo_with_location(
        &db,
        fid,
        "/photos/gps2.jpg",
        None,
        Some("Lyon"),
        "France",
        Some(45.7640),
        Some(4.8357),
    );
    let no_gps_id = db
        .upsert_media(fid, &sample_media("/photos/no_gps.jpg"))
        .unwrap();
    db.update_media_location(no_gps_id, "Berlin", "Germany")
        .unwrap();

    let stats = db.get_location_stats().unwrap();
    assert_eq!(stats.total_with_gps, 2);
    assert_eq!(stats.countries, 2);
    assert_eq!(stats.cities, 3);

    let geo_page = db.get_media_with_geo(1, 0).unwrap();
    assert_eq!(geo_page.len(), 1);
    assert!(geo_page[0].latitude.is_some());

    let all_geo = db.get_media_with_geo(10, 0).unwrap();
    assert_eq!(all_geo.len(), 2);

    let clusters = db.get_geo_clusters(1.0).unwrap();
    assert_eq!(clusters.len(), 2);
    assert!(clusters.iter().all(|c| c.count >= 1));
    assert!(!clusters[0].media_ids.is_empty());

    let tight_clusters = db.get_geo_clusters(0.001).unwrap();
    assert!(tight_clusters.len() >= 2);
}

#[test]
fn generate_list_and_get_memory_media() {
    let (db, _dir) = create_file_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let dt = NaiveDateTime::parse_from_str("2023-08-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    assert!(db.list_memories().unwrap().is_empty());
    assert!(db.generate_memories().unwrap().is_empty());

    for i in 0..5 {
        insert_photo_with_location(
            &db,
            fid,
            &format!("/photos/mem_{i}.jpg"),
            Some(dt + chrono::Duration::hours(i)),
            Some("上海"),
            "中国",
            None,
            None,
        );
    }

    let memories = db.generate_memories().unwrap();
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].media_count, 5);
    assert!(memories[0].title.contains("2023"));
    assert!(memories[0].title.contains("8"));
    assert!(memories[0].cover_media_id > 0);

    let listed = db.list_memories().unwrap();
    assert_eq!(listed.len(), 1);

    let memory_id = listed[0].id;
    let media = db.get_memory_media(memory_id, 10, 0).unwrap();
    assert_eq!(media.len(), 5);

    let page = db.get_memory_media(memory_id, 2, 0).unwrap();
    assert_eq!(page.len(), 2);

    assert!(db.get_memory_media(9999, 10, 0).unwrap().is_empty());

    insert_photo_with_location(
        &db,
        fid,
        "/photos/extra.jpg",
        Some(dt + chrono::Duration::days(1)),
        Some("上海"),
        "中国",
        None,
        None,
    );
    let regenerated = db.generate_memories().unwrap();
    assert_eq!(regenerated.len(), 1);
    assert_eq!(regenerated[0].media_count, 6);
}

#[test]
fn generate_memories_requires_five_photos_per_group() {
    let (db, _dir) = create_file_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let dt = NaiveDateTime::parse_from_str("2022-03-15 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..4 {
        insert_photo_with_location(
            &db,
            fid,
            &format!("/photos/few_{i}.jpg"),
            Some(dt + chrono::Duration::hours(i)),
            Some("北京"),
            "中国",
            None,
            None,
        );
    }

    assert!(db.generate_memories().unwrap().is_empty());
}

#[test]
fn screenshots_filter_by_type_and_count() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let id1 = db
        .upsert_media(fid, &sample_screenshot("/photos/screen1.png"))
        .unwrap();
    let id2 = db
        .upsert_media(fid, &sample_screenshot("/photos/screen2.png"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/photo.jpg"))
        .unwrap();

    db.set_screenshot_type(id1, "window").unwrap();
    db.set_screenshot_type(id2, "full").unwrap();

    assert_eq!(db.get_screenshot_count(None).unwrap(), 2);
    assert_eq!(db.get_screenshot_count(Some("window")).unwrap(), 1);
    assert_eq!(db.get_screenshot_count(Some("full")).unwrap(), 1);
    assert_eq!(db.get_screenshot_count(Some("missing")).unwrap(), 0);

    let all = db.get_screenshots(None, 10, 0).unwrap();
    assert_eq!(all.len(), 2);

    let windows = db.get_screenshots(Some("window"), 10, 0).unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].id, id1);

    assert!(db.get_screenshots(None, 0, 0).unwrap().is_empty());
}

#[test]
fn set_screenshot_type_on_nonexistent_media_is_no_op() {
    let db = create_test_db();
    db.set_screenshot_type(9999, "window").unwrap();
}

#[test]
fn find_similar_media_ranks_by_embedding_similarity() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let target_id = db
        .upsert_media(fid, &sample_media("/photos/target.jpg"))
        .unwrap();
    let similar_id = db
        .upsert_media(fid, &sample_media("/photos/similar.jpg"))
        .unwrap();
    let dissimilar_id = db
        .upsert_media(fid, &sample_media("/photos/different.jpg"))
        .unwrap();

    db.store_clip_embedding(target_id, &[1.0, 0.0, 0.0])
        .unwrap();
    db.store_clip_embedding(similar_id, &[0.95, 0.05, 0.0])
        .unwrap();
    db.store_clip_embedding(dissimilar_id, &[0.0, 1.0, 0.0])
        .unwrap();

    let results = db.find_similar_media(target_id, 0.5, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, similar_id);
    assert!(results[0].1 > 0.5);
    assert!(!results.iter().any(|(id, _)| *id == target_id));

    let strict = db.find_similar_media(target_id, 0.9999, 10).unwrap();
    assert!(strict.is_empty());

    let err = db.find_similar_media(9999, 0.5, 10).unwrap_err();
    assert!(err.to_string().contains("no CLIP embedding"));
}

#[test]
fn get_media_by_type_and_count() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    db.upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();
    let mut video = sample_media("/photos/v.mp4");
    video.media_type = MediaType::Video;
    db.upsert_media(fid, &video).unwrap();

    assert_eq!(db.get_media_count_by_type("Photo").unwrap(), 2);
    assert_eq!(db.get_media_count_by_type("Video").unwrap(), 1);
    assert_eq!(db.get_media_count_by_type("Screenshot").unwrap(), 0);

    let photos = db.get_media_by_type("Photo", 10, 0).unwrap();
    assert_eq!(photos.len(), 2);
    assert!(photos.iter().all(|m| m.media_type == MediaType::Photo));

    assert!(db.get_media_by_type("Photo", 0, 0).unwrap().is_empty());
    assert!(db.get_media_by_type("Unknown", 10, 0).unwrap().is_empty());
}

#[test]
fn get_media_by_ids_excludes_deleted_and_missing() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    assert!(db.get_media_by_ids(&[]).unwrap().is_empty());

    let dates = [
        "2024-01-01 10:00:00",
        "2024-01-02 10:00:00",
        "2024-01-03 10:00:00",
        "2024-01-04 10:00:00",
        "2024-01-05 10:00:00",
    ];
    let mut ids = Vec::new();
    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/win_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        ids.push(db.upsert_media(fid, &media).unwrap());
    }

    let map = db.get_media_by_ids(&[ids[0], ids[2], 9999]).unwrap();
    assert_eq!(map.len(), 2);
    assert!(map.contains_key(&ids[0]));
    assert!(map.contains_key(&ids[2]));

    db.set_deleted(ids[1], true).unwrap();
    let without_deleted = db.get_media_by_ids(&[ids[0], ids[1]]).unwrap();
    assert_eq!(without_deleted.len(), 1);
}

#[test]
fn get_media_window_navigation_via_neighbors() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let dates = [
        "2024-01-01 10:00:00",
        "2024-01-02 10:00:00",
        "2024-01-03 10:00:00",
        "2024-01-04 10:00:00",
        "2024-01-05 10:00:00",
    ];
    let mut ids = Vec::new();
    for (i, date) in dates.iter().enumerate() {
        let mut media = sample_media(&format!("/photos/window_{i}.jpg"));
        media.created_at = Some(NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").unwrap());
        ids.push(db.upsert_media(fid, &media).unwrap());
    }

    let nb = db.get_media_neighbors(ids[2]).unwrap();
    assert_eq!(nb.prev_id, Some(ids[3]));
    assert_eq!(nb.next_id, Some(ids[1]));

    let center = db.get_media_by_id(ids[2]).unwrap().unwrap();
    assert_eq!(center.id, ids[2]);
    assert_eq!(center.filename, "window_2.jpg");
}

#[test]
fn get_media_deletion_info_includes_deleted_rows() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/del.jpg"))
        .unwrap();

    let active = db.get_media_deletion_info(media_id).unwrap();
    assert!(active.is_some());
    let (path, hash, deleted) = active.unwrap();
    assert_eq!(path, "/photos/del.jpg");
    assert_eq!(hash.as_deref(), Some("abcdef1234567890"));
    assert_eq!(deleted, 0);

    db.set_deleted(media_id, true).unwrap();
    let deleted_info = db.get_media_deletion_info(media_id).unwrap();
    assert_eq!(deleted_info.unwrap().2, 1);

    assert!(db.get_media_deletion_info(9999).unwrap().is_none());
}

#[test]
fn split_face_from_person_creates_new_person_and_updates_counts() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/faces.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[
            FaceDetectionInput {
                bbox: [0.0, 0.0, 10.0, 10.0],
                confidence: 0.9,
                embedding: vec![1.0, 0.0],
            },
            FaceDetectionInput {
                bbox: [20.0, 20.0, 40.0, 40.0],
                confidence: 0.85,
                embedding: vec![0.0, 1.0],
            },
        ],
    )
    .unwrap();

    let faces = db.get_faces_for_media(media_id).unwrap();
    let person = db.create_person(Some("Group")).unwrap();
    db.assign_face_to_person(faces[0].id, person).unwrap();
    db.assign_face_to_person(faces[1].id, person).unwrap();

    let new_person = db
        .split_face_from_person(faces[0].id, Some("Split"))
        .unwrap();
    assert_ne!(new_person, person);
    assert_eq!(db.get_person_face_count(new_person).unwrap(), 1);
    assert_eq!(db.get_person_face_count(person).unwrap(), 1);

    let split_err = db.split_face_from_person(9999, None).unwrap_err();
    assert!(split_err.to_string().contains("face 9999 not found"));
}

#[test]
fn split_face_from_person_deletes_empty_old_person() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/solo.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.9,
            embedding: vec![1.0, 0.0],
        }],
    )
    .unwrap();

    let face_id = db.get_faces_for_media(media_id).unwrap()[0].id;
    let person = db.create_person(Some("Solo")).unwrap();
    db.assign_face_to_person(face_id, person).unwrap();

    let new_person = db.split_face_from_person(face_id, Some("New")).unwrap();
    assert_eq!(db.get_persons_count().unwrap(), 1);
    assert_eq!(db.get_person_face_count(new_person).unwrap(), 1);
    assert!(db.list_persons().unwrap().iter().all(|p| p.id != person));
}

#[test]
fn face_embedding_queries_and_unnamed_person_cleanup() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/embed.jpg"))
        .unwrap();

    db.store_face_detections(
        media_id,
        &[
            FaceDetectionInput {
                bbox: [0.0, 0.0, 10.0, 10.0],
                confidence: 0.9,
                embedding: vec![1.0, 0.0],
            },
            FaceDetectionInput {
                bbox: [20.0, 20.0, 40.0, 40.0],
                confidence: 0.85,
                embedding: vec![0.0, 1.0],
            },
        ],
    )
    .unwrap();

    let faces = db.get_faces_for_media(media_id).unwrap();
    let named = db.create_person(Some("Alice")).unwrap();
    let unnamed = db.create_person(None).unwrap();
    db.assign_face_to_person(faces[0].id, named).unwrap();
    db.assign_face_to_person(faces[1].id, unnamed).unwrap();

    let all = db.get_all_face_embeddings().unwrap();
    assert_eq!(all.len(), 2);

    let unassigned = db.get_unassigned_face_embeddings().unwrap();
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].0, faces[1].id);

    db.unassign_faces_from_unnamed_persons().unwrap();
    assert_eq!(db.get_person_face_count(unnamed).unwrap(), 0);
    db.delete_empty_unnamed_persons().unwrap();
    assert_eq!(db.get_persons_count().unwrap(), 1);

    db.clear_person_clusters().unwrap();
    assert_eq!(db.get_persons_count().unwrap(), 0);
    assert!(db.get_all_face_embeddings().unwrap().len() == 2);
    assert_eq!(db.get_unassigned_face_embeddings().unwrap().len(), 2);
}

#[test]
fn get_person_media_and_list_persons() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let media_a = db
        .upsert_media(fid, &sample_media("/photos/a.jpg"))
        .unwrap();
    let media_b = db
        .upsert_media(fid, &sample_media("/photos/b.jpg"))
        .unwrap();

    db.store_face_detections(
        media_a,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.9,
            embedding: vec![1.0, 0.0],
        }],
    )
    .unwrap();
    db.store_face_detections(
        media_b,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.9,
            embedding: vec![0.9, 0.1],
        }],
    )
    .unwrap();

    let face_a = db.get_faces_for_media(media_a).unwrap()[0].id;
    let face_b = db.get_faces_for_media(media_b).unwrap()[0].id;
    let person = db.create_person(Some("Traveler")).unwrap();
    db.assign_face_to_person(face_a, person).unwrap();
    db.assign_face_to_person(face_b, person).unwrap();

    assert_eq!(db.get_person_face_count(person).unwrap(), 2);
    assert_eq!(db.get_persons_count().unwrap(), 1);

    let persons = db.list_persons().unwrap();
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].name.as_deref(), Some("Traveler"));
    assert_eq!(persons[0].face_count, 2);
    assert_eq!(persons[0].sample_media_ids.len(), 2);

    let media = db.get_person_media(person, 10, 0).unwrap();
    assert_eq!(media.len(), 2);
    let paths: Vec<&str> = media.iter().map(|m| m.path.as_str()).collect();
    assert!(paths.contains(&"/photos/a.jpg"));
    assert!(paths.contains(&"/photos/b.jpg"));

    assert!(db.get_person_media(9999, 10, 0).unwrap().is_empty());
}

#[test]
fn list_media_without_clip_embedding_and_get_all_clip_embeddings() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let photo_id = db
        .upsert_media(fid, &sample_media("/photos/no_clip.jpg"))
        .unwrap();
    let mut video = sample_media("/photos/no_clip.mp4");
    video.media_type = MediaType::Video;
    db.upsert_media(fid, &video).unwrap();
    let shot_id = db
        .upsert_media(fid, &sample_screenshot("/photos/no_clip.png"))
        .unwrap();
    let with_clip = db
        .upsert_media(fid, &sample_media("/photos/has_clip.jpg"))
        .unwrap();
    db.store_clip_embedding(with_clip, &[1.0, 0.0]).unwrap();

    let missing = db.list_media_without_clip_embedding(10).unwrap();
    let missing_ids: Vec<i64> = missing.iter().map(|(id, _)| *id).collect();
    assert!(missing_ids.contains(&photo_id));
    assert!(missing_ids.contains(&shot_id));
    assert!(!missing_ids.contains(&with_clip));

    let limited = db.list_media_without_clip_embedding(1).unwrap();
    assert_eq!(limited.len(), 1);

    assert!(db.get_all_clip_embeddings().unwrap().len() == 1);

    db.store_clip_embedding(photo_id, &[0.5, 0.5]).unwrap();
    let all = db.get_all_clip_embeddings().unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|(id, _)| *id == photo_id));
}

#[test]
fn smart_album_media_query_and_delete() {
    use lightframe_db::SmartAlbumRule;

    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(fid, &sample_media("/photos/fav.jpg"))
        .unwrap();
    let fav_id = db
        .upsert_media(fid, &sample_media("/photos/fav2.jpg"))
        .unwrap();
    db.toggle_favorite(fav_id).unwrap();

    let rule = SmartAlbumRule {
        media_type: None,
        date_from: None,
        date_to: None,
        country: None,
        city: None,
        is_favorite: Some(true),
        min_size: None,
        has_gps: None,
    };
    let album = db.create_smart_album("Favorites", None, &rule).unwrap();

    let media = db.get_smart_album_media(album.id, 10, 0).unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].id, fav_id);

    let listed = db.list_smart_albums().unwrap();
    let favorites_album = listed
        .iter()
        .find(|a| a.name == "Favorites")
        .expect("Favorites smart album");
    assert_eq!(favorites_album.media_count, 1);

    db.delete_smart_album(album.id).unwrap();
    assert!(
        !db.list_smart_albums()
            .unwrap()
            .iter()
            .any(|a| a.name == "Favorites")
    );
    assert!(db.get_smart_album_media(album.id, 10, 0).is_err());
}

#[test]
fn batch_add_to_album_toggle_favorite_and_restore() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let album = db.create_album("Batch Album", None).unwrap();

    let id1 = db
        .upsert_media(fid, &sample_media("/photos/b1.jpg"))
        .unwrap();
    let id2 = db
        .upsert_media(fid, &sample_media("/photos/b2.jpg"))
        .unwrap();
    let id3 = db
        .upsert_media(fid, &sample_media("/photos/b3.jpg"))
        .unwrap();

    db.add_to_album(album.id, &[id1, id2, id3]).unwrap();
    let updated = db.get_album(album.id).unwrap().unwrap();
    assert_eq!(updated.media_count, 3);

    assert_eq!(db.batch_set_favorite(&[id1, id2], true).unwrap(), 2);
    assert!(db.is_favorite(id1).unwrap());
    assert!(db.is_favorite(id2).unwrap());
    assert!(!db.is_favorite(id3).unwrap());

    assert_eq!(db.batch_set_favorite(&[id1], false).unwrap(), 1);
    assert!(!db.is_favorite(id1).unwrap());

    db.batch_set_deleted(&[id2, id3], true).unwrap();
    assert_eq!(db.get_media_count().unwrap(), 1);

    assert_eq!(db.batch_set_deleted(&[id2, id3], false).unwrap(), 2);
    assert_eq!(db.get_media_count().unwrap(), 3);

    assert_eq!(db.batch_set_favorite(&[], true).unwrap(), 0);
    assert_eq!(db.batch_set_deleted(&[], false).unwrap(), 0);
}

#[test]
fn batch_favorite_skips_deleted_media() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/deleted.jpg"))
        .unwrap();
    db.set_deleted(media_id, true).unwrap();

    assert_eq!(db.batch_set_favorite(&[media_id], true).unwrap(), 0);
}

#[test]
fn get_duplicate_groups_count_empty_database() {
    let db = create_test_db();
    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
}

#[test]
fn set_folder_scan_status_updates_folder() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let before = db.get_watched_folder(fid).unwrap().unwrap();
    assert_eq!(before.scan_status, "idle");

    db.set_folder_scan_status(fid, "scanning").unwrap();
    let scanning = db.get_watched_folder(fid).unwrap().unwrap();
    assert_eq!(scanning.scan_status, "scanning");

    db.set_folder_scan_status(fid, "complete").unwrap();
    let complete = db.get_watched_folder(fid).unwrap().unwrap();
    assert_eq!(complete.scan_status, "complete");
}

fn set_deleted_at_days_ago(db: &Database, media_id: i64, days_ago: i64) {
    let conn = db.conn().unwrap();
    conn.execute(
        "UPDATE media_files SET deleted_at = datetime('now', ?1) WHERE id = ?2",
        rusqlite::params![format!("-{days_ago} days"), media_id],
    )
    .unwrap();
}

#[test]
fn list_expired_deleted_media_empty_when_no_deleted_items() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    db.upsert_media(fid, &sample_media("/photos/active.jpg"))
        .unwrap();

    assert!(db.list_expired_deleted_media(30).unwrap().is_empty());
}

#[test]
fn list_expired_deleted_media_empty_for_recently_deleted() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/recent.jpg"))
        .unwrap();

    db.set_deleted(media_id, true).unwrap();

    assert!(db.list_expired_deleted_media(30).unwrap().is_empty());
}

#[test]
fn list_expired_deleted_media_returns_items_deleted_over_30_days_ago() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/old.jpg"))
        .unwrap();

    db.set_deleted(media_id, true).unwrap();
    set_deleted_at_days_ago(&db, media_id, 40);

    let expired = db.list_expired_deleted_media(30).unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].0, "/photos/old.jpg");
}

#[test]
fn list_expired_deleted_media_returns_path_and_blake3_hash() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let mut media = sample_media("/photos/expired.jpg");
    media.blake3_hash = Some("expiredhash123".to_string());
    let media_id = db.upsert_media(fid, &media).unwrap();

    db.set_deleted(media_id, true).unwrap();
    set_deleted_at_days_ago(&db, media_id, 35);

    let expired = db.list_expired_deleted_media(30).unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].0, "/photos/expired.jpg");
    assert_eq!(expired[0].1.as_deref(), Some("expiredhash123"));
}

#[test]
fn list_expired_deleted_media_excludes_non_deleted_items() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let active_id = db
        .upsert_media(fid, &sample_media("/photos/active.jpg"))
        .unwrap();
    let deleted_id = db
        .upsert_media(fid, &sample_media("/photos/deleted.jpg"))
        .unwrap();

    db.set_deleted(deleted_id, true).unwrap();
    set_deleted_at_days_ago(&db, deleted_id, 45);

    let expired = db.list_expired_deleted_media(30).unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].0, "/photos/deleted.jpg");
    assert!(db.get_media_by_id(active_id).unwrap().is_some());
}

#[test]
fn list_media_hashes_by_folder_returns_hashes_for_media_in_folder() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/a.jpg", Some("hash_a"), None, None),
    )
    .unwrap();
    db.upsert_media(
        fid,
        &sample_media_with_hashes("/photos/b.jpg", Some("hash_b"), None, None),
    )
    .unwrap();

    let mut hashes = db.list_media_hashes_by_folder(fid).unwrap();
    hashes.sort();
    assert_eq!(hashes, vec!["hash_a".to_string(), "hash_b".to_string()]);
}

#[test]
fn list_media_hashes_by_folder_empty_for_empty_folder() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/empty");

    assert!(db.list_media_hashes_by_folder(fid).unwrap().is_empty());
}

#[test]
fn list_media_hashes_by_folder_excludes_other_folders() {
    let db = create_test_db();
    let f1 = insert_folder_id(&db, "/photos");
    let f2 = insert_folder_id(&db, "/backup");

    db.upsert_media(
        f1,
        &sample_media_with_hashes("/photos/a.jpg", Some("hash_photos"), None, None),
    )
    .unwrap();
    db.upsert_media(
        f2,
        &sample_media_with_hashes("/backup/b.jpg", Some("hash_backup"), None, None),
    )
    .unwrap();

    let hashes = db.list_media_hashes_by_folder(f1).unwrap();
    assert_eq!(hashes, vec!["hash_photos".to_string()]);
}

#[test]
fn cross_feature_delete_album_media_count_updates_on_soft_delete_and_restore() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/album.jpg"))
        .unwrap();
    let album = db.create_album("Trip", None).unwrap();
    db.add_to_album(album.id, &[media_id]).unwrap();

    assert_eq!(db.get_album_media(album.id, 10, 0).unwrap().len(), 1);

    db.set_deleted(media_id, true).unwrap();
    assert!(db.get_album_media(album.id, 10, 0).unwrap().is_empty());

    db.set_deleted(media_id, false).unwrap();
    assert_eq!(db.get_album_media(album.id, 10, 0).unwrap().len(), 1);
}

#[test]
fn cross_feature_delete_favorites_excludes_deleted_and_restores() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/fav.jpg"))
        .unwrap();
    db.toggle_favorite(media_id).unwrap();
    assert_eq!(db.get_favorites_count().unwrap(), 1);

    db.set_deleted(media_id, true).unwrap();
    assert!(db.get_favorites(10, 0).unwrap().is_empty());
    assert_eq!(db.get_favorites_count().unwrap(), 0);

    db.set_deleted(media_id, false).unwrap();
    assert_eq!(db.get_favorites_count().unwrap(), 1);
    assert_eq!(db.get_favorites(10, 0).unwrap()[0].id, media_id);
}

#[test]
fn cross_feature_delete_dedup_resolve_after_member_deleted() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");

    let keep_id = db
        .upsert_media(fid, &sample_media("/photos/keep.jpg"))
        .unwrap();
    let remove_id = db
        .upsert_media(fid, &sample_media("/photos/remove.jpg"))
        .unwrap();
    let group_id = db
        .create_duplicate_group("exact", &[keep_id, remove_id], &[1.0, 1.0])
        .unwrap();

    db.set_deleted(remove_id, true).unwrap();
    db.resolve_duplicate_group(group_id, keep_id, false)
        .unwrap();

    assert_eq!(db.get_duplicate_groups_count().unwrap(), 0);
    assert!(db.get_media_by_id(keep_id).unwrap().is_some());
    assert!(db.get_media_by_id(remove_id).unwrap().is_none());
}

#[test]
fn cross_feature_folder_removal_updates_album_counts() {
    let db = create_test_db();
    let f1 = insert_folder_id(&db, "/photos");
    let f2 = insert_folder_id(&db, "/backup");
    let id1 = db.upsert_media(f1, &sample_media("/photos/a.jpg")).unwrap();
    let id2 = db.upsert_media(f2, &sample_media("/backup/b.jpg")).unwrap();
    let album = db.create_album("All", None).unwrap();
    db.add_to_album(album.id, &[id1, id2]).unwrap();
    assert_eq!(db.get_album_media(album.id, 10, 0).unwrap().len(), 2);

    db.remove_watched_folder(f1).unwrap();
    assert_eq!(db.get_album_media(album.id, 10, 0).unwrap().len(), 1);
    assert_eq!(db.get_album(album.id).unwrap().unwrap().media_count, 1);
}

#[test]
fn cross_feature_clip_delete_excludes_from_semantic_search() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let id = db
        .upsert_media(fid, &sample_media("/photos/clip.jpg"))
        .unwrap();
    db.store_clip_embedding(id, &[1.0, 0.0, 0.0]).unwrap();

    let before = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.5, 10)
        .unwrap();
    assert_eq!(before.len(), 1);

    db.set_deleted(id, true).unwrap();
    let after = db
        .semantic_search_by_embedding(&[1.0, 0.0, 0.0], 0.5, 10)
        .unwrap();
    assert!(after.is_empty());
}

#[test]
fn cross_feature_face_delete_excludes_from_person_media() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let media_id = db
        .upsert_media(fid, &sample_media("/photos/face.jpg"))
        .unwrap();
    db.store_face_detections(
        media_id,
        &[FaceDetectionInput {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.95,
            embedding: vec![1.0, 0.0],
        }],
    )
    .unwrap();
    let face_id = db.get_faces_for_media(media_id).unwrap()[0].id;
    let person_id = db.create_person(Some("Alice")).unwrap();
    db.assign_face_to_person(face_id, person_id).unwrap();

    assert_eq!(db.get_person_media(person_id, 10, 0).unwrap().len(), 1);

    db.set_deleted(media_id, true).unwrap();
    assert!(db.get_person_media(person_id, 10, 0).unwrap().is_empty());
}

#[test]
fn cross_feature_fts_delete_excludes_from_search() {
    let db = create_test_db();
    let fid = insert_folder_id(&db, "/photos");
    let mut media = sample_media("/photos/searchable.jpg");
    media.filename = "unique_search_term.jpg".to_string();
    let media_id = db.upsert_media(fid, &media).unwrap();

    assert_eq!(
        db.search_media("unique_search_term", 10, 0).unwrap().len(),
        1
    );

    db.set_deleted(media_id, true).unwrap();
    assert!(
        db.search_media("unique_search_term", 10, 0)
            .unwrap()
            .is_empty()
    );
}
