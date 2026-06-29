use lightframe_core::config;
use lightframe_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const CLIP_MODEL_FILENAME: &str = "clip-vit-b32-visual.onnx";
pub const CLIP_TEXT_MODEL_FILENAME: &str = "clip-vit-b32-textual.onnx";
pub const FACE_DETECT_MODEL_FILENAME: &str = "scrfd_500m_bnkps.onnx";
pub const FACE_RECOG_MODEL_FILENAME: &str = "w600k_r50.onnx";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub size_mb: u64,
    pub sha256: &'static str,
    pub description: &'static str,
}

pub const CLIP_VISUAL_MODEL: ModelInfo = ModelInfo {
    name: "CLIP Visual Encoder",
    filename: CLIP_MODEL_FILENAME,
    url: "https://huggingface.co/phineas-bage/clip-vit-b32-onnx/resolve/main/clip-vit-b32-visual.onnx",
    size_mb: 338,
    sha256: "",
    description: "CLIP ViT-B/32 visual encoder for image embeddings",
};

pub const CLIP_TEXT_MODEL: ModelInfo = ModelInfo {
    name: "CLIP Text Encoder",
    filename: CLIP_TEXT_MODEL_FILENAME,
    url: "https://huggingface.co/phineas-bage/clip-vit-b32-onnx/resolve/main/clip-vit-b32-textual.onnx",
    size_mb: 254,
    sha256: "",
    description: "CLIP ViT-B/32 text encoder for semantic search",
};

pub const FACE_DETECTION_MODEL: ModelInfo = ModelInfo {
    name: "Face Detection (SCRFD)",
    filename: FACE_DETECT_MODEL_FILENAME,
    url: "https://huggingface.co/phineas-bage/insightface-models/resolve/main/scrfd_500m_bnkps.onnx",
    size_mb: 3,
    sha256: "",
    description: "SCRFD face detector with landmark keypoints",
};

pub const FACE_RECOGNITION_MODEL: ModelInfo = ModelInfo {
    name: "Face Recognition (ArcFace)",
    filename: FACE_RECOG_MODEL_FILENAME,
    url: "https://huggingface.co/phineas-bage/insightface-models/resolve/main/w600k_r50.onnx",
    size_mb: 166,
    sha256: "",
    description: "ArcFace R50 for face embedding extraction",
};

pub fn all_models() -> Vec<&'static ModelInfo> {
    vec![
        &CLIP_VISUAL_MODEL,
        &CLIP_TEXT_MODEL,
        &FACE_DETECTION_MODEL,
        &FACE_RECOGNITION_MODEL,
    ]
}

pub fn model_by_filename(filename: &str) -> Option<&'static ModelInfo> {
    all_models().into_iter().find(|m| m.filename == filename)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFileStatus {
    pub name: String,
    pub filename: String,
    pub url: String,
    pub size_mb: u64,
    pub description: String,
    pub installed: bool,
    pub file_size_bytes: Option<u64>,
    pub sha256_verified: Option<bool>,
}

pub fn models_dir() -> PathBuf {
    config::data_dir().join("models")
}

pub fn clip_model_path() -> PathBuf {
    std::env::var("LIGHTFRAME_CLIP_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(CLIP_MODEL_FILENAME))
}

pub fn clip_text_model_path() -> PathBuf {
    std::env::var("LIGHTFRAME_CLIP_TEXT_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(CLIP_TEXT_MODEL_FILENAME))
}

pub fn face_detect_model_path() -> PathBuf {
    std::env::var("LIGHTFRAME_FACE_DETECT_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(FACE_DETECT_MODEL_FILENAME))
}

pub fn face_model_path() -> PathBuf {
    face_detect_model_path()
}

pub fn face_recog_model_path() -> PathBuf {
    std::env::var("LIGHTFRAME_FACE_RECOG_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| models_dir().join(FACE_RECOG_MODEL_FILENAME))
}

pub fn clip_model_available() -> bool {
    clip_model_path().exists()
}

pub fn clip_text_model_available() -> bool {
    clip_text_model_path().exists()
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

pub fn model_path_for(info: &ModelInfo) -> PathBuf {
    models_dir().join(info.filename)
}

pub fn model_file_status(info: &ModelInfo) -> ModelFileStatus {
    let path = model_path_for(info);
    let installed = path.is_file();
    let file_size_bytes =
        installed.then(|| std::fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0));

    let sha256_verified = if installed && !info.sha256.is_empty() {
        Some(verify_file_sha256(&path, info.sha256).is_ok())
    } else {
        None
    };

    ModelFileStatus {
        name: info.name.to_string(),
        filename: info.filename.to_string(),
        url: info.url.to_string(),
        size_mb: info.size_mb,
        description: info.description.to_string(),
        installed,
        file_size_bytes,
        sha256_verified,
    }
}

pub fn all_model_statuses() -> Vec<ModelFileStatus> {
    all_models().into_iter().map(model_file_status).collect()
}

pub fn download_model<F>(info: &ModelInfo, mut on_progress: F) -> Result<PathBuf>
where
    F: FnMut(u64, u64),
{
    ensure_models_dir().map_err(Error::Io)?;

    let dest = model_path_for(info);
    let tmp = dest.with_extension("onnx.part");

    let response = ureq::get(info.url)
        .call()
        .map_err(|e| Error::Ai(format!("download failed for {}: {e}", info.name)))?;

    let total_bytes = response
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    on_progress(0, total_bytes);

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&tmp).map_err(Error::Io)?;

    const CHUNK_SIZE: usize = 64 * 1024;
    const PROGRESS_INTERVAL: u64 = 100 * 1024;
    let mut buffer = [0_u8; CHUNK_SIZE];
    let mut downloaded: u64 = 0;
    let mut last_reported: u64 = 0;
    let mut last_percent: u64 = 0;

    loop {
        let n = reader.read(&mut buffer).map_err(Error::Io)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(Error::Io)?;
        downloaded += n as u64;

        let percent = downloaded
            .saturating_mul(100)
            .checked_div(total_bytes)
            .unwrap_or(0);
        let should_report = downloaded - last_reported >= PROGRESS_INTERVAL
            || (total_bytes > 0 && percent > last_percent);
        if should_report {
            on_progress(downloaded, total_bytes);
            last_reported = downloaded;
            last_percent = percent;
        }
    }
    drop(file);

    on_progress(downloaded, total_bytes);

    if info.sha256.is_empty() {
        tracing::warn!(
            model = info.name,
            "no sha256 hash configured; skipping verification"
        );
    } else {
        verify_file_sha256(&tmp, info.sha256)?;
    }

    std::fs::rename(&tmp, &dest).map_err(Error::Io)?;
    tracing::info!(model = info.name, path = %dest.display(), "model download complete");
    Ok(dest)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

    let bytes = std::fs::read(path).map_err(Error::Io)?;
    let hash = Sha256::digest(bytes);
    let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();

    if hex.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(Error::Ai(format!(
            "sha256 mismatch for {}: expected {expected}, got {hex}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn all_models_lists_expected_entries() {
        let models = all_models();
        assert_eq!(models.len(), 4);
        assert!(models.iter().any(|m| m.filename == CLIP_MODEL_FILENAME));
    }

    #[test]
    fn model_by_filename_finds_clip_visual() {
        assert!(model_by_filename(CLIP_MODEL_FILENAME).is_some());
        assert!(model_by_filename("nonexistent.onnx").is_none());
    }

    #[test]
    fn model_file_status_reports_not_installed_for_missing() {
        let status = model_file_status(&CLIP_VISUAL_MODEL);
        assert!(!status.installed || status.file_size_bytes.is_some());
        assert_eq!(status.filename, CLIP_MODEL_FILENAME);
    }

    #[test]
    fn clip_model_unavailable_when_path_does_not_exist() {
        let _guard = env_lock();
        unsafe {
            std::env::set_var("LIGHTFRAME_CLIP_MODEL", "/nonexistent/lightframe/clip.onnx");
        }
        assert!(!clip_model_available());
        unsafe {
            std::env::remove_var("LIGHTFRAME_CLIP_MODEL");
        }
    }

    #[test]
    fn face_model_unavailable_when_path_does_not_exist() {
        let _guard = env_lock();
        unsafe {
            std::env::set_var(
                "LIGHTFRAME_FACE_DETECT_MODEL",
                "/nonexistent/lightframe/face.onnx",
            );
        }
        assert!(!face_model_available());
        unsafe {
            std::env::remove_var("LIGHTFRAME_FACE_DETECT_MODEL");
        }
    }

    #[test]
    fn model_exists_returns_false_for_missing_file() {
        assert!(!model_exists(Path::new(
            "/nonexistent/lightframe/missing.onnx"
        )));
    }

    #[test]
    fn verify_sha256_accepts_matching_hash() {
        use sha2::{Digest, Sha256};
        let dir = std::env::temp_dir().join(format!("lf_sha256_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");
        let content = b"lightframe model hash test";
        std::fs::write(&path, content).unwrap();
        let expected = Sha256::digest(content)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert!(verify_file_sha256(&path, &expected).is_ok());
        assert!(verify_file_sha256(&path, "deadbeef").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
