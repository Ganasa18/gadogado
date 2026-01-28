//! Type Definitions for RAG Commands
//!
//! This module contains shared type definitions used across
//! RAG Tauri commands for serialization/deserialization.

use crate::application::use_cases::rag_config::{ChunkingConfig, OcrConfig};
use crate::application::use_cases::rag_validation::{ValidationCase, ValidationOptions};
use crate::domain::rag_entities::RagDocumentChunk;
use serde::{Deserialize, Serialize};

// Re-export quality analytics types
pub use crate::domain::rag_entities::{
    CollectionQualityMetrics, DocumentWarning, DocumentWarningInput, RetrievalGap,
    RetrievalGapInput,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct RagQueryRequest {
    pub collection_id: i64,
    pub query: String,
    pub top_k: Option<usize>,
    /// Override retrieval candidate pool size (Phase 05 QA path)
    pub candidate_k: Option<usize>,
    /// Override reranker input size (Phase 05 QA path)
    pub rerank_k: Option<usize>,
    /// Enable few-shot conversational examples
    #[serde(default)]
    pub enable_few_shot: Option<bool>,
    /// Language for conversational responses ('id', 'en', 'indonesia', 'english', etc.)
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RagQueryResponse {
    pub prompt: String,
    pub results: Vec<crate::application::QueryResult>,
}

// ============================================================
// PHASE 6: BACKEND API EXTENSIONS
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedOcrRequest {
    pub file_path: String,
    pub config: Option<OcrConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SmartChunkingRequest {
    pub text: String,
    pub config: Option<ChunkingConfig>,
}

#[derive(Debug, Serialize)]
pub struct SmartChunk {
    pub index: usize,
    pub content: String,
    pub token_count: usize,
    pub quality_score: Option<f32>,
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HybridRetrievalOptions {
    pub top_k: Option<usize>,
    pub use_cache: Option<bool>,
    pub optimized: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct HybridRetrievalResponse {
    pub results: Vec<crate::application::QueryResult>,
    pub cache_hit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationSuiteRequest {
    pub cases: Vec<ValidationCase>,
    pub options: Option<ValidationOptions>,
}

/// Web crawl mode: "html" (fast) or "ocr" (accurate for JS-heavy sites)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebCrawlMode {
    #[default]
    Html,
    Ocr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagWebImportRequest {
    pub url: String,
    pub collection_id: Option<i64>,
    pub max_pages: Option<usize>,
    pub max_depth: Option<usize>,
    /// Crawl mode: "html" (default) or "ocr" (Playwright + Tesseract)
    #[serde(default)]
    pub mode: WebCrawlMode,
}

// ============================================================
// CHAT WITH CONTEXT TYPES
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatWithContextRequest {
    pub collection_id: i64,
    pub query: String,
    pub conversation_id: Option<i64>,
    pub messages: Option<Vec<ChatMessage>>,
    pub top_k: Option<usize>,
    #[serde(default)]
    pub enable_verification: bool,
    /// Language for conversational responses ('id', 'en', 'indonesia', 'english', etc.)
    #[serde(default)]
    pub language: Option<String>,
    /// LLM provider (for context management)
    #[serde(default)]
    pub provider: Option<String>,
    /// LLM model (for context management)
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatWithContextResponse {
    pub prompt: String,
    pub results: Vec<crate::application::QueryResult>,
    pub conversation_id: Option<i64>,
    pub context_summary: Option<String>,
    pub verified: bool,
    /// Context management metadata
    pub context_managed: Option<ContextManagedInfo>,
}

/// Information about how context was managed
#[derive(Debug, Serialize, Deserialize)]
pub struct ContextManagedInfo {
    pub was_compacted: bool,
    pub strategy_used: String,
    pub token_estimate: usize,
    pub messages_used: usize,
    pub messages_total: usize,
}

// ============================================================
// HEALTH CHECK TYPES
// ============================================================

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub status: String,
    pub components: HealthComponents,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct HealthComponents {
    pub database: ComponentHealth,
    pub embedding_service: ComponentHealth,
    pub embedding_cache: CacheHealth,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CacheHealth {
    pub status: String,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub max_size: usize,
}

// ============================================================
// CHUNK VIEWER TYPES
// ============================================================

#[derive(Debug, Serialize)]
pub struct ChunkWithQuality {
    pub chunk: RagDocumentChunk,
    pub quality_score: f32,
    pub has_embedding: bool,
    pub token_estimate: usize,
}

// ============================================================
// CSV PREPROCESSING TYPES
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvPreprocessingRequest {
    pub file_path: String,
    pub config: Option<CsvPreprocessingRequestConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvPreprocessingRequestConfig {
    pub min_value_length_threshold: Option<usize>,
    pub min_lexical_diversity: Option<f32>,
    pub max_numeric_ratio: Option<f32>,
    pub min_sample_rows: Option<usize>,
    pub max_sample_rows: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct CsvPreprocessingResponse {
    pub content_type: String,
    pub processed_text: String,
    pub row_count: usize,
    pub analysis: CsvFieldAnalysis,
    pub headers: Vec<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct CsvFieldAnalysis {
    pub avg_value_length: f32,
    pub lexical_diversity: f32,
    pub total_fields: usize,
    pub numeric_ratio: f32,
    pub row_count: usize,
    pub empty_field_count: usize,
    pub max_value_length: usize,
    pub min_value_length: usize,
    pub confidence_score: f32,
}

#[derive(Debug, Serialize)]
pub struct CsvPreviewRow {
    pub index: usize,
    pub content: String,
}

// ============================================================
// SQL-RAG TYPES
// ============================================================

#[derive(Debug, Deserialize)]
pub struct DbQueryRequest {
    pub collection_id: i64,
    pub query: String,
    pub limit: Option<i32>,
    pub final_k: Option<i32>,
    /// Flag to distinguish new query from regeneration
    /// - true: Start fresh, don't use template feedback
    /// - false/undefined: Use template feedback if available
    #[serde(default)]
    pub is_new_query: Option<bool>,
    /// Optional conversation history for NL response generation
    /// SQL generation remains standalone (without context) to maintain template matching accuracy
    /// NL response uses this history to provide contextual answers
    #[serde(default)]
    pub conversation_history: Option<Vec<ChatMessage>>,
}

#[derive(Debug, Serialize)]
pub struct DbQueryResponse {
    pub answer: String,
    pub citations: Vec<DbCitation>,
    pub telemetry: DbQueryTelemetry,
    pub plan: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DbCitation {
    pub table_name: String,
    pub row_id: String,
    pub columns: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQueryTelemetry {
    pub row_count: usize,
    pub latency_ms: i64,
    pub llm_route: String,
    pub query_plan: Option<String>,
    /// Actual executed SQL query (for debugging/transparency)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_sql: Option<String>,
    // Few-shot template info (Feature 31)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_match_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_templates: Option<Vec<TemplateMatchInfo>>,
    /// @deprecated: Use modified_where_clause instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_mappings: Option<std::collections::HashMap<String, String>>,
    /// Modified WHERE clause generated by LLM (for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_where_clause: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TemplateMatchInfo {
    pub template_id: i64,
    pub template_name: String,
    pub score: f32,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_pattern: Option<String>,
}

// ============================================================
// SQL-RAG TEMPLATE-FIRST QUERY TYPES (Feature 31 Enhancement)
// ============================================================

/// Request to query with a specific template (for regeneration)
#[derive(Debug, Deserialize)]
pub struct DbQueryWithTemplateRequest {
    pub collection_id: i64,
    pub query: String,
    pub template_id: i64,
    pub limit: Option<i32>,
    pub final_k: Option<i32>,
    /// Optional conversation history for NL response generation
    /// SQL generation remains standalone (without context) to maintain template matching accuracy
    /// NL response uses this history to provide contextual answers
    #[serde(default)]
    pub conversation_history: Option<Vec<ChatMessage>>,
}

/// Request to submit template feedback (learning)
#[derive(Debug, Deserialize)]
pub struct TemplateFeedbackRequest {
    pub collection_id: i64,
    pub query: String,
    pub auto_selected_template_id: Option<i64>,
    pub user_selected_template_id: i64,
}

/// Response for template feedback submission
#[derive(Debug, Serialize)]
pub struct TemplateFeedbackResponse {
    pub success: bool,
    pub message: String,
}

/// LLM template selection result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmTemplateSelection {
    pub selected_template_id: i64,
    pub extracted_params: std::collections::HashMap<String, String>,
    /// Modified WHERE clause with correct column mappings (if different from template)
    #[serde(default)]
    pub modified_where_clause: Option<String>,
    /// Auto-detected table name from user query (for pattern-agnostic templates)
    #[serde(default)]
    pub detected_table: Option<String>,
    pub confidence: f32,
    pub reasoning: String,
}
