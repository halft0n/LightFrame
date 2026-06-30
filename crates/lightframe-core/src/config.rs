use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
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
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("lightframe")
}

pub fn load_config_from(path: &std::path::Path) -> AppConfig {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

pub fn save_config_to(path: &std::path::Path, cfg: &AppConfig) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json =
        serde_json::to_string_pretty(cfg).map_err(|e| crate::Error::Config(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn save_config(cfg: &AppConfig) -> crate::Result<()> {
    save_config_to(&config_path(), cfg)
}

pub fn update_log_config(log: &LogConfig) -> crate::Result<()> {
    let mut cfg = load_config();
    cfg.log = log.clone();
    save_config(&cfg)
}

pub fn thumbnail_quality() -> u8 {
    load_config().thumbnail_quality.clamp(1, 100)
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
        assert_eq!(cfg.locale, "zh-CN");
    }

    #[test]
    fn save_and_load_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = AppConfig {
            locale: "en-US".to_string(),
            thumbnail_quality: 92,
            ai_enabled: true,
            python_path: None,
            log: LogConfig {
                level: "debug".to_string(),
                retention_days: 21,
                max_size_mb: 250,
            },
        };
        save_config_to(&path, &cfg).unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.locale, "en-US");
        assert_eq!(loaded.thumbnail_quality, 92);
        assert_eq!(loaded.log.level, "debug");
    }

    #[test]
    fn update_log_config_persists_log_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save_config_to(&path, &AppConfig::default()).unwrap();
        let new_log = LogConfig {
            level: "warn".to_string(),
            retention_days: 14,
            max_size_mb: 50,
        };
        let mut cfg = load_config_from(&path);
        cfg.log = new_log.clone();
        save_config_to(&path, &cfg).unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.log.level, "warn");
        assert_eq!(loaded.log.retention_days, 14);
    }

    #[test]
    fn data_dir_does_not_fallback_to_dot() {
        let dir = data_dir();
        assert!(!dir.starts_with(PathBuf::from(".")));
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
