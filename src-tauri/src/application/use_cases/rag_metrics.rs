use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ============================================================
// RAG METRICS - Performance Tracking
// ============================================================

/// Metrics for a single RAG operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagOperationMetrics {
    /// Unique operation ID
    pub operation_id: String,
    /// Type of operation (query, ingest, embed, etc.)
    pub operation_type: String,
    /// Total duration in milliseconds
    pub latency_ms: u64,
    /// Number of results returned
    pub result_count: usize,
    /// Average relevance score of results
    pub avg_relevance_score: Option<f32>,
    /// Number of chunks processed
    pub chunks_processed: Option<usize>,
    /// Embedding generation time in ms
    pub embedding_time_ms: Option<u64>,
    /// Retrieval time in ms
    pub retrieval_time_ms: Option<u64>,
    /// Reranking time in ms (if applicable)
    pub rerank_time_ms: Option<u64>,
    /// Timestamp when operation started
    pub timestamp: u64,
    /// Collection ID (if applicable)
    pub collection_id: Option<i64>,
    /// Whether operation was cache hit
    pub cache_hit: bool,
    /// Experiment ID if part of A/B test
    pub experiment_id: Option<String>,
    /// Experiment variant
    pub variant: Option<String>,
}

impl Default for RagOperationMetrics {
    fn default() -> Self {
        Self {
            operation_id: uuid_simple(),
            operation_type: "unknown".to_string(),
            latency_ms: 0,
            result_count: 0,
            avg_relevance_score: None,
            chunks_processed: None,
            embedding_time_ms: None,
            retrieval_time_ms: None,
            rerank_time_ms: None,
            timestamp: current_timestamp_ms(),
            collection_id: None,
            cache_hit: false,
            experiment_id: None,
            variant: None,
        }
    }
}

/// Aggregated metrics over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Time period start
    pub period_start: u64,
    /// Time period end
    pub period_end: u64,
    /// Total operations
    pub total_operations: usize,
    /// Average latency
    pub avg_latency_ms: f64,
    /// P50 latency
    pub p50_latency_ms: u64,
    /// P95 latency
    pub p95_latency_ms: u64,
    /// P99 latency
    pub p99_latency_ms: u64,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Average results per query
    pub avg_results_per_query: f32,
    /// Average relevance score
    pub avg_relevance: f32,
    /// Breakdown by operation type
    pub by_operation_type: HashMap<String, OperationTypeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTypeSummary {
    pub count: usize,
    pub avg_latency_ms: f64,
    pub total_latency_ms: u64,
}

/// Document processing quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentQualityMetrics {
    /// Document ID
    pub document_id: i64,
    /// Document name
    pub document_name: String,
    /// Total chunks created
    pub total_chunks: usize,
    /// Average chunk quality score
    pub avg_chunk_quality: f32,
    /// Chunks with quality below threshold
    pub low_quality_chunks: usize,
    /// Extraction method used
    pub extraction_method: String,
    /// OCR confidence (if OCR was used)
    pub ocr_confidence: Option<f32>,
    /// Processing time in ms
    pub processing_time_ms: u64,
    /// File size in bytes
    pub file_size_bytes: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Retrieval quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQualityMetrics {
    /// Query text
    pub query: String,
    /// Collection searched
    pub collection_id: i64,
    /// Number of results
    pub num_results: usize,
    /// Score distribution (min, max, avg, std_dev)
    pub score_distribution: ScoreDistribution,
    /// Retrieval method used
    pub retrieval_method: String,
    /// Whether BM25 was used
    pub used_bm25: bool,
    /// Whether vector search was used
    pub used_vector: bool,
    /// Whether reranking was applied
    pub used_reranking: bool,
    /// Time breakdown
    pub time_breakdown: RetrievalTimeBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistribution {
    pub min: f32,
    pub max: f32,
    pub avg: f32,
    pub std_dev: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTimeBreakdown {
    pub embedding_ms: u64,
    pub bm25_ms: u64,
    pub vector_search_ms: u64,
    pub reranking_ms: u64,
    pub fusion_ms: u64,
    pub total_ms: u64,
}

// ============================================================
// METRICS COLLECTOR
// ============================================================

/// Maximum number of metrics to keep in memory
const MAX_METRICS_HISTORY: usize = 10000;

pub struct RagMetricsCollector {
    /// Recent operation metrics
    metrics_history: Vec<RagOperationMetrics>,
    /// Document quality metrics
    document_quality: HashMap<i64, DocumentQualityMetrics>,
    /// Start time of the collector
    start_time: Instant,
}

impl RagMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics_history: Vec::with_capacity(1000),
            document_quality: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Record a new operation metric
    pub fn record_operation(&mut self, metrics: RagOperationMetrics) {
        // Evict old entries if at capacity
        if self.metrics_history.len() >= MAX_METRICS_HISTORY {
            // Remove oldest 10%
            let remove_count = MAX_METRICS_HISTORY / 10;
            self.metrics_history.drain(0..remove_count);
        }
        self.metrics_history.push(metrics);
    }

    /// Record document quality metrics
    pub fn record_document_quality(&mut self, metrics: DocumentQualityMetrics) {
        self.document_quality.insert(metrics.document_id, metrics);
    }

    /// Get metrics for a specific time range
    pub fn get_metrics_in_range(&self, start_ts: u64, end_ts: u64) -> Vec<&RagOperationMetrics> {
        self.metrics_history
            .iter()
            .filter(|m| m.timestamp >= start_ts && m.timestamp <= end_ts)
            .collect()
    }

    /// Get aggregated metrics for the last N minutes
    pub fn get_aggregated_metrics(&self, minutes: u64) -> AggregatedMetrics {
        let now = current_timestamp_ms();
        let start = now.saturating_sub(minutes * 60 * 1000);

        let relevant_metrics: Vec<&RagOperationMetrics> = self
            .metrics_history
            .iter()
            .filter(|m| m.timestamp >= start)
            .collect();

        if relevant_metrics.is_empty() {
            return AggregatedMetrics {
                period_start: start,
                period_end: now,
                total_operations: 0,
                avg_latency_ms: 0.0,
                p50_latency_ms: 0,
                p95_latency_ms: 0,
                p99_latency_ms: 0,
                cache_hit_rate: 0.0,
                avg_results_per_query: 0.0,
                avg_relevance: 0.0,
                by_operation_type: HashMap::new(),
            };
        }

        // Calculate latencies
        let mut latencies: Vec<u64> = relevant_metrics.iter().map(|m| m.latency_ms).collect();
        latencies.sort();

        let p50_idx = (latencies.len() as f64 * 0.5) as usize;
        let p95_idx = (latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;

        let total_latency: u64 = latencies.iter().sum();
        let avg_latency = total_latency as f64 / latencies.len() as f64;

        // Cache hit rate
        let cache_hits = relevant_metrics.iter().filter(|m| m.cache_hit).count();
        let cache_hit_rate = cache_hits as f32 / relevant_metrics.len() as f32;

        // Average results
        let total_results: usize = relevant_metrics.iter().map(|m| m.result_count).sum();
        let avg_results = total_results as f32 / relevant_metrics.len() as f32;

        // Average relevance
        let scores: Vec<f32> = relevant_metrics
            .iter()
            .filter_map(|m| m.avg_relevance_score)
            .collect();
        let avg_relevance = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f32>() / scores.len() as f32
        };

        // By operation type
        let mut by_type: HashMap<String, OperationTypeSummary> = HashMap::new();
        for m in &relevant_metrics {
            let entry = by_type
                .entry(m.operation_type.clone())
                .or_insert(OperationTypeSummary {
                    count: 0,
                    avg_latency_ms: 0.0,
                    total_latency_ms: 0,
                });
            entry.count += 1;
            entry.total_latency_ms += m.latency_ms;
        }
        for summary in by_type.values_mut() {
            summary.avg_latency_ms = summary.total_latency_ms as f64 / summary.count as f64;
        }

        AggregatedMetrics {
            period_start: start,
            period_end: now,
            total_operations: relevant_metrics.len(),
            avg_latency_ms: avg_latency,
            p50_latency_ms: latencies.get(p50_idx).copied().unwrap_or(0),
            p95_latency_ms: latencies.get(p95_idx).copied().unwrap_or(0),
            p99_latency_ms: latencies.get(p99_idx).copied().unwrap_or(0),
            cache_hit_rate,
            avg_results_per_query: avg_results,
            avg_relevance,
            by_operation_type: by_type,
        }
    }

    /// Get document quality summary
    pub fn get_document_quality_summary(&self) -> DocumentQualitySummary {
        let docs: Vec<&DocumentQualityMetrics> = self.document_quality.values().collect();

        if docs.is_empty() {
            return DocumentQualitySummary {
                total_documents: 0,
                total_chunks: 0,
                avg_chunk_quality: 0.0,
                low_quality_documents: 0,
                avg_processing_time_ms: 0.0,
            };
        }

        let total_chunks: usize = docs.iter().map(|d| d.total_chunks).sum();
        let total_quality: f32 = docs.iter().map(|d| d.avg_chunk_quality).sum();
        let low_quality_docs = docs.iter().filter(|d| d.avg_chunk_quality < 0.5).count();
        let total_processing: u64 = docs.iter().map(|d| d.processing_time_ms).sum();

        DocumentQualitySummary {
            total_documents: docs.len(),
            total_chunks,
            avg_chunk_quality: total_quality / docs.len() as f32,
            low_quality_documents: low_quality_docs,
            avg_processing_time_ms: total_processing as f64 / docs.len() as f64,
        }
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.metrics_history.clear();
        self.document_quality.clear();
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentQualitySummary {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub avg_chunk_quality: f32,
    pub low_quality_documents: usize,
    pub avg_processing_time_ms: f64,
}

// ============================================================
// METRICS TIMER - Helper for timing operations
// ============================================================

/// Helper struct for timing RAG operations
pub struct MetricsTimer {
    operation_type: String,
    start: Instant,
    embedding_start: Option<Instant>,
    embedding_end: Option<Instant>,
    retrieval_start: Option<Instant>,
    retrieval_end: Option<Instant>,
    rerank_start: Option<Instant>,
    rerank_end: Option<Instant>,
    collection_id: Option<i64>,
    cache_hit: bool,
    experiment_id: Option<String>,
    variant: Option<String>,
}

impl MetricsTimer {
    pub fn new(operation_type: &str) -> Self {
        Self {
            operation_type: operation_type.to_string(),
            start: Instant::now(),
            embedding_start: None,
            embedding_end: None,
            retrieval_start: None,
            retrieval_end: None,
            rerank_start: None,
            rerank_end: None,
            collection_id: None,
            cache_hit: false,
            experiment_id: None,
            variant: None,
        }
    }

    pub fn with_collection(mut self, collection_id: i64) -> Self {
        self.collection_id = Some(collection_id);
        self
    }

    pub fn with_experiment(mut self, experiment_id: &str, variant: &str) -> Self {
        self.experiment_id = Some(experiment_id.to_string());
        self.variant = Some(variant.to_string());
        self
    }

    pub fn mark_cache_hit(&mut self) {
        self.cache_hit = true;
    }

    pub fn start_embedding(&mut self) {
        self.embedding_start = Some(Instant::now());
    }

    pub fn end_embedding(&mut self) {
        self.embedding_end = Some(Instant::now());
    }

    pub fn start_retrieval(&mut self) {
        self.retrieval_start = Some(Instant::now());
    }

    pub fn end_retrieval(&mut self) {
        self.retrieval_end = Some(Instant::now());
    }

    pub fn start_rerank(&mut self) {
        self.rerank_start = Some(Instant::now());
    }

    pub fn end_rerank(&mut self) {
        self.rerank_end = Some(Instant::now());
    }

    /// Finalize and create metrics
    pub fn finish(
        self,
        result_count: usize,
        avg_score: Option<f32>,
        chunks_processed: Option<usize>,
    ) -> RagOperationMetrics {
        let total_duration = self.start.elapsed();

        let embedding_time = match (self.embedding_start, self.embedding_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_millis() as u64),
            _ => None,
        };

        let retrieval_time = match (self.retrieval_start, self.retrieval_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_millis() as u64),
            _ => None,
        };

        let rerank_time = match (self.rerank_start, self.rerank_end) {
            (Some(start), Some(end)) => Some(end.duration_since(start).as_millis() as u64),
            _ => None,
        };

        RagOperationMetrics {
            operation_id: uuid_simple(),
            operation_type: self.operation_type,
            latency_ms: total_duration.as_millis() as u64,
            result_count,
            avg_relevance_score: avg_score,
            chunks_processed,
            embedding_time_ms: embedding_time,
            retrieval_time_ms: retrieval_time,
            rerank_time_ms: rerank_time,
            timestamp: current_timestamp_ms(),
            collection_id: self.collection_id,
            cache_hit: self.cache_hit,
            experiment_id: self.experiment_id,
            variant: self.variant,
        }
    }
}

// ============================================================
// A/B EXPERIMENT CONFIGURATION
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// Unique experiment ID
    pub id: String,
    /// Experiment name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether experiment is active
    pub active: bool,
    /// Variants with their weights (should sum to 1.0)
    pub variants: Vec<ExperimentVariant>,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp (optional)
    pub end_time: Option<u64>,
    /// Metrics to track
    pub tracked_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentVariant {
    /// Variant ID (e.g., "control", "treatment_a")
    pub id: String,
    /// Variant name
    pub name: String,
    /// Traffic weight (0.0 to 1.0)
    pub weight: f32,
    /// Configuration overrides for this variant
    pub config: VariantConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariantConfig {
    /// Retrieval mode override
    pub retrieval_mode: Option<String>,
    /// Top-k override
    pub top_k: Option<usize>,
    /// Vector weight for hybrid search
    pub vector_weight: Option<f32>,
    /// BM25 weight for hybrid search
    pub bm25_weight: Option<f32>,
    /// Enable reranking
    pub enable_reranking: Option<bool>,
    /// Chunk strategy override
    pub chunk_strategy: Option<String>,
    /// Custom parameters
    pub custom: HashMap<String, String>,
}

pub struct ExperimentManager {
    experiments: HashMap<String, ExperimentConfig>,
    /// Track which users/sessions are assigned to which variants
    assignments: HashMap<String, (String, String)>, // session_id -> (experiment_id, variant_id)
}

impl ExperimentManager {
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
            assignments: HashMap::new(),
        }
    }

    /// Register a new experiment
    pub fn register_experiment(&mut self, config: ExperimentConfig) {
        self.experiments.insert(config.id.clone(), config);
    }

    /// Get experiment by ID
    pub fn get_experiment(&self, id: &str) -> Option<&ExperimentConfig> {
        self.experiments.get(id)
    }

    /// List all active experiments
    pub fn list_active_experiments(&self) -> Vec<&ExperimentConfig> {
        let now = current_timestamp_ms();
        self.experiments
            .values()
            .filter(|e| {
                e.active && e.start_time <= now && e.end_time.map(|end| end > now).unwrap_or(true)
            })
            .collect()
    }

    /// Assign a session to an experiment variant
    pub fn assign_variant(&mut self, session_id: &str, experiment_id: &str) -> Option<String> {
        // Check if already assigned
        if let Some((exp_id, variant_id)) = self.assignments.get(session_id) {
            if exp_id == experiment_id {
                return Some(variant_id.clone());
            }
        }

        // Get experiment
        let experiment = self.experiments.get(experiment_id)?;
        if !experiment.active {
            return None;
        }

        // Weighted random selection
        let mut rng_seed = session_id
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_add(b as u64));
        rng_seed = rng_seed.wrapping_mul(
            experiment_id
                .bytes()
                .fold(1u64, |acc, b| acc.wrapping_mul(b as u64 + 1)),
        );

        let random_val = (rng_seed % 10000) as f32 / 10000.0;
        let mut cumulative = 0.0;

        for variant in &experiment.variants {
            cumulative += variant.weight;
            if random_val < cumulative {
                self.assignments.insert(
                    session_id.to_string(),
                    (experiment_id.to_string(), variant.id.clone()),
                );
                return Some(variant.id.clone());
            }
        }

        // Fallback to first variant
        experiment.variants.first().map(|v| {
            self.assignments.insert(
                session_id.to_string(),
                (experiment_id.to_string(), v.id.clone()),
            );
            v.id.clone()
        })
    }

    /// Get the variant config for a session
    pub fn get_variant_config(
        &self,
        session_id: &str,
        experiment_id: &str,
    ) -> Option<&VariantConfig> {
        let (exp_id, variant_id) = self.assignments.get(session_id)?;
        if exp_id != experiment_id {
            return None;
        }

        let experiment = self.experiments.get(experiment_id)?;
        experiment
            .variants
            .iter()
            .find(|v| &v.id == variant_id)
            .map(|v| &v.config)
    }

    /// Deactivate an experiment
    pub fn deactivate_experiment(&mut self, experiment_id: &str) {
        if let Some(exp) = self.experiments.get_mut(experiment_id) {
            exp.active = false;
        }
    }

    /// Get experiment results summary
    pub fn get_experiment_results(
        &self,
        experiment_id: &str,
        metrics: &[RagOperationMetrics],
    ) -> Option<ExperimentResults> {
        let experiment = self.experiments.get(experiment_id)?;

        let mut variant_results: HashMap<String, VariantResults> = HashMap::new();

        // Initialize variant results
        for variant in &experiment.variants {
            variant_results.insert(
                variant.id.clone(),
                VariantResults {
                    variant_id: variant.id.clone(),
                    variant_name: variant.name.clone(),
                    sample_count: 0,
                    avg_latency_ms: 0.0,
                    avg_relevance: 0.0,
                    total_latency: 0,
                    total_relevance: 0.0,
                },
            );
        }

        // Aggregate metrics by variant
        for metric in metrics {
            if metric.experiment_id.as_deref() == Some(experiment_id) {
                if let Some(variant_id) = &metric.variant {
                    if let Some(result) = variant_results.get_mut(variant_id) {
                        result.sample_count += 1;
                        result.total_latency += metric.latency_ms;
                        if let Some(score) = metric.avg_relevance_score {
                            result.total_relevance += score;
                        }
                    }
                }
            }
        }

        // Calculate averages
        for result in variant_results.values_mut() {
            if result.sample_count > 0 {
                result.avg_latency_ms = result.total_latency as f64 / result.sample_count as f64;
                result.avg_relevance = result.total_relevance / result.sample_count as f32;
            }
        }

        Some(ExperimentResults {
            experiment_id: experiment_id.to_string(),
            experiment_name: experiment.name.clone(),
            variants: variant_results.into_values().collect(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: String,
    pub experiment_name: String,
    pub variants: Vec<VariantResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResults {
    pub variant_id: String,
    pub variant_name: String,
    pub sample_count: usize,
    pub avg_latency_ms: f64,
    pub avg_relevance: f32,
    #[serde(skip)]
    total_latency: u64,
    #[serde(skip)]
    total_relevance: f32,
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn uuid_simple() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);

    format!("{:016x}", hasher.finish())
}

// ============================================================
// THREAD-SAFE WRAPPER
// ============================================================

/// Thread-safe metrics collector
pub struct SharedMetricsCollector {
    inner: Arc<Mutex<RagMetricsCollector>>,
}

impl SharedMetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RagMetricsCollector::new())),
        }
    }

    pub fn record_operation(&self, metrics: RagOperationMetrics) {
        if let Ok(mut collector) = self.inner.lock() {
            collector.record_operation(metrics);
        }
    }

    pub fn record_document_quality(&self, metrics: DocumentQualityMetrics) {
        if let Ok(mut collector) = self.inner.lock() {
            collector.record_document_quality(metrics);
        }
    }

    pub fn get_aggregated_metrics(&self, minutes: u64) -> AggregatedMetrics {
        self.inner
            .lock()
            .map(|c| c.get_aggregated_metrics(minutes))
            .unwrap_or_else(|_| AggregatedMetrics {
                period_start: 0,
                period_end: 0,
                total_operations: 0,
                avg_latency_ms: 0.0,
                p50_latency_ms: 0,
                p95_latency_ms: 0,
                p99_latency_ms: 0,
                cache_hit_rate: 0.0,
                avg_results_per_query: 0.0,
                avg_relevance: 0.0,
                by_operation_type: HashMap::new(),
            })
    }

    pub fn get_document_quality_summary(&self) -> DocumentQualitySummary {
        self.inner
            .lock()
            .map(|c| c.get_document_quality_summary())
            .unwrap_or_else(|_| DocumentQualitySummary {
                total_documents: 0,
                total_chunks: 0,
                avg_chunk_quality: 0.0,
                low_quality_documents: 0,
                avg_processing_time_ms: 0.0,
            })
    }

    pub fn clear(&self) {
        if let Ok(mut collector) = self.inner.lock() {
            collector.clear();
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.inner.lock().map(|c| c.uptime_secs()).unwrap_or(0)
    }
}

impl Clone for SharedMetricsCollector {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Thread-safe experiment manager
pub struct SharedExperimentManager {
    inner: Arc<Mutex<ExperimentManager>>,
}

impl SharedExperimentManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ExperimentManager::new())),
        }
    }

    pub fn register_experiment(&self, config: ExperimentConfig) {
        if let Ok(mut manager) = self.inner.lock() {
            manager.register_experiment(config);
        }
    }

    pub fn assign_variant(&self, session_id: &str, experiment_id: &str) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|mut m| m.assign_variant(session_id, experiment_id))
    }

    pub fn get_variant_config(
        &self,
        session_id: &str,
        experiment_id: &str,
    ) -> Option<VariantConfig> {
        self.inner
            .lock()
            .ok()
            .and_then(|m| m.get_variant_config(session_id, experiment_id).cloned())
    }

    pub fn list_active_experiments(&self) -> Vec<ExperimentConfig> {
        self.inner
            .lock()
            .map(|m| m.list_active_experiments().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn deactivate_experiment(&self, experiment_id: &str) {
        if let Ok(mut manager) = self.inner.lock() {
            manager.deactivate_experiment(experiment_id);
        }
    }
}

impl Clone for SharedExperimentManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_timer() {
        let mut timer = MetricsTimer::new("query").with_collection(1);
        timer.start_embedding();
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.end_embedding();
        timer.start_retrieval();
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.end_retrieval();

        let metrics = timer.finish(5, Some(0.85), Some(100));

        assert_eq!(metrics.operation_type, "query");
        assert_eq!(metrics.result_count, 5);
        assert_eq!(metrics.collection_id, Some(1));
        assert!(metrics.embedding_time_ms.is_some());
        assert!(metrics.retrieval_time_ms.is_some());
    }

    #[test]
    fn test_aggregated_metrics() {
        let mut collector = RagMetricsCollector::new();

        for i in 0..10 {
            collector.record_operation(RagOperationMetrics {
                latency_ms: 100 + i * 10,
                result_count: 5,
                cache_hit: i % 2 == 0,
                avg_relevance_score: Some(0.8),
                ..Default::default()
            });
        }

        let aggregated = collector.get_aggregated_metrics(5);
        assert_eq!(aggregated.total_operations, 10);
        assert!(aggregated.avg_latency_ms > 0.0);
        assert_eq!(aggregated.cache_hit_rate, 0.5);
    }

    #[test]
    fn test_experiment_assignment() {
        let mut manager = ExperimentManager::new();

        manager.register_experiment(ExperimentConfig {
            id: "test_exp".to_string(),
            name: "Test Experiment".to_string(),
            description: "Testing".to_string(),
            active: true,
            variants: vec![
                ExperimentVariant {
                    id: "control".to_string(),
                    name: "Control".to_string(),
                    weight: 0.5,
                    config: VariantConfig::default(),
                },
                ExperimentVariant {
                    id: "treatment".to_string(),
                    name: "Treatment".to_string(),
                    weight: 0.5,
                    config: VariantConfig {
                        enable_reranking: Some(true),
                        ..Default::default()
                    },
                },
            ],
            start_time: 0,
            end_time: None,
            tracked_metrics: vec!["latency".to_string(), "relevance".to_string()],
        });

        // Same session should get same variant
        let variant1 = manager.assign_variant("session_123", "test_exp");
        let variant2 = manager.assign_variant("session_123", "test_exp");
        assert_eq!(variant1, variant2);

        // Different sessions may get different variants
        let _variant3 = manager.assign_variant("session_456", "test_exp");
        // Just verify it returns Some
        assert!(variant1.is_some());
    }
}
