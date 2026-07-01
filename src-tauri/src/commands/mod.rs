mod ai;
mod albums;
mod dedup;
mod media;
mod models;
mod scan;
mod settings;

pub use ai::*;
pub use albums::*;
pub use dedup::*;
pub use media::*;
pub use models::*;
pub use scan::*;
pub use settings::*;

use lightframe_core::media::ThumbnailSize;
use lightframe_thumbnail::thumb_path;

pub(crate) const MAX_BATCH_SIZE: usize = 900;

pub(crate) fn check_batch_size(media_ids: &[i64]) -> Result<(), String> {
    if media_ids.len() > MAX_BATCH_SIZE {
        return Err(format!(
            "batch size {} exceeds maximum {}",
            media_ids.len(),
            MAX_BATCH_SIZE
        ));
    }
    Ok(())
}

pub(crate) fn validate_media_path(db: &lightframe_db::Database, path: &str) -> Result<(), String> {
    let file_path = std::path::Path::new(path);
    if crate::original_protocol::path_contains_parent_dir(file_path) {
        return Err("invalid path".to_string());
    }
    let folders = db.list_watched_folders().map_err(|e| e.to_string())?;
    let check_path = match file_path.canonicalize() {
        Ok(raw) => crate::original_protocol::strip_extended_prefix(raw),
        Err(_) => {
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

pub(crate) fn db_err(cmd: &str, context: &str, e: impl std::fmt::Display) -> String {
    format!("{cmd}({context}): {e}")
}

pub(crate) fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub(crate) fn remove_media_from_disk(path: &str, hash: Option<&str>, db: &lightframe_db::Database) {
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
