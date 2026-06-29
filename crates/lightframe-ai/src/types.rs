use serde::{Deserialize, Serialize};

/// A detected face with optional embedding vector (from Rust ONNX or Python sidecar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub embedding: Vec<f32>,
}
