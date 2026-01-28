// ============================================================
#![allow(dead_code)]
// TODO: remove/tighten once CSV domain is fully used.

// CSV DOMAIN LAYER
// ============================================================
// Core types and value objects for CSV preprocessing
// No I/O, no async, no external dependencies

mod content_type;
mod csv_row;
mod field_analysis;
mod preprocessing_config;

pub use content_type::ContentType;
pub use csv_row::{CsvField, CsvRow, PreprocessedCsv};
pub use field_analysis::FieldAnalysis;
pub use preprocessing_config::PreprocessingConfig;

// Re-export commonly used types
pub use std::collections::HashMap;
pub type FieldMap = HashMap<String, String>;
