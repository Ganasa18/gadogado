use super::super::{AppError, ParseResult, ParsedContent, RagIngestionUseCase};

use crate::application::use_cases::chunking::PageContent;

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) fn parse_pdf(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;
        use lopdf::Document;

        let document = Document::load(file_path)
            .map_err(|e| AppError::Internal(format!("Failed to load PDF: {}", e)))?;

        let mut page_contents: Vec<PageContent> = Vec::new();
        let mut total_pages = 0i64;
        let mut has_text = false;

        // Extract text per page to preserve page boundaries
        for (page_num, (page_id, _)) in document.get_pages() {
            total_pages += 1;
            match document.extract_text(&[page_id]) {
                Ok(page_text) => {
                    let trimmed = page_text.trim();
                    if !trimmed.is_empty() {
                        has_text = true;
                        page_contents.push(PageContent {
                            page_number: page_num as i64,
                            content: trimmed.to_string(),
                        });
                    }
                }
                Err(_) => {}
            }
        }

        // If no text extracted (scanned PDF), fall back to OCR
        if !has_text {
            add_log(logs, "INFO", "RAG", "No text layer found, running OCR...");
            if let Some(ocr_pages) = self.ocr_pdf_with_grayscale(file_path, logs) {
                return Ok((ParsedContent::Pages(ocr_pages), total_pages, None));
            }
        }

        if page_contents.is_empty() {
            Ok((ParsedContent::Plain(None), total_pages, None))
        } else {
            Ok((ParsedContent::Pages(page_contents), total_pages, None))
        }
    }
}
