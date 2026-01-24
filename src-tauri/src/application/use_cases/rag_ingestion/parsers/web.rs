use super::super::{AppError, ParsedContent, ParseResult, RagIngestionUseCase};

use crate::application::use_cases::web_crawler::WebCrawler;

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) async fn parse_web(
        &self,
        url: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        self.parse_web_with_options(url, logs, 10, 2).await
    }

    pub(in crate::application::use_cases::rag_ingestion) async fn parse_web_with_options(
        &self,
        url: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
        max_pages: usize,
        max_depth: usize,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        let crawler = WebCrawler::new(max_pages, max_depth);

        add_log(logs, "INFO", "RAG", &format!("Crawling web site: {}", url));

        let pages = crawler
            .crawl_site(url, std::sync::Arc::clone(logs))
            .await
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to crawl site: {}", e),
                );
                AppError::Internal(format!("Failed to crawl site: {}", e))
            })?;

        let mut combined_content = String::new();
        for page in &pages {
            combined_content.push_str(&format!("\n--- Page: {} ---\n", page.url));
            combined_content.push_str(&format!("Title: {}\n", page.title));
            combined_content.push_str(&format!("Content:\n{}\n", page.content));
        }

        // Web pages are treated as plain text (no page structure)
        let content = if combined_content.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(combined_content.trim().to_string()))
        };

        Ok((content, pages.len() as i64, None))
    }
}
