//! Universal token counter for context window management
//! Uses character-based estimation that works for all providers
//!
//! This approach supports: OpenAI, Gemini, Anthropic, GLM, OpenRouter, etc.
//! without requiring provider-specific tokenizers
//!
//! Approximation: ~4 characters per token (works for most LLMs)

/// Universal token counter
pub struct TokenCounter;

impl TokenCounter {
    /// Estimate token count for text (universal, provider-agnostic)
    /// Uses ~4 characters per token approximation
    ///
    /// # Arguments
    /// * `text` - The text to estimate tokens for
    /// * `provider` - Provider hint (unused in universal approach, kept for API compatibility)
    ///
    /// # Returns
    /// Estimated token count
    pub fn estimate_tokens(text: &str, _provider: &str) -> usize {
        Self::estimate_char_tokens(text)
    }

    /// Universal character-based estimation
    /// Rule of thumb: ~4 characters = 1 token for most LLMs
    /// Accounts for message formatting overhead
    fn estimate_char_tokens(text: &str) -> usize {
        // Avoid division by zero
        if text.is_empty() {
            return 0;
        }
        (text.len() + 3) / 4
    }

    /// Estimate tokens for message array (universal)
    ///
    /// # Arguments
    /// * `messages` - Array of (role, content) tuples
    /// * `provider` - LLM provider
    ///
    /// # Returns
    /// Estimated token count including message formatting overhead
    pub fn estimate_messages_tokens(
        messages: &[(&str, &str)],
        provider: &str,
    ) -> usize {
        // Add overhead for message formatting (~4 tokens per message wrapper)
        let base_tokens = messages.len() * 4;
        let content_tokens: usize = messages
            .iter()
            .map(|(role, content)| {
                let full_message = format!("{}: {}", role, content);
                Self::estimate_char_tokens(&full_message)
            })
            .sum();
        base_tokens + content_tokens
    }

    /// Estimate remaining tokens in context window
    ///
    /// # Arguments
    /// * `used_tokens` - Tokens already used
    /// * `context_window` - Total context window size
    /// * `reserved_for_response` - Tokens reserved for LLM response
    ///
    /// # Returns
    /// Number of tokens remaining for additional content
    pub fn estimate_remaining(
        used_tokens: usize,
        context_window: usize,
        reserved_for_response: usize,
    ) -> usize {
        context_window
            .saturating_sub(used_tokens)
            .saturating_sub(reserved_for_response)
    }

    /// Estimate tokens for RAG context (messages + retrieved chunks)
    ///
    /// # Arguments
    /// * `messages` - Conversation messages
    /// * `rag_context` - Retrieved RAG context chunks
    /// * `provider` - LLM provider
    ///
    /// # Returns
    /// Total estimated token count
    pub fn estimate_rag_tokens(
        messages: &[(&str, &str)],
        rag_context: &str,
        provider: &str,
    ) -> usize {
        let messages_tokens = Self::estimate_messages_tokens(messages, provider);
        let rag_tokens = Self::estimate_tokens(rag_context, provider);
        messages_tokens + rag_tokens + 100 // Add small buffer for system prompt
    }

    /// Check if estimated tokens fit within context window
    ///
    /// # Arguments
    /// * `estimated` - Estimated token count
    /// * `context_window` - Total context window size
    ///
    /// # Returns
    /// true if tokens fit, false otherwise
    pub fn fits_in_context(estimated: usize, context_window: usize) -> bool {
        estimated < context_window
    }

    /// Calculate how many messages to keep based on token budget
    ///
    /// # Arguments
    /// * `messages` - Array of messages with (role, content)
    /// * `token_budget` - Maximum tokens available
    /// * `provider` - LLM provider
    ///
    /// # Returns
    /// Number of messages (from newest) that fit in the budget
    pub fn count_messages_that_fit(
        messages: &[(&str, &str)],
        token_budget: usize,
        provider: &str,
    ) -> usize {
        let mut count = 0;
        let mut used_tokens = 0;

        // Count from newest to oldest
        for (role, content) in messages.iter().rev() {
            // Combine role and content for token estimation
            let full_message = format!("{}: {}", role, content);
            let msg_tokens = Self::estimate_tokens(&full_message, provider);

            if used_tokens + msg_tokens > token_budget {
                break;
            }

            used_tokens += msg_tokens;
            count += 1;
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_char_tokens() {
        assert_eq!(TokenCounter::estimate_char_tokens(""), 0);
        assert_eq!(TokenCounter::estimate_char_tokens("a"), 1);
        assert_eq!(TokenCounter::estimate_char_tokens("abcd"), 1);
        assert_eq!(TokenCounter::estimate_char_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_estimate_tokens() {
        // ~4 chars per token
        let text = "Hello world! This is a test.";
        let tokens = TokenCounter::estimate_tokens(text, "openai");
        assert!(tokens > 0);

        // Provider doesn't matter for universal estimation
        let tokens2 = TokenCounter::estimate_tokens(text, "gemini");
        assert_eq!(tokens, tokens2);
    }

    #[test]
    fn test_estimate_messages_tokens() {
        let messages = [
            ("user", "Hello world"),
            ("assistant", "Hi there!"),
        ];
        let tokens = TokenCounter::estimate_messages_tokens(&messages, "openai");
        // Should be content + overhead
        assert!(tokens > 10); // At minimum
    }

    #[test]
    fn test_estimate_remaining() {
        // Full context available
        assert_eq!(TokenCounter::estimate_remaining(0, 8000, 1000), 7000);

        // Partially used
        assert_eq!(TokenCounter::estimate_remaining(2000, 8000, 1000), 5000);

        // Reserved for response
        assert_eq!(TokenCounter::estimate_remaining(7500, 8000, 1000), 0);
    }

    #[test]
    fn test_fits_in_context() {
        assert!(TokenCounter::fits_in_context(1000, 8000));
        assert!(!TokenCounter::fits_in_context(8000, 8000));
        assert!(!TokenCounter::fits_in_context(9000, 8000));
    }

    #[test]
    fn test_count_messages_that_fit() {
        let messages = [
            ("user", "This is a test message"),
            ("assistant", "This is a response"),
            ("user", "Another message"),
        ];

        // Budget for 1 message
        let count = TokenCounter::count_messages_that_fit(&messages, 50, "openai");
        assert!(count <= 2); // At most 1-2 messages

        // Larger budget
        let count2 = TokenCounter::count_messages_that_fit(&messages, 1000, "openai");
        assert!(count2 >= count);
    }

    #[test]
    fn test_estimate_rag_tokens() {
        let messages = [("user", "What is RAG?")];
        let rag_context = "Retrieval Augmented Generation is a technique...";

        let total = TokenCounter::estimate_rag_tokens(&messages, rag_context, "openai");
        assert!(total > messages.len() * 4); // At minimum messages + overhead
    }
}
