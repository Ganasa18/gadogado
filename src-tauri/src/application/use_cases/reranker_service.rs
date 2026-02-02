use crate::application::QueryResult;
use crate::domain::error::{AppError, Result};
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::sync::Mutex;

pub struct RerankerService {
    model: Mutex<Option<TextRerank>>,
    model_id: RerankerModel,
}

impl Default for RerankerService {
    fn default() -> Self {
        // Prefer multilingual rerankers (Phase 05).
        Self::new(RerankerModel::JINARerankerV2BaseMultiligual)
    }
}

impl RerankerService {
    pub fn new(model_id: RerankerModel) -> Self {
        Self {
            model: Mutex::new(None),
            model_id,
        }
    }

    fn init_model(&self) -> Result<TextRerank> {
        let opts = RerankInitOptions::new(self.model_id.clone());
        TextRerank::try_new(opts)
            .map_err(|e| AppError::Internal(format!("Failed to init reranker: {}", e)))
    }

    /// Rerank candidates locally; replaces `score` with the reranker score.
    pub fn rerank_with_info(
        &self,
        query: &str,
        candidates: Vec<QueryResult>,
    ) -> Result<(Vec<QueryResult>, bool)> {
        if candidates.is_empty() {
            return Ok((candidates, false));
        }

        let mut guard = self.model.lock().unwrap();
        let mut initialized = false;
        if guard.is_none() {
            *guard = Some(self.init_model()?);
            initialized = true;
        }

        let model = guard
            .as_mut()
            .ok_or_else(|| AppError::Internal("Reranker not initialized".to_string()))?;

        let docs: Vec<&str> = candidates.iter().map(|c| c.content.as_str()).collect();

        // Do not return documents (we already have them). Use default batch size.
        let reranked = model
            .rerank(query, docs, false, None)
            .map_err(|e| AppError::Internal(format!("Rerank failed: {}", e)))?;

        let mut out = Vec::with_capacity(reranked.len());
        for r in reranked {
            // fastembed-rs returns 0-based index into input docs.
            let idx = r.index;
            if let Some(mut item) = candidates.get(idx).cloned() {
                item.score = Some(r.score);
                out.push(item);
            }
        }

        // fastembed reranker scores are *relative* (not probabilities) and can be negative.
        // Normalize to 0..1 so the rest of the pipeline (UI + thresholds) behaves consistently.
        let mut min_s = f32::INFINITY;
        let mut max_s = f32::NEG_INFINITY;
        for item in &out {
            if let Some(s) = item.score {
                if s.is_finite() {
                    min_s = min_s.min(s);
                    max_s = max_s.max(s);
                }
            }
        }

        if min_s.is_finite() && max_s.is_finite() {
            let range = (max_s - min_s).max(1e-6);
            for item in &mut out {
                if let Some(s) = item.score {
                    if s.is_finite() {
                        let norm = (s - min_s) / range;
                        item.score = Some(norm.clamp(0.0, 1.0));
                    } else {
                        item.score = Some(0.0);
                    }
                }
            }
        }

        Ok((out, initialized))
    }
}
