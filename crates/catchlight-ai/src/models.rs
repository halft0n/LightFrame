use catchlight_core::config;
use std::path::{Path, PathBuf};

pub fn models_dir() -> PathBuf {
    config::data_dir().join("models")
}

pub fn clip_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_CLIP_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join("clip-vit-b32.onnx"))
}

pub fn face_detect_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_FACE_DETECT_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join("scrfd_500m.onnx"))
}

pub fn face_recog_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_FACE_RECOG_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join("w600k_r50.onnx"))
}

pub fn model_exists(path: &Path) -> bool {
    path.is_file()
}
