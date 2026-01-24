use super::super::{AppError, ParseResult, ParsedContent, RagIngestionUseCase};

use crate::application::use_cases::csv_preprocessor::CsvPreprocessor;

impl RagIngestionUseCase {
    pub(in crate::application::use_cases::rag_ingestion) fn parse_xlsx(
        &self,
        file_path: &str,
        logs: &std::sync::Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    ) -> ParseResult {
        use crate::interfaces::http::add_log;
        use calamine::{open_workbook, DataType, Reader, Xlsx};

        add_log(logs, "INFO", "RAG", "Parsing XLSX file...");

        let mut workbook: Xlsx<_> = open_workbook(file_path).map_err(|e| {
            add_log(
                logs,
                "ERROR",
                "RAG",
                &format!("Failed to open Excel file {}: {}", file_path, e),
            );
            AppError::Internal(format!("Failed to open Excel file: {}", e))
        })?;

        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| {
                add_log(logs, "ERROR", "RAG", "No worksheet found in Excel file");
                AppError::Internal("No worksheet found".to_string())
            })?
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to read Excel range {}: {}", file_path, e),
                );
                AppError::Internal(format!("Failed to read Excel range: {}", e))
            })?;

        // Convert Excel data to CSV format for preprocessing
        add_log(
            logs,
            "INFO",
            "RAG",
            "Converting Excel to CSV for smart preprocessing...",
        );

        let mut csv_lines = Vec::new();
        let mut rows = Vec::new();

        for row in range.rows() {
            let row_data: Vec<String> = row
                .iter()
                .map(|cell| {
                    cell.as_string()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}", cell))
                })
                .collect();

            // Convert to CSV format
            let csv_line = row_data
                .iter()
                .map(|field| {
                    // Escape fields containing commas or quotes
                    let field = field.replace('"', "\"\"");
                    if field.contains(',') || field.contains('"') || field.contains('\n') {
                        format!("\"{}\"", field)
                    } else {
                        field.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(",");

            csv_lines.push(csv_line);
            rows.push(row_data);
        }

        // Join all CSV lines
        let csv_content = csv_lines.join("\n");

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Excel converted to CSV format ({} rows)", rows.len()),
        );

        // Use the CSV preprocessor to detect content type and format
        let preprocessor = CsvPreprocessor::default();

        add_log(
            logs,
            "INFO",
            "RAG",
            "Running smart content detection on Excel data...",
        );

        let preprocessed = preprocessor
            .preprocess_csv_content(&csv_content)
            .map_err(|e| {
                add_log(
                    logs,
                    "ERROR",
                    "RAG",
                    &format!("Failed to preprocess Excel data: {}", e),
                );
                AppError::Internal(format!("Failed to preprocess Excel data: {}", e))
            })?;

        add_log(
            logs,
            "INFO",
            "RAG",
            &format!(
                "Excel preprocessing complete:\n\
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

        // Parse rows for Excel data storage (preserve original structure)
        let excel_rows = self.parse_excel_rows_for_storage(&range);

        // Use the preprocessed text for embedding
        let parsed_content = if preprocessed.processed_text.trim().is_empty() {
            ParsedContent::Plain(None)
        } else {
            ParsedContent::Plain(Some(preprocessed.processed_text))
        };

        Ok((parsed_content, 1, Some(excel_rows)))
    }

    /// Parse Excel rows for storage in excel_data table
    /// This preserves the original Excel structure for structured queries
    fn parse_excel_rows_for_storage(
        &self,
        range: &calamine::Range<calamine::Data>,
    ) -> Vec<Vec<String>> {
        use calamine::DataType;
        let mut rows = Vec::new();

        for row in range.rows() {
            let row_data: Vec<String> = row
                .iter()
                .map(|cell| {
                    cell.as_string()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}", cell))
                })
                .collect();
            rows.push(row_data);
        }

        rows
    }
}
