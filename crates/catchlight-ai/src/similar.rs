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
}
