// ============================================================
// PREPROCESSING CONFIGURATION
// ============================================================
// Configuration values for CSV content detection and formatting

use serde::{Deserialize, Serialize};

/// Configuration for CSV preprocessing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    /// Minimum average value length to classify as Narrative (default: 50)
    pub min_value_length_threshold: usize,

    /// Minimum lexical diversity ratio to classify as Narrative (default: 0.6)
    /// Calculated as: unique_words / total_words
    pub min_lexical_diversity: f32,

    /// Maximum ratio of numeric fields allowed for Narrative (default: 0.3)
    /// If more than 30% of fields are numeric, classify as Structured
    pub max_numeric_ratio: f32,

    /// Preserve original CSV data alongside processed text
    pub preserve_original: bool,

    /// Automatically chunk processed text after preprocessing
    pub chunk_after_preprocessing: bool,

    /// Minimum number of rows required for reliable detection (default: 2)
    pub min_sample_rows: usize,

    /// Maximum number of rows to analyze for detection (default: 1000)
    pub max_sample_rows: usize,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            min_value_length_threshold: 50,
            min_lexical_diversity: 0.6,
            max_numeric_ratio: 0.3,
            preserve_original: true,
            chunk_after_preprocessing: true,
            min_sample_rows: 2, // Reduced from 10 to support small datasets
            max_sample_rows: 1000,
        }
    }
}

impl PreprocessingConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config optimized for narrative text detection
    pub fn narrative() -> Self {
        Self {
            min_value_length_threshold: 40,
            min_lexical_diversity: 0.5,
            max_numeric_ratio: 0.2,
            ..Default::default()
        }
    }

    /// Create config optimized for structured data detection
    pub fn structured() -> Self {
        Self {
            min_value_length_threshold: 100,
            min_lexical_diversity: 0.8,
            max_numeric_ratio: 0.5,
            ..Default::default()
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.min_value_length_threshold == 0 {
            return Err("min_value_length_threshold must be > 0".to_string());
        }
        if !(0.0..=1.0).contains(&self.min_lexical_diversity) {
            return Err("min_lexical_diversity must be between 0.0 and 1.0".to_string());
        }
        if !(0.0..=1.0).contains(&self.max_numeric_ratio) {
            return Err("max_numeric_ratio must be between 0.0 and 1.0".to_string());
        }
        if self.min_sample_rows >= self.max_sample_rows {
            return Err("min_sample_rows must be < max_sample_rows".to_string());
        }
        Ok(())
    }
}
