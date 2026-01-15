use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagDocumentChunk {
    pub id: i64,
    pub doc_id: i64,
    pub content: String,
    pub page_number: Option<i64>,
    pub chunk_index: i64,
    pub token_count: Option<i64>,
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
