use crate::models::{clip_model_path, face_detect_model_path, model_exists};
use crate::python_sidecar::PythonSidecar;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatus {
    pub python_available: bool,
    pub clip_available: bool,
    pub face_available: bool,
    pub status_message: String,
}

fn is_python_installed() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn check_ai_status() -> AiStatus {
    let clip_available = model_exists(&clip_model_path());
    let face_available = model_exists(&face_detect_model_path());
    let python_available = is_python_installed() && PythonSidecar::can_spawn();

    let status_message = if clip_available && face_available {
        "ONNX models available for CLIP and face detection".to_string()
    } else if clip_available {
        "CLIP ONNX model available; face model not found".to_string()
    } else if face_available {
        "Face detection ONNX model available; CLIP model not found".to_string()
    } else if !python_available {
        "Place ONNX models in the data directory or install the Python AI extension".to_string()
    } else {
        "Python AI sidecar available; ONNX models not found locally".to_string()
    };

    AiStatus {
        python_available,
        clip_available,
        face_available,
        status_message,
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
    fn check_ai_status_returns_struct() {
        let status = check_ai_status();
        assert!(!status.status_message.is_empty());
    }

    #[test]
    fn check_ai_status_when_models_missing() {
        let _guard = env_lock();
        unsafe {
            std::env::set_var("LIGHTFRAME_CLIP_MODEL", "/nonexistent/clip.onnx");
            std::env::set_var("LIGHTFRAME_FACE_DETECT_MODEL", "/nonexistent/face.onnx");
        }
        let status = check_ai_status();
        assert!(!status.clip_available);
        assert!(!status.face_available);
        assert!(status.status_message.contains("ONNX") || status.status_message.contains("models"));
        unsafe {
            std::env::remove_var("LIGHTFRAME_CLIP_MODEL");
            std::env::remove_var("LIGHTFRAME_FACE_DETECT_MODEL");
        }
    }
}
