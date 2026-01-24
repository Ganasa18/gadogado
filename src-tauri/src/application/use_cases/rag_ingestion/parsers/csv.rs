use super::super::{AppError, ParseResult, ParsedContent, RagIngestionUseCase};

use crate::application::use_cases::csv_preprocessor::CsvPreprocessor;

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) fn parse_csv(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;

        add_log(logs, "INFO", "RAG", "Preprocessing CSV file...");

        // Use the new CSV preprocessor
        let preprocessor = CsvPreprocessor::default();

        // Read file content for preprocessing
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to read CSV file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to read CSV file: {}", e))
        })?;

        // Preprocess the CSV
        let preprocessed = preprocessor.preprocess_csv_content(&content).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to preprocess CSV: {}", e),
            );
            AppError::Internal(format!("Failed to preprocess CSV: {}", e))
        })?;

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!(
                "CSV preprocessing complete:\n\
                 - Content type: {:?}\n\
                 - Rows processed: {}\n\
                 - Avg field length: {:.1} chars\n\
                 - Lexical diversity: {:.2}\n\
                 - Confidence: {:.2}",
                preprocessed.content_type,
                preprocessed.row_count,
                preprocessed.analysis.avg_value_length,
                preprocessed.analysis.lexical_diversity,
                preprocessed.analysis.confidence_score()
            ),
        );

        // Parse rows for Excel data storage
        let rows = self.parse_csv_rows_for_storage(&content);

        // Use the preprocessed text for embedding
        let parsed_content = if preprocessed.processed_text.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(preprocessed.processed_text))
        };

        Ok((parsed_content, 1, Some(rows)))
    }

    /// Parse CSV rows for storage in excel_data table
    /// This preserves the original CSV structure for structured queries
    fn parse_csv_rows_for_storage(&self, content: &str) -> Vec<Vec<String>> {
        use crate::infrastructure::csv::CsvParser;
        use csv::{ReaderBuilder, Trim};

        let delimiter = CsvParser::detect_delimiter(content);
        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .trim(Trim::All)
            .flexible(true)
            .from_reader(content.as_bytes());

        let mut out: Vec<Vec<String>> = Vec::new();

        for result in reader.records() {
            if let Ok(record) = result {
                let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                if row.iter().all(|s| s.trim().is_empty()) {
                    continue;
                }
                out.push(row);
            }
        }

        out
    }

    /// Parse a single CSV line, handling quoted fields
    #[allow(dead_code)]
    fn parse_csv_line(&self, line: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes {
                        // Check for escaped quote ("")
                        if chars.peek() == Some(&'"') {
                            current_field.push('"');
                            chars.next();
                        } else {
                            in_quotes = false;
                        }
                    } else {
                        in_quotes = true;
                    }
                }
                ',' if !in_quotes => {
                    result.push(current_field.trim().to_string());
                    current_field = String::new();
                }
                _ => {
                    current_field.push(c);
                }
            }
        }

        // Don't forget the last field
        result.push(current_field.trim().to_string());
        result
    }
}
