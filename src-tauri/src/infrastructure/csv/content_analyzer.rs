// ============================================================
// CONTENT ANALYZER
// ============================================================
// Analyze CSV field statistics to detect content type

use crate::domain::csv::{CsvRow, FieldAnalysis, PreprocessingConfig};
use std::collections::HashSet;

/// Content analyzer for CSV data
pub struct ContentAnalyzer {
    config: PreprocessingConfig,
}

impl ContentAnalyzer {
    /// Create a new content analyzer
    pub fn new(config: PreprocessingConfig) -> Self {
        Self { config }
    }

    /// Analyze CSV rows to determine field statistics
    pub fn analyze(&self, rows: &[CsvRow]) -> FieldAnalysis {
        if rows.is_empty() {
            return FieldAnalysis::new();
        }

        let mut total_length = 0usize;
        let mut all_words = Vec::new();
        let mut unique_words = HashSet::new();
        let mut numeric_count = 0usize;
        let mut empty_count = 0usize;
        let mut max_length = 0usize;
        let mut min_length = usize::MAX;

        // Sample rows if too many
        let sample_rows = self.get_sample_rows(rows);

        for row in &sample_rows {
            for field in &row.fields {
                // Skip empty fields for length stats
                if field.is_empty {
                    empty_count += 1;
                    continue;
                }

                let value_len = field.value.len();
                total_length += value_len;

                if value_len > max_length {
                    max_length = value_len;
                }
                if value_len < min_length {
                    min_length = value_len;
                }

                // Count numeric fields
                if field.is_numeric {
                    numeric_count += 1;
                }

                // Extract words for diversity analysis
                let words: Vec<&str> = field
                    .value
                    .split_whitespace()
                    .filter(|w| w.len() > 2) // Skip short words
                    .collect();

                for word in words {
                    let lower = word.to_lowercase();
                    all_words.push(lower.clone());
                    unique_words.insert(lower);
                }
            }
        }

        let total_fields = sample_rows.iter().map(|r| r.fields.len()).sum::<usize>();

        // Calculate statistics
        let avg_value_length = if total_fields > 0 {
            total_length as f32 / total_fields as f32
        } else {
            0.0
        };

        let lexical_diversity = if !all_words.is_empty() {
            unique_words.len() as f32 / all_words.len() as f32
        } else {
            0.0
        };

        let numeric_ratio = if total_fields > 0 {
            numeric_count as f32 / total_fields as f32
        } else {
            0.0
        };

        FieldAnalysis {
            avg_value_length,
            lexical_diversity,
            total_fields,
            numeric_ratio,
            row_count: rows.len(),
            empty_field_count: empty_count,
            max_value_length: max_length,
            min_value_length: if min_length == usize::MAX {
                0
            } else {
                min_length
            },
        }
    }

    /// Detect content type based on analysis
    pub fn detect_content_type(&self, rows: &[CsvRow]) -> crate::domain::csv::ContentType {
        use crate::domain::csv::ContentType;

        let analysis = self.analyze(rows);

        if analysis.is_narrative(&self.config) {
            ContentType::Narrative
        } else {
            ContentType::Structured
        }
    }

    /// Get sample rows for analysis (to avoid processing very large files)
    fn get_sample_rows<'a>(&self, rows: &'a [CsvRow]) -> Vec<&'a CsvRow> {
        let row_count = rows.len();

        if row_count <= self.config.max_sample_rows {
            return rows.iter().collect();
        }

        // Sample evenly from the dataset
        let step = row_count / self.config.max_sample_rows;
        rows.iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .map(|(_, row)| row)
            .take(self.config.max_sample_rows)
            .collect()
    }

    /// Get detailed analysis report
    pub fn get_analysis_report(&self, rows: &[CsvRow]) -> String {
        let analysis = self.analyze(rows);
        let content_type = self.detect_content_type(rows);

        format!(
            "{}\n\nDetected Content Type: {}\nConfidence: {:.2}",
            analysis.summary(),
            content_type,
            analysis.confidence_score()
        )
    }
}

impl Default for ContentAnalyzer {
    fn default() -> Self {
        Self::new(PreprocessingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::csv::{CsvField, ContentType};

    fn create_test_row(index: usize, values: Vec<(&str, &str)>) -> CsvRow {
        let fields = values
            .into_iter()
            .map(|(name, value)| CsvField::new(name.to_string(), value.to_string()))
            .collect();

        CsvRow::new(index, fields)
    }

    #[test]
    fn test_detect_narrative_content() {
        let rows = vec![
            create_test_row(
                0,
                vec![
                    ("title", "Machine Learning Basics"),
                    ("content", "This is a long article about machine learning that discusses various algorithms and their applications in real-world scenarios."),
                ],
            ),
            create_test_row(
                1,
                vec![
                    ("title", "Neural Networks"),
                    ("content", "Neural networks are computing systems inspired by biological neural networks that form the basis of deep learning."),
                ],
            ),
        ];

        let config = PreprocessingConfig::narrative();
        let analyzer = ContentAnalyzer::new(config);

        assert_eq!(
            analyzer.detect_content_type(&rows),
            ContentType::Narrative
        );
    }

    #[test]
    fn test_detect_structured_content() {
        let rows = vec![
            create_test_row(
                0,
                vec![
                    ("id", "1001"),
                    ("name", "Alice"),
                    ("age", "30"),
                    ("salary", "75000"),
                ],
            ),
            create_test_row(
                1,
                vec![
                    ("id", "1002"),
                    ("name", "Bob"),
                    ("age", "25"),
                    ("salary", "65000"),
                ],
            ),
        ];

        let analyzer = ContentAnalyzer::default();

        assert_eq!(
            analyzer.detect_content_type(&rows),
            ContentType::Structured
        );
    }

    #[test]
    fn test_field_analysis() {
        let rows = vec![
            create_test_row(
                0,
                vec![
                    ("name", "Alice"),
                    ("description", "A long detailed description of something"),
                ],
            ),
        ];

        let analyzer = ContentAnalyzer::default();
        let analysis = analyzer.analyze(&rows);

        assert!(analysis.avg_value_length > 0.0);
        assert_eq!(analysis.row_count, 1);
    }
}
