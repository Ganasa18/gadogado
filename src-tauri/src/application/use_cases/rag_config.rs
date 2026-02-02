use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// RAG system configuration with all tunable parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    // Chunking configuration
    pub chunking: ChunkingConfig,

    // Retrieval configuration
    pub retrieval: RetrievalConfig,

    // Embedding configuration
    pub embedding: EmbeddingConfig,

    // OCR configuration
    pub ocr: OcrConfig,

    // Cache configuration
    pub cache: CacheConfig,

    // Chat configuration
    pub chat: ChatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Chunking strategy: "fixed_size", "content_aware", "semantic"
    pub strategy: String,

    /// Maximum chunk size in characters
    pub chunk_size: usize,

    /// Overlap between chunks in characters
    pub overlap: usize,

    /// Minimum chunk quality score (0.0 - 1.0)
    pub min_quality_score: f32,

    /// Whether to respect document boundaries (headers, paragraphs)
    pub respect_boundaries: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Retrieval mode: "vector", "keyword", "hybrid"
    pub mode: String,

    /// Number of results to retrieve
    pub top_k: usize,

    /// Weight for vector similarity in hybrid mode (0.0 - 1.0)
    pub vector_weight: f32,

    /// Weight for BM25 keyword score in hybrid mode (0.0 - 1.0)
    pub keyword_weight: f32,

    /// Whether to enable LLM-based reranking
    pub reranking_enabled: bool,

    /// Number of candidates to retrieve before reranking (QA mode).
    #[serde(default = "default_candidate_k")]
    pub candidate_k: usize,

    /// Number of candidates to send to the local reranker (QA mode).
    #[serde(default = "default_rerank_k")]
    pub rerank_k: usize,

    /// Minimum relevance score threshold (0.0 - 1.0)
    pub min_relevance_score: f32,

    /// Whether to use query expansion with synonyms
    pub query_expansion_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Embedding model name (e.g., "nomic-embed-text", "all-minilm")
    pub model: String,

    /// Embedding dimension
    pub dimension: usize,

    /// API endpoint for embedding service
    pub api_endpoint: String,

    /// Batch size for embedding generation
    pub batch_size: usize,

    /// Request timeout in milliseconds
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR engine: "tesseract", "paddle", "auto"
    pub engine: String,

    /// Languages for OCR (e.g., "eng+ind")
    pub languages: String,

    /// Whether to enable image preprocessing
    pub preprocessing_enabled: bool,

    /// Preprocessing mode: "auto", "grayscale", "otsu", "contrast"
    pub preprocessing_mode: String,

    /// Minimum confidence threshold for OCR results (0.0 - 1.0)
    pub min_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries in embedding cache
    pub embedding_cache_size: usize,

    /// Embedding cache TTL in seconds
    pub embedding_cache_ttl_secs: u64,

    /// Maximum number of entries in retrieval cache
    pub retrieval_cache_size: usize,

    /// Retrieval cache TTL in seconds
    pub retrieval_cache_ttl_secs: u64,

    /// Whether caching is enabled
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum conversation history length for context
    pub max_history_length: usize,

    /// Maximum tokens for conversation summary
    pub max_summary_tokens: usize,

    /// Whether to enable self-correcting RAG
    pub self_correction_enabled: bool,

    /// Whether to show confidence scores in responses
    pub show_confidence: bool,

    /// Whether to show source citations
    pub show_citations: bool,

    /// Whether to collect user feedback
    pub feedback_enabled: bool,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            chunking: ChunkingConfig::default(),
            retrieval: RetrievalConfig::default(),
            embedding: EmbeddingConfig::default(),
            ocr: OcrConfig::default(),
            cache: CacheConfig::default(),
            chat: ChatConfig::default(),
        }
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: "content_aware".to_string(),
            chunk_size: 500,
            overlap: 50,
            min_quality_score: 0.3,
            respect_boundaries: true,
        }
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            mode: "hybrid".to_string(),
            top_k: 5,
            vector_weight: 0.7,
            keyword_weight: 0.3,
            reranking_enabled: true,
            candidate_k: default_candidate_k(),
            rerank_k: default_rerank_k(),
            min_relevance_score: 0.1,
            query_expansion_enabled: true,
        }
    }
}

fn default_candidate_k() -> usize {
    100
}

fn default_rerank_k() -> usize {
    75
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            api_endpoint: "http://localhost:11434".to_string(),
            batch_size: 10,
            timeout_ms: 30000,
        }
    }
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            engine: "tesseract".to_string(),
            languages: "eng+ind".to_string(),
            preprocessing_enabled: true,
            preprocessing_mode: "auto".to_string(),
            min_confidence: 0.6,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            embedding_cache_size: 1000,
            embedding_cache_ttl_secs: 3600,
            retrieval_cache_size: 500,
            retrieval_cache_ttl_secs: 300,
            enabled: true,
        }
    }
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_history_length: 10,
            max_summary_tokens: 500,
            self_correction_enabled: true,
            show_confidence: true,
            show_citations: true,
            feedback_enabled: true,
        }
    }
}

/// Validation result for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RagConfig {
    /// Validate configuration values
    pub fn validate(&self) -> ConfigValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate chunking config
        if self.chunking.chunk_size < 100 {
            errors.push("Chunk size must be at least 100 characters".to_string());
        }
        if self.chunking.chunk_size > 2000 {
            warnings.push("Chunk sizes over 2000 may impact retrieval quality".to_string());
        }
        if self.chunking.overlap >= self.chunking.chunk_size {
            errors.push("Overlap must be less than chunk size".to_string());
        }
        if !["fixed_size", "content_aware", "semantic"].contains(&self.chunking.strategy.as_str()) {
            errors.push(format!(
                "Invalid chunking strategy: {}",
                self.chunking.strategy
            ));
        }

        // Validate retrieval config
        if self.retrieval.top_k == 0 {
            errors.push("top_k must be at least 1".to_string());
        }
        if self.retrieval.top_k > 50 {
            warnings.push("top_k over 50 may impact performance".to_string());
        }
        if !["vector", "keyword", "hybrid"].contains(&self.retrieval.mode.as_str()) {
            errors.push(format!("Invalid retrieval mode: {}", self.retrieval.mode));
        }
        let weight_sum = self.retrieval.vector_weight + self.retrieval.keyword_weight;
        if (weight_sum - 1.0).abs() > 0.01 {
            warnings.push(format!(
                "Retrieval weights sum to {:.2} instead of 1.0",
                weight_sum
            ));
        }

        // Validate embedding config
        if self.embedding.dimension == 0 {
            errors.push("Embedding dimension must be greater than 0".to_string());
        }
        if self.embedding.batch_size == 0 {
            errors.push("Batch size must be at least 1".to_string());
        }
        if self.embedding.timeout_ms < 1000 {
            warnings.push("Embedding timeout under 1 second may cause failures".to_string());
        }

        // Validate OCR config
        if !["tesseract", "paddle", "auto"].contains(&self.ocr.engine.as_str()) {
            errors.push(format!("Invalid OCR engine: {}", self.ocr.engine));
        }
        if !["auto", "grayscale", "otsu", "contrast"]
            .contains(&self.ocr.preprocessing_mode.as_str())
        {
            errors.push(format!(
                "Invalid preprocessing mode: {}",
                self.ocr.preprocessing_mode
            ));
        }

        // Validate cache config
        if self.cache.enabled && self.cache.embedding_cache_size == 0 {
            warnings.push("Cache enabled but embedding cache size is 0".to_string());
        }

        // Validate chat config
        if self.chat.max_history_length == 0 {
            warnings
                .push("Max history length is 0, no conversation context will be used".to_string());
        }

        ConfigValidation {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

/// Configuration manager for loading, saving, and updating config
pub struct ConfigManager {
    config: RagConfig,
    config_path: PathBuf,
    dirty: bool,
}

impl ConfigManager {
    /// Create a new config manager with default config
    pub fn new(config_dir: PathBuf) -> Self {
        let config_path = config_dir.join("rag_config.json");
        let config = Self::load_from_file(&config_path).unwrap_or_default();

        Self {
            config,
            config_path,
            dirty: false,
        }
    }

    /// Load config from file
    fn load_from_file(path: &PathBuf) -> Option<RagConfig> {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => Some(config),
                    Err(e) => {
                        eprintln!("Failed to parse config file: {}", e);
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read config file: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Get current configuration
    pub fn get_config(&self) -> &RagConfig {
        &self.config
    }

    /// Get mutable reference to config
    pub fn get_config_mut(&mut self) -> &mut RagConfig {
        self.dirty = true;
        &mut self.config
    }

    /// Update entire configuration
    pub fn update_config(&mut self, new_config: RagConfig) -> ConfigValidation {
        let validation = new_config.validate();
        if validation.valid {
            self.config = new_config;
            self.dirty = true;
        }
        validation
    }

    /// Update specific section of configuration
    pub fn update_chunking(&mut self, config: ChunkingConfig) {
        self.config.chunking = config;
        self.dirty = true;
    }

    pub fn update_retrieval(&mut self, config: RetrievalConfig) {
        self.config.retrieval = config;
        self.dirty = true;
    }

    pub fn update_embedding(&mut self, config: EmbeddingConfig) {
        self.config.embedding = config;
        self.dirty = true;
    }

    pub fn update_ocr(&mut self, config: OcrConfig) {
        self.config.ocr = config;
        self.dirty = true;
    }

    pub fn update_cache(&mut self, config: CacheConfig) {
        self.config.cache = config;
        self.dirty = true;
    }

    pub fn update_chat(&mut self, config: ChatConfig) {
        self.config.chat = config;
        self.dirty = true;
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) {
        self.config = RagConfig::default();
        self.dirty = true;
    }

    /// Save configuration to file
    pub fn save(&mut self) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config directory: {}", e))?;
            }
        }

        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        self.dirty = false;
        Ok(())
    }

    /// Check if config has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

/// Thread-safe shared config manager
pub struct SharedConfigManager {
    inner: Arc<Mutex<ConfigManager>>,
}

impl SharedConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ConfigManager::new(config_dir))),
        }
    }

    pub fn get_config(&self) -> RagConfig {
        self.inner.lock().unwrap().get_config().clone()
    }

    pub fn update_config(&self, new_config: RagConfig) -> ConfigValidation {
        self.inner.lock().unwrap().update_config(new_config)
    }

    pub fn update_chunking(&self, config: ChunkingConfig) {
        self.inner.lock().unwrap().update_chunking(config);
    }

    pub fn update_retrieval(&self, config: RetrievalConfig) {
        self.inner.lock().unwrap().update_retrieval(config);
    }

    pub fn update_embedding(&self, config: EmbeddingConfig) {
        self.inner.lock().unwrap().update_embedding(config);
    }

    pub fn update_ocr(&self, config: OcrConfig) {
        self.inner.lock().unwrap().update_ocr(config);
    }

    pub fn update_cache(&self, config: CacheConfig) {
        self.inner.lock().unwrap().update_cache(config);
    }

    pub fn update_chat(&self, config: ChatConfig) {
        self.inner.lock().unwrap().update_chat(config);
    }

    pub fn reset_to_defaults(&self) {
        self.inner.lock().unwrap().reset_to_defaults();
    }

    pub fn save(&self) -> Result<(), String> {
        self.inner.lock().unwrap().save()
    }

    pub fn validate(&self) -> ConfigValidation {
        self.inner.lock().unwrap().get_config().validate()
    }
}

impl Clone for SharedConfigManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// User feedback data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub query_id: String,
    pub query_text: String,
    pub response_text: String,
    pub rating: FeedbackRating,
    pub comment: Option<String>,
    pub timestamp: u64,
    pub collection_id: Option<i64>,
    pub retrieval_mode: Option<String>,
    pub chunks_used: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeedbackRating {
    ThumbsUp,
    ThumbsDown,
    Neutral,
}

/// Feedback collector for improving RAG quality
pub struct FeedbackCollector {
    feedback: Vec<UserFeedback>,
    max_entries: usize,
}

impl FeedbackCollector {
    pub fn new(max_entries: usize) -> Self {
        Self {
            feedback: Vec::new(),
            max_entries,
        }
    }

    pub fn add_feedback(&mut self, feedback: UserFeedback) {
        self.feedback.push(feedback);

        // Keep only the most recent entries
        if self.feedback.len() > self.max_entries {
            self.feedback.remove(0);
        }
    }

    pub fn get_all_feedback(&self) -> &[UserFeedback] {
        &self.feedback
    }

    pub fn get_positive_feedback(&self) -> Vec<&UserFeedback> {
        self.feedback
            .iter()
            .filter(|f| f.rating == FeedbackRating::ThumbsUp)
            .collect()
    }

    pub fn get_negative_feedback(&self) -> Vec<&UserFeedback> {
        self.feedback
            .iter()
            .filter(|f| f.rating == FeedbackRating::ThumbsDown)
            .collect()
    }

    pub fn get_feedback_stats(&self) -> FeedbackStats {
        let total = self.feedback.len();
        let positive = self
            .feedback
            .iter()
            .filter(|f| f.rating == FeedbackRating::ThumbsUp)
            .count();
        let negative = self
            .feedback
            .iter()
            .filter(|f| f.rating == FeedbackRating::ThumbsDown)
            .count();
        let neutral = total - positive - negative;

        FeedbackStats {
            total_count: total,
            positive_count: positive,
            negative_count: negative,
            neutral_count: neutral,
            positive_rate: if total > 0 {
                positive as f32 / total as f32
            } else {
                0.0
            },
        }
    }

    pub fn clear(&mut self) {
        self.feedback.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_count: usize,
    pub positive_count: usize,
    pub negative_count: usize,
    pub neutral_count: usize,
    pub positive_rate: f32,
}

/// Thread-safe shared feedback collector
pub struct SharedFeedbackCollector {
    inner: Arc<Mutex<FeedbackCollector>>,
}

impl SharedFeedbackCollector {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FeedbackCollector::new(max_entries))),
        }
    }

    pub fn add_feedback(&self, feedback: UserFeedback) {
        self.inner.lock().unwrap().add_feedback(feedback);
    }

    pub fn get_stats(&self) -> FeedbackStats {
        self.inner.lock().unwrap().get_feedback_stats()
    }

    pub fn get_recent_feedback(&self, limit: usize) -> Vec<UserFeedback> {
        let collector = self.inner.lock().unwrap();
        collector
            .feedback
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

impl Clone for SharedFeedbackCollector {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
