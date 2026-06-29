use crate::models::{face_detect_model_path, face_recog_model_path, model_exists};
use crate::similar::cosine_similarity;
use crate::types::FaceDetection;
use lightframe_core::Result;
use ort::session::Session;
use serde::{Deserialize, Serialize};
use std::path::Path;

const CLUSTER_THRESHOLD: f32 = 0.45;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonCluster {
    pub cluster_id: usize,
    pub faces: Vec<(i64, usize)>,
    pub centroid: Vec<f32>,
}

pub struct FaceDetector {
    detect_session: Option<Session>,
    #[allow(dead_code)]
    recognize_session: Option<Session>,
}

impl FaceDetector {
    pub fn new(detect_model: Option<&Path>, recog_model: Option<&Path>) -> Result<Self> {
        let detect_session = detect_model
            .filter(|p| model_exists(p))
            .map(|p| load_session(p, "face detection"))
            .transpose()?;

        let recognize_session = recog_model
            .filter(|p| model_exists(p))
            .map(|p| load_session(p, "face recognition"))
            .transpose()?;

        Ok(Self {
            detect_session,
            recognize_session,
        })
    }

    pub fn try_default() -> Result<Self> {
        Self::new(
            Some(&face_detect_model_path()),
            Some(&face_recog_model_path()),
        )
    }

    pub fn is_available(&self) -> bool {
        self.detect_session.is_some()
    }

    pub fn detect(&mut self, image_path: &Path) -> Result<Vec<FaceDetection>> {
        self.detect_faces(image_path)
    }

    pub fn detect_faces(&mut self, _image_path: &Path) -> Result<Vec<FaceDetection>> {
        if self.detect_session.is_none() {
            return Err(lightframe_core::Error::Ai(
                "face detection model not loaded".to_string(),
            ));
        }

        Err(lightframe_core::Error::Ai(
            "Rust ONNX face inference not yet implemented; use Python sidecar".to_string(),
        ))
    }

    pub fn cluster_faces(detections: &[(i64, Vec<FaceDetection>)]) -> Result<Vec<PersonCluster>> {
        let mut clusters: Vec<PersonCluster> = Vec::new();

        for (media_id, faces) in detections {
            for (face_idx, face) in faces.iter().enumerate() {
                if face.embedding.is_empty() {
                    continue;
                }

                let mut best_cluster: Option<usize> = None;
                let mut best_score = CLUSTER_THRESHOLD;

                for (idx, cluster) in clusters.iter().enumerate() {
                    let score = cosine_similarity(&face.embedding, &cluster.centroid);
                    if score > best_score {
                        best_score = score;
                        best_cluster = Some(idx);
                    }
                }

                if let Some(idx) = best_cluster {
                    let cluster = &mut clusters[idx];
                    let n = cluster.faces.len() as f32;
                    for (i, val) in face.embedding.iter().enumerate() {
                        if i < cluster.centroid.len() {
                            cluster.centroid[i] = (cluster.centroid[i] * n + val) / (n + 1.0);
                        }
                    }
                    cluster.faces.push((*media_id, face_idx));
                } else {
                    clusters.push(PersonCluster {
                        cluster_id: clusters.len(),
                        faces: vec![(*media_id, face_idx)],
                        centroid: face.embedding.clone(),
                    });
                }
            }
        }

        Ok(clusters)
    }
}

fn load_session(path: &Path, label: &str) -> Result<Session> {
    Session::builder()
        .map_err(|e| lightframe_core::Error::Ai(format!("{label} session builder: {e}")))?
        .commit_from_file(path)
        .map_err(|e| {
            lightframe_core::Error::Ai(format!(
                "failed to load {label} model at {}: {e}",
                path.display()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_faces_returns_empty_without_model() {
        let mut detector = FaceDetector::new(None, None).unwrap();
        let faces = detector
            .detect_faces(Path::new("/tmp/nonexistent.jpg"))
            .unwrap();
        assert!(faces.is_empty());
    }

    #[test]
    fn cluster_faces_groups_similar_embeddings() {
        let detections = vec![
            (
                1,
                vec![FaceDetection {
                    bbox: [0.0, 0.0, 10.0, 10.0],
                    confidence: 0.99,
                    embedding: vec![1.0, 0.0],
                }],
            ),
            (
                2,
                vec![FaceDetection {
                    bbox: [0.0, 0.0, 10.0, 10.0],
                    confidence: 0.98,
                    embedding: vec![0.99, 0.01],
                }],
            ),
            (
                3,
                vec![FaceDetection {
                    bbox: [0.0, 0.0, 10.0, 10.0],
                    confidence: 0.95,
                    embedding: vec![0.0, 1.0],
                }],
            ),
        ];

        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 2);
    }

    fn face_detection(embedding: Vec<f32>) -> FaceDetection {
        FaceDetection {
            bbox: [0.0, 0.0, 10.0, 10.0],
            confidence: 0.99,
            embedding,
        }
    }

    #[test]
    fn cluster_faces_empty_input_returns_empty() {
        let clusters = FaceDetector::cluster_faces(&[]).unwrap();
        assert!(clusters.is_empty());
    }

    #[test]
    fn cluster_faces_single_face_single_cluster() {
        let detections = vec![(1, vec![face_detection(vec![1.0, 0.0])])];
        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].faces.len(), 1);
    }

    #[test]
    fn cluster_faces_three_distinct_groups() {
        let detections = vec![
            (1, vec![face_detection(vec![1.0, 0.0, 0.0])]),
            (2, vec![face_detection(vec![0.99, 0.01, 0.0])]),
            (3, vec![face_detection(vec![0.0, 1.0, 0.0])]),
            (4, vec![face_detection(vec![0.0, 0.99, 0.01])]),
            (5, vec![face_detection(vec![0.0, 0.0, 1.0])]),
            (6, vec![face_detection(vec![0.01, 0.0, 0.99])]),
        ];
        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 3);
    }

    #[test]
    fn cluster_faces_identical_embeddings_one_cluster() {
        let detections = vec![
            (1, vec![face_detection(vec![0.6, 0.8])]),
            (2, vec![face_detection(vec![0.6, 0.8])]),
            (3, vec![face_detection(vec![0.6, 0.8])]),
        ];
        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].faces.len(), 3);
    }

    #[test]
    fn cluster_faces_maximally_different_yields_n_clusters() {
        let detections = vec![
            (1, vec![face_detection(vec![1.0, 0.0])]),
            (2, vec![face_detection(vec![0.0, 1.0])]),
            (3, vec![face_detection(vec![-1.0, 0.0])]),
        ];
        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 3);
    }

    #[test]
    fn cluster_faces_skips_empty_embeddings() {
        let detections = vec![
            (1, vec![face_detection(vec![])]),
            (2, vec![face_detection(vec![1.0, 0.0])]),
        ];
        let clusters = FaceDetector::cluster_faces(&detections).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].faces.len(), 1);
    }
}
