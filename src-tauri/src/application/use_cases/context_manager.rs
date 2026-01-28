//! Context manager for RAG queries
//! Handles adaptive context management based on model capabilities

use crate::application::use_cases::conversation_service::ConversationMessage;
use crate::domain::context_config::{CompactionStrategy, ContextWindowConfig};
use crate::domain::error::Result;
use crate::shared::TokenCounter;

/// Result from building context for RAG query
#[derive(Debug, Clone)]
pub struct BuildContext {
    pub messages: Vec<ConversationMessage>,
    pub summary: Option<String>,
    pub token_estimate: usize,
    pub was_compacted: bool,
    pub strategy_used: CompactionStrategy,
}

/// Context manager for building RAG query context
pub struct ContextManager {
    config: ContextWindowConfig,
    provider: String,
    model: String,
    model_context_window: usize,
}

impl ContextManager {
    /// Create a new context manager
    ///
    /// # Arguments
    /// * `config` - Context window configuration
    /// * `provider` - LLM provider name
    /// * `model` - LLM model name
    /// * `model_context_window` - Maximum context window for the model
    pub fn new(
        config: ContextWindowConfig,
        provider: String,
        model: String,
        model_context_window: usize,
    ) -> Self {
        Self {
            config,
            provider,
            model,
            model_context_window,
        }
    }

    /// Build context that fits within token budget
    /// Uses adaptive strategy based on model size
    ///
    /// # Arguments
    /// * `messages` - Conversation messages
    /// * `rag_context` - Retrieved RAG context
    ///
    /// # Returns
    /// Built context with messages, optional summary, and metadata
    pub fn build_context(
        &self,
        messages: Vec<ConversationMessage>,
        rag_context: &str,
    ) -> Result<BuildContext> {
        // 1. Calculate available tokens
        let reserved = self.config.reserved_for_response;
        let rag_tokens = TokenCounter::estimate_tokens(rag_context, &self.provider);
        let available_for_history = self
            .config
            .max_context_tokens
            .saturating_sub(rag_tokens + reserved);

        // 2. Determine strategy (adaptive or explicit)
        let strategy = self.determine_strategy();

        // 3. Apply compaction strategy if enabled
        let processed = if self.config.enable_compaction {
            self.apply_compaction(messages, available_for_history, strategy)?
        } else {
            self.simple_truncate(messages, available_for_history)?
        };

        // Add RAG context token estimate to total
        let total_tokens = processed.total_tokens + rag_tokens;

        Ok(BuildContext {
            messages: processed.messages,
            summary: processed.summary,
            token_estimate: total_tokens,
            was_compacted: processed.was_compacted,
            strategy_used: processed.strategy,
        })
    }

    /// Determine the appropriate compaction strategy
    fn determine_strategy(&self) -> CompactionStrategy {
        match &self.config.compaction_strategy {
            CompactionStrategy::Adaptive => {
                // Auto-select based on model context window
                if self.model_context_window <= self.config.small_model_threshold {
                    // Small model (e.g., local 4K): Use efficient truncation
                    CompactionStrategy::Truncate
                } else if self.model_context_window >= self.config.large_model_threshold {
                    // Large model (e.g., cloud 128K+): Use full summarization
                    CompactionStrategy::Summarize
                } else {
                    // Medium model: Use hybrid approach
                    CompactionStrategy::Hybrid
                }
            }
            explicit => explicit.clone(), // Use explicitly set strategy
        }
    }

    fn apply_compaction(
        &self,
        messages: Vec<ConversationMessage>,
        token_budget: usize,
        strategy: CompactionStrategy,
    ) -> Result<ProcessedContext> {
        match strategy {
            CompactionStrategy::Truncate => self.simple_truncate(messages, token_budget),
            CompactionStrategy::Summarize => self.summarize_all(messages, token_budget),
            CompactionStrategy::Hybrid => self.hybrid_compact(messages, token_budget),
            CompactionStrategy::Adaptive => {
                // Should never happen - determine_strategy resolves Adaptive
                Ok(ProcessedContext {
                    messages,
                    summary: None,
                    total_tokens: 0,
                    was_compacted: false,
                    strategy: CompactionStrategy::Truncate,
                })
            }
        }
    }

    /// Truncate oldest messages (efficient for small models)
    fn simple_truncate(
        &self,
        messages: Vec<ConversationMessage>,
        token_budget: usize,
    ) -> Result<ProcessedContext> {
        let mut result = Vec::new();
        let mut total_tokens = 0;

        // Add messages from newest to oldest until budget exceeded
        for msg in messages.iter().rev() {
            let full_msg = format!("user: {}", msg.content); // Simplified - assume user role
            let msg_tokens = TokenCounter::estimate_tokens(&full_msg, &self.provider);

            if total_tokens + msg_tokens > token_budget {
                break;
            }

            total_tokens += msg_tokens;
            result.insert(0, msg.clone());
        }

        let was_compacted = result.len() < messages.len();
        Ok(ProcessedContext {
            messages: result,
            summary: None,
            total_tokens,
            was_compacted,
            strategy: CompactionStrategy::Truncate,
        })
    }

    /// Hybrid: summarize old messages, keep recent verbatim
    fn hybrid_compact(
        &self,
        messages: Vec<ConversationMessage>,
        token_budget: usize,
    ) -> Result<ProcessedContext> {
        let threshold = self.config.summary_threshold;

        if messages.len() <= threshold {
            // No compaction needed
            return self.simple_truncate(messages, token_budget);
        }

        // Split into old and recent
        let split_at = messages.len().saturating_sub(threshold);
        let old = &messages[..split_at];
        let recent = &messages[split_at..];

        // Estimate tokens in old messages
        let old_tokens: usize = old
            .iter()
            .map(|m| {
                let full_msg = format!("user: {}", m.content);
                TokenCounter::estimate_tokens(&full_msg, &self.provider)
            })
            .sum();

        // If old messages fit, keep them
        if old_tokens <= token_budget / 2 {
            return Ok(ProcessedContext {
                messages,
                summary: None,
                total_tokens: old_tokens,
                was_compacted: false,
                strategy: CompactionStrategy::Hybrid,
            });
        }

        // Summarize old messages (placeholder for now)
        let summary = self.summarize_messages(old)?;
        let summary_tokens = TokenCounter::estimate_tokens(&summary, &self.provider);

        Ok(ProcessedContext {
            messages: recent.to_vec(),
            summary: Some(summary),
            total_tokens: summary_tokens,
            was_compacted: true,
            strategy: CompactionStrategy::Hybrid,
        })
    }

    /// Summarize all messages using LLM (for large models)
    fn summarize_all(
        &self,
        messages: Vec<ConversationMessage>,
        _token_budget: usize,
    ) -> Result<ProcessedContext> {
        if messages.is_empty() {
            return Ok(ProcessedContext {
                messages: vec![],
                summary: None,
                total_tokens: 0,
                was_compacted: false,
                strategy: CompactionStrategy::Summarize,
            });
        }

        // Create summary using LLM (placeholder for now)
        let summary = self.summarize_messages(&messages)?;
        let summary_tokens = TokenCounter::estimate_tokens(&summary, &self.provider);

        // Keep recent messages in addition to summary
        let recent_count = self.config.summary_threshold.min(3);
        let recent: Vec<_> = messages
            .iter()
            .rev()
            .take(recent_count)
            .collect();

        Ok(ProcessedContext {
            messages: recent.into_iter().rev().cloned().collect(),
            summary: Some(summary),
            total_tokens: summary_tokens,
            was_compacted: true,
            strategy: CompactionStrategy::Summarize,
        })
    }

    /// Call LLM to summarize messages
    /// TODO: Implement actual LLM call for summarization
    fn summarize_messages(&self, messages: &[ConversationMessage]) -> Result<String> {
        if messages.is_empty() {
            return Ok("No previous conversation".to_string());
        }

        // For now, return a placeholder
        // In production, this would call the LLM to generate a summary
        Ok(format!(
            "[Summary of {} previous messages. Last message: {}]",
            messages.len(),
            messages.last()
                .map(|m| m.content.chars().take(50).collect::<String>())
                .unwrap_or_else(|| "(empty)".to_string())
        ))
    }
}

struct ProcessedContext {
    messages: Vec<ConversationMessage>,
    summary: Option<String>,
    total_tokens: usize,
    was_compacted: bool,
    strategy: CompactionStrategy,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ContextWindowConfig {
        ContextWindowConfig {
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

    fn create_test_manager() -> ContextManager {
        ContextManager::new(
            create_test_config(),
            "openai".to_string(),
            "gpt-4o".to_string(),
            128000,
        )
    }

    fn create_test_messages() -> Vec<ConversationMessage> {
        vec![
            ConversationMessage {
                id: 1,
                conversation_id: 1,
                role: "user".to_string(),
                content: "First message".to_string(),
                sources: None,
                created_at: "2024-01-01T00:00:00".to_string(),
            },
            ConversationMessage {
                id: 2,
                conversation_id: 1,
                role: "assistant".to_string(),
                content: "First response".to_string(),
                sources: None,
                created_at: "2024-01-01T00:01:00".to_string(),
            },
        ]
    }

    #[test]
    fn test_determine_strategy_adaptive_small_model() {
        let config = create_test_config();
        let manager = ContextManager::new(
            config,
            "local".to_string(),
            "default".to_string(),
            4096, // Small model
        );

        assert_eq!(
            manager.determine_strategy(),
            CompactionStrategy::Truncate
        );
    }

    #[test]
    fn test_determine_strategy_adaptive_large_model() {
        let config = create_test_config();
        let manager = ContextManager::new(
            config,
            "openai".to_string(),
            "gpt-4o".to_string(),
            128000, // Large model
        );

        assert_eq!(
            manager.determine_strategy(),
            CompactionStrategy::Summarize
        );
    }

    #[test]
    fn test_determine_strategy_explicit() {
        let config = ContextWindowConfig {
            compaction_strategy: CompactionStrategy::Truncate,
            ..Default::default()
        };
        let manager = ContextManager::new(
            config,
            "openai".to_string(),
            "gpt-4o".to_string(),
            128000,
        );

        assert_eq!(
            manager.determine_strategy(),
            CompactionStrategy::Truncate
        );
    }
}
