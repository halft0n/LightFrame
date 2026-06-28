use catchlight_core::config;
use std::path::{Path, PathBuf};

pub const CLIP_MODEL_FILENAME: &str = "clip-vit-b32-visual.onnx";
pub const FACE_DETECT_MODEL_FILENAME: &str = "scrfd_500m_bnkps.onnx";
pub const FACE_RECOG_MODEL_FILENAME: &str = "w600k_r50.onnx";

pub fn models_dir() -> PathBuf {
    config::data_dir().join("models")
}

pub fn clip_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_CLIP_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(CLIP_MODEL_FILENAME))
}

pub fn face_detect_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_FACE_DETECT_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(FACE_DETECT_MODEL_FILENAME))
}

pub fn face_model_path() -> PathBuf {
    face_detect_model_path()
}

pub fn face_recog_model_path() -> PathBuf {
    std::env::var("CATCHLIGHT_FACE_RECOG_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(FACE_RECOG_MODEL_FILENAME))
}

pub fn clip_model_available() -> bool {
    clip_model_path().exists()
}

pub fn face_model_available() -> bool {
    face_model_path().exists()
}

pub fn ensure_models_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(models_dir())
}

pub fn model_exists(path: &Path) -> bool {
    path.is_file()
}
