use super::RetrievalService;
use std::cmp::Ordering;

impl RetrievalService {
    /// Combine expanded query results using weighted score fusion
    /// This preserves the actual semantic similarity scores instead of just using rank
    pub(super) fn weighted_score_fusion(
        &self,
        result_sets: Vec<Vec<(i64, f32)>>,
    ) -> Vec<(i64, f32)> {
        use std::collections::HashMap;

        let mut fused_scores: HashMap<i64, f32> = HashMap::new();
        let mut score_weights: HashMap<i64, f32> = HashMap::new();

        for results in result_sets {
            for (chunk_id, score) in results.iter() {
                // Use the actual score, not just rank
                // Higher scores from any method contribute more to the final score
                let weight = *score_weights.get(chunk_id).unwrap_or(&0.0) + 1.0;

                // Average of scores from all methods, weighted by the score itself
                // This gives more weight to methods that return higher confidence scores
                *fused_scores.entry(*chunk_id).or_insert(0.0) += score;
                *score_weights.entry(*chunk_id).or_insert(0.0) = weight;
            }
        }

        // Normalize by number of times each chunk appeared
        let mut combined: Vec<(i64, f32)> = fused_scores
            .into_iter()
            .map(|(chunk_id, total_score)| {
                let weight = score_weights.get(&chunk_id).unwrap_or(&1.0);
                let avg_score = total_score / weight;
                (chunk_id, avg_score)
            })
            .collect();

        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        combined
    }

    /// Legacy RRF method - kept for reference but not used
    #[allow(dead_code)]
    pub(super) fn reciprocal_rank_fusion(
        &self,
        result_sets: Vec<Vec<(i64, f32)>>,
        k: f32,
    ) -> Vec<(i64, f32)> {
        use std::collections::HashMap;

        let mut rrf_scores: HashMap<i64, f32> = HashMap::new();

        for results in result_sets {
            for (rank, (chunk_id, _original_score)) in results.iter().enumerate() {
                let rrf_score = 1.0 / (k + rank as f32 + 1.0);
                *rrf_scores.entry(*chunk_id).or_insert(0.0) += rrf_score;
            }
        }

        let mut combined: Vec<(i64, f32)> = rrf_scores.into_iter().collect();
        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        combined
    }
}
