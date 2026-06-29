/// Cosine similarity between two equal-length vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

#[derive(Debug, Clone)]
pub struct PersonCluster {
    pub person_id: i64,
    pub face_ids: Vec<i64>,
    pub centroid: Vec<f32>,
    pub avg_intra_cluster_distance: f32,
}

fn avg_intra_cluster_distance(face_embeddings: &[(i64, Vec<f32>)], centroid: &[f32]) -> f32 {
    if face_embeddings.is_empty() {
        return 0.0;
    }

    let sum: f32 = face_embeddings
        .iter()
        .map(|(_, emb)| 1.0 - cosine_similarity(emb, centroid))
        .sum();
    sum / face_embeddings.len() as f32
}

type FaceClusterState = (Vec<i64>, Vec<f32>, Vec<Vec<f32>>);

/// Simple agglomerative clustering based on embedding cosine similarity.
/// O(n × k) where n = faces and k = clusters.
/// For >10K unassigned faces, consider DBSCAN or HNSW-based approach.
pub fn cluster_face_embeddings(faces: &[(i64, Vec<f32>)], threshold: f32) -> Vec<PersonCluster> {
    if faces.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<FaceClusterState> = Vec::new();

    for (face_id, embedding) in faces {
        let mut best_cluster = None;
        let mut best_sim = 0.0f32;

        for (i, (_, centroid, _)) in clusters.iter().enumerate() {
            let sim = cosine_similarity(embedding, centroid);
            if sim > threshold && sim > best_sim {
                best_cluster = Some(i);
                best_sim = sim;
            }
        }

        if let Some(idx) = best_cluster {
            clusters[idx].0.push(*face_id);
            clusters[idx].2.push(embedding.clone());
            let n = clusters[idx].0.len() as f32;
            for (j, v) in embedding.iter().enumerate() {
                if j < clusters[idx].1.len() {
                    clusters[idx].1[j] = clusters[idx].1[j] * ((n - 1.0) / n) + v / n;
                }
            }
        } else {
            clusters.push((vec![*face_id], embedding.clone(), vec![embedding.clone()]));
        }
    }

    clusters
        .into_iter()
        .enumerate()
        .map(|(i, (face_ids, centroid, embeddings))| {
            let paired: Vec<(i64, Vec<f32>)> = face_ids
                .iter()
                .zip(embeddings.iter())
                .map(|(id, emb)| (*id, emb.clone()))
                .collect();
            PersonCluster {
                person_id: i as i64 + 1,
                face_ids,
                centroid: centroid.clone(),
                avg_intra_cluster_distance: avg_intra_cluster_distance(&paired, &centroid),
            }
        })
        .collect()
}

/// Find candidates whose cosine similarity to `target_embedding` exceeds `threshold`.
/// Returns `(media_id, similarity_score)` pairs sorted by score descending.
pub fn find_similar(
    target_embedding: &[f32],
    candidates: &[(i64, Vec<f32>)],
    threshold: f32,
    limit: usize,
) -> Vec<(i64, f32)> {
    let mut scored: Vec<(i64, f32)> = candidates
        .iter()
        .filter_map(|(id, emb)| {
            let score = cosine_similarity(target_embedding, emb);
            (score >= threshold).then_some((*id, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_have_similarity_one() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn orthogonal_vectors_have_zero_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn cluster_face_embeddings_groups_similar_faces() {
        let faces = vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.99, 0.01]),
            (3, vec![0.0, 1.0]),
        ];
        let clusters = cluster_face_embeddings(&faces, 0.45);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].face_ids.len() + clusters[1].face_ids.len(), 3);
    }

    #[test]
    fn cluster_face_embeddings_empty_input() {
        assert!(cluster_face_embeddings(&[], 0.5).is_empty());
    }

    #[test]
    fn find_similar_respects_threshold_and_limit() {
        let target = vec![1.0, 0.0];
        let candidates = vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.9, 0.1]),
            (3, vec![0.0, 1.0]),
        ];
        let results = find_similar(&target, &candidates, 0.5, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn find_similar_orders_by_descending_similarity() {
        let target = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            (1, vec![0.5, 0.5, 0.0]),
            (2, vec![1.0, 0.0, 0.0]),
            (3, vec![0.8, 0.2, 0.0]),
        ];
        let results = find_similar(&target, &candidates, 0.0, 10);
        assert_eq!(results[0].0, 2);
        assert_eq!(results[1].0, 3);
        assert_eq!(results[2].0, 1);
    }

    #[test]
    fn cosine_similarity_empty_vectors_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[]), 0.0);
    }

    #[test]
    fn cosine_similarity_mismatched_lengths_returns_zero() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
    }

    #[test]
    fn cosine_similarity_all_zero_vectors_returns_zero() {
        let zeros = vec![0.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&zeros, &zeros), 0.0);
    }

    #[test]
    fn cosine_similarity_handles_negative_values() {
        let a = vec![1.0, -1.0];
        let b = vec![-1.0, 1.0];
        assert!(cosine_similarity(&a, &b) < 0.0);
    }

    #[test]
    fn cosine_similarity_large_vectors() {
        let dim = 10_000;
        let a: Vec<f32> = (0..dim).map(|i| (i as f32).sin()).collect();
        let b = a.clone();
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn find_similar_threshold_one_only_exact_matches() {
        let target = vec![1.0, 0.0];
        let candidates = vec![(1, vec![1.0, 0.0]), (2, vec![0.99, 0.01])];
        let results = find_similar(&target, &candidates, 1.0, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn find_similar_threshold_zero_includes_non_negative_matches() {
        let target = vec![1.0, 0.0];
        let candidates = vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.0, 1.0]),
            (3, vec![-1.0, 0.0]),
        ];
        let results = find_similar(&target, &candidates, 0.0, 10);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(id, _)| *id == 1 || *id == 2));
    }

    #[test]
    fn find_similar_limit_zero_returns_empty() {
        let target = vec![1.0, 0.0];
        let candidates = vec![(1, vec![1.0, 0.0])];
        assert!(find_similar(&target, &candidates, 0.0, 0).is_empty());
    }

    #[test]
    fn cluster_face_embeddings_single_face_single_cluster() {
        let faces = vec![(1, vec![1.0, 0.0])];
        let clusters = cluster_face_embeddings(&faces, 0.45);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].face_ids, vec![1]);
    }

    #[test]
    fn cluster_face_embeddings_three_distinct_groups() {
        let faces = vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.99, 0.01, 0.0]),
            (3, vec![0.0, 1.0, 0.0]),
            (4, vec![0.0, 0.99, 0.01]),
            (5, vec![0.0, 0.0, 1.0]),
            (6, vec![0.01, 0.0, 0.99]),
        ];
        let clusters = cluster_face_embeddings(&faces, 0.45);
        assert_eq!(clusters.len(), 3);
    }

    #[test]
    fn cluster_face_embeddings_identical_embeddings_one_cluster() {
        let faces = vec![
            (1, vec![0.6, 0.8]),
            (2, vec![0.6, 0.8]),
            (3, vec![0.6, 0.8]),
        ];
        let clusters = cluster_face_embeddings(&faces, 0.45);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].face_ids.len(), 3);
    }

    #[test]
    fn cluster_face_embeddings_maximally_different_yields_n_clusters() {
        let faces = vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.0, 1.0]),
            (3, vec![-1.0, 0.0]),
        ];
        let clusters = cluster_face_embeddings(&faces, 0.45);
        assert_eq!(clusters.len(), 3);
    }

    #[test]
    fn cluster_face_embeddings_threshold_zero_merges_similar_faces() {
        let faces = vec![(1, vec![1.0, 0.0]), (2, vec![0.99, 0.01])];
        let clusters = cluster_face_embeddings(&faces, 0.0);
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn cluster_face_embeddings_threshold_one_keeps_faces_separate() {
        let faces = vec![(1, vec![1.0, 0.0]), (2, vec![1.0, 0.0])];
        let clusters = cluster_face_embeddings(&faces, 1.0);
        assert_eq!(clusters.len(), 2);
    }
}
