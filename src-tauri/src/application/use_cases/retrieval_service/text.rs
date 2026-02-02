use super::{Bm25Scorer, QueryResult, RetrievalService};
use crate::application::use_cases::rag_config::RagConfig;
use crate::domain::error::Result;
use crate::infrastructure::db::rag::repository::{ChunkWithMetadata, ChunkWithMetadataScore};
use crate::interfaces::http::add_log;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

impl RetrievalService {
    pub(super) async fn retrieve_text_chunks(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let chunks = self
            .rag_repository
            .search_chunks_by_collection(collection_id, 1000)
            .await?;

        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let has_embeddings = chunks.iter().any(|chunk| chunk.embedding.is_some());

        // Build chunk map for lookups
        let chunk_map: HashMap<i64, &ChunkWithMetadata> = chunks.iter().map(|c| (c.id, c)).collect();

        // Collect all result sets for RRF fusion
        let mut all_result_sets: Vec<Vec<(i64, f32)>> = Vec::new();

        // 1. BM25 keyword search (always run)
        let bm25_results = self.bm25_search(&chunks, query_text, top_k * 2);
        if !bm25_results.is_empty() {
            all_result_sets.push(bm25_results);
        }

        // 2. Vector search with query expansion (if embeddings available)
        if has_embeddings {
            let expanded_queries = self.expand_query(query_text);

            for expanded_query in &expanded_queries {
                if let Ok(query_embedding) = self.embedding_service.generate_embedding(expanded_query).await {
                    let search_results = self.vector_search.search_with_metadata(
                        &query_embedding,
                        &chunks,
                        top_k * 2,
                    );

                    let result_set: Vec<(i64, f32)> = search_results.iter().map(|r| (r.chunk_id, r.score)).collect();

                    if !result_set.is_empty() {
                        all_result_sets.push(result_set);
                    }
                }
            }
        }

        // 3. Combine all results using weighted score fusion (preserves actual confidence scores)
        let mut results = Vec::new();

        if all_result_sets.len() > 1 {
            // Multiple result sets - fuse them
            let fused_results = self.weighted_score_fusion(all_result_sets);

            for (chunk_id, score) in fused_results.iter().take(top_k) {
                if let Some(chunk) = chunk_map.get(chunk_id) {
                    results.push(QueryResult {
                        content: chunk.content.clone(),
                        source_type: "text_chunk".to_string(),
                        source_id: chunk.id,
                        score: Some(*score),
                        page_number: chunk.page_number,
                        page_offset: chunk.page_offset,
                        doc_name: Some(chunk.doc_name.clone()),
                    });
                }
            }
        } else if !all_result_sets.is_empty() {
            // Single result set - use directly
            for (chunk_id, score) in all_result_sets[0].iter().take(top_k) {
                if let Some(chunk) = chunk_map.get(chunk_id) {
                    results.push(QueryResult {
                        content: chunk.content.clone(),
                        source_type: "text_chunk".to_string(),
                        source_id: chunk.id,
                        score: Some(*score),
                        page_number: chunk.page_number,
                        page_offset: chunk.page_offset,
                        doc_name: Some(chunk.doc_name.clone()),
                    });
                }
            }
        }

        // Fallback to enhanced keyword search if no results
        if results.is_empty() {
            results = self.keyword_fallback_results(&chunks, query_text, top_k);
        }

        Ok(results)
    }

    pub(super) async fn retrieve_text_chunks_multiway(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
        cfg: &RagConfig,
        logs: Option<&Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<Vec<QueryResult>> {
        let query_hash = Self::hash_query(query_text);

        let candidate_k = cfg.retrieval.candidate_k.max(top_k).max(1);
        let rerank_k = cfg.retrieval.rerank_k.clamp(top_k, candidate_k);

        // 1) FTS5 candidates (keyword heavy)
        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "FTS search start (limit={}, query_hash={})",
                    candidate_k, query_hash
                ),
            );
        }

        let fts_start = Instant::now();
        let mut keyword_scores: HashMap<i64, f32> = HashMap::new();
        let mut fts_meta: HashMap<i64, ChunkWithMetadataScore> = HashMap::new();
        match self
            .rag_repository
            .search_chunks_fts_by_collection(collection_id, query_text, candidate_k as i64)
            .await
        {
            Ok(rows) => {
                if let Some(logs) = logs {
                    add_log(
                        logs,
                        "INFO",
                        "RAG",
                        &format!(
                            "FTS search done (count={}, ms={}, query_hash={})",
                            rows.len(),
                            fts_start.elapsed().as_millis(),
                            query_hash
                        ),
                    );
                }
                for r in rows {
                    let bm25 = if r.score.is_finite() { r.score } else { 0.0 };
                    let bm25_pos = bm25.max(0.0);
                    let normalized = 1.0 / (1.0 + bm25_pos);
                    keyword_scores.insert(r.id, normalized);
                    fts_meta.insert(r.id, r);
                }
            }
            Err(e) => {
                if let Some(logs) = logs {
                    add_log(
                        logs,
                        "WARN",
                        "RAG",
                        &format!(
                            "FTS search failed (ms={}, query_hash={}): {}",
                            fts_start.elapsed().as_millis(),
                            query_hash,
                            e
                        ),
                    );
                }
            }
        }

        // 2) Vector candidates (semantic)
        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("Loading chunks start (query_hash={})", query_hash),
            );
        }

        let chunks_start = Instant::now();
        let chunks = self
            .rag_repository
            .search_chunks_by_collection(collection_id, 2000)
            .await?;

        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Loading chunks done (count={}, ms={}, query_hash={})",
                    chunks.len(),
                    chunks_start.elapsed().as_millis(),
                    query_hash
                ),
            );
        }

        let chunk_map: HashMap<i64, &ChunkWithMetadata> = chunks.iter().map(|c| (c.id, c)).collect();

        let mut vector_scores: HashMap<i64, f32> = HashMap::new();
        let has_embeddings = chunks.iter().any(|c| c.embedding.is_some());

        if has_embeddings {
            let vec_start = Instant::now();
            let mut expanded_queries = if cfg.retrieval.query_expansion_enabled {
                let mut q = self.expand_query(query_text);
                if !q.iter().any(|s| s == query_text) {
                    q.insert(0, query_text.to_string());
                }
                q
            } else {
                vec![query_text.to_string()]
            };
            expanded_queries.truncate(3);

            if let Some(logs) = logs {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!(
                        "Vector search start (expanded_queries={}, limit={}, query_hash={})",
                        expanded_queries.len(),
                        candidate_k,
                        query_hash
                    ),
                );
            }

            for q in expanded_queries {
                if let Ok(query_embedding) = self.embedding_service.generate_embedding(&q).await {
                    let res = self
                        .vector_search
                        .search_with_metadata(&query_embedding, &chunks, candidate_k);
                    for r in res {
                        vector_scores
                            .entry(r.chunk_id)
                            .and_modify(|s| {
                                if r.score > *s {
                                    *s = r.score;
                                }
                            })
                            .or_insert(r.score);
                    }
                }
            }

            if let Some(logs) = logs {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!(
                        "Vector search done (unique_hits={}, ms={}, query_hash={})",
                        vector_scores.len(),
                        vec_start.elapsed().as_millis(),
                        query_hash
                    ),
                );
            }
        }

        // 3) Merge candidates by chunk_id (weighted fusion)
        let mut all_ids: HashSet<i64> = HashSet::new();
        all_ids.extend(keyword_scores.keys().copied());
        all_ids.extend(vector_scores.keys().copied());

        let mut fused: Vec<(i64, f32)> = Vec::with_capacity(all_ids.len());
        for id in all_ids {
            let kw = keyword_scores.get(&id).copied().unwrap_or(0.0);
            let vec = vector_scores.get(&id).copied().unwrap_or(0.0);
            let score = cfg.retrieval.keyword_weight * kw + cfg.retrieval.vector_weight * vec;
            if score > 0.0 {
                fused.push((id, score));
            }
        }
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        fused.truncate(candidate_k);

        let mut candidates: Vec<QueryResult> = Vec::with_capacity(fused.len());
        for (chunk_id, score) in fused {
            if let Some(chunk) = chunk_map.get(&chunk_id) {
                candidates.push(QueryResult {
                    content: chunk.content.clone(),
                    source_type: "text_chunk".to_string(),
                    source_id: chunk.id,
                    score: Some(score),
                    page_number: chunk.page_number,
                    page_offset: chunk.page_offset,
                    doc_name: Some(chunk.doc_name.clone()),
                });
                continue;
            }
            if let Some(chunk) = fts_meta.get(&chunk_id) {
                candidates.push(QueryResult {
                    content: chunk.content.clone(),
                    source_type: "text_chunk".to_string(),
                    source_id: chunk.id,
                    score: Some(score),
                    page_number: chunk.page_number,
                    page_offset: chunk.page_offset,
                    doc_name: Some(chunk.doc_name.clone()),
                });
            }
        }

        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Candidates fused (candidate_count={}, reranking_enabled={}, query_hash={})",
                    candidates.len(),
                    cfg.retrieval.reranking_enabled,
                    query_hash
                ),
            );
        }

        // 4) Local rerank (QA mode only)
        if cfg.retrieval.reranking_enabled && !candidates.is_empty() {
            let mut to_rerank = candidates.clone();
            to_rerank.truncate(rerank_k);

            if let Some(logs) = logs {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!(
                        "Rerank start (rerank_k={}, query_hash={})",
                        to_rerank.len(),
                        query_hash
                    ),
                );
            }

            let rerank_start = Instant::now();

            match self.reranker_service.rerank_with_info(query_text, to_rerank) {
                Ok((mut reranked, initialized)) => {
                    if let Some(logs) = logs {
                        add_log(
                            logs,
                            "INFO",
                            "RAG",
                            &format!(
                                "Rerank applied (reranked_count={}, initialized={}, ms={}, query_hash={})",
                                reranked.len(),
                                initialized,
                                rerank_start.elapsed().as_millis(),
                                query_hash
                            ),
                        );
                    }
                    // Ensure sorted by reranker score desc
                    reranked.sort_by(|a, b| {
                        b.score
                            .unwrap_or(0.0)
                            .partial_cmp(&a.score.unwrap_or(0.0))
                            .unwrap_or(Ordering::Equal)
                    });
                    return Ok(reranked);
                }
                Err(e) => {
                    if let Some(logs) = logs {
                        add_log(
                            logs,
                            "WARN",
                            "RAG",
                            &format!(
                                "Rerank failed; using fused candidates (ms={}, query_hash={}): {}",
                                rerank_start.elapsed().as_millis(),
                                query_hash,
                                e
                            ),
                        );
                    }
                }
            }
        }

        Ok(candidates)
    }

    /// Perform BM25 keyword search on chunks
    fn bm25_search(&self, chunks: &[ChunkWithMetadata], query: &str, top_k: usize) -> Vec<(i64, f32)> {
        // Build BM25 scorer from all document contents
        let documents: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
        let scorer = Bm25Scorer::from_documents(&documents);

        // Score each chunk
        let mut scored: Vec<(i64, f32)> = chunks
            .iter()
            .map(|chunk| {
                let score = scorer.score(query, &chunk.content);
                (chunk.id, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        scored.truncate(top_k);

        // Normalize BM25 scores to 0-1 range for fair fusion with cosine similarity
        // Use sigmoid-like normalization: score / (1 + score)
        // This maps any positive score to 0-1 range while preserving relative ordering
        scored
            .into_iter()
            .map(|(id, score)| {
                // Normalize using min-max scaling with soft clipping
                // Typical BM25 scores are 0-10, so we normalize to 0-1
                let normalized = score / (1.0 + score);
                (id, normalized)
            })
            .collect()
    }

    fn keyword_fallback_results(
        &self,
        chunks: &[ChunkWithMetadata],
        query_text: &str,
        top_k: usize,
    ) -> Vec<QueryResult> {
        // Expand query for better keyword matching
        let expanded_queries = self.expand_query(query_text);

        // Collect all unique tokens from expanded queries
        let mut all_tokens: HashSet<String> = HashSet::new();
        for query in &expanded_queries {
            for token in query.split_whitespace() {
                let clean_token = token
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if clean_token.len() > 2 {
                    all_tokens.insert(clean_token);
                }
            }
        }

        let query_tokens: Vec<String> = all_tokens.into_iter().collect();

        let mut scored_chunks: Vec<(f32, &ChunkWithMetadata)> = chunks
            .iter()
            .map(|chunk| {
                let content_lower = chunk.content.to_lowercase();
                let score = if query_tokens.is_empty() {
                    0.0
                } else {
                    // Count matches with bonus for exact phrase matches
                    let mut match_score = 0.0;
                    for token in &query_tokens {
                        if content_lower.contains(token.as_str()) {
                            match_score += 1.0;
                            // Bonus for word boundary matches
                            let pattern = format!(" {} ", token);
                            if content_lower.contains(&pattern) {
                                match_score += 0.5;
                            }
                        }
                    }
                    match_score / query_tokens.len() as f32
                };
                (score, chunk)
            })
            .collect();

        scored_chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

        // Filter out zero-score results
        scored_chunks.retain(|(score, _)| *score > 0.0);
        scored_chunks.truncate(top_k);

        scored_chunks
            .into_iter()
            .map(|(score, chunk)| QueryResult {
                content: chunk.content.clone(),
                source_type: "text_chunk".to_string(),
                source_id: chunk.id,
                score: Some(score),
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                doc_name: Some(chunk.doc_name.clone()),
            })
            .collect()
    }
}
