// ============================================================
// CSV DOMAIN LAYER
// ============================================================
// Core types and value objects for CSV preprocessing
// No I/O, no async, no external dependencies

mod content_type;
mod preprocessing_config;
mod field_analysis;
mod csv_row;

pub use content_type::ContentType;
pub use preprocessing_config::PreprocessingConfig;
pub use field_analysis::FieldAnalysis;
pub use csv_row::{CsvRow, CsvField, PreprocessedCsv};

// Re-export commonly used types
pub use std::collections::HashMap;
pub type FieldMap = HashMap<String, String>;
