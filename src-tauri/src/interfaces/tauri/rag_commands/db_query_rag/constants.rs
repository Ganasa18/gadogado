use crate::application::use_cases::data_protection::ExternalLlmPolicy;

/// Default allowlist profile ID when not specified in collection config
pub const DEFAULT_ALLOWLIST_PROFILE_ID: i64 = 1;

/// Default row limit when not specified in collection config
pub const DEFAULT_LIMIT: i32 = 50;

/// Maximum query length to display in logs (truncated with "...")
pub const MAX_QUERY_LOG_LENGTH: usize = 50;

/// Number of candidate rows to fetch for reranking
pub const CANDIDATE_K: i32 = 100;

/// Number of final results to return after reranking
pub const FINAL_K: i32 = 10;

/// Batch size for template retrieval
pub const TEMPLATE_BATCH_SIZE: i64 = 5;

/// Minimum score threshold for stopping batch retrieval
pub const TEMPLATE_STOP_THRESHOLD: f32 = 0.7;

/// Maximum batches to fetch (prevents infinite loops)
pub const MAX_TEMPLATE_BATCHES: i64 = 20;

/// LLM timeout for semantic matching (seconds)
pub const SEMANTIC_LLM_TIMEOUT_SECS: u64 = 15;

/// LLM timeout for NL response generation (seconds)
pub const NL_RESPONSE_TIMEOUT_SECS: u64 = 30;

/// Minimum score threshold for using template-first approach
pub const TEMPLATE_MATCH_THRESHOLD: f32 = 0.3;

/// Maximum number of templates to show to user
pub const MAX_TEMPLATES_FOR_USER: usize = 3;

pub fn default_external_llm_policy() -> ExternalLlmPolicy {
    "always_block".into()
}
