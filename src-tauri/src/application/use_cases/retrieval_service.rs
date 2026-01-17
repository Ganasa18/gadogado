use crate::application::use_cases::embedding_service::{EmbeddingService, VectorSearch};
use crate::domain::error::Result;
// use crate::domain::rag_entities::{RagDocumentChunk, RagExcelData};
use crate::infrastructure::db::rag::repository::{ChunkWithMetadata, RagRepository};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================
// RETRIEVAL CACHE
// ============================================================

/// Cache entry for query results
#[derive(Clone)]
struct RetrievalCacheEntry {
    results: Vec<QueryResult>,
    created_at: Instant,
}

/// LRU cache for retrieval results with TTL
pub struct RetrievalCache {
    cache: HashMap<String, RetrievalCacheEntry>,
    max_size: usize,
    ttl: Duration,
    access_order: Vec<String>,
    /// Cache statistics
    hits: usize,
    misses: usize,
}

impl RetrievalCache {
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            ttl: Duration::from_secs(ttl_secs),
            access_order: Vec::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Create a cache key from query parameters
    fn make_key(collection_id: i64, query: &str, top_k: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        collection_id.hash(&mut hasher);
        query.to_lowercase().hash(&mut hasher);
        top_k.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Get results from cache if valid
    pub fn get(&mut self, collection_id: i64, query: &str, top_k: usize) -> Option<Vec<QueryResult>> {
        let key = Self::make_key(collection_id, query, top_k);

        let result = if let Some(entry) = self.cache.get(&key) {
            if entry.created_at.elapsed() < self.ttl {
                Some(entry.results.clone())
            } else {
                None
            }
        } else {
            None
        };

        if result.is_some() {
            self.hits += 1;
            self.touch(&key);
        } else {
            self.misses += 1;
            // Remove expired entry if exists
            if self.cache.contains_key(&key) {
                self.cache.remove(&key);
                self.access_order.retain(|k| k != &key);
            }
        }

        result
    }

    /// Store results in cache
    pub fn put(&mut self, collection_id: i64, query: &str, top_k: usize, results: Vec<QueryResult>) {
        let key = Self::make_key(collection_id, query, top_k);

        // Evict oldest entries if at capacity
        while self.cache.len() >= self.max_size && !self.access_order.is_empty() {
            let oldest = self.access_order.remove(0);
            self.cache.remove(&oldest);
        }

        self.cache.insert(
            key.clone(),
            RetrievalCacheEntry {
                results,
                created_at: Instant::now(),
            },
        );
        self.access_order.push(key);
    }

    /// Update access order for LRU
    fn touch(&mut self, key: &str) {
        self.access_order.retain(|k| k != key);
        self.access_order.push(key.to_string());
    }

    /// Get cache statistics
    pub fn stats(&self) -> RetrievalCacheStats {
        let total_requests = self.hits + self.misses;
        let hit_rate = if total_requests > 0 {
            self.hits as f32 / total_requests as f32
        } else {
            0.0
        };

        let valid_entries = self
            .cache
            .values()
            .filter(|e| e.created_at.elapsed() < self.ttl)
            .count();

        RetrievalCacheStats {
            total_entries: self.cache.len(),
            valid_entries,
            max_size: self.max_size,
            hits: self.hits,
            misses: self.misses,
            hit_rate,
        }
    }

    /// Invalidate cache for a specific collection
    pub fn invalidate_collection(&mut self, _collection_id: i64) {
        // Since our key includes collection_id hash, we need to track collection->keys
        // For now, we'll clear all (simpler, safe invalidation)
        // A more sophisticated approach would track keys per collection
        self.cache.clear();
        self.access_order.clear();
    }

    /// Clear expired entries
    pub fn cleanup(&mut self) {
        let expired_keys: Vec<String> = self
            .cache
            .iter()
            .filter(|(_, entry)| entry.created_at.elapsed() >= self.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
            self.access_order.retain(|k| k != &key);
        }
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalCacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub max_size: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f32,
}

/// Default cache size
const DEFAULT_RETRIEVAL_CACHE_SIZE: usize = 500;
/// Default TTL in seconds (5 minutes for retrieval results)
const DEFAULT_RETRIEVAL_CACHE_TTL_SECS: u64 = 300;

// ============================================================
// BM25 IMPLEMENTATION
// ============================================================

/// BM25 scoring parameters
const BM25_K1: f32 = 1.2; // Term frequency saturation
const BM25_B: f32 = 0.75; // Length normalization

/// Simple BM25 scorer for keyword-based retrieval
pub struct Bm25Scorer {
    /// Document frequencies: term -> number of documents containing term
    doc_frequencies: HashMap<String, usize>,
    /// Total number of documents
    total_docs: usize,
    /// Average document length
    avg_doc_len: f32,
}

impl Bm25Scorer {
    /// Build a BM25 scorer from a collection of documents
    pub fn from_documents(documents: &[&str]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        let mut total_length = 0usize;

        for doc in documents {
            let tokens = Self::tokenize(doc);
            let unique_tokens: HashSet<_> = tokens.iter().collect();

            for token in unique_tokens {
                *doc_frequencies.entry(token.clone()).or_insert(0) += 1;
            }

            total_length += tokens.len();
        }

        let avg_doc_len = if documents.is_empty() {
            1.0
        } else {
            total_length as f32 / documents.len() as f32
        };

        Self {
            doc_frequencies,
            total_docs: documents.len(),
            avg_doc_len,
        }
    }

    /// Score a document against a query
    pub fn score(&self, query: &str, document: &str) -> f32 {
        let query_tokens = Self::tokenize(query);
        let doc_tokens = Self::tokenize(document);
        let doc_len = doc_tokens.len() as f32;

        // Count term frequencies in document
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        for token in &doc_tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        let mut score = 0.0f32;

        for term in &query_tokens {
            let tf = *term_freqs.get(term).unwrap_or(&0) as f32;
            let df = *self.doc_frequencies.get(term).unwrap_or(&0) as f32;

            if tf > 0.0 && df > 0.0 {
                // IDF component
                let idf = ((self.total_docs as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();

                // TF component with length normalization
                let tf_component = (tf * (BM25_K1 + 1.0))
                    / (tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (doc_len / self.avg_doc_len)));

                score += idf * tf_component;
            }
        }

        score
    }

    /// Tokenize text into lowercase terms
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub content: String,
    pub source_type: String,
    pub source_id: i64,
    pub score: Option<f32>,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub doc_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryAnalysis {
    pub query_type: QueryType,
    pub numeric_queries: Vec<NumericQuery>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryType {
    TextOnly,
    NumericOnly,
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NumericQuery {
    pub column: String,
    pub operator: String,
    pub value: String,
}

pub struct RetrievalService {
    rag_repository: Arc<RagRepository>,
    embedding_service: Arc<EmbeddingService>,
    vector_search: VectorSearch,
    /// Retrieval results cache
    cache: Arc<Mutex<RetrievalCache>>,
}

impl RetrievalService {
    pub fn new(
        rag_repository: Arc<RagRepository>,
        embedding_service: Arc<EmbeddingService>,
    ) -> Self {
        let vector_search = VectorSearch::new(embedding_service.clone());
        Self {
            rag_repository,
            embedding_service,
            vector_search,
            cache: Arc::new(Mutex::new(RetrievalCache::new(
                DEFAULT_RETRIEVAL_CACHE_SIZE,
                DEFAULT_RETRIEVAL_CACHE_TTL_SECS,
            ))),
        }
    }

    /// Get retrieval cache statistics
    pub fn cache_stats(&self) -> RetrievalCacheStats {
        self.cache.lock().unwrap().stats()
    }

    /// Clear the retrieval cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Invalidate cache for a specific collection (call after document updates)
    pub fn invalidate_collection_cache(&self, collection_id: i64) {
        self.cache.lock().unwrap().invalidate_collection(collection_id);
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        self.cache.lock().unwrap().cleanup();
    }

    // ============================================================
    // QUERY EXPANSION
    // ============================================================

    /// Expand the query with synonyms and related terms for better retrieval
    fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expansions = Vec::new();
        let lowercase_query = query.to_lowercase();

        // Original query (normalized)
        expansions.push(lowercase_query.clone());

        // Common technical synonyms mapping
        let synonyms: &[(&[&str], &[&str])] = &[
            // Programming terms
            (&["function", "func", "fn"], &["method", "procedure", "routine"]),
            (&["variable", "var"], &["parameter", "argument", "field"]),
            (&["class"], &["type", "struct", "object"]),
            (&["error", "exception"], &["bug", "issue", "problem", "failure"]),
            (&["create", "make"], &["generate", "build", "construct", "initialize"]),
            (&["delete", "remove"], &["destroy", "drop", "clear", "erase"]),
            (&["update", "modify"], &["change", "edit", "alter", "patch"]),
            (&["get", "fetch", "retrieve"], &["obtain", "load", "read", "query"]),
            (&["send", "post"], &["submit", "transmit", "push"]),
            (&["array", "list"], &["collection", "vector", "sequence"]),
            (&["config", "configuration"], &["settings", "options", "preferences"]),
            (&["api"], &["endpoint", "interface", "service"]),
            (&["database", "db"], &["storage", "datastore", "repository"]),
            (&["authentication", "auth"], &["login", "signin", "authorization"]),
            (&["user"], &["account", "member", "client"]),
            // Document terms
            (&["page"], &["section", "chapter", "part"]),
            (&["summary"], &["overview", "abstract", "synopsis"]),
            (&["detail", "details"], &["information", "specifics", "particulars"]),
        ];

        // Apply synonym expansion
        for (terms, related) in synonyms {
            for term in *terms {
                if lowercase_query.contains(term) {
                    for synonym in *related {
                        let expanded = lowercase_query.replace(term, synonym);
                        if !expansions.contains(&expanded) {
                            expansions.push(expanded);
                        }
                    }
                    break;
                }
            }
        }

        // Extract key terms (remove stop words)
        let stop_words: HashSet<&str> = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "shall", "can", "need", "dare",
            "ought", "used", "to", "of", "in", "for", "on", "with", "at", "by",
            "from", "as", "into", "through", "during", "before", "after", "above",
            "below", "between", "under", "again", "further", "then", "once", "here",
            "there", "when", "where", "why", "how", "all", "each", "few", "more",
            "most", "other", "some", "such", "no", "nor", "not", "only", "own",
            "same", "so", "than", "too", "very", "just", "and", "but", "if", "or",
            "because", "until", "while", "about", "against", "this", "that", "these",
            "those", "what", "which", "who", "whom", "whose", "i", "me", "my", "we",
            "our", "you", "your", "he", "him", "his", "she", "her", "it", "its",
            "they", "them", "their",
        ].iter().cloned().collect();

        let key_terms: Vec<&str> = lowercase_query
            .split_whitespace()
            .filter(|word| {
                word.len() > 2 && !stop_words.contains(word.trim_matches(|c: char| !c.is_alphanumeric()))
            })
            .collect();

        // Add individual key terms if query has multiple words
        if key_terms.len() > 1 {
            for term in &key_terms {
                let term_str = term.to_string();
                if !expansions.contains(&term_str) {
                    expansions.push(term_str);
                }
            }
        }

        // Limit expansions to avoid query explosion
        expansions.truncate(5);
        expansions
    }

    /// Combine expanded query results using Reciprocal Rank Fusion (RRF)
    fn reciprocal_rank_fusion(
        &self,
        result_sets: Vec<Vec<(i64, f32)>>,  // Vec of (chunk_id, score) tuples
        k: f32,  // RRF constant (typically 60)
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

    pub async fn query(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let analysis = self.analyze_query(query_text);
        let mut results = Vec::new();

        match analysis.query_type {
            QueryType::NumericOnly => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k)
                    .await?;
                results.extend(excel_results);
            }
            QueryType::TextOnly => {
                let text_results = self
                    .retrieve_text_chunks(collection_id, query_text, top_k)
                    .await?;
                results.extend(text_results);
            }
            QueryType::Hybrid => {
                let excel_results = self
                    .retrieve_excel_data(collection_id, &analysis.numeric_queries, top_k / 2)
                    .await?;
                let text_results = self
                    .retrieve_text_chunks(collection_id, query_text, top_k / 2)
                    .await?;
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

    // ============================================================
    // CONTEXT OPTIMIZATION
    // ============================================================

    /// Optimize retrieved results by removing duplicates and enriching metadata
    pub fn optimize_context(&self, results: Vec<QueryResult>) -> Vec<QueryResult> {
        let mut optimized = self.remove_duplicates(results);
        optimized = self.merge_adjacent_chunks(optimized);
        self.enrich_metadata(optimized)
    }

    /// Remove duplicate or near-duplicate results based on content similarity
    fn remove_duplicates(&self, results: Vec<QueryResult>) -> Vec<QueryResult> {
        let mut unique_results: Vec<QueryResult> = Vec::new();
        let similarity_threshold = 0.85; // Content overlap threshold

        for result in results {
            let is_duplicate = unique_results.iter().any(|existing| {
                self.content_similarity(&existing.content, &result.content) > similarity_threshold
            });

            if !is_duplicate {
                unique_results.push(result);
            }
        }

        unique_results
    }

    /// Calculate Jaccard similarity between two pieces of text
    fn content_similarity(&self, text1: &str, text2: &str) -> f32 {
        let text1_lower = text1.to_lowercase();
        let text2_lower = text2.to_lowercase();

        let words1: HashSet<&str> = text1_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        let words2: HashSet<&str> = text2_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        intersection as f32 / union as f32
    }

    /// Merge adjacent chunks from the same document for better context
    fn merge_adjacent_chunks(&self, mut results: Vec<QueryResult>) -> Vec<QueryResult> {
        if results.len() < 2 {
            return results;
        }

        // Sort by document and page/offset for adjacency detection
        results.sort_by(|a, b| {
            let doc_cmp = a.doc_name.cmp(&b.doc_name);
            if doc_cmp != Ordering::Equal {
                return doc_cmp;
            }
            let page_cmp = a.page_number.cmp(&b.page_number);
            if page_cmp != Ordering::Equal {
                return page_cmp;
            }
            a.page_offset.cmp(&b.page_offset)
        });

        let mut merged: Vec<QueryResult> = Vec::new();
        let mut i = 0;

        while i < results.len() {
            let mut current = results[i].clone();

            // Look for adjacent chunks from same document
            while i + 1 < results.len() {
                let next = &results[i + 1];

                // Check if adjacent (same doc, consecutive pages or close offsets)
                let is_adjacent = current.doc_name == next.doc_name
                    && current.page_number == next.page_number
                    && match (current.page_offset, next.page_offset) {
                        (Some(curr_off), Some(next_off)) => {
                            // Adjacent if within 600 chars (chunk size + overlap)
                            (next_off - curr_off).abs() < 600
                        }
                        _ => false,
                    };

                if is_adjacent {
                    // Merge content
                    current.content = format!("{}\n\n{}", current.content, next.content);
                    // Take max score
                    current.score = match (current.score, next.score) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (a, b) => a.or(b),
                    };
                    i += 1;
                } else {
                    break;
                }
            }

            merged.push(current);
            i += 1;
        }

        // Re-sort by score
        merged.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
        });

        merged
    }

    /// Enrich metadata for better citation and context
    fn enrich_metadata(&self, mut results: Vec<QueryResult>) -> Vec<QueryResult> {
        for result in &mut results {
            // Add content preview if content is very long
            if result.content.len() > 500 {
                // Keep full content but could add a preview field in the future
            }

            // Ensure source_type is descriptive
            if result.source_type == "text_chunk" {
                if let Some(ref doc_name) = result.doc_name {
                    if doc_name.ends_with(".pdf") {
                        result.source_type = "pdf_chunk".to_string();
                    } else if doc_name.ends_with(".docx") {
                        result.source_type = "docx_chunk".to_string();
                    } else if doc_name.ends_with(".md") {
                        result.source_type = "markdown_chunk".to_string();
                    }
                }
            }
        }

        results
    }

    /// Query with context optimization applied
    pub async fn query_optimized(
        &self,
        collection_id: i64,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        // Retrieve more results than needed to account for duplicate removal
        let raw_results = self.query(collection_id, query_text, top_k * 2).await?;

        // Optimize and limit to requested count
        let mut optimized = self.optimize_context(raw_results);
        optimized.truncate(top_k);

        Ok(optimized)
    }

    fn analyze_query(&self, query: &str) -> QueryAnalysis {
        let lowercase_query = query.to_lowercase();
        let mut numeric_queries = Vec::new();
        let mut has_numeric = false;
        let mut has_text = false;

        let keywords = ["val_a", "val_b", "column", "field", "value", "numeric"];
        for keyword in &keywords {
            if lowercase_query.contains(keyword) {
                has_numeric = true;
                break;
            }
        }

        if lowercase_query.contains("=") || lowercase_query.contains("equals") {
            if let Some(part) = lowercase_query.split('=').nth(1) {
                let value = part.trim().to_string();
                numeric_queries.push(NumericQuery {
                    column: "val_a".to_string(),
                    operator: "=".to_string(),
                    value,
                });
                has_numeric = true;
            }
        }

        let text_keywords = ["what", "how", "why", "explain", "describe", "summarize"];
        for keyword in &text_keywords {
            if lowercase_query.contains(keyword) {
                has_text = true;
                break;
            }
        }

        if !has_text && !has_numeric {
            has_text = true;
        }

        let query_type = if has_numeric && has_text {
            QueryType::Hybrid
        } else if has_numeric {
            QueryType::NumericOnly
        } else {
            QueryType::TextOnly
        };

        QueryAnalysis {
            query_type,
            numeric_queries,
        }
    }

    async fn retrieve_text_chunks(
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
        let chunk_map: HashMap<i64, &ChunkWithMetadata> = chunks
            .iter()
            .map(|c| (c.id, c))
            .collect();

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
                if let Ok(query_embedding) = self
                    .embedding_service
                    .generate_embedding(expanded_query)
                    .await
                {
                    let search_results = self.vector_search
                        .search_with_metadata(&query_embedding, &chunks, top_k * 2);

                    let result_set: Vec<(i64, f32)> = search_results
                        .iter()
                        .map(|r| (r.chunk_id, r.score))
                        .collect();

                    if !result_set.is_empty() {
                        all_result_sets.push(result_set);
                    }
                }
            }
        }

        // 3. Combine all results using Reciprocal Rank Fusion
        let mut results = Vec::new();

        if all_result_sets.len() > 1 {
            // Multiple result sets - fuse them
            let fused_results = self.reciprocal_rank_fusion(all_result_sets, 60.0);

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

        scored
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

    async fn retrieve_excel_data(
        &self,
        collection_id: i64,
        queries: &[NumericQuery],
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        let mut column_a = None;
        let mut column_b = None;

        for query in queries {
            if query.column == "val_a" && query.operator == "=" {
                column_a = Some(query.value.as_str());
            } else if query.column == "val_b" && query.operator == "=" {
                column_b = Some(query.value.as_str());
            }
        }

        let excel_data = self
            .rag_repository
            .search_excel_by_collection_with_filter(collection_id, column_a, column_b, top_k as i64)
            .await?;

        let mut results = Vec::new();
        for row in excel_data {
            let content = format!(
                "Row {}: val_a={}, val_b={}, val_c={}",
                row.row_index,
                row.val_a.unwrap_or_else(|| "null".to_string()),
                row.val_b.unwrap_or_else(|| "null".to_string()),
                row.val_c
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            );
            results.push(QueryResult {
                content,
                source_type: "excel_data".to_string(),
                source_id: row.id,
                score: None,
                page_number: None,
                page_offset: None,
                doc_name: None,
            });
        }

        Ok(results)
    }
}
