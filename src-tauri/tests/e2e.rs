use image::{ImageBuffer, Rgb, RgbImage};
use lightframe_app::export_edited_image;
use lightframe_core::media::{MediaFile, MediaType, ThumbnailSize};
use lightframe_db::{Database, SmartAlbumRule};
use chrono::NaiveDateTime;
use lightframe_indexer::{
    classify_extension, is_media_change_event, is_media_remove_event, is_media_rename_event,
    scan_folder,
};
use lightframe_thumbnail::thumb_path;
use notify::event::{CreateKind, EventKind, ModifyKind, RemoveKind, RenameMode};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

struct TestEnv {
    _root: TempDir,
    db: Arc<Database>,
    photos_dir: PathBuf,
    folder_id: i64,
}

impl TestEnv {
    fn new() -> Self {
        let root = TempDir::new().expect("temp dir");
        let db_path = root.path().join("library.db");
        let db = Arc::new(Database::open(&db_path).expect("open db"));
        let photos_dir = root.path().join("photos");
        std::fs::create_dir_all(&photos_dir).expect("create photos dir");
        let folder = db
            .add_watched_folder(photos_dir.to_str().unwrap())
            .expect("add folder");

        Self {
            _root: root,
            db,
            photos_dir,
            folder_id: folder.id,
        }
    }

    fn write_png(&self, name: &str, tint: u8) -> PathBuf {
        let path = self.photos_dir.join(name);
        write_test_png(&path, tint);
        path
    }

    fn write_jpeg(&self, name: &str, tint: u8) -> PathBuf {
        let path = self.photos_dir.join(name);
        write_test_jpeg(&path, tint);
        path
    }

    fn write_avif(&self, name: &str, tint: u8) -> PathBuf {
        let path = self.photos_dir.join(name);
        write_test_avif(&path, tint);
        path
    }

    fn write_fake_heic(&self, name: &str) -> PathBuf {
        let path = self.photos_dir.join(name);
        std::fs::write(&path, b"fake heic payload").expect("write heic");
        path
    }

    fn write_fake_video(&self, name: &str) -> PathBuf {
        let path = self.photos_dir.join(name);
        std::fs::write(&path, b"fake mp4 payload").expect("write video");
        path
    }

    fn write_unsupported(&self, name: &str) -> PathBuf {
        let path = self.photos_dir.join(name);
        std::fs::write(&path, b"plain text").expect("write txt");
        path
    }

    fn write_raw_with_preview(&self, name: &str) -> PathBuf {
        let path = self.photos_dir.join(name);
        let img: RgbImage =
            ImageBuffer::from_fn(16, 16, |x, y| Rgb([(x * 15) as u8, (y * 15) as u8, 128]));
        let mut jpeg = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .expect("encode jpeg");
        let mut data = Vec::from(b"TIFF\x00\x01fake-raw-header\x00");
        data.extend_from_slice(&jpeg);
        std::fs::write(&path, &data).expect("write raw");
        path
    }

    async fn scan(&self) {
        index_folder(Arc::clone(&self.db), self.folder_id)
            .await
            .expect("scan folder");
    }
}

fn write_test_png(path: &Path, tint: u8) {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(64, 64, |x, y| Rgb([tint, (x % 256) as u8, (y % 256) as u8]));
    img.save(path).expect("write png");
}

fn write_test_jpeg(path: &Path, tint: u8) {
    let img: RgbImage =
        ImageBuffer::from_fn(64, 64, |x, y| Rgb([tint, (x % 256) as u8, (y % 256) as u8]));
    img.save_with_format(path, image::ImageFormat::Jpeg)
        .expect("write jpeg");
}

fn write_test_avif(path: &Path, tint: u8) {
    let img: RgbImage =
        ImageBuffer::from_fn(32, 32, |x, y| Rgb([tint, (x % 256) as u8, (y % 256) as u8]));
    img.save_with_format(path, image::ImageFormat::Avif)
        .expect("write avif");
}

async fn index_folder(db: Arc<Database>, folder_id: i64) -> lightframe_core::Result<()> {
    let folder = db
        .get_watched_folder(folder_id)?
        .ok_or_else(|| lightframe_core::Error::Other(format!("folder {folder_id} not found")))?;

    db.set_folder_scan_status(folder_id, "scanning")?;

    let root = PathBuf::from(&folder.path);
    let files = scan_folder(&root)
        .await
        .map_err(|e| lightframe_core::Error::Other(e.to_string()))?;

    for path in files {
        index_file(&db, folder_id, &path).await?;
    }

    db.update_last_scan_at(folder_id)?;
    db.set_folder_scan_status(folder_id, "idle")?;
    Ok(())
}

async fn index_file(db: &Database, folder_id: i64, path: &Path) -> lightframe_core::Result<()> {
    let media_type = classify_extension(path);

    let fs_meta = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || std::fs::metadata(&path)
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))??;

    let modified_at = fs_meta
        .modified()
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc())
        .unwrap_or_default();

    let path_str = path.to_string_lossy();
    if let Ok(Some(existing)) = db.get_media_by_path(&path_str)
        && existing.size_bytes == fs_meta.len()
        && existing.modified_at == modified_at
    {
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let meta = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || lightframe_metadata::extract(&path)
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))??;

    let blake3_hash = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || lightframe_dedup::file_hash(&path)
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))??;

    let (dhash, phash, micro_blob) = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        let hash = blake3_hash.clone();
        move || -> (Option<u64>, Option<u64>, Option<Vec<u8>>) {
            let decoded = match lightframe_core::decode::decode_image(&path) {
                Ok(d) => d,
                Err(_) => return (None, None, None),
            };

            let dhash = lightframe_dedup::dhash_from_decoded(&decoded).ok();
            let phash = lightframe_dedup::phash_from_decoded(&decoded).ok();
            let _ =
                lightframe_thumbnail::generate_from_decoded(&decoded, &hash, ThumbnailSize::Small);
            let micro = lightframe_thumbnail::micro_blob_from_decoded(&decoded).ok();
            (dhash, phash, micro)
        }
    })
    .await
    .map_err(|e| lightframe_core::Error::Other(e.to_string()))?;

    let media = MediaFile {
        id: 0,
        path: path.to_string_lossy().to_string(),
        filename,
        media_type,
        size_bytes: fs_meta.len(),
        width: meta.width,
        height: meta.height,
        created_at: meta.date_taken,
        modified_at,
        blake3_hash: Some(blake3_hash),
        dhash,
        phash,
        latitude: meta.latitude,
        longitude: meta.longitude,
    };

    let media_id = db.upsert_media(folder_id, &media)?;
    if let Some(blob) = micro_blob {
        let _ = db.set_micro_thumb(media_id, &blob);
    }

    Ok(())
}

#[tokio::test]
async fn e2e_add_folder_scan_and_browse() {
    let env = TestEnv::new();
    env.write_png("sunset.png", 40);
    env.write_png("beach.png", 80);

    env.scan().await;

    assert_eq!(env.db.get_media_count().unwrap(), 2);

    let media = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(media.len(), 2);

    for item in &media {
        let hash = item.blake3_hash.as_ref().expect("blake3 hash");
        let small = thumb_path(hash, ThumbnailSize::Small);
        assert!(
            small.exists(),
            "small thumbnail should exist for {}",
            item.filename
        );
    }

    let timeline = env.db.get_timeline_groups(10, None).unwrap();
    assert!(!timeline.is_empty());
    assert!(timeline.iter().any(|g| g.count >= 1));

    let search = env.db.search_media("sunset", 10, 0).unwrap();
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].filename, "sunset.png");

    env.db.clear_duplicate_groups().unwrap();
    let _exact = env.db.find_exact_duplicates().unwrap();
    let _perceptual = env.db.find_perceptual_duplicates(10).unwrap();
}

#[tokio::test]
async fn e2e_album_workflow() {
    let env = TestEnv::new();
    let _ = env.write_png("album-a.png", 10);
    let _ = env.write_png("album-b.png", 20);
    env.scan().await;

    let media = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(media.len(), 2);

    let album = env.db.create_album("Trip", None).unwrap();
    env.db
        .add_to_album(album.id, &[media[0].id, media[1].id])
        .unwrap();

    let album_media = env.db.get_album_media(album.id, 10, 0).unwrap();
    assert_eq!(album_media.len(), 2);

    env.db.set_album_cover(album.id, media[0].id).unwrap();
    env.db.update_album(album.id, "Summer Trip", None).unwrap();

    let updated = env.db.get_album(album.id).unwrap().unwrap();
    assert_eq!(updated.name, "Summer Trip");
    assert_eq!(updated.cover_media_id, Some(media[0].id));

    env.db.remove_from_album(album.id, media[1].id).unwrap();
    assert_eq!(env.db.get_album_media(album.id, 10, 0).unwrap().len(), 1);

    env.db.delete_album(album.id).unwrap();
    assert!(env.db.get_album(album.id).unwrap().is_none());
}

#[tokio::test]
async fn e2e_edit_and_export() {
    let env = TestEnv::new();
    let path = env.write_png("edit-me.png", 55);
    env.scan().await;

    let media = env.db.get_all_media(1, 0).unwrap();
    let media_id = media[0].id;

    let params = r#"{"exposure":0.2,"contrast":0.1}"#;
    env.db.save_edit_params(media_id, params).unwrap();
    assert!(env.db.has_edits(media_id).unwrap());

    let stored = env.db.get_edit_params(media_id).unwrap().unwrap();
    assert_eq!(stored, params);

    let output = env.photos_dir.join("edited-export.jpg");
    export_edited_image(&path, &output, params, 85).expect("export edited");
    assert!(output.exists());
    assert!(output.metadata().unwrap().len() > 0);

    env.db.clear_edit_params(media_id).unwrap();
    assert!(!env.db.has_edits(media_id).unwrap());
}

#[tokio::test]
async fn e2e_delete_workflow() {
    let env = TestEnv::new();
    let path = env.write_png("delete-me.png", 90);
    env.scan().await;

    let media_id = env.db.get_all_media(1, 0).unwrap()[0].id;
    assert!(path.exists());

    env.db.set_deleted(media_id, true).unwrap();
    let deleted = env.db.list_deleted_media().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, media_id);

    env.db.set_deleted(media_id, false).unwrap();
    assert!(env.db.list_deleted_media().unwrap().is_empty());
    assert!(env.db.get_media_by_id(media_id).unwrap().is_some());

    env.db.set_deleted(media_id, true).unwrap();
    env.db.permanently_delete_media(media_id).unwrap();
    std::fs::remove_file(&path).expect("remove file from disk");
    assert!(env.db.get_media_by_id(media_id).unwrap().is_none());
    assert!(!path.exists());
}

#[tokio::test]
async fn e2e_favorite_and_search() {
    let env = TestEnv::new();
    env.write_png("favorite-shot.png", 30);
    env.write_png("other.png", 60);
    env.scan().await;

    let items = env.db.get_all_media(10, 0).unwrap();
    let target = items
        .iter()
        .find(|m| m.filename == "favorite-shot.png")
        .expect("favorite-shot.png");

    let favorited = env.db.toggle_favorite(target.id).unwrap();
    assert!(favorited);
    assert!(env.db.is_favorite(target.id).unwrap());

    let favorites = env.db.get_favorites(10, 0).unwrap();
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].id, target.id);
    assert_eq!(env.db.get_favorites_count().unwrap(), 1);

    let results = env.db.search_media("favorite", 10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].filename, "favorite-shot.png");
    assert_eq!(env.db.search_media_count("favorite").unwrap(), 1);
}

#[tokio::test]
async fn e2e_mixed_file_types_scan() {
    let env = TestEnv::new();
    env.write_png("photo.png", 10);
    env.write_jpeg("photo.jpg", 20);
    env.write_avif("photo.avif", 30);
    env.write_fake_heic("photo.heic");
    env.write_fake_video("clip.mp4");
    let txt = env.write_unsupported("readme.txt");

    env.scan().await;

    assert!(!txt.exists() || classify_extension(&txt) == MediaType::Unknown);

    let media = env.db.get_all_media(20, 0).unwrap();
    let filenames: Vec<_> = media.iter().map(|m| m.filename.as_str()).collect();

    assert!(filenames.contains(&"photo.png"));
    assert!(filenames.contains(&"photo.jpg"));
    assert!(filenames.contains(&"photo.avif"));
    assert!(filenames.contains(&"photo.heic"));
    assert!(filenames.contains(&"clip.mp4"));
    assert!(!filenames.contains(&"readme.txt"));
    assert_eq!(media.len(), 5);

    let avif = media.iter().find(|m| m.filename == "photo.avif").unwrap();
    if let Some(hash) = avif.blake3_hash.as_ref() {
        let thumb = thumb_path(hash, ThumbnailSize::Small);
        if lightframe_core::decode::decode_image(&env.photos_dir.join("photo.avif")).is_ok() {
            assert!(
                thumb.exists(),
                "AVIF should produce a thumbnail when decode succeeds"
            );
        }
    }

    let heic = media.iter().find(|m| m.filename == "photo.heic").unwrap();
    if let Some(hash) = heic.blake3_hash.as_ref() {
        assert!(
            !thumb_path(hash, ThumbnailSize::Small).exists(),
            "HEIC should skip thumbnail generation without libheif"
        );
    }
}

#[tokio::test]
async fn e2e_skip_unchanged_rescan() {
    let env = TestEnv::new();
    let path = env.write_png("stable.png", 42);
    env.scan().await;

    let before = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(before.len(), 1);
    let original_modified = before[0].modified_at;

    env.scan().await;

    let after_rescan = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(after_rescan.len(), 1);
    assert_eq!(after_rescan[0].modified_at, original_modified);

    std::thread::sleep(std::time::Duration::from_millis(50));
    let img: RgbImage =
        ImageBuffer::from_fn(128, 64, |x, y| Rgb([99, (x % 256) as u8, (y % 256) as u8]));
    img.save(&path).expect("rewrite png");
    env.scan().await;

    let after_change = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(after_change.len(), 1);
    assert_ne!(after_change[0].size_bytes, before[0].size_bytes);
}

#[test]
fn e2e_watcher_event_classification() {
    use notify::Event;

    let create = Event {
        kind: EventKind::Create(CreateKind::File),
        paths: vec![PathBuf::from("/library/new.jpg")],
        attrs: notify::event::EventAttributes::default(),
    };
    assert!(is_media_change_event(&create));
    assert!(!is_media_remove_event(&create));
    assert!(!is_media_rename_event(&create));

    let remove = Event {
        kind: EventKind::Remove(RemoveKind::File),
        paths: vec![PathBuf::from("/library/old.png")],
        attrs: notify::event::EventAttributes::default(),
    };
    assert!(is_media_change_event(&remove));
    assert!(is_media_remove_event(&remove));

    let rename = Event {
        kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
        paths: vec![
            PathBuf::from("/library/old.jpg"),
            PathBuf::from("/library/new.jpg"),
        ],
        attrs: notify::event::EventAttributes::default(),
    };
    assert!(is_media_rename_event(&rename));
    assert!(is_media_change_event(&rename));
}

#[tokio::test]
async fn e2e_watcher_remove_soft_deletes_media() {
    let env = TestEnv::new();
    let path = env.write_png("watched.png", 15);
    env.scan().await;

    let media = env.db.get_all_media(1, 0).unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].filename, "watched.png");

    let deleted = env.db.soft_delete_by_path(&path.to_string_lossy()).unwrap();
    assert!(deleted);

    let removed = env.db.list_deleted_media().unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].filename, "watched.png");
}

#[tokio::test]
async fn e2e_raw_scan_and_thumbnail() {
    let env = TestEnv::new();
    let path = env.write_raw_with_preview("landscape.cr2");
    env.scan().await;

    let media = env.db.get_all_media(10, 0).unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].filename, "landscape.cr2");
    assert_eq!(media[0].media_type, MediaType::Raw);

    let hash = media[0].blake3_hash.as_ref().expect("blake3 hash");
    let small = thumb_path(hash, ThumbnailSize::Small);
    assert!(
        small.exists(),
        "RAW with embedded preview should produce a thumbnail"
    );
    assert!(path.exists());
}

#[tokio::test]
async fn e2e_soft_delete_and_permanent_delete_by_id() {
    let env = TestEnv::new();
    let path_a = env.write_png("keep-by-id.png", 11);
    let path_b = env.write_png("trash-by-id.png", 22);
    env.scan().await;

    let items = env.db.get_all_media(10, 0).unwrap();
    let id_a = items.iter().find(|m| m.filename == "keep-by-id.png").unwrap().id;
    let id_b = items.iter().find(|m| m.filename == "trash-by-id.png").unwrap().id;

    env.db.set_deleted(id_b, true).unwrap();
    let deleted = env.db.list_deleted_media().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, id_b);
    assert!(env.db.get_media_by_id(id_b).unwrap().is_none());
    assert!(env.db.get_media_by_id(id_a).unwrap().is_some());

    env.db.permanently_delete_media(id_b).unwrap();
    std::fs::remove_file(&path_b).expect("remove trashed file");
    assert!(env.db.get_media_by_id(id_b).unwrap().is_none());
    assert!(env.db.get_media_by_id(id_a).unwrap().is_some());
    assert!(path_a.exists());
}

#[tokio::test]
async fn e2e_batch_delete_favorite_and_add_to_album() {
    let env = TestEnv::new();
    env.write_png("batch-a.png", 10);
    env.write_png("batch-b.png", 20);
    env.write_png("batch-c.png", 30);
    env.scan().await;

    let items = env.db.get_all_media(10, 0).unwrap();
    let ids: Vec<i64> = items.iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), 3);

    assert_eq!(env.db.batch_set_favorite(&ids[0..2], true).unwrap(), 2);
    assert!(env.db.is_favorite(ids[0]).unwrap());
    assert!(env.db.is_favorite(ids[1]).unwrap());
    assert!(!env.db.is_favorite(ids[2]).unwrap());

    let album = env.db.create_album("Batch Trip", None).unwrap();
    env.db.add_to_album(album.id, &ids).unwrap();
    assert_eq!(env.db.get_album_media(album.id, 10, 0).unwrap().len(), 3);

    assert_eq!(env.db.batch_set_deleted(&ids[1..3], true).unwrap(), 2);
    assert_eq!(env.db.get_media_count().unwrap(), 1);
    assert_eq!(env.db.list_deleted_media().unwrap().len(), 2);

    assert_eq!(env.db.batch_set_deleted(&ids[1..3], false).unwrap(), 2);
    assert_eq!(env.db.get_media_count().unwrap(), 3);
}

#[tokio::test]
async fn e2e_smart_album_crud() {
    let env = TestEnv::new();
    env.write_png("smart-fav.png", 40);
    env.write_png("smart-plain.png", 50);
    env.scan().await;

    let items = env.db.get_all_media(10, 0).unwrap();
    let fav_item = items.iter().find(|m| m.filename == "smart-fav.png").unwrap();
    env.db.toggle_favorite(fav_item.id).unwrap();

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
    let album = env
        .db
        .create_smart_album("E2E Favorites", Some("star"), &rule)
        .unwrap();
    assert_eq!(album.name, "E2E Favorites");
    assert_eq!(album.media_count, 1);

    let listed = env.db.list_smart_albums().unwrap();
    assert!(listed.iter().any(|a| a.id == album.id));

    let media = env.db.get_smart_album_media(album.id, 10, 0).unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].id, fav_item.id);

    env.db.delete_smart_album(album.id).unwrap();
    assert!(
        !env
            .db
            .list_smart_albums()
            .unwrap()
            .iter()
            .any(|a| a.id == album.id)
    );
}

#[tokio::test]
async fn e2e_memories_generation() {
    let env = TestEnv::new();
    let dt = NaiveDateTime::parse_from_str("2023-08-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..5 {
        let path = env.write_png(&format!("mem-{i}.png"), 60 + i);
        env.scan().await;
        let mut media = env.db.get_media_by_path(&path.to_string_lossy()).unwrap().unwrap();
        media.created_at = Some(dt + chrono::Duration::hours(i as i64));
        media.latitude = Some(31.2304);
        media.longitude = Some(121.4737);
        env.db.upsert_media(env.folder_id, &media).unwrap();
        env.db
            .update_media_location(media.id, "上海", "中国")
            .unwrap();
    }

    let memories = env.db.generate_memories().unwrap();
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].media_count, 5);
    assert!(memories[0].title.contains("2023"));

    let listed = env.db.list_memories().unwrap();
    assert_eq!(listed.len(), 1);

    let memory_media = env.db.get_memory_media(listed[0].id, 10, 0).unwrap();
    assert_eq!(memory_media.len(), 5);
}

#[tokio::test]
async fn e2e_timeline_cursor_pagination() {
    let env = TestEnv::new();
    for i in 0..6 {
        let path = env.write_png(&format!("timeline-{i}.png"), 70 + i);
        env.scan().await;
        let date = format!("2024-06-{:02} 10:00:00", 10 + i);
        let parsed = NaiveDateTime::parse_from_str(&date, "%Y-%m-%d %H:%M:%S").unwrap();
        let mut media = env.db.get_media_by_path(&path.to_string_lossy()).unwrap().unwrap();
        media.created_at = Some(parsed);
        env.db.upsert_media(env.folder_id, &media).unwrap();
    }

    let page1 = env.db.get_timeline_groups(3, None).unwrap();
    assert_eq!(page1.iter().map(|g| g.count).sum::<i64>(), 3);

    let page1_ids: Vec<i64> = page1
        .iter()
        .flat_map(|g| g.media.iter().map(|m| m.id))
        .collect();
    let last_id = *page1_ids.last().expect("page1 item");
    let cursor_ts = "2024-06-13 10:00:00".to_string();

    let page2 = env
        .db
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
