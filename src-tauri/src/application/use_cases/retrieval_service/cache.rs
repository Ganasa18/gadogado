use super::types::QueryResult;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

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
    pub fn get(
        &mut self,
        collection_id: i64,
        query: &str,
        top_k: usize,
    ) -> Option<Vec<QueryResult>> {
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
    pub fn put(
        &mut self,
        collection_id: i64,
        query: &str,
        top_k: usize,
        results: Vec<QueryResult>,
    ) {
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
pub(super) const DEFAULT_RETRIEVAL_CACHE_SIZE: usize = 500;
/// Default TTL in seconds (5 minutes for retrieval results)
pub(super) const DEFAULT_RETRIEVAL_CACHE_TTL_SECS: u64 = 300;
