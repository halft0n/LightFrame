use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub watched_folders: Vec<PathBuf>,
    pub locale: String,
    pub thumbnail_quality: u8,
    pub ai_enabled: bool,
    pub python_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watched_folders: Vec::new(),
            locale: "zh-CN".to_string(),
            thumbnail_quality: 85,
            ai_enabled: false,
            python_path: None,
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("catchlight")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn db_path() -> PathBuf {
    data_dir().join("library.db")
}

pub fn thumb_cache_dir() -> PathBuf {
    data_dir().join("cache").join("thumbs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = AppConfig::default();
        assert!(cfg.watched_folders.is_empty());
        assert_eq!(cfg.locale, "zh-CN");
        assert_eq!(cfg.thumbnail_quality, 85);
        assert!(!cfg.ai_enabled);
        assert!(cfg.python_path.is_none());
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = AppConfig {
            watched_folders: vec![PathBuf::from("/photos"), PathBuf::from("/videos")],
            locale: "en".to_string(),
            thumbnail_quality: 90,
            ai_enabled: true,
            python_path: Some(PathBuf::from("/usr/bin/python3")),
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.watched_folders.len(), 2);
        assert_eq!(back.locale, "en");
        assert!(back.ai_enabled);
    }

    #[test]
    fn data_dir_ends_with_catchlight() {
        let dir = data_dir();
        assert!(dir.ends_with("catchlight"));
    }

    #[test]
    fn config_path_is_json() {
        let path = config_path();
        assert_eq!(path.extension().unwrap(), "json");
    }

    #[test]
    fn db_path_is_under_data_dir() {
        let db = db_path();
        let data = data_dir();
        assert!(db.starts_with(&data));
        assert_eq!(db.file_name().unwrap(), "library.db");
    }

    #[test]
    fn thumb_cache_dir_structure() {
        let dir = thumb_cache_dir();
        assert!(dir.ends_with("thumbs"));
        assert!(dir.to_string_lossy().contains("cache"));
    }
}
