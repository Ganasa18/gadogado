// ============================================================
// CONTENT TYPE ENUM
// ============================================================
// Determines how CSV data should be formatted for embedding

use serde::{Deserialize, Serialize};

/// Content type detected in CSV data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// Long-form descriptive text (articles, comments, descriptions)
    /// Average value length > 50 chars, high lexical diversity
    Narrative,

    /// Short categorical or numeric data (IDs, names, prices, dates)
    Structured,
}

impl ContentType {
    /// Get the format extension for this content type
    pub fn format_extension(&self) -> &'static str {
        match self {
            ContentType::Narrative => "txt",
            ContentType::Structured => "md",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ContentType::Narrative => {
                "Long-form text with descriptive content, formatted as plain text"
            }
            ContentType::Structured => {
                "Short categorical or numeric data, formatted as Markdown"
            }
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Narrative => write!(f, "Narrative"),
            ContentType::Structured => write!(f, "Structured"),
        }
    }
}
