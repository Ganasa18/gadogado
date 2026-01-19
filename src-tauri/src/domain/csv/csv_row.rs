// ============================================================
// CSV ROW TYPES
// ============================================================
// Data structures representing parsed CSV content

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{ContentType, FieldAnalysis};

/// A single field in a CSV row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvField {
    /// Original field name (header)
    pub name: String,

    /// Cleaned field name (for output formatting)
    pub clean_name: String,

    /// Field value
    pub value: String,

    /// Whether the value is empty
    pub is_empty: bool,

    /// Whether the value appears to be numeric
    pub is_numeric: bool,
}

impl CsvField {
    /// Create a new CSV field
    pub fn new(name: String, value: String) -> Self {
        let is_empty = value.trim().is_empty();
        let is_numeric = Self::is_numeric_value(&value);
        let clean_name = Self::clean_field_name(&name);

        Self {
            name,
            clean_name,
            value,
            is_empty,
            is_numeric,
        }
    }

    /// Check if a string value is numeric
    fn is_numeric_value(value: &str) -> bool {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Try to parse as f64
        trimmed.parse::<f64>().is_ok()
            || trimmed.parse::<i64>().is_ok()
            || trimmed.replace(',', "").parse::<f64>().is_ok()
    }

    /// Clean field name for output formatting
    /// Replace special characters with underscores, keep only alphanumeric
    fn clean_field_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else if c.is_whitespace() {
                    '_'
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    }

    /// Get the formatted output for narrative style
    pub fn format_narrative(&self) -> String {
        if self.is_empty {
            String::new()
        } else {
            format!("{}: {}", self.clean_name, self.value)
        }
    }

    /// Get the formatted output for structured style
    pub fn format_structured(&self) -> String {
        if self.is_empty {
            String::new()
        } else {
            format!("- {}: {}", self.clean_name, self.value)
        }
    }
}

/// A single row in a CSV file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvRow {
    /// Row index (0-based)
    pub index: usize,

    /// All fields in this row
    pub fields: Vec<CsvField>,

    /// Field map for easy access
    pub field_map: HashMap<String, String>,
}

impl CsvRow {
    /// Create a new CSV row
    pub fn new(index: usize, fields: Vec<CsvField>) -> Self {
        let field_map = fields
            .iter()
            .filter(|f| !f.is_empty)
            .map(|f| (f.clean_name.clone(), f.value.clone()))
            .collect();

        Self { index, fields, field_map }
    }

    /// Get non-empty fields only
    pub fn non_empty_fields(&self) -> Vec<&CsvField> {
        self.fields.iter().filter(|f| !f.is_empty).collect()
    }

    /// Format this row as narrative text
    pub fn format_narrative(&self) -> String {
        self.non_empty_fields()
            .iter()
            .map(|f| f.format_narrative())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format this row as structured markdown
    pub fn format_structured(&self) -> String {
        self.non_empty_fields()
            .iter()
            .map(|f| f.format_structured())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Result of CSV preprocessing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessedCsv {
    /// Detected content type
    pub content_type: ContentType,

    /// Processed text ready for embedding
    pub processed_text: String,

    /// Number of rows processed
    pub row_count: usize,

    /// Field analysis statistics
    pub analysis: FieldAnalysis,

    /// Original headers
    pub headers: Vec<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}
