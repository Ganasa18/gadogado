use super::{QueryResult, QueryType, RetrievalService};
use crate::application::use_cases::rag_config::RagConfig;
use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Mutex;

impl RetrievalService {
    pub async fn query(&self, collection_id: i64, query_text: &str, top_k: usize) -> Result<Vec<QueryResult>> {
        let analysis = self.analyze_query(query_text);
        let mut results = Vec::new();

        match analysis.query_type {
            QueryType::Structured => {
                let structured_results = self
                    .retrieve_structured_rows(collection_id, query_text, &analysis.structured, top_k)
                    .await?;
                results.extend(structured_results);
            }
            QueryType::NumericOnly => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k)
                    .await?;
                results.extend(excel_results);
            }
            QueryType::TextOnly => {
                let text_results = self.retrieve_text_chunks(collection_id, query_text, top_k).await?;
                results.extend(text_results);
            }
            QueryType::Hybrid => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k / 2)
                    .await?;
                let text_results = self.retrieve_text_chunks(collection_id, query_text, top_k / 2).await?;
                results.extend(excel_results);
                results.extend(text_results);
            }
        }

        Ok(results)
    }

    /// Query with caching support - returns cached results if available
    pub async fn query_cached(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<(Vec<QueryResult>, bool)> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached_results) = cache.get(collection_id, query_text, top_k) {
                return Ok((cached_results, true)); // true = cache hit
            }
        }

        // Cache miss - perform actual query
        let results = self.query(collection_id, query_text, top_k).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(collection_id, query_text, top_k, results.clone());
        }

        Ok((results, false)) // false = cache miss
    }

    /// Query with context optimization applied
    pub async fn query_optimized(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        self.query_optimized_impl(collection_id, query_text, top_k, None, None)
            .await
    }

    pub async fn query_optimized_with_config(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
        rag_config: &RagConfig,
        logs: &Mutex<Vec<crate::interfaces::http::LogEntry>>,
    ) -> Result<Vec<QueryResult>> {
        self.query_optimized_impl(
            collection_id,
            query_text,
            top_k,
            Some(rag_config),
            Some(logs),
        )
        .await
    }

    async fn query_optimized_impl(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
        rag_config: Option<&RagConfig>,
        logs: Option<&Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<Vec<QueryResult>> {
        let default_config = RagConfig::default();
        let cfg = rag_config.unwrap_or(&default_config);

        let query_len = query_text.chars().count();
        let query_hash = Self::hash_query(query_text);

        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Retrieval start (collection_id={}, query_len={}, query_hash={})",
                    collection_id, query_len, query_hash
                ),
            );
        }

        let analysis = self.analyze_query(query_text);

        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Intent route (query_type={:?}, wants_aggregate={}, wants_count={}, wants_sources={}, wants_titles={}, has_category={}, has_source={}, has_keyword={}, query_hash={})",
                    analysis.query_type,
                    analysis.structured.wants_aggregate,
                    analysis.structured.wants_count,
                    analysis.structured.wants_sources,
                    analysis.structured.wants_titles,
                    analysis.structured.category.is_some(),
                    analysis.structured.source.is_some(),
                    analysis.structured.keyword.is_some(),
                    query_hash
                ),
            );
        }

        // Phase 05 applies multi-way recall + local reranking for QA (TextOnly) mode.
        let raw_results = match analysis.query_type {
            QueryType::TextOnly => {
                self.retrieve_text_chunks_multiway(collection_id, query_text, top_k, cfg, logs)
                    .await?
            }
            QueryType::Structured => {
                // Preserve structured route, but if a collection has no structured rows,
                // fall back to QA retrieval over text chunks.
                let structured_results = self
                    .retrieve_structured_rows(collection_id, query_text, &analysis.structured, top_k * 2)
                    .await?;

                if !structured_results.is_empty() {
                    structured_results
                } else {
                    let structured_count = self
                        .rag_repository
                        .count_structured_rows_by_collection(
                            collection_id,
                            analysis.structured.category.as_deref(),
                            analysis.structured.source.as_deref(),
                            analysis.structured.keyword.as_deref(),
                        )
                        .await
                        .unwrap_or(0);

                    if let Some(logs) = logs {
                        add_log(
                            logs,
                            "INFO",
                            "RAG",
                            &format!(
                                "Structured retrieval empty (structured_rows_count={}, query_hash={})",
                                structured_count, query_hash
                            ),
                        );
                    }

                    if structured_count == 0 {
                        self.retrieve_text_chunks_multiway(collection_id, query_text, top_k, cfg, logs)
                            .await?
                    } else {
                        structured_results
                    }
                }
            }
            _ => {
                // Preserve existing behavior for structured/numeric/hybrid.
                self.query(collection_id, query_text, top_k * 2).await?
            }
        };

        // Optimize and limit to requested count
        let mut optimized = self.optimize_context(raw_results);
        optimized.truncate(top_k);

        if let Some(logs) = logs {
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Retrieval done (final_context_count={}, query_hash={})",
                    optimized.len(),
                    query_hash
                ),
            );
        }

        Ok(optimized)
    }
}
