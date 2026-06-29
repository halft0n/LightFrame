use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub watched_folders: Vec<PathBuf>,
    pub locale: String,
    pub thumbnail_quality: u8,
    pub ai_enabled: bool,
    pub python_path: Option<PathBuf>,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub retention_days: u32,
    pub max_size_mb: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            retention_days: 7,
            max_size_mb: 100,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watched_folders: Vec::new(),
            locale: "zh-CN".to_string(),
            thumbnail_quality: 85,
            ai_enabled: false,
            python_path: None,
            log: LogConfig::default(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lightframe")
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
    fn default_config_has_log_defaults() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.log.level, "info");
        assert_eq!(cfg.log.retention_days, 7);
        assert_eq!(cfg.log.max_size_mb, 100);
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = AppConfig {
            watched_folders: vec![PathBuf::from("/photos"), PathBuf::from("/videos")],
            locale: "en".to_string(),
            thumbnail_quality: 90,
            ai_enabled: true,
            python_path: Some(PathBuf::from("/usr/bin/python3")),
            log: LogConfig {
                level: "debug".to_string(),
                retention_days: 14,
                max_size_mb: 200,
            },
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.watched_folders.len(), 2);
        assert_eq!(back.locale, "en");
        assert!(back.ai_enabled);
        assert_eq!(back.log.level, "debug");
        assert_eq!(back.log.retention_days, 14);
    }

    #[test]
    fn config_serde_backward_compat() {
        let json = r#"{"watched_folders":[],"locale":"zh-CN","thumbnail_quality":85,"ai_enabled":false,"python_path":null}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.log.level, "info");
        assert_eq!(cfg.log.retention_days, 7);
    }

    #[test]
    fn data_dir_ends_with_lightframe() {
        let dir = data_dir();
        assert!(dir.ends_with("lightframe"));
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
