// ============================================================
// FIELD ANALYSIS
// ============================================================
// Statistical analysis of CSV fields for content type detection

use super::PreprocessingConfig;
use serde::{Deserialize, Serialize};

/// Statistical analysis of CSV field content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAnalysis {
    /// Average length of all field values (in characters)
    pub avg_value_length: f32,

    /// Lexical diversity: ratio of unique words to total words (0.0 - 1.0)
    pub lexical_diversity: f32,

    /// Total number of fields analyzed
    pub total_fields: usize,

    /// Ratio of fields that contain primarily numeric data (0.0 - 1.0)
    pub numeric_ratio: f32,

    /// Total number of rows analyzed
    pub row_count: usize,

    /// Number of empty fields encountered
    pub empty_field_count: usize,

    /// Maximum value length found
    pub max_value_length: usize,

    /// Minimum value length found (excluding empty)
    pub min_value_length: usize,
}

impl FieldAnalysis {
    /// Create a new field analysis result
    pub fn new() -> Self {
        Self {
            avg_value_length: 0.0,
            lexical_diversity: 0.0,
            total_fields: 0,
            numeric_ratio: 0.0,
            row_count: 0,
            empty_field_count: 0,
            max_value_length: 0,
            min_value_length: usize::MAX,
        }
    }

    /// Determine if this analysis suggests narrative content
    pub fn is_narrative(&self, config: &PreprocessingConfig) -> bool {
        self.avg_value_length > config.min_value_length_threshold as f32
            && self.lexical_diversity >= config.min_lexical_diversity
            && self.numeric_ratio <= config.max_numeric_ratio
    }

    /// Get a confidence score for the content type classification
    /// Returns 0.0 (low confidence) to 1.0 (high confidence)
    pub fn confidence_score(&self) -> f32 {
        // Higher confidence when metrics are clearly one type or the other
        let length_score = if self.avg_value_length > 100.0 {
            1.0 // Clearly narrative
        } else if self.avg_value_length < 20.0 {
            1.0 // Clearly structured
        } else {
            0.5 // Ambiguous
        };

        let diversity_score = if self.lexical_diversity > 0.8 {
            1.0
        } else if self.lexical_diversity < 0.3 {
            1.0
        } else {
            0.5
        };

        (length_score + diversity_score) / 2.0
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Field Analysis ({} rows, {} fields):\n\
             - Avg length: {:.1} chars\n\
             - Lexical diversity: {:.2}\n\
             - Numeric ratio: {:.2}\n\
             - Empty fields: {}",
            self.row_count,
            self.total_fields,
            self.avg_value_length,
            self.lexical_diversity,
            self.numeric_ratio,
            self.empty_field_count
        )
    }
}

impl Default for FieldAnalysis {
    fn default() -> Self {
        Self::new()
    }
}
