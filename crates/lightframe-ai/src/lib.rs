pub mod dispatcher;
pub mod models;
pub mod python_sidecar;
pub mod screenshot;
pub mod similar;
pub mod status;
pub mod types;

#[cfg(feature = "clip")]
pub mod clip;

#[cfg(feature = "face")]
pub mod face;

pub use dispatcher::AiDispatcher;
pub use models::{
    ModelFileStatus, ModelInfo, all_model_statuses, all_models, download_model,
    download_model_cancellable, model_by_filename,
};
pub use python_sidecar::PythonSidecar;
pub use screenshot::{ScreenshotScore, ScreenshotType, classify_screenshot, detect_screenshot};
pub use similar::{cluster_face_embeddings, cosine_similarity, find_similar};
pub use status::{AiStatus, check_ai_status};
pub use types::FaceDetection;

#[cfg(feature = "clip")]
pub use clip::{ClipEncoder, ClipTextEncoder};

#[cfg(feature = "face")]
pub use face::{FaceDetector, PersonCluster};
