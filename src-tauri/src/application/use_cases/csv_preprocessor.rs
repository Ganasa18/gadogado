// ============================================================
// CSV PREPROCESSOR USE CASE
// ============================================================
// Orchestrate CSV parsing, content detection, and formatting

use std::path::Path;
use std::time::Instant;

use crate::domain::csv::{ContentType, CsvRow, PreprocessedCsv, PreprocessingConfig};
use crate::domain::error::AppError;
use crate::infrastructure::csv::{ContentAnalyzer, CsvParser};

/// CSV preprocessing use case
pub struct CsvPreprocessor {
    config: PreprocessingConfig,
}

impl CsvPreprocessor {
    /// Create a new CSV preprocessor
    pub fn new(config: PreprocessingConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(PreprocessingConfig::default())
    }

    /// Process a CSV file and return formatted text
    pub async fn preprocess_csv(&self, csv_path: &Path) -> Result<PreprocessedCsv, AppError> {
        let start = Instant::now();

        // Validate configuration
        self.config.validate().map_err(|e| {
            AppError::ValidationError(format!("Invalid preprocessing config: {}", e))
        })?;

        // Parse CSV file
        let parser = CsvParser::parse_file_auto_detect(csv_path)
            .map_err(|e| AppError::ParseError(format!("Failed to parse CSV file: {}", e)))?;

        // Check minimum row count
        if parser.len() < self.config.min_sample_rows {
            return Err(AppError::ValidationError(format!(
                "CSV file has too few rows ({}), minimum required: {}",
                parser.len(),
                self.config.min_sample_rows
            )));
        }

        // Detect content type
        let analyzer = ContentAnalyzer::new(self.config.clone());
        let content_type = analyzer.detect_content_type(&parser);
        let analysis = analyzer.analyze(&parser);

        // Extract headers
        let headers = if let Some(first_row) = parser.first() {
            first_row.fields.iter().map(|f| f.name.clone()).collect()
        } else {
            Vec::new()
        };

        // Format based on content type
        let processed_text = match content_type {
            ContentType::Narrative => self.format_narrative(&parser),
            ContentType::Structured => self.format_structured(&parser),
        };

        let processing_time = start.elapsed();

        Ok(PreprocessedCsv {
            content_type,
            processed_text,
            row_count: parser.len(),
            analysis,
            headers,
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }

    /// Format CSV rows as narrative text (plain text style)
    fn format_narrative(&self, rows: &[CsvRow]) -> String {
        rows.iter()
            .map(|row| row.format_narrative())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n---\n")
    }

    /// Format CSV rows as structured text
    /// Uses keyword-dense format optimized for semantic search
    /// Important keywords like "email" are repeated for better retrieval
    fn format_structured(&self, rows: &[CsvRow]) -> String {
        rows.iter()
            .enumerate()
            .map(|(_idx, row)| {
                let mut parts: Vec<String> = Vec::new();
                let mut email_parts: Vec<String> = Vec::new();

                // Extract and emphasize important fields
                for field in row.fields.iter().filter(|f| !f.value.is_empty()) {
                    let clean_name_lower = field.clean_name.to_lowercase();

                    if clean_name_lower.contains("email") || clean_name_lower.contains("mail") {
                        // Repeat email keyword multiple times for better matching
                        parts.push(format!("email: {}", field.value));
                        parts.push(format!("{} email address", field.value));
                        email_parts.push(field.value.clone());
                    } else if clean_name_lower.contains("name") {
                        parts.push(format!("name: {}", field.value));
                    } else if clean_name_lower.contains("id") {
                        parts.push(format!("ID: {}", field.value));
                    } else {
                        parts.push(format!("{}: {}", field.clean_name, field.value));
                    }
                }

                if parts.is_empty() {
                    String::new()
                } else {
                    // Join all parts with clear separators
                    parts.join(". ")
                }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n\n") // Triple newline for clear record boundaries
    }

    /// Process CSV from string content (for testing or in-memory data)
    pub fn preprocess_csv_content(&self, content: &str) -> Result<PreprocessedCsv, AppError> {
        let start = Instant::now();

        // Parse CSV content
        let delimiter = CsvParser::detect_delimiter(content);
        let parser = CsvParser::new().with_delimiter(delimiter);
        let rows = parser
            .parse_content(content)
            .map_err(|e| AppError::ParseError(format!("Failed to parse CSV content: {}", e)))?;

        // Check minimum row count
        if rows.len() < self.config.min_sample_rows {
            return Err(AppError::ValidationError(format!(
                "CSV content has too few rows ({}), minimum required: {}",
                rows.len(),
                self.config.min_sample_rows
            )));
        }

        // Detect content type
        let analyzer = ContentAnalyzer::new(self.config.clone());
        let content_type = analyzer.detect_content_type(&rows);
        let analysis = analyzer.analyze(&rows);

        // Extract headers
        let headers = if let Some(first_row) = rows.first() {
            first_row.fields.iter().map(|f| f.name.clone()).collect()
        } else {
            Vec::new()
        };

        // Format based on content type
        let processed_text = match content_type {
            ContentType::Narrative => self.format_narrative(&rows),
            ContentType::Structured => self.format_structured(&rows),
        };

        let processing_time = start.elapsed();

        Ok(PreprocessedCsv {
            content_type,
            processed_text,
            row_count: rows.len(),
            analysis,
            headers,
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }

    /// Get analysis report without full preprocessing
    pub fn analyze_csv(&self, content: &str) -> Result<String, AppError> {
        let delimiter = CsvParser::detect_delimiter(content);
        let parser = CsvParser::new().with_delimiter(delimiter);
        let rows = parser
            .parse_content(content)
            .map_err(|e| AppError::ParseError(format!("Failed to parse CSV content: {}", e)))?;

        let analyzer = ContentAnalyzer::new(self.config.clone());
        Ok(analyzer.get_analysis_report(&rows))
    }

    /// Preview first N rows in processed format
    pub fn preview_rows(
        &self,
        content: &str,
        preview_count: usize,
    ) -> Result<Vec<String>, AppError> {
        let delimiter = CsvParser::detect_delimiter(content);
        let parser = CsvParser::new().with_delimiter(delimiter);
        let rows = parser
            .parse_content(content)
            .map_err(|e| AppError::ParseError(format!("Failed to parse CSV content: {}", e)))?;

        let analyzer = ContentAnalyzer::new(self.config.clone());
        let content_type = analyzer.detect_content_type(&rows);

        let preview: Vec<String> = rows
            .iter()
            .take(preview_count)
            .map(|row| match content_type {
                ContentType::Narrative => row.format_narrative(),
                ContentType::Structured => {
                    let formatted = row.format_structured();
                    format!("## Record #{}\n{}", row.index + 1, formatted)
                }
            })
            .collect();

        Ok(preview)
    }
}

impl Default for CsvPreprocessor {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NARRATIVE_CSV: &str = "\
title,author,content,category
Machine Learning Basics,Jane Doe,Machine learning is a subset of artificial intelligence that focuses on building systems that can learn from data,Technology
Introduction to Neural Networks,John Smith,Neural networks are computing systems inspired by biological neural networks that form the basis of deep learning,Education
Data Science Best Practices,Alice Johnson,Data science combines statistics, programming, and domain knowledge to extract insights from data,Technology";

    const STRUCTURED_CSV: &str = "\
employee_id,first_name,last_name,department,salary
1001,Alice,Johnson,Sales,75000
1002,Bob,Smith,Engineering,85000
1003,Carol,Williams,Marketing,70000";

    #[test]
    fn test_detect_narrative_csv() {
        let preprocessor = CsvPreprocessor::default();
        let result = preprocessor.preprocess_csv_content(NARRATIVE_CSV).unwrap();

        assert_eq!(result.content_type, ContentType::Narrative);
        assert!(result.processed_text.contains("title:"));
        assert!(result.processed_text.contains("---"));
    }

    #[test]
    fn test_detect_structured_csv() {
        let preprocessor = CsvPreprocessor::default();
        let result = preprocessor.preprocess_csv_content(STRUCTURED_CSV).unwrap();

        assert_eq!(result.content_type, ContentType::Structured);
        assert!(result.processed_text.contains("## Record #"));
        assert!(result.processed_text.contains("- employee_id:"));
    }

    #[test]
    fn test_narrative_formatting() {
        let preprocessor = CsvPreprocessor::default();
        let result = preprocessor.preprocess_csv_content(NARRATIVE_CSV).unwrap();

        // Check for narrative format elements
        assert!(result
            .processed_text
            .contains("title: Machine Learning Basics"));
        assert!(result.processed_text.contains("---"));
        assert!(!result.processed_text.contains("## Record"));
        assert!(!result.processed_text.contains("|"));
    }

    #[test]
    fn test_structured_formatting() {
        let preprocessor = CsvPreprocessor::default();
        let result = preprocessor.preprocess_csv_content(STRUCTURED_CSV).unwrap();

        // Check for structured format elements
        assert!(result.processed_text.contains("## Record #1"));
        assert!(result.processed_text.contains("- employee_id: 1001"));
        assert!(!result.processed_text.contains("---"));
        assert!(!result.processed_text.contains("|"));
    }

    #[test]
    fn test_preview_rows() {
        let preprocessor = CsvPreprocessor::default();
        let preview = preprocessor.preview_rows(NARRATIVE_CSV, 1).unwrap();

        assert_eq!(preview.len(), 1);
        assert!(preview[0].contains("title:"));
    }

    #[test]
    fn test_field_name_cleaning() {
        let csv_with_spaces = "\
First Name,Last Name,User ID
Alice,Johnson,1001";

        let preprocessor = CsvPreprocessor::default();
        let result = preprocessor
            .preprocess_csv_content(csv_with_spaces)
            .unwrap();

        // Check that field names are cleaned
        assert!(result.processed_text.contains("first_name:"));
        assert!(result.processed_text.contains("last_name:"));
        assert!(result.processed_text.contains("user_id:"));
    }

    #[test]
    fn test_analysis_report() {
        let preprocessor = CsvPreprocessor::default();
        let report = preprocessor.analyze_csv(NARRATIVE_CSV).unwrap();

        assert!(report.contains("Field Analysis"));
        assert!(report.contains("Avg length"));
        assert!(report.contains("Detected Content Type"));
    }
}
