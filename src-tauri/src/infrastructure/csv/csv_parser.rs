// ============================================================
// CSV PARSER
// ============================================================
// Parse CSV files with encoding detection and error handling

use std::path::Path;
use csv::{ReaderBuilder, StringRecord, Trim};
use crate::domain::csv::{CsvRow, CsvField};
use crate::domain::error::{AppError};

/// CSV parser with encoding detection
pub struct CsvParser {
    /// Delimiter character (default: comma)
    delimiter: u8,

    /// Whether to trim whitespace from values
    trim: bool,

    /// Maximum allowed record length
    max_record_length: usize,
}

impl Default for CsvParser {
    fn default() -> Self {
        Self {
            delimiter: b',',
            trim: true,
            max_record_length: 1024 * 1024, // 1MB
        }
    }
}

impl CsvParser {
    /// Create a new CSV parser with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom delimiter
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set whether to trim whitespace
    pub fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Parse a CSV file and return rows
    pub fn parse_file(&self, path: &Path) -> Result<Vec<CsvRow>, AppError> {
        // Detect encoding and read file
        let content = self.read_with_encoding_detection(path)?;

        // Parse CSV content
        self.parse_content(&content)
    }

    /// Parse CSV content from string
    pub fn parse_content(&self, content: &str) -> Result<Vec<CsvRow>, AppError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .trim(if self.trim { Trim::All } else { Trim::None })
            .flexible(true) // Allow rows with different lengths
            .from_reader(content.as_bytes());

        // Get headers
        let headers = reader.headers().map_err(|e| {
            AppError::ParseError(format!("Failed to read CSV headers: {}", e))
        })?.clone();

        // Parse rows
        let mut rows = Vec::new();
        let mut index = 0;

        for result in reader.records() {
            let record = result.map_err(|e| {
                AppError::ParseError(format!("Failed to parse CSV row {}: {}", index + 1, e))
            })?;

            let row = self.parse_row(index, &headers, &record)?;
            rows.push(row);
            index += 1;
        }

        Ok(rows)
    }

    /// Read file with encoding detection
    fn read_with_encoding_detection(&self, path: &Path) -> Result<String, AppError> {
        use std::fs::File;
        use std::io::Read;

        // Try UTF-8 first
        if let Ok(mut file) = File::open(path) {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() {
                // Try UTF-8
                if let Ok(content) = std::str::from_utf8(&buffer) {
                    return Ok(content.to_string());
                }

                // Try Latin-1 (fallback)
                if let Ok(content) = std::str::from_utf8(&buffer) {
                    return Ok(content.to_string());
                }
            }
        }

        // If encoding detection fails, read as raw bytes and replace invalid UTF-8
        let mut file = File::open(path).map_err(|e| {
            AppError::IoError(format!("Failed to open file: {}", e))
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| {
            AppError::IoError(format!("Failed to read file: {}", e))
        })?;

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Parse a single CSV row
    fn parse_row(
        &self,
        index: usize,
        headers: &StringRecord,
        record: &StringRecord,
    ) -> Result<CsvRow, AppError> {
        let mut fields = Vec::new();

        for (idx, header) in headers.iter().enumerate() {
            let value = record.get(idx).unwrap_or("").to_string();
            let field = CsvField::new(header.to_string(), value);
            fields.push(field);
        }

        Ok(CsvRow::new(index, fields))
    }

    /// Detect delimiter from content (comma, semicolon, tab, pipe)
    pub fn detect_delimiter(content: &str) -> u8 {
        let candidates = [b',', b';', b'\t', b'|'];

        let mut best_delimiter = b',';
        let mut best_score = 0.0f32;

        for &delimiter in &candidates {
            let sample_lines: Vec<_> = content.lines().take(10).collect();

            if sample_lines.is_empty() {
                continue;
            }

            let mut field_counts = Vec::new();

            for line in &sample_lines {
                let count = line.chars().filter(|&c| c as u8 == delimiter).count();
                field_counts.push(count);
            }

            // Score by consistency (low standard deviation) and frequency
            if !field_counts.is_empty() {
                let avg = field_counts.iter().sum::<usize>() as f32 / field_counts.len() as f32;
                let variance = field_counts
                    .iter()
                    .map(|&x| (x as f32 - avg).powi(2))
                    .sum::<f32>()
                    / field_counts.len() as f32;

                let score = avg / (1.0 + variance.sqrt());

                if score > best_score {
                    best_score = score;
                    best_delimiter = delimiter;
                }
            }
        }

        best_delimiter
    }

    /// Parse CSV file with automatic delimiter detection
    pub fn parse_file_auto_detect(path: &Path) -> Result<Vec<CsvRow>, AppError> {
        // Read a sample for delimiter detection
        let content_sample = {
            use std::fs::File;
            use std::io::Read;

            let mut file = File::open(path).map_err(|e| {
                AppError::IoError(format!("Failed to open file: {}", e))
            })?;

            let mut buffer = vec![0u8; 4096];
            file.read(&mut buffer).ok();
            String::from_utf8_lossy(&buffer).to_string()
        };

        let delimiter = Self::detect_delimiter(&content_sample);

        let parser = Self::default().with_delimiter(delimiter);
        parser.parse_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_csv() {
        let content = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let parser = CsvParser::new();
        let rows = parser.parse_content(content).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fields.len(), 3);
        assert_eq!(rows[0].fields[0].clean_name, "name");
        assert_eq!(rows[0].fields[0].value, "Alice");
    }

    #[test]
    fn test_detect_delimiter() {
        assert_eq!(CsvParser::detect_delimiter("a,b,c\nd,e,f"), b',');
        assert_eq!(CsvParser::detect_delimiter("a;b;c\nd;e;f"), b';');
    }

    #[test]
    fn test_field_cleaning() {
        let field = CsvField::new("First Name".to_string(), "John".to_string());
        assert_eq!(field.clean_name, "first_name");
    }
}
