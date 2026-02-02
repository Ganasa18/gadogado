use crate::application::use_cases::chunking::PageContent;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ParsedContent {
    /// Content with page structure (PDF, DOCX with page breaks)
    Pages(Vec<PageContent>),
    /// Plain text without page structure (TXT, web, single-page docs)
    Plain(Option<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPage {
    pub page_number: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub pages: Option<Vec<OcrPage>>,
    pub total_pages: usize,
    pub engine: String,
    pub preprocessing_mode: String,
    pub preprocessing_enabled: bool,
}

/// Document quality analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentQualityAnalysis {
    pub document_id: i64,
    pub document_name: String,
    pub total_chunks: usize,
    pub avg_chunk_quality: f32,
    pub min_chunk_quality: f32,
    pub max_chunk_quality: f32,
    pub low_quality_chunk_count: usize,
    pub avg_chunk_length: usize,
    pub total_tokens: usize,
    pub extraction_quality: ExtractionQuality,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExtractionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Unknown,
}
