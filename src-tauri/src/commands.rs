use crate::scan;
use crate::state::{AppState, ScanProgress};
use crate::watcher;
use lightframe_ai::AiStatus;
use lightframe_core::config::AppConfig;
use lightframe_core::media::{MediaFile, ThumbnailSize};
use lightframe_db::{
    Album, DuplicateGroupDetail, GeoCluster, LocationGroup, LocationStats, MediaNeighbors, Memory,
    Person, SmartAlbum, SmartAlbumRule, TimelineGroup, WatchedFolder,
};
use lightframe_thumbnail::thumb_path;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

const MAX_BATCH_SIZE: usize = 900;

fn check_batch_size(media_ids: &[i64]) -> Result<(), String> {
    if media_ids.len() > MAX_BATCH_SIZE {
        return Err(format!(
            "batch size {} exceeds maximum {}",
            media_ids.len(),
            MAX_BATCH_SIZE
        ));
    }
    Ok(())
}

fn validate_media_path(db: &lightframe_db::Database, path: &str) -> Result<(), String> {
    let file_path = std::path::Path::new(path);
    if crate::original_protocol::path_contains_parent_dir(file_path) {
        return Err("invalid path".to_string());
    }
    let folders = db.list_watched_folders().map_err(|e| e.to_string())?;
    let check_path = match file_path.canonicalize() {
        Ok(raw) => crate::original_protocol::strip_extended_prefix(raw),
        Err(_) => {
            // File doesn't exist yet — try canonicalizing parent to resolve short names (Windows)
            if let Some(parent) = file_path.parent()
                && let Ok(canonical_parent) = parent.canonicalize()
                && let Some(name) = file_path.file_name()
            {
                crate::original_protocol::strip_extended_prefix(canonical_parent).join(name)
            } else {
                file_path.to_path_buf()
            }
        }
    };
    if !crate::original_protocol::path_is_in_watched_folders(&check_path, &folders) {
        return Err("path outside watched folders".to_string());
    }
    Ok(())
}

fn db_err(cmd: &str, context: &str, e: impl std::fmt::Display) -> String {
    format!("{cmd}({context}): {e}")
}

#[derive(Serialize)]
pub struct DedupScanResult {
    pub exact_groups: usize,
    pub perceptual_groups: usize,
    pub total_duplicates: usize,
}

#[tauri::command]
pub fn get_log_directory() -> String {
    crate::logging::log_directory()
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn get_log_files() -> Vec<crate::logging::LogFileInfo> {
    crate::logging::list_log_files()
}

#[tauri::command]
pub fn cleanup_logs() -> Result<(), String> {
    let dir = crate::logging::log_directory();
    crate::logging::cleanup_logs(&dir);
    Ok(())
}

#[tauri::command]
pub fn get_log_config() -> lightframe_core::config::LogConfig {
    crate::logging::get_log_config()
}

#[tauri::command]
pub fn set_log_config(config: lightframe_core::config::LogConfig) -> Result<(), String> {
    crate::logging::set_log_config(config.clone());
    lightframe_core::config::update_log_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.clone()
}

#[tauri::command]
pub fn add_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<WatchedFolder, String> {
    let canonical =
        std::fs::canonicalize(&path).map_err(|e| format!("cannot resolve path: {e}"))?;
    #[cfg(windows)]
    let canonical = crate::original_protocol::strip_extended_prefix(canonical);
    if !canonical.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let path_str = canonical
        .to_str()
        .ok_or_else(|| "path contains invalid unicode".to_string())?;

    let folder = state
        .db
        .add_watched_folder(path_str)
        .map_err(|e| e.to_string())?;

    scan::spawn_scan(app.clone(), &state, folder.id);
    if let Err(e) = watcher::start(&app, &state) {
        tracing::warn!(folder_id = folder.id, "failed to start file watcher: {e}");
    }
    Ok(folder)
}

#[tauri::command]
pub fn remove_watched_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let hashes = state
        .db
        .list_media_hashes_by_folder(id)
        .map_err(|e| e.to_string())?;
    state
        .db
        .remove_watched_folder(id)
        .map_err(|e| e.to_string())?;
    for hash in hashes {
        if hash.len() >= 4 {
            for size in [
                ThumbnailSize::Micro,
                ThumbnailSize::Small,
                ThumbnailSize::Large,
            ] {
                let thumb = thumb_path(&hash, size);
                if thumb.exists()
                    && let Err(e) = std::fs::remove_file(&thumb)
                {
                    tracing::warn!(
                        "remove folder: failed to remove thumbnail {}: {e}",
                        thumb.display()
                    );
                }
            }
        }
    }
    watcher::start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_watched_folders(state: State<'_, AppState>) -> Result<Vec<WatchedFolder>, String> {
    state.db.list_watched_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_list(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_all_media(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn get_media_page(
    state: State<'_, AppState>,
    limit: i64,
    cursor: Option<(String, i64)>,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    state
        .db
        .get_media_page(limit, cursor)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count(state: State<'_, AppState>) -> Result<i64, String> {
    state.db.get_media_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_folder(
    state: State<'_, AppState>,
    folder_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_folder(folder_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count_by_folder(
    state: State<'_, AppState>,
    folder_id: i64,
) -> Result<i64, String> {
    state
        .db
        .get_media_count_by_folder(folder_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_export(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
    output_dir: String,
) -> Result<usize, String> {
    batch_export_impl(&state, media_ids, output_dir)
}

pub(crate) fn batch_export_impl(
    state: &AppState,
    media_ids: Vec<i64>,
    output_dir: String,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    let output = std::path::Path::new(&output_dir);
    if !output.is_dir() {
        return Err(format!("not a directory: {output_dir}"));
    }

    let mut exported = 0usize;
    let mut copy_attempts = 0usize;
    for media_id in media_ids {
        let Some(media) = state
            .db
            .get_media_by_id(media_id)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };

        if let Err(e) = validate_media_path(&state.db, &media.path) {
            tracing::warn!("batch export: skipping media {}: {e}", media_id);
            continue;
        }

        let source = std::path::Path::new(&media.path);
        if !source.is_file() {
            tracing::warn!("batch export: source file not found: {}", media.path);
            continue;
        }

        let sanitized_name = sanitize_filename(&media.filename);
        let dest = unique_export_path(output, &sanitized_name);
        if !dest.starts_with(output) {
            return Err("invalid export path".to_string());
        }
        copy_attempts += 1;
        if let Err(e) = std::fs::copy(source, &dest) {
            tracing::warn!(
                "batch export: failed to copy {} to {}: {e}",
                media.path,
                dest.display()
            );
            continue;
        }
        exported += 1;
    }

    if copy_attempts > 0 && exported == 0 {
        return Err("batch export failed for all items".to_string());
    }

    Ok(exported)
}

fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("unnamed");
    let base = if base.is_empty() || base == "." || base == ".." {
        "unnamed"
    } else {
        base
    };
    base.to_string()
}

fn unique_export_path(dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let filename = sanitize_filename(filename);
    let mut dest = dir.join(&filename);
    if !dest.exists() {
        return dest;
    }

    let path = std::path::Path::new(&filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename);
    let ext = path.extension().and_then(|s| s.to_str());

    for i in 1..1000 {
        let candidate_name = match ext {
            Some(ext) => format!("{stem}_{i}.{ext}"),
            None => format!("{stem}_{i}"),
        };
        dest = dir.join(&candidate_name);
        if !dest.exists() {
            return dest;
        }
    }

    dir.join(&filename)
}

#[cfg(test)]
mod command_validation_tests {
    use super::*;

    #[test]
    fn check_batch_size_accepts_empty_array() {
        assert!(check_batch_size(&[]).is_ok());
    }

    #[test]
    fn check_batch_size_accepts_max_allowed() {
        let ids: Vec<i64> = (1..=MAX_BATCH_SIZE as i64).collect();
        assert!(check_batch_size(&ids).is_ok());
    }

    #[test]
    fn check_batch_size_rejects_over_max() {
        let ids: Vec<i64> = (0..=MAX_BATCH_SIZE as i64).collect();
        let err = check_batch_size(&ids).unwrap_err();
        assert!(err.contains("batch size"));
        assert!(err.contains(&MAX_BATCH_SIZE.to_string()));
    }

    #[test]
    fn check_batch_size_accepts_single_item() {
        assert!(check_batch_size(&[42]).is_ok());
    }

    #[test]
    fn check_batch_size_rejects_one_over_max() {
        let ids: Vec<i64> = (0..=MAX_BATCH_SIZE as i64).collect();
        assert_eq!(ids.len(), MAX_BATCH_SIZE + 1);
        let err = check_batch_size(&ids).unwrap_err();
        assert!(err.contains(&format!("batch size {}", MAX_BATCH_SIZE + 1)));
    }

    #[test]
    fn db_err_includes_command_context_and_error() {
        let msg = db_err("batch_delete_media", "3 ids", "database locked");
        assert_eq!(msg, "batch_delete_media(3 ids): database locked");
    }

    #[test]
    fn truncate_utf8_zero_limit_returns_empty() {
        assert_eq!(truncate_utf8("hello", 0), "");
        assert_eq!(truncate_utf8("你好", 0), "");
    }

    #[test]
    fn save_edit_params_size_limit_constant() {
        const MAX_EDIT_PARAMS_SIZE: usize = 64 * 1024;
        let oversized = "x".repeat(MAX_EDIT_PARAMS_SIZE + 1);
        assert!(oversized.len() > MAX_EDIT_PARAMS_SIZE);
        let valid = "x".repeat(MAX_EDIT_PARAMS_SIZE);
        assert_eq!(valid.len(), MAX_EDIT_PARAMS_SIZE);
    }

    #[test]
    fn save_edit_params_parses_valid_json_under_limit() {
        let params = r#"{"exposure":0.1,"contrast":0.2}"#;
        assert!(params.len() < 64 * 1024);
        crate::image_edit::parse_edit_params(params).expect("valid edit params");
    }

    #[test]
    fn save_edit_params_rejects_invalid_json() {
        let params = "{not valid json";
        assert!(crate::image_edit::parse_edit_params(params).is_err());
    }

    #[test]
    fn truncate_utf8_ascii_shorter_than_limit_unchanged() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
    }

    #[test]
    fn truncate_utf8_ascii_at_limit_unchanged() {
        assert_eq!(truncate_utf8("hello", 5), "hello");
    }

    #[test]
    fn truncate_utf8_ascii_over_limit_truncated() {
        assert_eq!(truncate_utf8("hello world", 5), "hello");
    }

    #[test]
    fn truncate_utf8_cjk_truncates_at_char_boundary() {
        assert_eq!(truncate_utf8("你好世界", 7), "你好");
    }

    #[test]
    fn truncate_utf8_empty_string_returns_empty() {
        assert_eq!(truncate_utf8("", 10), "");
    }

    #[test]
    fn truncate_utf8_single_multibyte_char_limit_one_returns_empty() {
        assert_eq!(truncate_utf8("你", 1), "");
    }

    #[test]
    fn truncate_utf8_mixed_ascii_and_cjk() {
        assert_eq!(truncate_utf8("ab你好cd", 5), "ab你");
    }
}

#[cfg(test)]
mod path_validation_tests {
    use super::*;

    fn test_db_with_watched_dir(dir: &std::path::Path) -> lightframe_db::Database {
        let canonical =
            crate::original_protocol::strip_extended_prefix(std::fs::canonicalize(dir).unwrap());
        let db = lightframe_db::Database::open(std::path::Path::new(":memory:")).unwrap();
        db.add_watched_folder(canonical.to_str().unwrap()).unwrap();
        db
    }

    #[test]
    fn validate_media_path_rejects_traversal() {
        let db = lightframe_db::Database::open(std::path::Path::new(":memory:")).unwrap();
        db.add_watched_folder("/photos").unwrap();

        assert!(validate_media_path(&db, "/photos/../etc/passwd").is_err());
        assert!(validate_media_path(&db, "..\\photos\\secret.jpg").is_err());
    }

    #[test]
    fn validate_media_path_rejects_outside_watched_folders() {
        let watched = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let db = test_db_with_watched_dir(watched.path());

        let file = outside.path().join("secret.jpg");
        std::fs::write(&file, b"jpeg").unwrap();

        let err = validate_media_path(&db, file.to_str().unwrap()).unwrap_err();
        assert!(err.contains("outside watched folders"));
    }

    #[test]
    fn validate_media_path_accepts_file_under_watched_folder() {
        let dir = tempfile::tempdir().unwrap();
        let db = test_db_with_watched_dir(dir.path());
        let file = dir.path().join("photo.jpg");
        std::fs::write(&file, b"jpeg").unwrap();

        assert!(validate_media_path(&db, &file.to_string_lossy()).is_ok());
    }

    #[test]
    fn validate_media_path_nonexistent_file_without_traversal_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let db = test_db_with_watched_dir(dir.path());
        let missing = dir.path().join("missing.jpg");

        assert!(validate_media_path(&db, &missing.to_string_lossy()).is_ok());
    }

    #[test]
    fn validate_media_path_rejects_empty_watched_folder_list() {
        let db = lightframe_db::Database::open(std::path::Path::new(":memory:")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("photo.jpg");
        std::fs::write(&file, b"jpeg").unwrap();

        let err = validate_media_path(&db, file.to_str().unwrap()).unwrap_err();
        assert!(err.contains("outside watched folders"));
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn sanitize_filename_strips_path_traversal() {
        assert_eq!(sanitize_filename("../../outside.jpg"), "outside.jpg");
        assert_eq!(sanitize_filename("normal.jpg"), "normal.jpg");
        assert_eq!(sanitize_filename("sub/file.jpg"), "file.jpg");
        assert_eq!(sanitize_filename("..\\..\\evil.png"), "evil.png");
        assert_eq!(sanitize_filename(""), "unnamed");
        assert_eq!(sanitize_filename(".."), "unnamed");
    }

    #[test]
    fn unique_export_path_stays_within_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dest = unique_export_path(dir.path(), "../../outside.jpg");
        assert!(dest.starts_with(dir.path()));
        assert_eq!(
            dest.file_name().and_then(|n| n.to_str()),
            Some("outside.jpg")
        );
    }

    #[test]
    fn unique_export_path_sanitizes_separators_in_name() {
        let dir = tempfile::tempdir().unwrap();
        let dest = unique_export_path(dir.path(), "foo/bar.jpg");
        assert_eq!(dest, dir.path().join("bar.jpg"));
    }

    #[test]
    fn unique_export_path_adds_numbered_suffix_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("photo.jpg"), b"original").unwrap();

        let dest = unique_export_path(dir.path(), "photo.jpg");
        assert_eq!(dest, dir.path().join("photo_1.jpg"));
        assert!(!dest.exists());
    }

    #[test]
    fn unique_export_path_increments_suffix_until_free() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("photo.jpg"), b"1").unwrap();
        std::fs::write(dir.path().join("photo_1.jpg"), b"2").unwrap();
        std::fs::write(dir.path().join("photo_2.jpg"), b"3").unwrap();

        let dest = unique_export_path(dir.path(), "photo.jpg");
        assert_eq!(dest, dir.path().join("photo_3.jpg"));
    }

    #[test]
    fn unique_export_path_handles_extensionless_collision() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README"), b"v1").unwrap();

        let dest = unique_export_path(dir.path(), "README");
        assert_eq!(dest, dir.path().join("README_1"));
    }

    #[test]
    fn sanitize_filename_dot_segments_become_unnamed() {
        assert_eq!(sanitize_filename("   "), "   ");
        assert_eq!(sanitize_filename(""), "unnamed");
        assert_eq!(sanitize_filename("///"), "unnamed");
        assert_eq!(sanitize_filename("."), "unnamed");
    }
}

#[cfg(test)]
mod batch_export_tests {
    use super::*;
    use crate::state::ScanStatus;
    use lightframe_core::media::{MediaFile, MediaType};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn sample_media(path: &str) -> MediaFile {
        MediaFile {
            id: 0,
            path: path.to_string(),
            filename: path.rsplit('/').next().unwrap_or(path).to_string(),
            media_type: MediaType::Photo,
            size_bytes: 512,
            width: Some(64),
            height: Some(64),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        }
    }

    fn test_app_state(db: Arc<lightframe_db::Database>) -> AppState {
        AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
        }
    }

    fn setup_watched_env() -> (tempfile::TempDir, AppState, i64, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("photos");
        std::fs::create_dir_all(&watched).unwrap();
        let watched_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        );
        let watched_str = watched_canonical.to_str().unwrap().to_string();

        let db_path = root.path().join("library.db");
        let db = Arc::new(lightframe_db::Database::open(&db_path).unwrap());
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;
        let state = test_app_state(db);
        (root, state, folder_id, watched_canonical)
    }

    #[test]
    fn batch_export_empty_batch_returns_zero() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let db = Arc::new(
            lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap(),
        );
        let state = test_app_state(db);

        let exported =
            batch_export_impl(&state, vec![], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
    }

    #[test]
    fn batch_export_all_invalid_ids_returns_zero() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db);

        let exported =
            batch_export_impl(&state, vec![999, 1000], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
        assert!(std::fs::read_dir(&out).unwrap().next().is_none());
    }

    #[test]
    fn batch_export_mixed_valid_and_invalid_exports_only_valid() {
        let (_root, state, folder_id, watched) = setup_watched_env();
        let out = _root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();

        let valid_path = watched.join("good.jpg");
        std::fs::write(&valid_path, b"jpeg-bytes").unwrap();
        let valid_id = state
            .db
            .upsert_media(folder_id, &sample_media(valid_path.to_str().unwrap()))
            .unwrap();

        let exported = batch_export_impl(
            &state,
            vec![valid_id, 99999],
            out.to_string_lossy().to_string(),
        )
        .unwrap();
        assert_eq!(exported, 1);
        assert!(out.join("good.jpg").is_file());
    }

    #[test]
    fn batch_export_rejects_non_directory_output() {
        let (_root, state, folder_id, watched) = setup_watched_env();
        let file_path = watched.join("photo.jpg");
        std::fs::write(&file_path, b"jpeg").unwrap();
        let media_id = state
            .db
            .upsert_media(folder_id, &sample_media(file_path.to_str().unwrap()))
            .unwrap();

        let err = batch_export_impl(
            &state,
            vec![media_id],
            file_path.to_string_lossy().to_string(),
        )
        .unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn batch_export_skips_path_outside_watched_folders() {
        let root = tempfile::tempdir().unwrap();
        let watched = root.path().join("watched");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&watched).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let watched_str = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&watched).unwrap(),
        )
        .to_str()
        .unwrap()
        .to_string();

        let db_path = root.path().join("library.db");
        let db = Arc::new(lightframe_db::Database::open(&db_path).unwrap());
        let folder_id = db.add_watched_folder(&watched_str).unwrap().id;
        let state = test_app_state(db);

        let outside_canonical = crate::original_protocol::strip_extended_prefix(
            std::fs::canonicalize(&outside).unwrap(),
        );
        let outside_file = outside_canonical.join("secret.jpg");
        std::fs::write(&outside_file, b"jpeg").unwrap();
        let media_id = state
            .db
            .upsert_media(folder_id, &sample_media(outside_file.to_str().unwrap()))
            .unwrap();

        let out = root.path().join("export");
        std::fs::create_dir_all(&out).unwrap();
        let exported =
            batch_export_impl(&state, vec![media_id], out.to_string_lossy().to_string()).unwrap();
        assert_eq!(exported, 0);
    }

    #[test]
    fn face_detecting_guard_releases_on_drop() {
        let flag = Arc::new(AtomicBool::new(true));
        struct FaceDetectingGuard(Arc<AtomicBool>);
        impl Drop for FaceDetectingGuard {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
            }
        }
        {
            let _guard = FaceDetectingGuard(Arc::clone(&flag));
            assert!(flag.load(Ordering::SeqCst));
        }
        assert!(!flag.load(Ordering::SeqCst));
    }
}

#[cfg(test)]
mod rename_person_tests {
    use super::*;
    use crate::state::ScanStatus;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_app_state(db: Arc<lightframe_db::Database>) -> AppState {
        AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
        }
    }

    #[test]
    fn rename_person_rejects_empty_and_whitespace_names() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db.clone());
        let person_id = db.create_person(Some("Original")).unwrap();

        for name in ["", "  ", "\t", "   \n  "] {
            let err = rename_person_impl(&state, person_id, name.to_string()).unwrap_err();
            assert!(err.contains("cannot be empty"), "name={name:?}: {err}");
        }
    }

    #[test]
    fn rename_person_trims_whitespace() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db.clone());
        let person_id = db.create_person(Some("Original")).unwrap();

        rename_person_impl(&state, person_id, "  Alicia  ".to_string()).unwrap();
        let persons = db.list_persons().unwrap();
        assert_eq!(persons[0].name.as_deref(), Some("Alicia"));
    }
}

#[cfg(test)]
mod edit_persistence_tests {
    use super::*;
    use crate::state::ScanStatus;
    use lightframe_core::media::{MediaFile, MediaType};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn sample_media(path: &str) -> MediaFile {
        MediaFile {
            id: 0,
            path: path.to_string(),
            filename: path.rsplit('/').next().unwrap_or(path).to_string(),
            media_type: MediaType::Photo,
            size_bytes: 512,
            width: Some(64),
            height: Some(64),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: None,
            dhash: None,
            phash: None,
            latitude: None,
            longitude: None,
        }
    }

    #[test]
    fn edit_save_load_revert_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let db = lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap();
        let folder_id = db.add_watched_folder("/photos").unwrap().id;
        let media_id = db
            .upsert_media(folder_id, &sample_media("/photos/edit.jpg"))
            .unwrap();

        let params = r#"{"brightness":12.0,"contrast":-5.0}"#;
        crate::image_edit::parse_edit_params(params).expect("valid params");
        db.save_edit_params(media_id, params).unwrap();
        assert!(db.has_edits(media_id).unwrap());

        let loaded = db.get_edit_params(media_id).unwrap().expect("saved params");
        assert_eq!(loaded, params);

        db.clear_edit_params(media_id).unwrap();
        assert!(!db.has_edits(media_id).unwrap());
        assert!(db.get_edit_params(media_id).unwrap().is_none());
    }

    #[test]
    fn edit_save_rejects_invalid_json_via_parse() {
        let invalid = r#"{"brightness":"not-a-number"}"#;
        assert!(crate::image_edit::parse_edit_params(invalid).is_err());
    }

    #[tokio::test]
    async fn detect_faces_releases_ai_lock_on_not_found() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(
            lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap(),
        );
        let state = AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
        };

        let err = detect_and_store_faces_for_media(&state, 99999)
            .await
            .unwrap_err();
        assert!(err.contains("not found"));

        let lock = state.ai.try_lock();
        assert!(
            lock.is_ok(),
            "AI mutex should be released after early return"
        );
    }
}

#[tauri::command]
pub fn get_media_by_id(state: State<'_, AppState>, id: i64) -> Result<Option<MediaFile>, String> {
    state
        .db
        .get_media_by_id(id)
        .map_err(|e| db_err("get_media_by_id", &id.to_string(), e))
}

#[tauri::command]
#[tracing::instrument(skip(app, state))]
pub fn scan_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_id: i64,
) -> Result<(), String> {
    state
        .db
        .get_watched_folder(folder_id)
        .map_err(|e| db_err("scan_folder", &folder_id.to_string(), e))?
        .ok_or_else(|| format!("scan_folder({folder_id}): folder not found"))?;

    scan::spawn_scan(app, &state, folder_id);
    Ok(())
}

#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> ScanProgress {
    state.scan_status.snapshot()
}

#[tauri::command]
pub fn start_watching(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    watcher::start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_watching(state: State<'_, AppState>) -> Result<(), String> {
    watcher::stop(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_timeline_groups(
    state: State<'_, AppState>,
    limit: Option<i64>,
    cursor_created_at: Option<String>,
    cursor_id: Option<i64>,
) -> Result<Vec<TimelineGroup>, String> {
    let limit = limit.unwrap_or(200).clamp(1, 500);
    let cursor = match (cursor_created_at, cursor_id) {
        (Some(ts), Some(id)) => Some((ts, id)),
        _ => None,
    };
    state
        .db
        .get_timeline_groups(limit, cursor)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_neighbors(state: State<'_, AppState>, id: i64) -> Result<MediaNeighbors, String> {
    state
        .db
        .get_media_neighbors(id)
        .map_err(|e| db_err("get_media_neighbors", &id.to_string(), e))
}

#[tauri::command]
pub async fn get_media_window(
    state: State<'_, AppState>,
    media_id: i64,
    radius: usize,
) -> Result<Vec<MediaFile>, String> {
    let radius = radius.min(50);
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.get_media_window(media_id, radius)
            .map_err(|e| db_err("get_media_window", &media_id.to_string(), e))
    })
    .await
    .map_err(|e| format!("get_media_window({media_id}): {e}"))?
}

#[tauri::command]
pub async fn run_dedup_scan(state: State<'_, AppState>) -> Result<DedupScanResult, String> {
    use std::sync::atomic::Ordering;

    if state
        .dedup_scanning
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("dedup scan already in progress".to_string());
    }

    struct DedupScanningGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DedupScanningGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = DedupScanningGuard(std::sync::Arc::clone(&state.dedup_scanning));

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.clear_duplicate_groups().map_err(|e| e.to_string())?;

        let exact_groups = db.find_exact_duplicates().map_err(|e| e.to_string())?;
        let perceptual_groups = db
            .find_perceptual_duplicates(10)
            .map_err(|e| e.to_string())?;

        let total_duplicates = exact_groups
            .iter()
            .chain(perceptual_groups.iter())
            .map(|g| g.members.len())
            .sum();

        Ok(DedupScanResult {
            exact_groups: exact_groups.len(),
            perceptual_groups: perceptual_groups.len(),
            total_duplicates,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_groups(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroupDetail>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.list_duplicate_groups().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_duplicate_groups_count().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_duplicate(
    state: State<'_, AppState>,
    group_id: i64,
    keep_media_id: i64,
    delete_files: bool,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.resolve_duplicate_group(group_id, keep_media_id, delete_files)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn dismiss_duplicate_group(
    state: State<'_, AppState>,
    group_id: i64,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.delete_duplicate_group(group_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_location_groups(state: State<'_, AppState>) -> Result<Vec<LocationGroup>, String> {
    state.db.get_location_groups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_location(
    state: State<'_, AppState>,
    country: String,
    city: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_location(&country, city.as_deref(), limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_location_stats(state: State<'_, AppState>) -> Result<LocationStats, String> {
    state.db.get_location_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_by_type(
    state: State<'_, AppState>,
    media_type: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_media_by_type(&media_type, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_count_by_type(
    state: State<'_, AppState>,
    media_type: String,
) -> Result<i64, String> {
    state
        .db
        .get_media_count_by_type(&media_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_screenshots(
    state: State<'_, AppState>,
    screenshot_type: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_screenshots(screenshot_type.as_deref(), limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_screenshot_count(
    state: State<'_, AppState>,
    screenshot_type: Option<String>,
) -> Result<i64, String> {
    state
        .db
        .get_screenshot_count(screenshot_type.as_deref())
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ModelStatus {
    pub models_dir: String,
    pub clip_available: bool,
    pub face_available: bool,
    pub models: Vec<lightframe_ai::ModelFileStatus>,
}

#[tauri::command]
pub async fn get_model_status() -> Result<ModelStatus, String> {
    Ok(ModelStatus {
        models_dir: lightframe_ai::models::models_dir()
            .to_string_lossy()
            .to_string(),
        clip_available: lightframe_ai::models::clip_model_available(),
        face_available: lightframe_ai::models::face_model_available(),
        models: lightframe_ai::all_model_statuses(),
    })
}

#[derive(Clone, serde::Serialize)]
struct ModelDownloadProgress {
    filename: String,
    downloaded: u64,
    total: u64,
}

#[tauri::command]
pub async fn download_model(app: AppHandle, filename: String) -> Result<String, String> {
    let model = lightframe_ai::model_by_filename(&filename)
        .ok_or_else(|| format!("unknown model: {filename}"))?;

    let emit_filename = filename.clone();
    let path = tokio::task::spawn_blocking(move || {
        lightframe_ai::download_model(model, move |downloaded, total| {
            let _ = app.emit(
                "model-download-progress",
                ModelDownloadProgress {
                    filename: emit_filename.clone(),
                    downloaded,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_models_dir() -> Result<(), String> {
    lightframe_ai::models::ensure_models_dir().map_err(|e| e.to_string())?;
    let path = lightframe_ai::models::models_dir();
    open_in_file_manager(&path)
}

fn open_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn create_album(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Album, String> {
    if name.trim().is_empty() {
        return Err("album name cannot be empty".to_string());
    }
    state
        .db
        .create_album(&name, description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_album(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_album(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_album(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("album name cannot be empty".to_string());
    }
    state
        .db
        .update_album(id, &name, description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_album_cover(
    state: State<'_, AppState>,
    album_id: i64,
    media_id: i64,
) -> Result<(), String> {
    state
        .db
        .set_album_cover(album_id, media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_albums(state: State<'_, AppState>) -> Result<Vec<Album>, String> {
    state.db.list_albums().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_ids: Vec<i64>,
) -> Result<(), String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .add_to_album(album_id, &media_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_id: i64,
) -> Result<(), String> {
    state
        .db
        .remove_from_album(album_id, media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_album_media(
    state: State<'_, AppState>,
    album_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_album_media(album_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_favorite(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state
        .db
        .toggle_favorite(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorites(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_favorites(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_favorites_count(state: State<'_, AppState>) -> Result<i64, String> {
    state.db.get_favorites_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_favorite(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state
        .db
        .is_favorite(media_id)
        .map_err(|e| db_err("is_favorite", &media_id.to_string(), e))
}

#[tauri::command]
pub fn delete_media(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .set_deleted(media_id, true)
        .map_err(|e| db_err("delete_media", &media_id.to_string(), e))
}

#[tauri::command]
pub fn get_deleted_media(state: State<'_, AppState>) -> Result<Vec<MediaFile>, String> {
    state.db.list_deleted_media().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_media(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .set_deleted(media_id, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn permanently_delete(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    let Some((path, hash, deleted)) = state
        .db
        .get_media_deletion_info(media_id)
        .map_err(|e| db_err("permanently_delete", &media_id.to_string(), e))?
    else {
        return Err(format!("permanently_delete({media_id}): media not found"));
    };
    if deleted == 0 {
        return Err(format!(
            "permanently_delete({media_id}): media is not in trash; soft-delete before permanent delete"
        ));
    }

    remove_media_from_disk(&path, hash.as_deref(), &state.db);
    state
        .db
        .permanently_delete_media(media_id)
        .map_err(|e| db_err("permanently_delete", &media_id.to_string(), e))?;
    state.thumb_cache.invalidate_media(media_id);
    Ok(())
}

#[tauri::command]
pub fn batch_delete_media(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .batch_set_deleted(&media_ids, true)
        .map_err(|e| db_err("batch_delete_media", &format!("{} ids", media_ids.len()), e))
}

#[tauri::command]
pub fn batch_add_to_album(
    state: State<'_, AppState>,
    album_id: i64,
    media_ids: Vec<i64>,
) -> Result<(), String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .add_to_album(album_id, &media_ids)
        .map_err(|e| db_err("batch_add_to_album", &format!("album {album_id}"), e))
}

#[tauri::command]
pub fn batch_toggle_favorite(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
    favorite: bool,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state
        .db
        .batch_set_favorite(&media_ids, favorite)
        .map_err(|e| {
            db_err(
                "batch_toggle_favorite",
                &format!("{} ids", media_ids.len()),
                e,
            )
        })
}

#[tauri::command]
pub fn batch_restore_media(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    state.db.batch_set_deleted(&media_ids, false).map_err(|e| {
        db_err(
            "batch_restore_media",
            &format!("{} ids", media_ids.len()),
            e,
        )
    })
}

#[tauri::command]
pub fn batch_permanent_delete(
    state: State<'_, AppState>,
    media_ids: Vec<i64>,
) -> Result<usize, String> {
    check_batch_size(&media_ids)?;
    let mut to_remove = Vec::new();
    for media_id in &media_ids {
        if let Some((path, hash, deleted)) = state
            .db
            .get_media_deletion_info(*media_id)
            .map_err(|e| e.to_string())?
            && deleted != 0
        {
            to_remove.push((path, hash));
        }
    }

    for (path, hash) in &to_remove {
        remove_media_from_disk(path, hash.as_deref(), &state.db);
    }

    let affected = state.db.batch_permanent_delete(&media_ids).map_err(|e| {
        db_err(
            "batch_permanent_delete",
            &format!("{} ids", media_ids.len()),
            e,
        )
    })?;

    for media_id in &media_ids {
        state.thumb_cache.invalidate_media(*media_id);
    }

    Ok(affected)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub fn search_media(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .search_media(&query, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_media_count(state: State<'_, AppState>, query: String) -> Result<i64, String> {
    state
        .db
        .search_media_count(&query)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct SearchResult {
    pub media_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub relevance: f32,
}

#[derive(Serialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SearchResult>,
    pub used_semantic: bool,
}

const SEMANTIC_SEARCH_THRESHOLD: f32 = 0.15;
const MAX_SEMANTIC_QUERY_LEN: usize = 512;

fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// CLIP vector search when text embedding is available; otherwise FTS5 keyword fallback.
#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    query_text: String,
    limit: Option<usize>,
) -> Result<SemanticSearchResponse, String> {
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let query_text = if query_text.len() > MAX_SEMANTIC_QUERY_LEN {
        truncate_utf8(&query_text, MAX_SEMANTIC_QUERY_LEN).to_string()
    } else {
        query_text
    };
    let trimmed = query_text.trim();
    if trimmed.is_empty() {
        return Ok(SemanticSearchResponse {
            results: Vec::new(),
            used_semantic: false,
        });
    }

    let text_embedding = {
        let mut ai = state.ai.lock().await;
        ai.ensure_python()
            .await
            .map_err(|e| format!("semantic_search: {e}"))?;
        ai.compute_text_embedding(trimmed)
            .await
            .map_err(|e| format!("semantic_search({trimmed}): {e}"))?
    };

    if let Some(embedding) = text_embedding {
        tracing::info!(query = trimmed, "semantic search using CLIP text embedding");
        let matches = state
            .db
            .semantic_search_by_embedding(&embedding, SEMANTIC_SEARCH_THRESHOLD, limit)
            .map_err(|e| db_err("semantic_search", trimmed, e))?;

        return Ok(SemanticSearchResponse {
            used_semantic: true,
            results: matches
                .into_iter()
                .map(|(m, score)| SearchResult {
                    media_id: m.id,
                    file_name: m.filename,
                    file_path: m.path,
                    relevance: score,
                })
                .collect(),
        });
    }

    tracing::info!(
        query = trimmed,
        "CLIP text embedding unavailable, falling back to FTS5"
    );
    let media = state
        .db
        .search_media(trimmed, limit as i64, 0)
        .map_err(|e| db_err("semantic_search", trimmed, e))?;

    Ok(SemanticSearchResponse {
        used_semantic: false,
        results: media
            .into_iter()
            .enumerate()
            .map(|(i, m)| SearchResult {
                media_id: m.id,
                file_name: m.filename,
                file_path: m.path,
                relevance: 1.0 - (i as f32 * 0.01).min(0.9),
            })
            .collect(),
    })
}

#[tauri::command]
pub fn create_smart_album(
    state: State<'_, AppState>,
    name: String,
    icon: Option<String>,
    rule: SmartAlbumRule,
) -> Result<SmartAlbum, String> {
    if name.trim().is_empty() {
        return Err("smart album name cannot be empty".to_string());
    }
    state
        .db
        .create_smart_album(&name, icon.as_deref(), &rule)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_smart_albums(state: State<'_, AppState>) -> Result<Vec<SmartAlbum>, String> {
    state.db.list_smart_albums().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_smart_album(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_smart_album(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_smart_album_media(
    state: State<'_, AppState>,
    id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_smart_album_media(id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_on_this_day(state: State<'_, AppState>, limit: i64) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    state
        .db
        .get_on_this_day_media(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.generate_memories().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    state.db.list_memories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memory_media(
    state: State<'_, AppState>,
    memory_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_memory_media(memory_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ai_status() -> AiStatus {
    lightframe_ai::check_ai_status()
}

#[derive(Serialize)]
pub struct SimilarPhoto {
    pub media_id: i64,
    pub similarity: f32,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct FaceInfo {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub person_id: Option<i64>,
}

fn face_record_to_info(r: lightframe_db::FaceDetectionRecord) -> FaceInfo {
    FaceInfo {
        id: r.id,
        media_id: r.media_id,
        bbox: [r.bbox_x, r.bbox_y, r.bbox_x + r.bbox_w, r.bbox_y + r.bbox_h],
        confidence: r.confidence,
        person_id: r.person_id,
    }
}

const SIMILAR_THRESHOLD: f32 = 0.65;

#[tauri::command]
pub async fn compute_clip_embedding(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<(), String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let mut ai = state.ai.lock().await;
    ai.ensure_python().await.map_err(|e| e.to_string())?;

    let embedding = ai
        .compute_embedding(path)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "CLIP embedding unavailable — place a CLIP ONNX model in the data/models directory or install the Python AI extension".to_string()
        })?;

    state
        .db
        .store_clip_embedding(media_id, &embedding)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BatchEmbedResult {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn compute_clip_embeddings_batch(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<BatchEmbedResult, String> {
    let limit = limit.unwrap_or(32).clamp(1, 256) as i64;
    let candidates = state
        .db
        .list_media_without_clip_embedding(limit)
        .map_err(|e| e.to_string())?;

    if candidates.is_empty() {
        return Ok(BatchEmbedResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }

    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    for (media_id, path_str) in &candidates {
        let path = std::path::Path::new(path_str);
        if !path.is_file() {
            failed += 1;
            errors.push(format!("media {media_id}: file not found"));
            continue;
        }

        let result = {
            let ai = state.ai.lock().await;
            ai.compute_embedding(path).await
        };

        match result {
            Ok(Some(embedding)) => {
                if let Err(e) = state.db.store_clip_embedding(*media_id, &embedding) {
                    failed += 1;
                    errors.push(format!("media {media_id}: {e}"));
                } else {
                    succeeded += 1;
                }
            }
            Ok(None) => {
                failed += 1;
                errors.push(format!("media {media_id}: CLIP embedding unavailable"));
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("media {media_id}: {e}"));
            }
        }
    }

    Ok(BatchEmbedResult {
        processed: candidates.len(),
        succeeded,
        failed,
        errors,
    })
}

#[tauri::command]
pub async fn find_similar_photos(
    state: State<'_, AppState>,
    media_id: i64,
    limit: Option<usize>,
) -> Result<Vec<SimilarPhoto>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 100);

    if state
        .db
        .get_clip_embedding(media_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        compute_clip_embedding(state.clone(), media_id).await?;
    }

    let similar = state
        .db
        .find_similar_media(media_id, SIMILAR_THRESHOLD, limit)
        .map_err(|e| e.to_string())?;

    let ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();
    let media_map = state.db.get_media_by_ids(&ids).map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(similar.len());
    for (id, score) in similar {
        let media = media_map
            .get(&id)
            .ok_or_else(|| format!("media {id} not found"))?;
        results.push(SimilarPhoto {
            media_id: id,
            similarity: score,
            file_name: media.filename.clone(),
            file_path: media.path.clone(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn detect_faces(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<Vec<FaceInfo>, String> {
    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    detect_and_store_faces_for_media(&state, media_id).await?;

    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[tauri::command]
pub fn get_faces(state: State<'_, AppState>, media_id: i64) -> Result<Vec<FaceInfo>, String> {
    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[derive(Serialize, Clone)]
pub struct FaceDetectionProgress {
    pub processed: i64,
    pub total: i64,
    pub faces_found: i64,
    pub status: String,
}

#[derive(Serialize)]
pub struct FaceDetectionBatchResult {
    pub media_processed: i64,
    pub faces_found: i64,
}

#[tauri::command]
pub async fn detect_faces_batch(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<FaceDetectionBatchResult, String> {
    use std::sync::atomic::Ordering;

    if state
        .face_detecting
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("face detection already in progress".to_string());
    }

    struct FaceDetectingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for FaceDetectingGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = FaceDetectingGuard(std::sync::Arc::clone(&state.face_detecting));

    let media_ids = state
        .db
        .get_media_ids_without_faces(500)
        .map_err(|e| e.to_string())?;

    let total = media_ids.len() as i64;
    let mut processed = 0i64;
    let mut faces_found = 0i64;

    let emit_progress = |processed: i64, faces_found: i64, status: &str| {
        let payload = FaceDetectionProgress {
            processed,
            total,
            faces_found,
            status: status.to_string(),
        };
        if let Err(e) = app.emit("face-detection-progress", &payload) {
            tracing::warn!("failed to emit face-detection-progress: {e}");
        }
    };

    emit_progress(0, 0, "detecting");

    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    for media_id in media_ids {
        match detect_and_store_faces_for_media(&state, media_id).await {
            Ok(count) => faces_found += count,
            Err(e) => tracing::warn!(media_id, "face detection failed: {e}"),
        }
        processed += 1;
        emit_progress(processed, faces_found, "detecting");
    }

    emit_progress(processed, faces_found, "complete");

    Ok(FaceDetectionBatchResult {
        media_processed: processed,
        faces_found,
    })
}

async fn detect_and_store_faces_for_media(state: &AppState, media_id: i64) -> Result<i64, String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    validate_media_path(&state.db, &media.path)?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let faces = {
        let ai = state.ai.lock().await;
        ai.detect_faces_in_image(path)
            .await
            .map_err(|e| e.to_string())?
    };

    let count = faces.len() as i64;
    let inputs: Vec<lightframe_db::FaceDetectionInput> = faces
        .iter()
        .map(|f| lightframe_db::FaceDetectionInput {
            bbox: f.bbox,
            confidence: f.confidence,
            embedding: f.embedding.clone(),
        })
        .collect();
    state
        .db
        .store_face_detections(media_id, &inputs)
        .map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub fn get_person_faces(
    state: State<'_, AppState>,
    person_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<FaceInfo>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    let records = state
        .db
        .get_faces_for_person(person_id, limit, offset)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[tauri::command]
pub fn list_persons(state: State<'_, AppState>) -> Result<Vec<Person>, String> {
    state.db.list_persons().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_person_media(
    state: State<'_, AppState>,
    person_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_person_media(person_id, limit, offset)
        .map_err(|e| e.to_string())
}

pub(crate) fn rename_person_impl(
    state: &AppState,
    person_id: i64,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("person name cannot be empty".to_string());
    }
    state
        .db
        .rename_person(person_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_person(
    state: State<'_, AppState>,
    person_id: i64,
    name: String,
) -> Result<(), String> {
    rename_person_impl(&state, person_id, name)
}

#[derive(Serialize)]
pub struct PersonClusterInfo {
    pub person_id: i64,
    pub name: Option<String>,
    pub face_count: i64,
    pub avg_intra_cluster_distance: f32,
}

const DEFAULT_FACE_CLUSTER_THRESHOLD: f32 = 0.45;

#[tauri::command]
pub async fn cluster_faces(
    state: State<'_, AppState>,
    threshold: Option<f32>,
) -> Result<Vec<PersonClusterInfo>, String> {
    let threshold = threshold.unwrap_or(DEFAULT_FACE_CLUSTER_THRESHOLD);
    if threshold.is_nan() || !(0.0..=1.0).contains(&threshold) {
        return Err("cluster threshold must be between 0.0 and 1.0".into());
    }
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.unassign_faces_from_unnamed_persons()
            .map_err(|e| e.to_string())?;
        let faces = db
            .get_unassigned_face_embeddings()
            .map_err(|e| e.to_string())?;
        if faces.is_empty() {
            return Ok(Vec::new());
        }

        let clusters = lightframe_ai::cluster_face_embeddings(&faces, threshold);
        let mut results = Vec::with_capacity(clusters.len());

        for cluster in clusters {
            let face_count = cluster.face_ids.len() as i64;
            let person_id = db.create_person(None).map_err(|e| e.to_string())?;
            for face_id in &cluster.face_ids {
                db.assign_face_to_person(*face_id, person_id)
                    .map_err(|e| e.to_string())?;
            }
            results.push(PersonClusterInfo {
                person_id,
                name: None,
                face_count,
                avg_intra_cluster_distance: cluster.avg_intra_cluster_distance,
            });
        }

        db.delete_empty_unnamed_persons()
            .map_err(|e| e.to_string())?;

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn merge_persons(
    state: State<'_, AppState>,
    person_id_a: i64,
    person_id_b: i64,
) -> Result<(), String> {
    if person_id_a == person_id_b {
        return Err("cannot merge a person with itself".to_string());
    }
    state
        .db
        .merge_persons(person_id_a, &[person_id_b])
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn split_face_from_person(
    state: State<'_, AppState>,
    face_id: i64,
    new_person_name: Option<String>,
) -> Result<i64, String> {
    state
        .db
        .split_face_from_person(face_id, new_person_name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_edit(state: State<'_, AppState>, media_id: i64, params: String) -> Result<(), String> {
    const MAX_EDIT_PARAMS_SIZE: usize = 64 * 1024; // 64KB
    if params.len() > MAX_EDIT_PARAMS_SIZE {
        return Err("edit parameters too large".to_string());
    }
    crate::image_edit::parse_edit_params(&params)?;
    state
        .db
        .save_edit_params(media_id, &params)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_edit(state: State<'_, AppState>, media_id: i64) -> Result<Option<String>, String> {
    state
        .db
        .get_edit_params(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn revert_edit(state: State<'_, AppState>, media_id: i64) -> Result<(), String> {
    state
        .db
        .clear_edit_params(media_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_edits(state: State<'_, AppState>, media_id: i64) -> Result<bool, String> {
    state.db.has_edits(media_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_edited(
    state: State<'_, AppState>,
    media_id: i64,
    output_path: String,
    quality: u8,
) -> Result<(), String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let params = state
        .db
        .get_edit_params(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no edit params saved for this media".to_string())?;

    validate_media_path(&state.db, &media.path)?;

    let quality = quality.clamp(1, 100);
    let out = std::path::Path::new(&output_path);
    if crate::original_protocol::path_contains_parent_dir(out) {
        return Err("output path contains path traversal".to_string());
    }
    if let Some(parent) = out.parent()
        && !parent.exists()
    {
        return Err("output directory does not exist".to_string());
    }
    crate::image_edit::export_edited_image(std::path::Path::new(&media.path), out, &params, quality)
}

#[tauri::command]
pub async fn regenerate_thumbnails(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::thumb_regen::ThumbnailRegenResult, String> {
    crate::thumb_regen::regenerate_all_thumbnails(app, &state).await
}

#[tauri::command]
pub fn regenerate_thumbnail_single(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<bool, String> {
    crate::thumb_regen::regenerate_thumbnails_for_media(&state, media_id)
}

#[tauri::command]
pub fn get_media_with_geo(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 5000);
    let offset = offset.max(0);
    state
        .db
        .get_media_with_geo(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_geo_clusters(
    state: State<'_, AppState>,
    grid_size: f64,
) -> Result<Vec<GeoCluster>, String> {
    state
        .db
        .get_geo_clusters(grid_size)
        .map_err(|e| e.to_string())
}

fn remove_media_from_disk(path: &str, hash: Option<&str>, db: &lightframe_db::Database) {
    let file_path = std::path::Path::new(path);
    if crate::original_protocol::path_contains_parent_dir(file_path) {
        tracing::error!("permanent delete: refusing path with traversal: {path}");
        return;
    }

    let watched_folders = match db.list_watched_folders() {
        Ok(folders) => folders,
        Err(e) => {
            tracing::error!("permanent delete: failed to list watched folders: {e}");
            return;
        }
    };

    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => crate::original_protocol::strip_extended_prefix(p),
        Err(e) => {
            tracing::warn!("permanent delete: cannot canonicalize {path} (file may be gone): {e}");
            return;
        }
    };

    if !crate::original_protocol::path_is_in_watched_folders(&canonical, &watched_folders) {
        tracing::error!(
            "permanent delete: refusing path outside watched folders: {}",
            canonical.display()
        );
        return;
    }

    match std::fs::remove_file(&canonical) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("permanent delete: file not found on disk: {path}");
        }
        Err(e) => tracing::warn!("permanent delete: failed to remove file {path}: {e}"),
    }

    if let Some(hash) = hash.filter(|h| h.len() >= 4) {
        for size in [
            ThumbnailSize::Micro,
            ThumbnailSize::Small,
            ThumbnailSize::Large,
        ] {
            let thumb = thumb_path(hash, size);
            if thumb.exists()
                && let Err(e) = std::fs::remove_file(&thumb)
            {
                tracing::warn!(
                    "permanent delete: failed to remove thumbnail {}: {e}",
                    thumb.display()
                );
            }
        }
    }
}
