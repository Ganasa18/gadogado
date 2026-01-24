use super::super::{AppError, ParseResult, ParsedContent, RagIngestionUseCase};

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) fn parse_txt(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        let text = std::fs::read_to_string(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to read TXT file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to read TXT file: {}", e))
        })?;

        // TXT files are single-page, use Plain content type
        let content = if text.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(text.trim().to_string()))
        };

        Ok((content, 1, None))
    }
}
