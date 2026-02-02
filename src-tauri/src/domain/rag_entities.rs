use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Collection kind for routing between file-based and DB-based RAG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollectionKind {
    Files,
    Db,
}

impl Default for CollectionKind {
    fn default() -> Self {
        CollectionKind::Files
    }
}

impl From<CollectionKind> for String {
    fn from(kind: CollectionKind) -> String {
        match kind {
            CollectionKind::Files => "files".to_string(),
            CollectionKind::Db => "db".to_string(),
        }
    }
}

impl From<String> for CollectionKind {
    fn from(s: String) -> Self {
        match s.as_str() {
            "db" => CollectionKind::Db,
            _ => CollectionKind::Files,
        }
    }
}

impl From<&str> for CollectionKind {
    fn from(s: &str) -> Self {
        match s {
            "db" => CollectionKind::Db,
            _ => CollectionKind::Files,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagCollection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub kind: CollectionKind,
    pub config_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagDocument {
    pub id: i64,
    pub collection_id: Option<i64>,
    pub file_name: String,
    pub file_path: Option<String>,
    pub file_type: String,
    pub language: String,
    pub total_pages: i64,
    pub quality_score: Option<f64>, // Overall document quality (0.0-1.0)
    pub ocr_confidence: Option<f64>, // Average OCR confidence (0.0-1.0)
    pub chunk_count: i64,           // Total number of chunks
    pub warning_count: i64,         // Number of quality warnings
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagDocumentChunk {
    pub id: i64,
    pub doc_id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub chunk_index: i64,
    pub token_count: Option<i64>,
    pub chunk_quality: Option<f64>,   // Chunk quality score (0.0-1.0)
    pub content_type: Option<String>, // Detected content type
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagExcelData {
    pub id: i64,
    pub doc_id: i64,
    pub row_index: i64,
    pub data_json: Option<String>,
    pub val_a: Option<String>,
    pub val_b: Option<String>,
    pub val_c: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RagCollectionInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RagDocumentInput {
    pub collection_id: Option<i64>,
    pub file_name: String,
    pub file_path: Option<String>,
    pub file_type: String,
    pub language: Option<String>,
    pub total_pages: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RagDocumentChunkInput {
    pub doc_id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub page_offset: Option<i64>,
    pub chunk_index: i64,
    pub token_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RagExcelDataInput {
    pub doc_id: i64,
    pub row_index: i64,
    pub data_json: Option<String>,
    pub val_a: Option<String>,
    pub val_b: Option<String>,
    pub val_c: Option<f64>,
}

// ============================================================
// QUALITY ANALYTICS ENTITIES
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentWarning {
    pub id: i64,
    pub doc_id: i64,
    pub warning_type: String, // ocr_low_confidence, table_structure_lost, short_chunk, etc.
    pub page_number: Option<i64>,
    pub chunk_index: Option<i64>,
    pub severity: String,           // info, warning, error
    pub message: String,            // Human-readable message
    pub suggestion: Option<String>, // Actionable suggestion
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentWarningInput {
    pub doc_id: i64,
    pub warning_type: String,
    pub page_number: Option<i64>,
    pub chunk_index: Option<i64>,
    pub severity: String,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectionQualityMetrics {
    pub id: i64,
    pub collection_id: i64,
    pub computed_at: DateTime<Utc>,
    pub avg_quality_score: Option<f64>,
    pub avg_ocr_confidence: Option<f64>,
    pub total_documents: i64,
    pub documents_with_warnings: i64,
    pub total_chunks: i64,
    pub avg_chunk_quality: Option<f64>,
    pub best_reranker: Option<String>,
    pub reranker_score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetrievalGap {
    pub id: i64,
    pub collection_id: i64,
    pub query_hash: String,
    pub query_length: Option<i64>,
    pub result_count: Option<i64>,
    pub max_confidence: Option<f64>,
    pub avg_confidence: Option<f64>,
    pub gap_type: Option<String>, // no_results, low_confidence, partial_match
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RetrievalGapInput {
    pub collection_id: i64,
    pub query_hash: String,
    pub query_length: Option<i64>,
    pub result_count: Option<i64>,
    pub max_confidence: Option<f64>,
    pub avg_confidence: Option<f64>,
    pub gap_type: Option<String>,
}

// ============================================================
// DB CONNECTOR ENTITIES
// ============================================================

/// Database connection configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbConnection {
    pub id: i64,
    pub name: String,
    pub db_type: String, // postgres | sqlite
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub password_ref: Option<String>, // Reference to secure storage
    pub ssl_mode: String,
    pub is_enabled: bool,
    pub config_json: Option<String>, // JSON configuration for selected tables/columns
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new DB connection
#[derive(Debug, Deserialize)]
pub struct DbConnectionInput {
    pub name: String,
    pub db_type: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub password: String, // Will be stored as reference
    pub ssl_mode: Option<String>,
}

/// Allowlist profile defining security boundaries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbAllowlistProfile {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub rules_json: String,
    pub created_at: DateTime<Utc>,
}

/// Query template for few-shot learning (Feature 31)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplate {
    pub id: i64,
    pub allowlist_profile_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub intent_keywords: Vec<String>, // Parsed from JSON
    pub example_question: String,
    pub query_pattern: String,
    pub pattern_type: String, // 'select_where_in' | 'select_where_eq' | 'select_with_join' | 'aggregate' | 'custom'
    pub tables_used: Vec<String>, // Parsed from JSON
    pub priority: i32,
    pub is_enabled: bool,
    pub is_pattern_agnostic: bool, // true = pattern works across any table (abstract SQL pattern), false = table-specific
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating/updating query templates
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplateInput {
    pub allowlist_profile_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub intent_keywords: Vec<String>,
    pub example_question: String,
    pub query_pattern: String,
    pub pattern_type: String,
    pub tables_used: Vec<String>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub is_pattern_agnostic: Option<bool>, // Optional in input, defaults to false
}

// ============================================================
// QUERY TEMPLATE IMPORT (Preview + Selective Import)
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplateDuplicateInfo {
    pub kind: String, // exact | name | pattern
    pub existing_template_id: i64,
    pub existing_template_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplateImportPreviewItem {
    pub key: String,
    pub original_allowlist_profile_id: i64,
    pub template: QueryTemplateInput,
    pub issues: Vec<String>,
    pub duplicate: Option<QueryTemplateDuplicateInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplateImportPreview {
    pub file_path: String,
    pub target_profile_id: i64,
    pub statement_count: i64,
    pub parsed_count: i64,
    pub ok_count: i64,
    pub warning_count: i64,
    pub error_count: i64,
    pub duplicate_count: i64,
    pub statement_errors: Vec<String>,
    pub items: Vec<QueryTemplateImportPreviewItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryTemplateImportResult {
    pub requested: i64,
    pub imported: i64,
    pub skipped_duplicates: i64,
}

/// Information about a database table
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableInfo {
    pub table_name: String,
    pub table_schema: Option<String>, // For PostgreSQL: schema name
    pub row_count: Option<i64>,       // Optional: approximate row count
}

/// Information about a table column
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub position: i32,
}

/// Configuration for DB collections
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbCollectionConfig {
    pub db_conn_id: i64,
    pub allowlist_profile_id: i64,
    pub selected_tables: Vec<String>,
    pub default_limit: i32,
    pub max_limit: i32,
    pub external_llm_policy: String,
}

/// SQL-RAG query plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub mode: String, // exact | list | aggregate
    pub table: String,
    pub select: Vec<String>,
    pub filters: Vec<QueryFilter>,
    pub limit: i32,
    pub order_by: Option<OrderBy>,
    pub joins: Option<Vec<Join>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub column: String,
    pub operator: String, // eq | in | gte | lte | between | like
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub column: String,
    pub direction: String, // asc | desc
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Join {
    pub table: String,
    pub on_column: String,
    pub join_type: String, // inner | left
}

/// Dynamic configuration stored in DbConnection.config_json
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DbConnectionConfig {
    pub profile_id: Option<i64>,
    pub selected_tables: Vec<String>,
    pub selected_columns: std::collections::HashMap<String, Vec<String>>,
    pub default_limit: Option<i32>,
    pub updated_at: Option<String>,
}

/// SQL-RAG response
#[derive(Debug, Serialize)]
pub struct SqlRagResponse {
    pub answer: String,
    pub citations: Vec<DbCitation>,
    pub telemetry: QueryTelemetry,
}

#[derive(Debug, Serialize)]
pub struct DbCitation {
    pub table_name: String,
    pub row_id: String,
    pub columns: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct QueryTelemetry {
    pub row_count: usize,
    pub latency_ms: i64,
    pub llm_route: String, // local | external | blocked
}

/// Test connection result
#[derive(Debug, Serialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
}
