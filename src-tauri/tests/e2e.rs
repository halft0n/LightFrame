use image::{ImageBuffer, Rgb};
use lightframe_app::export_edited_image;
use lightframe_core::media::{MediaFile, ThumbnailSize};
use lightframe_db::Database;
use lightframe_indexer::classify_extension;
use lightframe_thumbnail::thumb_path;
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

async fn index_folder(db: Arc<Database>, folder_id: i64) -> lightframe_core::Result<()> {
    let folder = db
        .get_watched_folder(folder_id)?
        .ok_or_else(|| lightframe_core::Error::Other(format!("folder {folder_id} not found")))?;

    db.set_folder_scan_status(folder_id, "scanning")?;

    let root = PathBuf::from(&folder.path);
    let files = lightframe_indexer::scan_folder(&root)
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

            let dhash = Some(lightframe_dedup::dhash_from_decoded(&decoded));
            let phash = Some(lightframe_dedup::phash_from_decoded(&decoded));
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

    let timeline = env.db.get_timeline_groups(10, 0).unwrap();
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
