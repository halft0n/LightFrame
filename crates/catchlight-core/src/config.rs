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
