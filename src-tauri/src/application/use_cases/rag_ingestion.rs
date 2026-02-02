use crate::application::use_cases::chunking::ChunkEngine;
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::rag_entities::{
    RagDocument, RagDocumentChunkInput, RagDocumentInput, RagExcelDataInput,
};
use crate::infrastructure::db::rag::repository::RagRepository;

use std::path::Path;
use std::sync::Arc;

mod ocr;
mod parsers;
mod structured_rows;
mod types;

pub use types::{DocumentQualityAnalysis, ExtractionQuality, OcrPage, OcrResult, ParsedContent};

/// Result type for parsing: (pages_content, total_pages, excel_data)
/// pages_content: Vec of PageContent for documents with page structure (PDF, DOCX)
/// or plain text for single-page documents (TXT, web)
type ParseResult = Result<(ParsedContent, i64, Option<Vec<Vec<String>>>)>;

use self::structured_rows::{
    build_row_content, redact_row_for_storage, split_header_and_rows, StructuredRowMapping,
};

pub struct RagIngestionUseCase {
    rag_repository: Arc<RagRepository>,
    embedding_service: Arc<EmbeddingService>,
    chunk_engine: ChunkEngine,
}

impl RagIngestionUseCase {
    pub fn new(rag_repository: Arc<RagRepository>, config: LLMConfig) -> Self {
        Self {
            rag_repository,
            embedding_service: Arc::new(EmbeddingService::new(config)),
            chunk_engine: ChunkEngine::default(),
        }
    }

    pub fn with_embedding_service(
        rag_repository: Arc<RagRepository>,
        embedding_service: Arc<EmbeddingService>,
    ) -> Self {
        Self {
            rag_repository,
            embedding_service,
            chunk_engine: ChunkEngine::default(),
        }
    }

    pub async fn ingest_file(
        &self,
        file_path: &str,
        collection_id: Option<i64>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        let path = Path::new(file_path);

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "Starting import: {}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ),
        );

        if !path.exists() {
            add_log(
                &logs,
                "ERROR",
                "RAG",
                &format!("File not found: {}", file_path),
            );
            return Err(AppError::NotFound(format!("File not found: {}", file_path)));
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::ValidationError("Invalid file name".to_string()))?
            .to_string();

        let file_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| AppError::ValidationError("Invalid file extension".to_string()))?
            .to_lowercase();

        let file_type = match file_extension.as_str() {
            "pdf" => "pdf",
            "docx" => "docx",
            "xlsx" => "xlsx",
            "csv" => "csv",
            "txt" => "txt",
            "md" => "md",
            "web" => "web",
            _ => {
                add_log(
                    &logs,
                    "ERROR",
                    "RAG",
                    &format!("Unsupported file type: {}", file_extension),
                );
                return Err(AppError::ValidationError(format!(
                    "Unsupported file type: {}",
                    file_extension
                )));
            }
        };

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsing {} file...", file_type.to_uppercase()),
        );

        let (parsed_content, pages, excel_data) = match file_type {
            "pdf" => self.parse_pdf(file_path, &logs)?,
            "docx" => self.parse_docx(file_path, &logs)?,
            "xlsx" => self.parse_xlsx(file_path, &logs)?,
            "csv" => self.parse_csv(file_path, &logs)?,
            "txt" | "md" => self.parse_txt(file_path, &logs)?,
            "web" => self.parse_web(file_path, &logs).await?,
            _ => unreachable!(),
        };

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsed file: {} pages", pages),
        );

        let document_input = RagDocumentInput {
            collection_id,
            file_name: file_name.clone(),
            file_path: Some(file_path.to_string()),
            file_type: file_type.to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(pages),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");

        let document = self.rag_repository.create_document(&document_input).await?;
        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created with ID: {}", document.id),
        );

        if let Some(excel_rows) = excel_data {
            add_log(
                &logs,
                "INFO",
                "RAG",
                &format!("Storing {} structured rows...", excel_rows.len()),
            );

            // 1) Keep the old excel_data table for backward compatibility.
            // 2) Also populate structured_rows so aggregate/list/count queries are accurate.
            //
            // Mapping strategy (v1):
            // - Treat the first row as header if it looks like headers.
            // - Map category/source/title/created_at by matching header names.

            let (header, data_rows) = split_header_and_rows(&excel_rows);
            let mapping = StructuredRowMapping::from_header(header.as_deref());

            let mut structured_batch: Vec<(
                i64,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
            )> = Vec::new();

            for (row_index, data) in data_rows.iter().enumerate() {
                let redacted_row = redact_row_for_storage(header.as_deref(), data);
                let data_json =
                    serde_json::to_string::<Vec<String>>(&redacted_row).map_err(|e| {
                        AppError::Internal(format!("Failed to serialize row data: {}", e))
                    })?;

                // Legacy excel_data storage (val_a/val_b/val_c) - store redacted cells.
                let excel_input = RagExcelDataInput {
                    doc_id: document.id,
                    row_index: row_index as i64,
                    data_json: Some(data_json.clone()),
                    val_a: redacted_row.get(0).cloned(),
                    val_b: redacted_row.get(1).cloned(),
                    val_c: redacted_row.get(2).and_then(|s| s.parse::<f64>().ok()),
                };

                self.rag_repository
                    .create_excel_data(&excel_input)
                    .await
                    .map_err(|e| {
                        add_log(
                            &logs,
                            "ERROR",
                            "RAG",
                            &format!("Failed to store Excel row {}: {}", row_index + 1, e),
                        );
                        AppError::Internal(format!("Failed to store Excel row: {}", e))
                    })?;

                let extracted = mapping.extract(data);

                // Structured rows: include a readable content string.
                let content = Some(build_row_content(header.as_deref(), data));

                structured_batch.push((
                    row_index as i64,
                    extracted.category,
                    extracted.source,
                    extracted.title,
                    extracted.created_at_text,
                    extracted.created_at,
                    content,
                    data_json,
                ));
            }

            self.rag_repository
                .insert_structured_rows(document.id, structured_batch)
                .await
                .map_err(|e| {
                    add_log(
                        &logs,
                        "ERROR",
                        "RAG",
                        &format!("Failed to store structured rows: {}", e),
                    );
                    AppError::Internal(format!("Failed to store structured rows: {}", e))
                })?;

            add_log(&logs, "INFO", "RAG", "Structured rows stored");
        }

        self.store_chunks_for_document(&document, parsed_content, &file_name, file_type, &logs)
            .await?;

        Ok(document)
    }

    pub async fn ingest_web_html(
        &self,
        url: &str,
        collection_id: Option<i64>,
        max_pages: Option<usize>,
        max_depth: Option<usize>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        let max_pages = max_pages.unwrap_or(10);
        let max_depth = max_depth.unwrap_or(2);

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Starting HTML web import: {}", url),
        );

        let file_name = url
            .split('/')
            .last()
            .and_then(|segment| {
                let trimmed = segment.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .unwrap_or("web")
            .split('?')
            .next()
            .unwrap_or("web")
            .to_string();

        let (parsed_content, pages, _) = self
            .parse_web_with_options(url, &logs, max_pages, max_depth)
            .await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Parsed web content: {} pages", pages),
        );

        let document_input = RagDocumentInput {
            collection_id,
            file_name: file_name.clone(),
            file_path: Some(url.to_string()),
            file_type: "web".to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(pages),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");

        let document = self.rag_repository.create_document(&document_input).await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created with ID: {}", document.id),
        );

        self.store_chunks_for_document(&document, parsed_content, &file_name, "web", &logs)
            .await?;

        Ok(document)
    }

    async fn store_chunks_for_document(
        &self,
        document: &RagDocument,
        parsed_content: ParsedContent,
        file_name: &str,
        file_type: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<()> {
        use crate::interfaces::http::add_log;

        let chunks = match parsed_content {
            ParsedContent::Pages(ref page_contents) => {
                let total_chars: usize = page_contents.iter().map(|p| p.content.len()).sum();
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!(
                        "Chunking {} pages ({} chars total) with page tracking...",
                        page_contents.len(),
                        total_chars
                    ),
                );

                self.chunk_engine
                    .chunk_pages(page_contents)
                    .map_err(|e| AppError::Internal(format!("Failed to chunk pages: {}", e)))?
            }
            ParsedContent::Plain(Some(ref text_content)) => {
                add_log(
                    logs,
                    "INFO",
                    "RAG",
                    &format!("Chunking plain text ({} chars)...", text_content.len()),
                );

                self.chunk_engine
                    .chunk_text(text_content)
                    .map_err(|e| AppError::Internal(format!("Failed to chunk text: {}", e)))?
            }
            ParsedContent::Plain(None) => {
                add_log(
                    logs,
                    "WARN",
                    "RAG",
                    &format!(
                        "No text extracted from {} ({}); embeddings skipped",
                        file_name, file_type
                    ),
                );
                add_log(logs, "INFO", "RAG", "Import completed (no text content)");
                return Ok(());
            }
        };

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Created {} chunks", chunks.len()),
        );

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let page_info = chunk
                .page_number
                .map(|p| format!(" (page {})", p))
                .unwrap_or_default();
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!(
                    "Processing chunk {}/{}{}...",
                    chunk_index + 1,
                    chunks.len(),
                    page_info
                ),
            );

            let chunk_input = RagDocumentChunkInput {
                doc_id: document.id,
                content: chunk.content.clone(),
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                chunk_index: chunk_index as i64,
                token_count: Some(chunk.token_count as i64),
            };

            let created_chunk = self
                .rag_repository
                .create_chunk(&chunk_input)
                .await
                .map_err(|e| {
                    add_log(
                        logs,
                        "ERROR",
                        "RAG",
                        &format!("Failed to store chunk {}: {}", chunk_index + 1, e),
                    );
                    AppError::Internal(format!("Failed to store chunk: {}", e))
                })?;

            // Update chunk quality if available
            if let Some(quality) = chunk.quality_score {
                let _ = self
                    .rag_repository
                    .update_chunk_quality(
                        created_chunk.id,
                        quality as f64,
                        chunk.content_type.as_deref(),
                    )
                    .await;
            }

            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("Generating embedding for chunk {}...", chunk_index + 1),
            );

            match self
                .embedding_service
                .generate_embedding(&chunk.content)
                .await
            {
                Ok(embedding) => {
                    let embedding_bytes = EmbeddingService::embedding_to_bytes(&embedding);
                    self.rag_repository
                        .update_chunk_embedding(created_chunk.id, &embedding_bytes)
                        .await
                        .map_err(|e| {
                            add_log(
                                logs,
                                "ERROR",
                                "RAG",
                                &format!(
                                    "Failed to update chunk embedding {}: {}",
                                    chunk_index + 1,
                                    e
                                ),
                            );
                            AppError::Internal(format!("Failed to update chunk embedding: {}", e))
                        })?;
                    add_log(
                        logs,
                        "INFO",
                        "RAG",
                        &format!("Chunk {}/{} processed", chunk_index + 1, chunks.len()),
                    );
                }
                Err(e) => {
                    add_log(
                        logs,
                        "ERROR",
                        "RAG",
                        &format!(
                            "Failed to generate embedding for chunk {}: {}",
                            chunk_index + 1,
                            e
                        ),
                    );
                }
            }
        }

        add_log(logs, "INFO", "RAG", "All chunks processed successfully");

        // Compute and store document quality metrics
        add_log(logs, "INFO", "RAG", "Computing document quality metrics...");

        // Calculate average chunk quality
        let chunk_count = chunks.len() as i64;
        let total_quality: f64 = chunks
            .iter()
            .filter_map(|c| c.quality_score.map(|q| q as f64))
            .sum();
        let avg_quality = if chunk_count > 0 && total_quality > 0.0 {
            Some(total_quality / chunk_count as f64)
        } else {
            None
        };

        // Update document with quality metrics
        self.rag_repository
            .update_document_quality(document.id, avg_quality, None, chunk_count, 0)
            .await
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to update document quality: {}", e),
                );
                AppError::Internal(format!("Failed to update document quality: {}", e))
            })?;

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Document quality: {:.2}", avg_quality.unwrap_or(0.0)),
        );

        add_log(logs, "INFO", "RAG", "Import completed successfully");

        Ok(())
    }

    // ============================================================
    // DOCUMENT QUALITY ANALYSIS
    // ============================================================

    /// Analyze document quality after ingestion
    /// Returns quality metrics for the document and its chunks
    pub async fn analyze_document_quality(
        &self,
        document_id: i64,
    ) -> Result<DocumentQualityAnalysis> {
        // Get document info
        let doc = self.rag_repository.get_document(document_id).await?;

        // Get all chunks for the document (use a high limit to get all)
        let chunks = self.rag_repository.get_chunks(document_id, 10000).await?;

        if chunks.is_empty() {
            return Ok(DocumentQualityAnalysis {
                document_id,
                document_name: doc.file_name,
                total_chunks: 0,
                avg_chunk_quality: 0.0,
                min_chunk_quality: 0.0,
                max_chunk_quality: 0.0,
                low_quality_chunk_count: 0,
                avg_chunk_length: 0,
                total_tokens: 0,
                extraction_quality: ExtractionQuality::Unknown,
                issues: vec!["No chunks created".to_string()],
            });
        }

        // Calculate quality metrics
        let mut total_quality = 0.0;
        let mut min_quality = 1.0f32;
        let mut max_quality = 0.0f32;
        let mut low_quality_count = 0;
        let mut total_length = 0;
        let mut total_tokens = 0i64;

        for chunk in &chunks {
            // Estimate quality based on content characteristics
            let quality = self.estimate_chunk_quality(&chunk.content);
            total_quality += quality;
            if quality < min_quality {
                min_quality = quality;
            }
            if quality > max_quality {
                max_quality = quality;
            }
            if quality < 0.5 {
                low_quality_count += 1;
            }
            total_length += chunk.content.len();
            total_tokens += chunk.token_count.unwrap_or(0);
        }

        let avg_quality = total_quality / chunks.len() as f32;
        let avg_length = total_length / chunks.len();

        // Determine extraction quality
        let extraction_quality = if avg_quality >= 0.8 {
            ExtractionQuality::Excellent
        } else if avg_quality >= 0.6 {
            ExtractionQuality::Good
        } else if avg_quality >= 0.4 {
            ExtractionQuality::Fair
        } else {
            ExtractionQuality::Poor
        };

        // Identify issues
        let mut issues = Vec::new();
        if low_quality_count > chunks.len() / 3 {
            issues.push(format!(
                "{}% of chunks have low quality (< 0.5)",
                low_quality_count * 100 / chunks.len()
            ));
        }
        if avg_length < 100 {
            issues.push("Average chunk length is very short".to_string());
        }
        if avg_length > 2000 {
            issues.push("Average chunk length is very long".to_string());
        }

        Ok(DocumentQualityAnalysis {
            document_id,
            document_name: doc.file_name,
            total_chunks: chunks.len(),
            avg_chunk_quality: avg_quality,
            min_chunk_quality: min_quality,
            max_chunk_quality: max_quality,
            low_quality_chunk_count: low_quality_count,
            avg_chunk_length: avg_length,
            total_tokens: total_tokens as usize,
            extraction_quality,
            issues,
        })
    }

    /// Estimate quality of a chunk based on content characteristics
    fn estimate_chunk_quality(&self, content: &str) -> f32 {
        let mut score = 1.0f32;

        // Check content length
        if content.len() < 50 {
            score *= 0.5;
        } else if content.len() < 100 {
            score *= 0.7;
        }

        // Check for meaningful content (not just numbers/symbols)
        let alpha_ratio = content.chars().filter(|c| c.is_alphabetic()).count() as f32
            / content.len().max(1) as f32;
        if alpha_ratio < 0.3 {
            score *= 0.6;
        }

        // Check for OCR noise patterns (repeated chars, unusual sequences)
        let has_noise =
            content.contains("|||") || content.contains("___") || content.contains("...");
        if has_noise {
            score *= 0.7;
        }

        // Check for sentence structure (has periods, question marks, etc.)
        let has_sentence_end =
            content.contains('.') || content.contains('?') || content.contains('!');
        if !has_sentence_end && content.len() > 100 {
            score *= 0.8;
        }

        // Check for proper capitalization
        let first_char_caps = content
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        if first_char_caps {
            score *= 1.05;
        }

        // Check for excessive whitespace
        let whitespace_ratio = content.chars().filter(|c| c.is_whitespace()).count() as f32
            / content.len().max(1) as f32;
        if whitespace_ratio > 0.5 {
            score *= 0.7;
        }

        score.clamp(0.0, 1.0)
    }
}
