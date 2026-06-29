use crate::python_sidecar::PythonSidecar;
use crate::types::FaceDetection;
use catchlight_core::Result;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

#[cfg(any(feature = "clip", feature = "face"))]
use std::sync::Mutex;

#[cfg(feature = "clip")]
use crate::models::{clip_model_path, model_exists};

#[cfg(feature = "face")]
use crate::models::{face_detect_model_path, face_recog_model_path, model_exists as _};

#[cfg(feature = "clip")]
use crate::clip::ClipEncoder;

#[cfg(feature = "face")]
use crate::face::FaceDetector;

/// AI operations are serialized through a single Mutex to prevent
/// concurrent ONNX Runtime sessions from exhausting GPU/CPU memory.
/// This is intentional for the current scope but should be replaced
/// with a bounded worker pool for production use.
pub struct AiDispatcher {
    #[cfg(feature = "clip")]
    clip: Option<Mutex<ClipEncoder>>,
    #[cfg(feature = "face")]
    face: Option<Mutex<FaceDetector>>,
    python: Option<Arc<AsyncMutex<PythonSidecar>>>,
}

impl AiDispatcher {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "clip")]
            clip: Self::try_init_clip(),
            #[cfg(feature = "face")]
            face: Self::try_init_face(),
            python: None,
        }
    }

    #[cfg(feature = "clip")]
    fn try_init_clip() -> Option<Mutex<ClipEncoder>> {
        let path = clip_model_path();
        if !model_exists(&path) {
            tracing::debug!("CLIP model not found at {}", path.display());
            return None;
        }
        match ClipEncoder::new(&path) {
            Ok(enc) => {
                info!("CLIP encoder loaded from {}", path.display());
                Some(Mutex::new(enc))
            }
            Err(e) => {
                tracing::warn!("failed to load CLIP encoder: {e}");
                None
            }
        }
    }

    #[cfg(feature = "face")]
    fn try_init_face() -> Option<Mutex<FaceDetector>> {
        let detect = face_detect_model_path();
        let recog = face_recog_model_path();
        if !model_exists(&detect) && !model_exists(&recog) {
            tracing::debug!("face models not found");
            return None;
        }
        match FaceDetector::new(Some(&detect), Some(&recog)) {
            Ok(det) if det.is_available() => {
                info!("face detector loaded");
                Some(Mutex::new(det))
            }
            Ok(_) => None,
            Err(e) => {
                tracing::warn!("failed to load face detector: {e}");
                None
            }
        }
    }

    pub fn is_clip_available(&self) -> bool {
        #[cfg(feature = "clip")]
        {
            self.clip.is_some()
        }
        #[cfg(not(feature = "clip"))]
        {
            false
        }
    }

    pub fn is_face_available(&self) -> bool {
        #[cfg(feature = "face")]
        {
            self.face.is_some()
        }
        #[cfg(not(feature = "face"))]
        {
            false
        }
    }

    pub fn is_python_available(&self) -> bool {
        self.python.is_some()
    }

    pub async fn ensure_python(&mut self) -> Result<()> {
        if self.python.is_none() {
            match PythonSidecar::spawn().await {
                Ok(sidecar) => {
                    info!("Python AI sidecar started");
                    self.python = Some(Arc::new(AsyncMutex::new(sidecar)));
                }
                Err(e) => {
                    tracing::warn!("Python sidecar unavailable, AI features degraded: {e}");
                }
            }
        }
        Ok(())
    }

    pub fn python_sidecar(&self) -> Option<Arc<AsyncMutex<PythonSidecar>>> {
        self.python.clone()
    }

    pub async fn compute_embedding(&self, path: &Path) -> Result<Option<Vec<f32>>> {
        #[cfg(feature = "clip")]
        if let Some(clip) = &self.clip {
            let mut guard = clip
                .lock()
                .map_err(|_| catchlight_core::Error::Ai("CLIP encoder lock poisoned".into()))?;
            match guard.encode_image(path) {
                Ok(embedding) => return Ok(Some(embedding)),
                Err(e) => tracing::warn!("Rust CLIP encoding failed: {e}"),
            }
        }

        if let Some(python) = &self.python {
            let sidecar = python.lock().await;
            let result = sidecar
                .call(
                    "compute_clip_embedding",
                    json!({ "image_path": path.to_string_lossy() }),
                )
                .await?;

            if let Some(embedding) = parse_embedding(&result) {
                return Ok(Some(embedding));
            }
        }

        Ok(None)
    }

    pub async fn detect_faces_in_image(&self, path: &Path) -> Result<Vec<FaceDetection>> {
        #[cfg(feature = "face")]
        if let Some(face) = &self.face {
            let mut guard = face
                .lock()
                .map_err(|_| catchlight_core::Error::Ai("face detector lock poisoned".into()))?;
            match guard.detect_faces(path) {
                Ok(faces) if !faces.is_empty() => return Ok(faces),
                Ok(_) => {}
                Err(e) => tracing::warn!("Rust face detection failed: {e}"),
            }
        }

        if let Some(python) = &self.python {
            let sidecar = python.lock().await;
            let result = sidecar
                .call(
                    "detect_faces",
                    json!({ "image_path": path.to_string_lossy() }),
                )
                .await?;
            return Ok(parse_face_detections(&result));
        }

        Ok(Vec::new())
    }
}

impl Default for AiDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_embedding(value: &Value) -> Option<Vec<f32>> {
    match value {
        Value::Null => None,
        Value::Array(arr) => {
            let embedding: Option<Vec<f32>> =
                arr.iter().map(|v| v.as_f64().map(|f| f as f32)).collect();
            embedding.filter(|v| !v.is_empty())
        }
        Value::Object(obj) => obj.get("embedding").and_then(parse_embedding),
        _ => None,
    }
}

fn parse_face_detections(value: &Value) -> Vec<FaceDetection> {
    let Some(faces) = value.get("faces").and_then(Value::as_array) else {
        return Vec::new();
    };

    faces
        .iter()
        .filter_map(|face| {
            let bbox = face.get("bbox")?.as_array()?;
            if bbox.len() != 4 {
                return None;
            }
            let confidence = face.get("confidence")?.as_f64()? as f32;
            let embedding = face
                .get("embedding")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect()
                })
                .unwrap_or_default();

            Some(FaceDetection {
                bbox: [
                    bbox[0].as_f64()? as f32,
                    bbox[1].as_f64()? as f32,
                    bbox[2].as_f64()? as f32,
                    bbox[3].as_f64()? as f32,
                ],
                confidence,
                embedding,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_dispatcher_has_no_python() {
        let d = AiDispatcher::new();
        assert!(!d.is_python_available());
    }

    #[test]
    fn default_dispatcher_has_no_python() {
        let d = AiDispatcher::default();
        assert!(!d.is_python_available());
    }

    #[tokio::test]
    async fn ensure_python_degrades_gracefully() {
        let mut d = AiDispatcher::new();
        let result = d.ensure_python().await;
        assert!(result.is_ok());
    }

    #[test]
    fn parse_embedding_from_array() {
        let value = json!([0.1, 0.2, 0.3]);
        let emb = parse_embedding(&value).unwrap();
        assert_eq!(emb.len(), 3);
    }

    #[test]
    fn parse_embedding_null_returns_none() {
        assert!(parse_embedding(&Value::Null).is_none());
    }
}
