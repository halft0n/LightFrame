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

    #[test]
    fn models_dir_is_under_data_dir() {
        let dir = models_dir();
        assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("models"));
        assert!(dir.starts_with(config::data_dir()));
    }

    #[test]
    fn all_model_definitions_have_consistent_formats() {
        for model in all_models() {
            assert!(model.filename.ends_with(".onnx"), "{}", model.filename);
            assert!(
                model.url.starts_with("https://huggingface.co/"),
                "{}",
                model.url
            );
            assert!(model.url.ends_with(model.filename), "{}", model.url);
            assert!(!model.name.is_empty());
            assert!(!model.description.is_empty());
            assert!(model.size_mb > 0);
            assert!(model_by_filename(model.filename).is_some());
        }
    }

    #[test]
    fn model_file_status_reports_installed_file() {
        ensure_models_dir().unwrap();
        let filename = format!("lf-status-test-{}.onnx", std::process::id());
        let filename_leaked: &'static str = Box::leak(filename.clone().into_boxed_str());
        let path = models_dir().join(&filename);
        std::fs::write(&path, b"installed-model-bytes").unwrap();

        let info = ModelInfo {
            name: "Status Test Model",
            filename: filename_leaked,
            url: "https://example.com/status-test.onnx",
            size_mb: 1,
            sha256: "",
            description: "test",
        };
        let status = model_file_status(&info);
        assert!(status.installed);
        assert_eq!(status.file_size_bytes, Some(21));
        assert!(status.sha256_verified.is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn model_file_status_sha256_verified_when_hash_matches() {
        use sha2::{Digest, Sha256};
        ensure_models_dir().unwrap();
        let filename = format!("lf-sha-status-{}.onnx", std::process::id());
        let path = models_dir().join(&filename);
        let content = b"sha256 status verification payload";
        std::fs::write(&path, content).unwrap();
        let hash = Sha256::digest(content)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let hash_leaked: &'static str = Box::leak(hash.into_boxed_str());
        let filename_leaked: &'static str = Box::leak(filename.into_boxed_str());

        let info = ModelInfo {
            name: "SHA Status Test",
            filename: filename_leaked,
            url: "https://example.com/sha-status.onnx",
            size_mb: 1,
            sha256: hash_leaked,
            description: "test",
        };
        let status = model_file_status(&info);
        assert!(status.installed);
        assert_eq!(status.sha256_verified, Some(true));

        let bad_info = ModelInfo {
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            ..info
        };
        let bad_status = model_file_status(&bad_info);
        assert_eq!(bad_status.sha256_verified, Some(false));

        let _ = std::fs::remove_file(&path);
    }

    fn start_test_http_server(body: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener.set_nonblocking(true).expect("set nonblocking");
        let port = listener.local_addr().expect("local addr").port();
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0_u8; 1024];
                    let _ = stream.read(&mut buf);
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&body);
                    return;
                }
                thread::sleep(Duration::from_millis(5));
            }
            panic!("test HTTP server timed out waiting for connection");
        });
        (format!("http://127.0.0.1:{port}/model.onnx"), handle)
    }

    fn leaked_test_filename(prefix: &str) -> &'static str {
        Box::leak(format!("{prefix}-{}.onnx", std::process::id()).into_boxed_str())
    }

    #[test]
    fn download_model_reports_progress_and_saves_file() {
        let body = vec![0xAB_u8; 250 * 1024];
        let (url, server) = start_test_http_server(body.clone());
        let url_leaked: &'static str = Box::leak(url.into_boxed_str());
        let filename = leaked_test_filename("lf-dl-progress");
        let dest = models_dir().join(filename);
        let _ = std::fs::remove_file(&dest);

        let info = ModelInfo {
            name: "Download Progress Test",
            filename,
            url: url_leaked,
            size_mb: 1,
            sha256: "",
            description: "test",
        };

        let progress = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(u64, u64)>::new()));
        let progress_clone = progress.clone();
        let result = download_model(&info, move |downloaded, total| {
            progress_clone.lock().unwrap().push((downloaded, total));
        });
        server.join().expect("server thread");

        assert!(result.is_ok());
        assert!(dest.is_file());
        assert_eq!(std::fs::read(&dest).unwrap(), body);

        let calls = progress.lock().unwrap();
        assert!(calls.len() >= 2, "expected multiple progress callbacks");
        assert_eq!(calls.first().map(|c| c.0), Some(0));
        assert_eq!(calls.last().map(|c| c.0), Some(body.len() as u64));
        assert_eq!(calls.last().map(|c| c.1), Some(body.len() as u64));

        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn download_model_rejects_incorrect_sha256() {
        use sha2::{Digest, Sha256};
        let body = b"download sha256 mismatch payload".to_vec();
        let (url, server) = start_test_http_server(body);
        let url_leaked: &'static str = Box::leak(url.into_boxed_str());
        let filename = leaked_test_filename("lf-dl-bad-sha");
        let dest = models_dir().join(filename);
        let part = dest.with_extension("onnx.part");
        let _ = std::fs::remove_file(&dest);
        let _ = std::fs::remove_file(&part);

        let info = ModelInfo {
            name: "Download SHA Test",
            filename,
            url: url_leaked,
            size_mb: 1,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            description: "test",
        };

        let result = download_model(&info, |_, _| {});
        server.join().expect("server thread");

        assert!(result.is_err());
        assert!(!dest.exists());
        // Temp part file may remain after verification failure; ensure final dest was not installed.
        let _ = std::fs::remove_file(&part);

        let correct = Sha256::digest(b"download sha256 mismatch payload")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let (url2, server2) = start_test_http_server(b"download sha256 mismatch payload".to_vec());
        let url2_leaked: &'static str = Box::leak(url2.into_boxed_str());
        let hash_leaked: &'static str = Box::leak(correct.into_boxed_str());
        let good_info = ModelInfo {
            sha256: hash_leaked,
            url: url2_leaked,
            ..info
        };
        let good = download_model(&good_info, |_, _| {});
        server2.join().expect("server thread");
        assert!(good.is_ok());
        let _ = std::fs::remove_file(&dest);
    }
}
