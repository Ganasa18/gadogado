use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagCollection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
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
