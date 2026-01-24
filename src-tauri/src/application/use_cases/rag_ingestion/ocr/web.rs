use std::path::PathBuf;

use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::web_crawler::WebOcrCapture;

use super::super::{
    AppError, RagDocument, RagDocumentChunkInput, RagDocumentInput, RagIngestionUseCase, Result,
};

impl RagIngestionUseCase {
    /// Ingest a web page using screenshot OCR mode (Playwright + Tesseract).
    pub async fn ingest_web_ocr(
        &self,
        url: &str,
        collection_id: Option<i64>,
        logs: std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> Result<RagDocument> {
        use crate::interfaces::http::add_log;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Starting OCR web capture: {}", url),
        );

        let script_path = self.get_playwright_script_path()?;
        let temp_dir = std::env::temp_dir().join("gadogado_web_ocr");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| AppError::Internal(format!("Failed to create temp directory: {}", e)))?;

        let ocr_capture = WebOcrCapture::new(script_path, temp_dir);
        let result = ocr_capture.capture_url(url, logs.clone()).await?;

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "OCR capture complete: {} - {} characters",
                result.title,
                result.content.len()
            ),
        );

        let document_input = RagDocumentInput {
            collection_id,
            file_name: result.title.clone(),
            file_path: Some(url.to_string()),
            file_type: "web_ocr".to_string(),
            language: Some("auto".to_string()),
            total_pages: Some(result.manifest.tiles.len() as i64),
        };

        add_log(&logs, "INFO", "RAG", "Creating document record...");
        let document = self.rag_repository.create_document(&document_input).await?;
        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Document created: ID {}", document.id),
        );

        let chunks = self.chunk_engine.chunk_text(&result.content)?;
        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!("Created {} chunks", chunks.len()),
        );
        add_log(&logs, "INFO", "RAG", "Generating embeddings for chunks...");

        let total_chunks = chunks.len();
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let embedding = self
                .embedding_service
                .generate_embedding(&chunk.content)
                .await
                .map_err(|e| {
                    add_log(
                        &logs,
                        "WARN",
                        "RAG",
                        &format!(
                            "Failed to generate embedding for chunk {}: {}",
                            chunk_index, e
                        ),
                    );
                    e
                })?;

            let chunk_input = RagDocumentChunkInput {
                doc_id: document.id,
                content: chunk.content.clone(),
                page_number: chunk.page_number,
                page_offset: chunk.page_offset,
                chunk_index: chunk_index as i64,
                token_count: Some(chunk.token_count as i64),
            };

            let created_chunk = self.rag_repository.create_chunk(&chunk_input).await?;
            let embedding_bytes = EmbeddingService::embedding_to_bytes(&embedding);
            self.rag_repository
                .update_chunk_embedding(created_chunk.id, &embedding_bytes)
                .await?;

            if (chunk_index + 1) % 10 == 0 || chunk_index + 1 == total_chunks {
                add_log(
                    &logs,
                    "INFO",
                    "RAG",
                    &format!("Processed {}/{} chunks", chunk_index + 1, total_chunks),
                );
            }
        }

        if let Err(e) = ocr_capture.cleanup(&result.output_dir) {
            add_log(
                &logs,
                "WARN",
                "RAG",
                &format!("Failed to clean up temp files: {}", e),
            );
        }

        add_log(
            &logs,
            "INFO",
            "RAG",
            &format!(
                "Web OCR import complete: {} chunks from {}",
                total_chunks, url
            ),
        );

        Ok(document)
    }

    fn get_playwright_script_path(&self) -> Result<PathBuf> {
        let possible_paths = vec![
            PathBuf::from("resources/scripts/playwright-capture.js"),
            PathBuf::from("../resources/scripts/playwright-capture.js"),
            std::env::current_dir()
                .unwrap_or_default()
                .join("resources/scripts/playwright-capture.js"),
        ];

        for path in &possible_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bundled_path = exe_dir.join("resources/scripts/playwright-capture.js");
                if bundled_path.exists() {
                    return Ok(bundled_path);
                }
            }
        }

        Err(AppError::Internal(
            "Playwright capture script not found. Ensure playwright-capture.js is in resources/scripts/".to_string(),
        ))
    }
}
