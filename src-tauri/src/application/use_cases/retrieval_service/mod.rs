mod analysis;
mod bm25;
mod cache;
mod context;
mod excel;
mod fusion;
mod query_expansion;
mod service;
mod structured;
mod text;
mod types;

pub use bm25::Bm25Scorer;
pub use cache::{RetrievalCache, RetrievalCacheStats};
pub use types::{NumericQuery, QueryAnalysis, QueryResult, QueryType, StructuredQueryHints};

use crate::application::use_cases::embedding_service::{EmbeddingService, VectorSearch};
use crate::application::use_cases::reranker_service::RerankerService;
use crate::infrastructure::db::rag::repository::RagRepository;
use std::sync::{Arc, Mutex};

pub struct RetrievalService {
    rag_repository: Arc<RagRepository>,
    embedding_service: Arc<EmbeddingService>,
    vector_search: VectorSearch,
    reranker_service: Arc<RerankerService>,
    /// Retrieval results cache
    cache: Arc<Mutex<RetrievalCache>>,
}

impl RetrievalService {
    pub fn new(rag_repository: Arc<RagRepository>, embedding_service: Arc<EmbeddingService>) -> Self {
        let vector_search = VectorSearch::new(embedding_service.clone());
        let reranker_service = Arc::new(RerankerService::default());
        Self {
            rag_repository,
            embedding_service,
            vector_search,
            reranker_service,
            cache: Arc::new(Mutex::new(RetrievalCache::new(
                cache::DEFAULT_RETRIEVAL_CACHE_SIZE,
                cache::DEFAULT_RETRIEVAL_CACHE_TTL_SECS,
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
        self.cache
            .lock()
            .unwrap()
            .invalidate_collection(collection_id);
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        self.cache.lock().unwrap().cleanup();
    }
}
