//! Context configuration for RAG queries
//! Handles dynamic context length management and compaction strategies

use serde::{Deserialize, Serialize};
use std::fmt;

// Forward declaration - ConversationMessage is defined in conversation_service
// We use a placeholder here and will use the actual type in the service layer

/// Strategy for compacting conversation history when context is full
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompactionStrategy {
    /// Auto-select based on model context window size
    Adaptive,

    /// Simply cut off oldest messages (most efficient for small models)
    Truncate,

    /// Summarize older messages using LLM (best for large models)
    Summarize,

    /// Summarize very old messages, keep recent verbatim (balanced)
    Hybrid,
}

impl fmt::Display for CompactionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adaptive => write!(f, "adaptive"),
            Self::Truncate => write!(f, "truncate"),
            Self::Summarize => write!(f, "summarize"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl std::str::FromStr for CompactionStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "adaptive" => Ok(Self::Adaptive),
            "truncate" => Ok(Self::Truncate),
            "summarize" => Ok(Self::Summarize),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(format!("Unknown compaction strategy: {}", s)),
        }
    }
}

/// Context window configuration for RAG queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextWindowConfig {
    /// Maximum context tokens to use for RAG queries
    pub max_context_tokens: usize,

    /// Maximum number of history messages to include
    pub max_history_messages: usize,

    /// Whether to enable context compaction
    pub enable_compaction: bool,

    /// Strategy to use for compaction
    pub compaction_strategy: CompactionStrategy,

    /// Number of messages before compaction triggers
    pub summary_threshold: usize,

    /// Tokens to reserve for LLM response
    pub reserved_for_response: usize,

    /// Below this threshold = use efficient truncation (for local models)
    pub small_model_threshold: usize,

    /// Above this threshold = use full summarization (for cloud models)
    pub large_model_threshold: usize,
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8000,
            max_history_messages: 10,
            enable_compaction: true,
            compaction_strategy: CompactionStrategy::Adaptive,
            summary_threshold: 5,
            reserved_for_response: 2048,
            small_model_threshold: 8000,
            large_model_threshold: 32000,
        }
    }
}

/// Model context limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelContextLimit {
    pub id: i64,
    pub provider: String,
    pub model_name: String,
    pub context_window: usize,
    pub max_output_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_strategy_display() {
        assert_eq!(CompactionStrategy::Adaptive.to_string(), "adaptive");
        assert_eq!(CompactionStrategy::Truncate.to_string(), "truncate");
        assert_eq!(CompactionStrategy::Summarize.to_string(), "summarize");
        assert_eq!(CompactionStrategy::Hybrid.to_string(), "hybrid");
    }

    #[test]
    fn test_compaction_strategy_from_str() {
        assert_eq!(CompactionStrategy::from_str("adaptive"), Ok(CompactionStrategy::Adaptive));
        assert_eq!(CompactionStrategy::from_str("Adaptive"), Ok(CompactionStrategy::Adaptive));
        assert_eq!(CompactionStrategy::from_str("TRUNCATE"), Ok(CompactionStrategy::Truncate));
        assert!(CompactionStrategy::from_str("unknown").is_err());
    }

    #[test]
    fn test_context_window_config_default() {
        let config = ContextWindowConfig::default();
        assert_eq!(config.max_context_tokens, 8000);
        assert_eq!(config.max_history_messages, 10);
        assert!(config.enable_compaction);
        assert_eq!(config.compaction_strategy, CompactionStrategy::Adaptive);
    }
}
