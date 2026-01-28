//! DB Query RAG Command
//!
//! This module provides the main SQL-RAG query command that:
//! - Validates queries against allowlist
//! - Compiles natural language to SQL
//! - Executes queries with rate limiting
//! - Reranks results for relevance scoring
//! - Returns citations and telemetry

use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::audit_service::{AuditLogEntry, AuditService};
use crate::application::use_cases::data_protection::{ExternalLlmPolicy, LlmRoute};
use crate::application::use_cases::rate_limiter::RateLimitResult;
use crate::application::use_cases::semantic_matcher::{
    fuse_keyword_semantic_matches, SemanticMatcher,
};
use crate::application::use_cases::sql_compiler::{DbType, SqlCompiler};
use crate::application::use_cases::sql_rag_router::SqlRagRouter;
use crate::application::use_cases::template_matcher::{TemplateMatch, TemplateMatcher};
use crate::application::QueryResult;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::rag_entities::{DbConnectionConfig, QueryPlan, QueryTemplate};
use crate::interfaces::http::add_log;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

use super::types::*;

// ============================================================================
// Constants
// ============================================================================

/// Default allowlist profile ID when not specified in collection config
const DEFAULT_ALLOWLIST_PROFILE_ID: i64 = 1;

/// Default row limit when not specified in collection config
const DEFAULT_LIMIT: i32 = 50;

/// Maximum query length to display in logs (truncated with "...")
const MAX_QUERY_LOG_LENGTH: usize = 50;

/// Number of candidate rows to fetch for reranking
const CANDIDATE_K: i32 = 100;

/// Number of final results to return after reranking
const FINAL_K: i32 = 10;

// ============================================================================
// Template Batching Constants
// ============================================================================

/// Batch size for template retrieval
const TEMPLATE_BATCH_SIZE: i64 = 5;

/// Minimum score threshold for stopping batch retrieval
const TEMPLATE_STOP_THRESHOLD: f32 = 0.7;

/// Maximum batches to fetch (prevents infinite loops)
const MAX_TEMPLATE_BATCHES: i64 = 20;

// ============================================================================
// Helper Functions
// ============================================================================

/// Logs an error and returns the provided error
fn log_and_return_error(
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    context: &str,
    message: &str,
    error: crate::domain::error::AppError,
) -> crate::domain::error::AppError {
    add_log(logs, "ERROR", context, message);
    error
}

/// Formats validation errors into a human-readable string
fn format_validation_errors<E>(errors: &[E]) -> String
where
    E: std::fmt::Debug,
{
    errors
        .iter()
        .map(|e| format!("{:?}", e))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Truncates query string for logging purposes
fn truncate_query_for_log(query: &str) -> String {
    if query.len() > MAX_QUERY_LOG_LENGTH {
        format!("{}...", &query[..MAX_QUERY_LOG_LENGTH])
    } else {
        query.to_string()
    }
}

/// Checks rate limit and returns error if exceeded
fn check_rate_limit(
    rate_limit_result: &RateLimitResult,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Result<()> {
    match rate_limit_result {
        RateLimitResult::Allowed => {
            add_log(logs, "DEBUG", "SQL-RAG", "Rate limit check passed");
            Ok(())
        }
        RateLimitResult::Exceeded {
            retry_after_seconds,
        } => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("Rate limit exceeded, retry after {}s", retry_after_seconds),
            );
            Err(crate::domain::error::AppError::ValidationError(format!(
                "Rate limit exceeded. Please try again in {} seconds.",
                retry_after_seconds
            )))
        }
        RateLimitResult::CooldownActive {
            retry_after_seconds,
        } => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("Cooldown active, retry after {}s", retry_after_seconds),
            );
            Err(crate::domain::error::AppError::ValidationError(format!(
                "Too many blocked queries. Please try again in {} seconds.",
                retry_after_seconds
            )))
        }
    }
}

/// Parses collection configuration JSON with error logging
fn parse_collection_config(
    config_json: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Result<serde_json::Value> {
    serde_json::from_str(config_json).map_err(|e| {
        log_and_return_error(
            logs,
            "SQL-RAG",
            &format!("Failed to parse collection config: {}", e),
            crate::domain::error::AppError::ValidationError(format!(
                "Invalid collection config: {}",
                e
            )),
        )
    })
}

/// Extracts configuration values from collection config JSON
struct CollectionConfig {
    db_conn_id: i64,
    allowlist_profile_id: i64,
    selected_tables: Vec<String>,
    external_llm_policy: ExternalLlmPolicy,
}

impl CollectionConfig {
    fn from_json(config: &serde_json::Value) -> Result<Self> {
        let db_conn_id = config["db_conn_id"].as_i64().ok_or_else(|| {
            crate::domain::error::AppError::ValidationError(
                "Missing db_conn_id in collection config".to_string(),
            )
        })?;

        let allowlist_profile_id = config["allowlist_profile_id"]
            .as_i64()
            .unwrap_or(DEFAULT_ALLOWLIST_PROFILE_ID);

        let selected_tables = config["selected_tables"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let external_llm_policy: ExternalLlmPolicy = config["external_llm_policy"]
            .as_str()
            .unwrap_or("always_block")
            .into();

        Ok(Self {
            db_conn_id,
            allowlist_profile_id,
            selected_tables,
            external_llm_policy,
        })
    }
}

/// Validates query plan against allowlist with error handling
fn validate_query_plan(
    validator: &AllowlistValidator,
    plan: &QueryPlan,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Result<QueryPlan> {
    let validation_result = validator.validate_plan(plan);

    if !validation_result.is_valid {
        let error_messages = format_validation_errors(&validation_result.errors);

        add_log(
            logs,
            "WARN",
            "SQL-RAG",
            &format!("Query validation failed: {:?}", error_messages),
        );

        return Err(crate::domain::error::AppError::ValidationError(format!(
            "Query validation failed: {}",
            error_messages
        )));
    }

    // Apply limit adjustment if needed
    Ok(
        if let Some(adjusted_limit) = validation_result.adjusted_limit {
            let mut adjusted_plan = plan.clone();
            adjusted_plan.limit = adjusted_limit;
            adjusted_plan
        } else {
            plan.clone()
        },
    )
}

/// Validates compiled SQL with error handling
fn validate_compiled_sql(
    validator: &AllowlistValidator,
    compiled: &crate::application::use_cases::sql_compiler::CompiledQuery,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Result<()> {
    let sql_validation = validator.validate_sql(&compiled.sql);

    if !sql_validation.is_valid {
        let error_messages = format_validation_errors(&sql_validation.errors);

        add_log(
            logs,
            "WARN",
            "SQL-RAG",
            &format!("SQL validation failed: {:?}", error_messages),
        );

        return Err(crate::domain::error::AppError::ValidationError(format!(
            "SQL validation failed: {}",
            error_messages
        )));
    }

    Ok(())
}

/// Converts database query result rows to QueryResult format for reranking
/// Each row is converted to a text representation for relevance scoring
fn convert_db_rows_to_candidates(
    rows: &[std::collections::HashMap<String, serde_json::Value>],
    table_name: &str,
) -> Vec<QueryResult> {
    rows.iter()
        .enumerate()
        .map(|(idx, row)| {
            // Convert row to a readable text format for reranking
            let content = row
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ");

            QueryResult {
                content,
                source_type: "db_row".to_string(),
                source_id: idx as i64,
                score: None,
                page_number: None,
                page_offset: None,
                doc_name: Some(table_name.to_string()),
            }
        })
        .collect()
}

/// Restores original row data from reranked candidates
fn restore_rows_from_candidates(
    candidates: Vec<QueryResult>,
    original_rows: &[std::collections::HashMap<String, serde_json::Value>],
) -> Vec<(
    usize,
    std::collections::HashMap<String, serde_json::Value>,
    Option<f32>,
)> {
    candidates
        .into_iter()
        .filter_map(|c| {
            let idx = c.source_id as usize;
            if idx < original_rows.len() {
                Some((idx, original_rows[idx].clone(), c.score))
            } else {
                None
            }
        })
        .collect()
}

/// Format SQL results as a readable context for LLM
fn format_sql_results_for_llm(
    rows: &[(
        usize,
        std::collections::HashMap<String, serde_json::Value>,
        Option<f32>,
    )],
    table_name: &str,
) -> String {
    if rows.is_empty() {
        return "No results found.".to_string();
    }

    // Collect all unique column names across all rows (sorted for consistency)
    let mut all_columns: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_row_idx, row, _score) in rows.iter() {
        for key in row.keys() {
            all_columns.insert(key.clone());
        }
    }

    let columns: Vec<String> = all_columns.into_iter().collect();

    // Build markdown table
    let mut table = String::new();
    table.push_str(&format!("Found {} result(s) from table '{}':\n\n", rows.len(), table_name));

    // Header row
    table.push_str("| ");
    for col in &columns {
        table.push_str(&format!("{} | ", col));
    }
    table.push('\n');

    // Separator row
    table.push_str("|");
    for _ in &columns {
        table.push_str("---|");
    }
    table.push('\n');

    // Data rows
    for (_row_idx, row, _score) in rows.iter() {
        table.push_str("| ");
        for col in &columns {
            let value = row.get(col).and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| {
                        // Handle multiline strings and escape pipe characters
                        let cleaned = s.replace('|', "\\|");
                        cleaned.replace(|c: char| c == '\n' || c == '\r', " ")
                    })
                } else if v.is_null() {
                    Some("NULL".to_string())
                } else {
                    Some(v.to_string().replace('|', "\\|"))
                }
            }).unwrap_or_else(|| "NULL".to_string());

            table.push_str(&format!("{} | ", value));
        }
        table.push('\n');
    }

    table
}

/// Detect if query is in Indonesian (basic keyword detection)
fn detect_indonesian(query: &str) -> bool {
    let indonesian_keywords = [
        "tampilkan",
        "cari",
        "semua",
        "data",
        "yang",
        "dengan",
        "dari",
        "adalah",
        "berapa",
        "jumlah",
        "daftar",
        "user",
        "pengguna",
        "alamat",
        "nama",
        "id",
        "filter",
        "berdasarkan",
        "urutkan",
        "terbesar",
        "terkecil",
    ];

    let query_lower = query.to_lowercase();
    indonesian_keywords
        .iter()
        .any(|&keyword| query_lower.contains(keyword))
}

/// Generate a fallback response when LLM times out or fails
/// Returns the raw results in a readable format
fn generate_fallback_response(results_context: &str, is_indonesian: bool) -> String {
    if results_context.contains("No results found") || results_context.is_empty() {
        if is_indonesian {
            "Tidak ada hasil yang ditemukan untuk query Anda.".to_string()
        } else {
            "No results found for your query.".to_string()
        }
    } else {
        if is_indonesian {
            format!(
                "Berikut hasil query Anda:\n\n{}\n\n(Silakan lihat bagian sumber untuk detail lengkap)",
                results_context
            )
        } else {
            format!(
                "Here are your query results:\n\n{}\n\n(Please see the source section for detailed results)",
                results_context
            )
        }
    }
}

/// Generate natural language response using LLM
async fn generate_nl_response(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    results_context: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    // Optional conversation history for contextual responses
    conversation_history: Option<&str>,
) -> String {
    // Detect language and set response language
    let is_indonesian = detect_indonesian(user_query);
    let response_lang_instruction = if is_indonesian {
        "Respond in Indonesian (Bahasa Indonesia)."
    } else {
        "Respond in English."
    };

    let system_prompt = format!(
        r#"You are a friendly and helpful database assistant. {}
Your task is to present SQL query results in a CLEAR, STRUCTURED format.

## MANDATORY OUTPUT FORMAT

### For RESULTS FOUND (1+ rows):
**Line 1**: Single sentence summary (e.g., "Found 5 users:" or "Nemu 5 data:")
**Line 2**: Empty line
**Line 3+**: Markdown table with ALL results
**Last**: Optional one-line note

### For NO RESULTS:
**Line 1**: Simple one-line message (e.g., "No data found." or "Gak ada data.")

## TABLE FORMAT RULES
- MUST use markdown table format with | separators
- First row: column headers
- Second row: separator line with ---|
- Keep column names SHORT (use aliases if needed)
- Only show relevant columns (skip internal IDs unless asked)

## EXAMPLES

### Indonesian - Hasil Ditemukan:
```
Nemu 3 user dengan role admin:

| nama | email | role |
|------|-------|------|
| Budi | budi@mail.com | admin |
| Siti | siti@mail.com | admin |
| Andi | andi@mail.com | admin |
```

### English - Results Found:
```
Found 5 orders:

| order_id | total | status |
|----------|-------|--------|
| ORD001 | $150 | completed |
| ORD002 | $75 | pending |
| ORD003 | $200 | completed |
```

### Indonesian - Tidak Ada Hasil:
```
Hmm, gak ada data yang cocok dengan kriteria tersebut.
```

## CRITICAL RULES
1. NEVER write long paragraphs describing each row
2. NEVER repeat column names in sentences
3. ALWAYS use tables for 2+ results
4. Keep summary to ONE sentence only
5. Keep note to ONE line max (or skip it)
6. Format dates consistently (YYYY-MM-DD preferred)

## RESPONSE MUST FIT THIS PATTERN:
[Summary sentence]

| col1 | col2 | col3 |
|------|------|------|
| data | data | data |

[Optional note]

That's it. Be concise!"#,
        response_lang_instruction
    );

    let user_prompt = if let Some(history) = conversation_history {
        format!(
            r#"Previous conversation:
{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's latest question based on these results and the conversation context above."#,
            history,
            user_query.trim(),
            results_context
        )
    } else {
        format!(
            r#"User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's question based on these results."#,
            user_query.trim(),
            results_context
        )
    };

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Generating NL response with LLM (is_indonesian: {}, timeout: {}s)",
            is_indonesian, NL_RESPONSE_TIMEOUT_SECS
        ),
    );

    use std::time::Duration;
    use tokio::time::timeout;

    // Wrap LLM call with timeout to prevent hanging
    let llm_result = timeout(
        Duration::from_secs(NL_RESPONSE_TIMEOUT_SECS),
        llm_client.generate(config, &system_prompt, &user_prompt),
    )
    .await;

    match llm_result {
        Ok(Ok(response)) => {
            // Clean up the response
            let cleaned = response.trim().to_string();
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("LLM response generated: {} chars", cleaned.len()),
            );
            cleaned
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM response generation failed: {}, using fallback", e),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "LLM response generation timed out after {}s, using fallback",
                    NL_RESPONSE_TIMEOUT_SECS
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
    }
}

/// Generate natural language response with few-shot prompt
async fn generate_nl_response_with_few_shot(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    results_context: &str,
    few_shot_prompt: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    // Optional conversation history for contextual responses
    conversation_history: Option<&str>,
) -> String {
    // Detect language and set response language
    let is_indonesian = detect_indonesian(user_query);
    let response_lang_instruction = if is_indonesian {
        "Respond in Indonesian (Bahasa Indonesia)."
    } else {
        "Respond in English."
    };

    let system_prompt = format!(
        r#"You are a friendly and helpful database assistant. {}
Your task is to present SQL query results in a CLEAR, STRUCTURED format.

## MANDATORY OUTPUT FORMAT

### For RESULTS FOUND (1+ rows):
**Line 1**: Single sentence summary (e.g., "Found 5 users:" or "Nemu 5 data:")
**Line 2**: Empty line
**Line 3+**: Markdown table with ALL results
**Last**: Optional one-line note

### For NO RESULTS:
**Line 1**: Simple one-line message (e.g., "No data found." or "Gak ada data.")

## TABLE FORMAT RULES
- MUST use markdown table format with | separators
- First row: column headers
- Second row: separator line with ---|
- Keep column names SHORT (use aliases if needed)
- Only show relevant columns (skip internal IDs unless asked)

## EXAMPLES

### Indonesian - Hasil Ditemukan:
```
Nemu 3 user dengan role admin:

| nama | email | role |
|------|-------|------|
| Budi | budi@mail.com | admin |
| Siti | siti@mail.com | admin |
| Andi | andi@mail.com | admin |
```

### English - Results Found:
```
Found 5 orders:

| order_id | total | status |
|----------|-------|--------|
| ORD001 | $150 | completed |
| ORD002 | $75 | pending |
| ORD003 | $200 | completed |
```

### Indonesian - Tidak Ada Hasil:
```
Hmm, gak ada data yang cocok dengan kriteria tersebut.
```

## CRITICAL RULES
1. NEVER write long paragraphs describing each row
2. NEVER repeat column names in sentences
3. ALWAYS use tables for 2+ results
4. Keep summary to ONE sentence only
5. Keep note to ONE line max (or skip it)
6. Format dates consistently (YYYY-MM-DD preferred)

## RESPONSE MUST FIT THIS PATTERN:
[Summary sentence]

| col1 | col2 | col3 |
|------|------|------|
| data | data | data |

[Optional note]

That's it. Be concise!"#,
        response_lang_instruction
    );

    // Combine few-shot examples with the query and results
    let user_prompt = if let Some(history) = conversation_history {
        format!(
            r#"Previous conversation:
{}

{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's latest question based on these results and the conversation context above."#,
            history,
            few_shot_prompt,
            user_query.trim(),
            results_context
        )
    } else {
        format!(
            r#"{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's question based on these results."#,
            few_shot_prompt,
            user_query.trim(),
            results_context
        )
    };

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Generating NL response with few-shot prompt (is_indonesian: {}, prompt: {} chars, timeout: {}s)",
            is_indonesian,
            few_shot_prompt.len(),
            NL_RESPONSE_TIMEOUT_SECS
        ),
    );

    use std::time::Duration;
    use tokio::time::timeout;

    // Wrap LLM call with timeout to prevent hanging
    let llm_result = timeout(
        Duration::from_secs(NL_RESPONSE_TIMEOUT_SECS),
        llm_client.generate(config, &system_prompt, &user_prompt),
    )
    .await;

    match llm_result {
        Ok(Ok(response)) => {
            // Clean up the response
            let cleaned = response.trim().to_string();
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("Few-shot LLM response generated: {} chars", cleaned.len()),
            );
            // Log the actual response content for debugging
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!(
                    "Response content preview: {}",
                    &cleaned[..cleaned.len().min(200)]
                ),
            );
            cleaned
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Few-shot LLM response generation failed: {}, using fallback",
                    e
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Few-shot LLM response generation timed out after {}s, using fallback",
                    NL_RESPONSE_TIMEOUT_SECS
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
    }
}

// ============================================================================
// BATCHED TEMPLATE RETRIEVAL (Feature 31 Enhancement)
// ============================================================================

/// Load templates with batching to improve efficiency when there are many templates
/// Returns matched templates sorted by score (highest first)
async fn load_templates_with_batching(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    profile_id: i64,
    query: &str,
    detected_tables: &[String],
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Vec<crate::application::use_cases::template_matcher::TemplateMatch> {
    use crate::application::use_cases::template_matcher::{TemplateMatch, TemplateMatcher};
    use std::collections::HashSet;

    let mut all_matches: Vec<TemplateMatch> = Vec::new();
    let mut offset = 0i64;
    let mut batch_count = 0;

    let query_lower = query.to_lowercase();
    let query_words: HashSet<&str> = query_lower.split_whitespace().collect();
    let detected_table_set: HashSet<String> =
        detected_tables.iter().map(|s| s.to_lowercase()).collect();

    loop {
        // Fetch next batch
        let batch = match repository
            .list_query_templates_batched(
                Some(profile_id),
                offset,
                TEMPLATE_BATCH_SIZE,
                true, // Prioritize pattern-agnostic templates
            )
            .await
        {
            Ok(b) => b,
            Err(e) => {
                add_log(
                    logs,
                    "WARN",
                    "SQL-RAG",
                    &format!("Failed to load template batch {}: {}", batch_count + 1, e),
                );
                break;
            }
        };

        if batch.is_empty() {
            break; // No more templates
        }

        batch_count += 1;
        if batch_count > MAX_TEMPLATE_BATCHES {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Reached max batch limit ({}), stopping retrieval",
                    MAX_TEMPLATE_BATCHES
                ),
            );
            break;
        }

        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Loaded template batch {} with {} templates",
                batch_count,
                batch.len()
            ),
        );

        // Create matcher for this batch and score templates
        let matcher = TemplateMatcher::new(batch.clone());

        let batch_matches: Vec<TemplateMatch> = batch
            .iter()
            .map(|template| {
                let (score, reason) = matcher.score_template(
                    &query_lower,
                    &query_words,
                    &detected_table_set,
                    template,
                );
                TemplateMatch {
                    template: template.clone(),
                    score,
                    reason,
                }
            })
            .filter(|m| m.score > 0.0)
            .collect();

        all_matches.extend(batch_matches);

        // Check if we have a good enough match
        let best_score = all_matches.iter().map(|m| m.score).fold(0.0f32, f32::max);

        if best_score >= TEMPLATE_STOP_THRESHOLD {
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!(
                    "Found good match (score: {:.2}) after {} batches, stopping retrieval",
                    best_score, batch_count
                ),
            );
            break;
        }

        offset += TEMPLATE_BATCH_SIZE;
    }

    // Sort by score and return top-K
    all_matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.template.priority.cmp(&a.template.priority))
    });

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Batched retrieval complete: {} batches, {} total matches, best score: {:.2}",
            batch_count,
            all_matches.len(),
            all_matches.first().map(|m| m.score).unwrap_or(0.0)
        ),
    );

    all_matches.truncate(MAX_TEMPLATES_FOR_USER);
    all_matches
}

// ============================================================================
// SEMANTIC TEMPLATE RETRIEVAL WITH LLM (Cross-Language Matching)
// ============================================================================

/// LLM timeout for semantic matching (15 seconds - increased for external LLM providers)
const SEMANTIC_LLM_TIMEOUT_SECS: u64 = 15;

/// LLM timeout for NL response generation (30 seconds)
const NL_RESPONSE_TIMEOUT_SECS: u64 = 30;

/// Load templates using both keyword AND semantic matching (LLM-based)
/// Returns fused TemplateMatch results sorted by final score (highest first)
async fn load_templates_with_semantic_matching(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    profile_id: i64,
    query: &str,
    detected_tables: &[String],
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    llm_config: &crate::domain::llm_config::LLMConfig,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Vec<TemplateMatch> {
    use std::time::Duration;
    use tokio::time::timeout;

    // 1. Load all enabled templates (no batching for semantic path)
    let all_templates = match repository.list_query_templates(Some(profile_id)).await {
        Ok(templates) => templates
            .into_iter()
            .filter(|t| t.is_enabled)
            .collect::<Vec<_>>(),
        Err(e) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("Failed to load templates: {}", e),
            );
            return Vec::new();
        }
    };

    if all_templates.is_empty() {
        return Vec::new();
    }

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Loaded {} templates for semantic matching",
            all_templates.len()
        ),
    );

    // 2. Wrap LLM client to implement our LLMClient trait
    struct LLMClientWrapper {
        client: Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
        config: crate::domain::llm_config::LLMConfig,
    }

    // Implement our semantic_matcher LLMClient trait
    impl crate::application::use_cases::semantic_matcher::LLMClient for LLMClientWrapper {
        fn generate(
            &self,
            _config: &LLMConfig,
            system_prompt: &str,
            user_prompt: &str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'static>>
        {
            // Clone needed data for the async block
            let client = self.client.clone();
            let config = self.config.clone();
            let system_prompt = system_prompt.to_string();
            let user_prompt = user_prompt.to_string();

            Box::pin(async move {
                // Use the wrapped client to generate
                client
                    .generate(&config, &system_prompt, &user_prompt)
                    .await
                    .map_err(|e| crate::domain::error::AppError::LLMError(e.to_string()))
            })
        }
    }

    let wrapper = LLMClientWrapper {
        client: llm_client.clone(),
        config: llm_config.clone(),
    };

    let semantic_matcher = SemanticMatcher::new(Arc::new(wrapper), llm_config.clone());

    // 3. Parallel execution: keyword + semantic matching
    let keyword_future = async {
        let matcher = TemplateMatcher::new(all_templates.clone());
        Ok::<_, AppError>(matcher.find_matches(query, detected_tables, usize::MAX))
    };

    let semantic_future = async {
        timeout(
            Duration::from_secs(SEMANTIC_LLM_TIMEOUT_SECS),
            // Use batched method for 100+ templates
            semantic_matcher.match_templates_batched(&all_templates, query, detected_tables),
        )
        .await
    };

    // 4. Run both in parallel
    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        "Running keyword and semantic matching in parallel...",
    );

    let (keyword_result, semantic_result) = tokio::join!(keyword_future, semantic_future);

    // 5. Extract results with error handling
    let keyword_matches = keyword_result.unwrap_or_default();

    let semantic_matches = match semantic_result {
        Ok(Ok(matches)) => {
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("LLM semantic matching succeeded: {} matches", matches.len()),
            );
            matches
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM semantic matching failed: {}, using keyword-only", e),
            );
            Vec::new()
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "LLM semantic matching timed out after {}s, falling back to keyword-only",
                    SEMANTIC_LLM_TIMEOUT_SECS
                ),
            );
            Vec::new()
        }
    };

    // 6. Fuse scores from both methods
    let fused_matches =
        fuse_keyword_semantic_matches(&all_templates, keyword_matches, semantic_matches);

    // 7. Sort by final score and return top-N
    let mut matches = fused_matches;
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.template.priority.cmp(&a.template.priority))
    });

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Semantic matching complete: {} total matches, best score: {:.2}",
            matches.len(),
            matches.first().map(|m| m.score).unwrap_or(0.0)
        ),
    );

    matches.truncate(MAX_TEMPLATES_FOR_USER);
    matches
}

// ============================================================================
// TEMPLATE-FIRST QUERY FUNCTIONS (Feature 31 Enhancement)
// ============================================================================

/// Minimum score threshold for using template-first approach
const TEMPLATE_MATCH_THRESHOLD: f32 = 0.5;

/// Maximum number of templates to show to user
const MAX_TEMPLATES_FOR_USER: usize = 3;

/// Use LLM to select the best template and extract parameters from user query
async fn select_template_with_llm(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    matched_templates: &[TemplateMatch],
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    schema_context: Option<&str>,
) -> Option<LlmTemplateSelection> {
    if matched_templates.is_empty() {
        return None;
    }

    // Build template options for LLM
    let template_options: String = matched_templates
        .iter()
        .enumerate()
        .map(|(idx, tm)| {
            let pattern_type_label = if tm.template.is_pattern_agnostic {
                format!(" [PATTERN-AGNOSTIC]")
            } else {
                String::new()
            };
            format!(
                "{}. Template: \"{}\"{} (ID: {})\n   Example: \"{}\"\n   Pattern: {}",
                idx + 1,
                tm.template.name,
                pattern_type_label,
                tm.template.id,
                tm.template.example_question,
                tm.template.query_pattern
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Build enhanced prompt with schema context
    let system_prompt = if let Some(schema) = schema_context {
        format!(
            r#"You are an intelligent SQL query adapter that transforms template patterns into actual queries.

DATABASE SCHEMA:
{schema}

PATTERN-AGNOSTIC TEMPLATE ADAPTATION:
Templates marked [PATTERN-AGNOSTIC] are abstract SQL patterns that work across ANY table.
Your task is to ADAPT these patterns to the ACTUAL table and columns from the schema.

ADAPTATION RULES:
1. Templates are ABSTRACT PATTERNS - not tied to specific tables
2. Your task is to ADAPT the template to use the ACTUAL table and columns from the schema
3. Extract filter values from the user's query
4. DETECT which table the user is querying from their query
5. If the template WHERE clause uses a generic column (e.g., "role", "id"), replace it with the RELEVANT column from the detected schema
6. Match user's intent to the correct column (e.g., "merchant id" → merchant_id column, "user id" → id column)

EXAMPLE TRANSFORMATION:
Template: SELECT {{{{columns}}}} FROM {{{{table}}}} WHERE role = '{{{{search_term}}}}'
User Query: "cari data dari table ms_loan_merchant dengan merchant id M_B8RjeABb"
Detected Table: ms_loan_merchant
Detected Columns: [id, merchant_id, loan_channel_id, status, ...]

You should respond:
{{
  "selected_template_id": 1,
  "extracted_params": {{
    "search_term": "M_B8RjeABb"
  }},
  "modified_where_clause": "WHERE merchant_id = '{{{{search_term}}}}'",
  "detected_table": "ms_loan_merchant",
  "confidence": 0.95,
  "reasoning": "Detected table ms_loan_merchant from user query, adapted template's generic role filter to use merchant_id column from actual schema"
}}

Then the system will:
1. Replace {{{{table}}}} with ms_loan_merchant (from detected_table)
2. Replace {{{{columns}}}} with the allowed columns
3. Replace {{{{search_term}}}} with 'M_B8RjeABb'
Final SQL: SELECT ... FROM ms_loan_merchant WHERE merchant_id = 'M_B8RjeABb'

ANOTHER EXAMPLE (no adaptation needed):
Template: WHERE role = '{{{{search_term}}}}'
User Query: "cari user dengan role admin"
Detected Table: users
Detected Columns: [id, role, name, email, ...]

You should respond:
{{
  "selected_template_id": 1,
  "extracted_params": {{
    "search_term": "admin"
  }},
  "modified_where_clause": null,
  "detected_table": "users",
  "confidence": 0.95,
  "reasoning": "Template WHERE clause is already correct for this query"
}}

CTE (Common Table Expression) QUERIES:
For templates starting with "WITH ...", these are CTE queries. Handle them specially:
- DO NOT modify WHERE clause (it's embedded in the CTE structure)
- Extract ALL parameter placeholders: {{{{event_type}}}}, {{{{status}}}}, {{{{merchant_id}}}}, etc.
- Each placeholder in the template must have a corresponding extracted_params entry

CTE Example:
Template: WITH filtered AS (SELECT * FROM {{{{table}}}} WHERE event_type = '{{{{event_type}}}}' AND status = '{{{{status}}}}') SELECT * FROM filtered
User Query: "cari loan merchant dengan event type MANUAL dan status DELETED"
You should respond:
{{
  "selected_template_id": 6,
  "extracted_params": {{
    "event_type": "MANUAL",
    "status": "DELETED"
  }},
  "modified_where_clause": null,
  "detected_table": "ms_loan_merchant",
  "confidence": 0.95,
  "reasoning": "CTE query with multiple conditions - extracted event_type and status from user query"
}}

IMPORTANT:
- Keep {{{{columns}}}} and {{{{table}}}} placeholders intact - don't modify them
- The system will use detected_table to replace {{{{table}}}} placeholder
- Only modify the WHERE clause column name if needed for the actual schema (NON-CTE queries only)
- Keep parameter placeholders (like {{{{search_term}}}}) intact - the system will replace them
- For pattern-agnostic templates, ALWAYS check if the column name needs adaptation
- ALWAYS include detected_table field with the table name detected from user query
- For CTE queries, extract ALL parameters - don't try to modify WHERE clause

PARAMETER EXTRACTION RULES (CRITICAL):
- extracted_params values MUST be RAW VALUES ONLY - NO curly braces
- DO NOT include the placeholder, bracket or anything syntax in the value
- For multiple parameters, extract each one separately:
- ✅ CORRECT: "event_type": "MANUAL", "status": "DELETED"
- ❌ INCORRECT: "event_type": "{{MANUAL}}"
- The system will automatically replace each placeholder with its value

ORDER BY AND GROUP BY PARAMETER EXTRACTION:
When the template includes ORDER BY or GROUP BY placeholders, extract them from the user's query:

ORDER BY Detection:
- Look for keywords: "urutkan", "order by", "sort", "ascending", "descending", "ASC", "DESC"
- Extract the column name for {{{{order_by_column}}}}
- Extract the direction for {{{{sort_direction}}}} (ASC or DESC)
- Default to ASC if not specified

GROUP BY Detection:
- Look for keywords: "per", "group by", "jumlah per", "count per"
- Extract the column name for {{{{group_by_column}}}}

ORDER BY Example:
Template: SELECT {{{{columns}}}} FROM {{{{table}}}} WHERE role = '{{{{search_term}}}}' ORDER BY {{{{order_by_column}}}} {{{{sort_direction}}}}
User Query: "tampilkan user dengan role admin, urutkan by name descending"
You should respond:
{{
  "selected_template_id": 1,
  "extracted_params": {{
    "search_term": "admin",
    "order_by_column": "name",
    "sort_direction": "DESC"
  }},
  "modified_where_clause": null,
  "detected_table": "users",
  "confidence": 0.95,
  "reasoning": "Extracted search_term, order_by_column from 'name', and sort_direction from 'descending'"
}}

GROUP BY Example:
Template: SELECT {{{{group_by_column}}}}, COUNT(*) as count FROM {{{{table}}}} GROUP BY {{{{group_by_column}}}}
User Query: "tampilkan jumlah user per role"
You should respond:
{{
  "selected_template_id": 2,
  "extracted_params": {{
    "group_by_column": "role"
  }},
  "modified_where_clause": null,
  "detected_table": "users",
  "confidence": 0.95,
  "reasoning": "Extracted group_by_column from 'per role' in user query"
}}

Respond in JSON format only:
{{
  "selected_template_id": <id>,
  "extracted_params": {{
    "<param_name>": "<raw value without braces>",
    ...
  }},
  "modified_where_clause": "<complete modified WHERE clause OR null if template is correct>",
  "detected_table": "<table name detected from user query>",
  "confidence": <0.0-1.0>,
  "reasoning": "<explain your parameter extraction, WHERE clause modifications, and table detection>"
}}"#,
            schema = schema
        )
    } else {
        // Fallback to basic prompt without schema context
        r#"You are a SQL query template matcher. Your task is to:
1. Select the BEST matching template for the user's natural language query
2. Extract parameter values from the user's query
3. Detect which table the user is querying from their query

NOTE: Templates marked [PATTERN-AGNOSTIC] are abstract patterns that can work across different tables.

ORDER BY AND GROUP BY PARAMETER EXTRACTION:
When the template includes ORDER BY or GROUP BY placeholders, extract them from the user's query:

ORDER BY Detection:
- Look for keywords: "urutkan", "order by", "sort", "ascending", "descending", "ASC", "DESC"
- Extract the column name for {{{{order_by_column}}}}
- Extract the direction for {{{{sort_direction}}}} (ASC or DESC)
- Default to ASC if not specified

GROUP BY Detection:
- Look for keywords: "per", "group by", "jumlah per", "count per"
- Extract the column name for {{{{group_by_column}}}}

Respond in JSON format only:
{
  "selected_template_id": <id>,
  "extracted_params": {
    "<param_name>": "<value>",
    ...
  },
  "modified_where_clause": null,
  "detected_table": "<table name detected from user query, or null if unclear>",
  "confidence": <0.0-1.0>,
  "reasoning": "<brief explanation>"
}

Parameter placeholders in templates:
- {columns} - auto-filled, skip
- {table} - auto-filled, skip
- {order_by_column} - column name for ORDER BY clause
- {sort_direction} - ASC or DESC for ORDER BY
- {group_by_column} - column name for GROUP BY clause
- 'value' or {search_term} - literal values to extract from user query
- Look for filter conditions like "WHERE column = 'value'"

CRITICAL - Parameter Extraction Rules:
- extracted_params values MUST be RAW VALUES ONLY - NO curly braces
- ✅ CORRECT: "search_term": "M_B8RjeABb", "order_by_column": "name", "sort_direction": "DESC"
- ❌ INCORRECT: "search_term": "{M_B8RjeABb}"

Extract the actual values the user wants to filter by.
Detect the table name from phrases like "from table X", "data dari table X", etc."#.to_string()
    };

    let user_prompt = format!(
        r#"User Query: "{}"

Available Templates:
{}

Select the best template and extract any parameter values from the user's query."#,
        user_query, template_options
    );

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Asking LLM to select from {} templates",
            matched_templates.len()
        ),
    );

    match llm_client
        .generate(config, &system_prompt, &user_prompt)
        .await
    {
        Ok(response) => {
            // Parse JSON response
            let cleaned = response
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            match serde_json::from_str::<LlmTemplateSelection>(cleaned) {
                Ok(selection) => {
                    add_log(
                        logs,
                        "DEBUG",
                        "SQL-RAG",
                        &format!(
                            "LLM selected template {} with confidence {:.2}",
                            selection.selected_template_id, selection.confidence
                        ),
                    );
                    Some(selection)
                }
                Err(e) => {
                    add_log(
                        logs,
                        "WARN",
                        "SQL-RAG",
                        &format!("Failed to parse LLM template selection: {}", e),
                    );
                    // Fallback to highest scoring template
                    Some(LlmTemplateSelection {
                        selected_template_id: matched_templates[0].template.id,
                        extracted_params: std::collections::HashMap::new(),
                        modified_where_clause: None,
                        detected_table: None,
                        confidence: matched_templates[0].score,
                        reasoning: "Fallback to highest matching template".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM template selection failed: {}", e),
            );
            // Fallback to highest scoring template
            Some(LlmTemplateSelection {
                selected_template_id: matched_templates[0].template.id,
                extracted_params: std::collections::HashMap::new(),
                modified_where_clause: None,
                detected_table: None,
                confidence: matched_templates[0].score,
                reasoning: "Fallback due to LLM error".to_string(),
            })
        }
    }
}

/// Build SQL from template pattern with extracted parameters
fn build_sql_from_template(
    template: &QueryTemplate,
    selection: &LlmTemplateSelection,
    allowed_columns: &[String],
    table_name: &str,
    limit: i32,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Option<(String, String)> {
    let mut sql = template.query_pattern.clone();

    // Detect if this is a CTE (Common Table Expression) query
    let is_cte_query = sql.trim().to_uppercase().starts_with("WITH ");

    if is_cte_query {
        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            "Detected CTE query - using CTE-safe parameter replacement",
        );
    }

    // Replace {columns} with allowed columns
    // For CTE queries, only replace if placeholder exists (columns are usually hardcoded in CTE)
    let columns_str = if allowed_columns.is_empty() {
        "*".to_string()
    } else {
        allowed_columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if sql.contains("{columns}") {
        sql = sql.replace("{columns}", &columns_str);
    }

    // Replace {table} with actual table name
    // Use detected_table from LLM if available (for pattern-agnostic templates)
    let final_table = selection
        .detected_table
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or(table_name);
    sql = sql.replace("{table}", &format!("\"{}\"", final_table));

    // Apply modified WHERE clause if provided by LLM
    // SKIP WHERE replacement for CTE queries - conditions are embedded in the template via params
    if !is_cte_query {
        if let Some(where_clause) = &selection.modified_where_clause {
            // Replace the existing WHERE clause with the modified one
            // Rust regex doesn't support lookahead, so we use a different approach:
            // 1. Find WHERE position
            // 2. Find end of WHERE clause (LIMIT/ORDER BY/GROUP BY or end)
            // 3. Replace that portion

            let where_pos = sql.find("WHERE").or_else(|| sql.find("where"));
            if let Some(pos) = where_pos {
                // Get everything before WHERE
                let before_where = sql[..pos].trim_end();

                // Find where the WHERE clause ends (look for LIMIT, ORDER BY, GROUP BY)
                let after_where = &sql[pos..];
                let where_end = after_where
                    .find(|c: char| {
                        c.to_ascii_uppercase() == 'L'
                            || c.to_ascii_uppercase() == 'O'
                            || c.to_ascii_uppercase() == 'G'
                    })
                    .and_then(|p| {
                        let rest = &after_where[p..];
                        // Check if it's LIMIT, ORDER BY, or GROUP BY (case-insensitive)
                        if rest.to_uppercase().starts_with("LIMIT ")
                            || rest.to_uppercase().starts_with("ORDER BY ")
                            || rest.to_uppercase().starts_with("GROUP BY ")
                        {
                            Some(pos + p)
                        } else {
                            None
                        }
                    });

                // Build the new SQL
                if let Some(end) = where_end {
                    // Preserve LIMIT/ORDER BY/GROUP BY
                    let suffix = &after_where[end..];
                    sql = format!("{} {} {}", before_where, where_clause, suffix);
                } else {
                    // No LIMIT/ORDER BY/GROUP BY, just replace everything from WHERE
                    sql = format!("{} {}", before_where, where_clause);
                }
            }
        }
    } else if selection.modified_where_clause.is_some() {
        // For CTE queries, log if WHERE modification was requested but skipped
        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            "Skipping WHERE modification for CTE query - use parameter placeholders instead",
        );
    }

    // Replace extracted parameters
    // IMPORTANT: Parameter values from LLM should be RAW VALUES only (no curly braces)
    // This supports multiple named parameters: {event_type}, {status}, {merchant_id}, etc.
    // NOTE: Templates in Rust strings use {{ and }} for literal braces, so we need to
    // check both {{{param}}} (for templates stored as raw) and {{param}} (for escaped)
    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Replacing params: {:?}", selection.extracted_params),
    );

    for (param, value) in &selection.extracted_params {
        // Handle different parameter types
        let replacement = match param.as_str() {
            // Column names for ORDER BY and GROUP BY - treat as identifiers
            "order_by_column" | "group_by_column" => {
                // Remove any existing quotes AND curly braces, then wrap in double quotes
                let cleaned = value.trim()
                    .trim_matches('\'')
                    .trim_matches('"')
                    .trim_start_matches('{')
                    .trim_end_matches('}');
                format!("\"{}\"", cleaned)
            }
            // Sort direction - keep as-is (ASC/DESC keywords)
            "sort_direction" => {
                value.to_uppercase()
            }
            // All other parameters - escape single quotes AND strip curly braces for SQL injection prevention
            _ => {
                let cleaned = value.trim()
                    .trim_start_matches('{')
                    .trim_end_matches('}');
                cleaned.replace('\'', "''")
            }
        };

        // Try both {{param}} and {param} patterns
        // Templates may be stored with double braces for escaping in Rust strings
        // IMPORTANT: Check double braces FIRST because {param} is a substring of {{param}}
        let single_brace = format!("{{{}}}", param);  // {param}
        let double_brace = format!("{{{{{}}}}}", param); // {{param}}

        // First try double brace pattern (must check first - single brace is substring of double)
        if sql.contains(&double_brace) {
            sql = sql.replace(&double_brace, &replacement);
        }
        // Then try single brace pattern
        else if sql.contains(&single_brace) {
            sql = sql.replace(&single_brace, &replacement);
        }
    }

    // Check for unreplaced placeholders and warn
    let mut unreplaced: Vec<String> = Vec::new();
    let mut start = 0;
    while let Some(open_pos) = sql[start..].find('{') {
        let abs_open = start + open_pos;
        if let Some(close_offset) = sql[abs_open..].find('}') {
            let placeholder = &sql[abs_open..abs_open + close_offset + 1];
            let inner = &placeholder[1..placeholder.len() - 1]; // Content between { and }

            // Only flag if it looks like a genuine parameter placeholder:
            // - Contains only alphanumeric chars and underscores (valid param name)
            // - Doesn't contain special chars like @, ., -, etc. (indicates it's a value, not a param)
            // - Length is reasonable (param names are typically short)
            if !inner.is_empty()
                && inner.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !placeholder.contains(':')
                && !placeholder.contains('"')
                && !placeholder.contains(' ')
                && placeholder.len() < 30
            {
                unreplaced.push(placeholder.to_string());
            }
            start = abs_open + close_offset + 1;
        } else {
            break;
        }
    }

    if !unreplaced.is_empty() {
        add_log(
            logs,
            "WARN",
            "SQL-RAG",
            &format!(
                "Template has unreplaced placeholders: {:?}. LLM may need to extract these values.",
                unreplaced
            ),
        );
    }

    // Add LIMIT if not present (for both regular and CTE queries)
    // For CTE, LIMIT goes at the end of outer SELECT
    if !sql.to_lowercase().contains("limit") {
        sql = format!("{} LIMIT {}", sql.trim_end_matches(';').trim(), limit);
    }

    // Build description
    let description = format!(
        "Template query: {} ({}){}",
        template.name,
        if selection.modified_where_clause.is_some() && !is_cte_query {
            format!(
                "modified WHERE: {}",
                selection.modified_where_clause.as_ref().unwrap()
            )
        } else if selection.extracted_params.is_empty() {
            "no params".to_string()
        } else {
            selection
                .extracted_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        },
        if is_cte_query { " [CTE]" } else { "" }
    );

    Some((sql, description))
}

/// Hash a query string for feedback lookup
fn hash_query(query: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let normalized = query.to_lowercase().trim().to_string();
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Check for user feedback/preference for this query
async fn get_user_template_preference(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    query_hash: &str,
    collection_id: i64,
) -> Option<i64> {
    // Query the db_query_template_feedback table for user preference
    repository
        .get_preferred_template(query_hash, collection_id)
        .await
        .ok()
        .flatten()
}

/// Build schema context for LLM parameter extraction
fn build_schema_context_for_llm(
    _default_table_name: &str,
    _selected_columns: &[String],
    all_selected_columns: &std::collections::HashMap<String, Vec<String>>,
) -> String {
    let mut context = String::from("AVAILABLE TABLES AND COLUMNS:\n");
    context.push_str("(You must detect which table the user is querying from their question)\n\n");

    // List ALL available tables with their columns
    for (table_name, cols) in all_selected_columns {
        context.push_str(&format!("Table: {}\n", table_name));
        context.push_str("  Columns:\n");
        if cols.is_empty() {
            context.push_str("    (all columns accessible)\n");
        } else {
            for col in cols {
                context.push_str(&format!("    - {}\n", col));
            }
        }
        context.push_str("\n");
    }

    context.push_str("IMPORTANT:\n");
    context.push_str("- Detect the correct table from user's query keywords\n");
    context.push_str("- Match user's filter column to actual column names in that table\n");
    context.push_str("- Examples: 'merchant id' → merchant_id, 'user role' → role, 'loan channel' → loan_channel_id\n");

    context
}

/// Build few-shot examples for NATURAL LANGUAGE responses (not SQL generation)
/// This creates conversational examples to guide the LLM in responding to users
fn build_nl_few_shot_examples(user_query: &str, matched_templates: &[TemplateMatch]) -> String {
    let is_indonesian = detect_indonesian(user_query);

    // Build conversational examples based on the matched templates
    let mut examples = String::from("## Example Response Format\n\n");

    // Add a complete table example
    if is_indonesian {
        examples.push_str(
            r#"### Contoh Format WAJIB

**Query**: Cari semua user dengan role admin

**Response BENAR**:
```
Nemu 3 user dengan role admin:

| nama | email | role |
|------|-------|------|
| Budi | budi@mail.com | admin |
| Siti | siti@mail.com | admin |
| Andi | andi@mail.com | admin |
```

**Response SALAH** (jangan seperti ini):
```
Oke, aku nemu 3 user dengan role admin. Ada user Budi dengan email budi@mail.com dan role admin. Ada juga user Siti dengan email siti@mail.com dan role admin. Terakhir ada user Andi dengan email andi@mail.com dan role admin. Semuanya aktif.
```

"#,
        );
    } else {
        examples.push_str(
            r#"### Required Format

**Query**: Find all users with admin role

**CORRECT Response**:
```
Found 3 users with admin role:

| name | email | role |
|------|-------|------|
| John | john@mail.com | admin |
| Jane | jane@mail.com | admin |
| Bob | bob@mail.com | admin |
```

**WRONG Response** (don't do this):
```
I found 3 users with admin role. There's John with email john@mail.com and role admin. There's also Jane with email jane@mail.com and role admin. Finally there's Bob with email bob@mail.com and role admin. All are active.
```

"#,
        );
    }

    // Take up to 2 templates and create NL examples from them
    for (idx, template_match) in matched_templates.iter().take(2).enumerate() {
        let template = &template_match.template;

        if is_indonesian {
            writeln!(
                examples,
                "### Contoh {}\n**Pertanyaan:** {}\n**Format Jawaban:**\n- Summary 1 kalimat\n- Tabel markdown\n- Catatan singkat (opsional)\n",
                idx + 1,
                template.example_question
            ).unwrap();
        } else {
            writeln!(
                examples,
                "### Example {}\n**Question:** {}\n**Response Format:**\n- 1 sentence summary\n- Markdown table\n- Brief note (optional)\n",
                idx + 1,
                template.example_question
            ).unwrap();
        }
    }

    // Add explicit instruction at the end
    if is_indonesian {
        writeln!(examples,
            "\n### Format Wajib:\n1. **Line 1**: Summary 1 kalimal (cth: \"Nemu 3 data:\")\n2. **Line 2**: Kosong\n3. **Line 3+**: Tabel markdown\n4. **Terakhir**: Catatan 1 baris (opsional)\n\nJANGAN tulis narasi panjang. Gunakan tabel."
        ).unwrap();
    } else {
        writeln!(examples,
            "\n### Required Format:\n1. **Line 1**: 1 sentence summary (e.g. \"Found 3 records:\")\n2. **Line 2**: Empty\n3. **Line 3+**: Markdown table\n4. **Last**: 1-line note (optional)\n\nNO long narratives. Use tables."
        ).unwrap();
    }

    examples
}

#[tauri::command]
pub async fn db_query_rag(
    state: State<'_, Arc<super::AppState>>,
    request: DbQueryRequest,
) -> Result<DbQueryResponse> {
    let start = Instant::now();

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Processing query for collection {}: {}",
            request.collection_id,
            truncate_query_for_log(&request.query)
        ),
    );

    // Step 0: Check rate limit FIRST
    let rate_limit_result = state
        .rate_limiter
        .check_rate_limit(request.collection_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Rate limit check failed: {}", e),
            );
            e
        })?;

    check_rate_limit(&rate_limit_result, &state.logs)?;

    // Step 1: Get the collection and verify it's a DB collection
    let collection = match state
        .rag_repository
        .get_collection(request.collection_id)
        .await
    {
        Ok(col) => col,
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get collection: {}", e),
            );
            // Record block for rate limiting
            let _ = state.rate_limiter.record_block(request.collection_id).await;
            return Err(e);
        }
    };

    if !matches!(
        collection.kind,
        crate::domain::rag_entities::CollectionKind::Db
    ) {
        add_log(
            &state.logs,
            "ERROR",
            "SQL-RAG",
            "SQL-RAG query called on non-DB collection",
        );
        return Err(crate::domain::error::AppError::ValidationError(
            "This collection is not a DB collection. Use standard RAG query instead.".to_string(),
        ));
    }

    // Step 2: Parse collection config
    let config_json = parse_collection_config(&collection.config_json, &state.logs)?;
    let collection_config = CollectionConfig::from_json(&config_json)?;

    // Step 3: Get DB connection
    let db_conn = state
        .rag_repository
        .get_db_connection(collection_config.db_conn_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get DB connection: {}", e),
            );
            e
        })?;

    // Step 4: Get allowlist profile
    let allowlist_profile = state
        .rag_repository
        .get_allowlist_profile(collection_config.allowlist_profile_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get allowlist profile: {}", e),
            );
            e
        })?;

    // Step 5: Parse DbConnection config_json to get user-selected columns
    let conn_config: DbConnectionConfig = if let Some(ref json) = db_conn.config_json {
        serde_json::from_str(json).unwrap_or_else(|_| DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        })
    } else {
        DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        }
    };

    // Step 6: Create router and generate query plan
    let router = SqlRagRouter::from_profile(
        &allowlist_profile,
        collection_config.selected_tables.clone(),
        conn_config.selected_columns.clone(),
    )
    .map_err(|e| {
        add_log(
            &state.logs,
            "ERROR",
            "SQL-RAG",
            &format!("Failed to create SQL router: {}", e),
        );
        e
    })?;

    // Step 6.5: Template loading will be done on-demand with batching
    // No need to preload all templates anymore
    let router = router;

    let effective_limit = request
        .limit
        .unwrap_or(conn_config.default_limit.unwrap_or(DEFAULT_LIMIT));
    let final_k = request.final_k.unwrap_or(FINAL_K);

    // Step 6.6: TEMPLATE-FIRST APPROACH WITH FEEDBACK LEARNING
    // Only check template preference if this is NOT a new query
    let query_hash = hash_query(&request.query);
    let is_new_query = request.is_new_query.unwrap_or(false);

    let preferred_template_id = if is_new_query {
        // User sent a new query - start fresh without template preference
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "New query detected - skipping template feedback lookup",
        );
        None
    } else {
        // Check for user feedback preference from previous queries
        get_user_template_preference(&state.rag_repository, &query_hash, request.collection_id)
            .await
    };

    if preferred_template_id.is_some() {
        add_log(
            &state.logs,
            "INFO",
            "SQL-RAG",
            &format!(
                "Found user preferred template from feedback: template_id={:?}",
                preferred_template_id
            ),
        );
    }

    // Match templates BEFORE generating plan - use template SQL if good match exists
    // Use semantic matching with LLM for cross-language support
    let detected_tables = collection_config.selected_tables.clone();

    // Get LLM config for semantic matching
    let llm_config = state.last_config.lock().unwrap().clone();

    let matched_templates = load_templates_with_semantic_matching(
        &state.rag_repository,
        collection_config.allowlist_profile_id,
        &request.query,
        &detected_tables,
        &state.llm_client,
        &llm_config,
        &state.logs,
    )
    .await;

    // Track which template was selected (for telemetry)
    let mut selected_template_id: Option<i64> = None;
    let mut selected_template_name: Option<String> = None;
    let mut llm_template_selection: Option<LlmTemplateSelection> = None;

    // Determine if we should use template-first approach
    // Priority: 1) User has a preferred template from feedback, 2) Score-based matching
    let use_template_first = if preferred_template_id.is_some() {
        // User has a preferred template - use template-first regardless of score
        true
    } else {
        // No feedback yet - use score-based matching
        !matched_templates.is_empty() && matched_templates[0].score >= TEMPLATE_MATCH_THRESHOLD
    };

    if !matched_templates.is_empty() {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Found {} matching templates: {} (best score: {:.2})",
                matched_templates.len(),
                matched_templates
                    .iter()
                    .map(|m| m.template.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                matched_templates[0].score
            ),
        );
    }

    // Get LLM config for template selection
    let llm_config = state.last_config.lock().unwrap().clone();

    // Try template-first approach if we have good template matches
    let (sql_to_execute, sql_description, final_plan, sql_params) = if use_template_first {
        if preferred_template_id.is_some() {
            add_log(
                &state.logs,
                "INFO",
                "SQL-RAG",
                "Using TEMPLATE-FIRST approach (user preferred template from feedback)",
            );
        } else {
            add_log(
                &state.logs,
                "INFO",
                "SQL-RAG",
                "Using TEMPLATE-FIRST approach (good template match found)",
            );
        }

        // Check if user has a preferred template from feedback
        let selection = if let Some(preferred_id) = preferred_template_id {
            // Find the preferred template in matched_templates
            if matched_templates
                .iter()
                .any(|tm| tm.template.id == preferred_id)
            {
                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!("Using preferred template {} from feedback", preferred_id),
                );
                // Create a selection for the preferred template
                Some(LlmTemplateSelection {
                    selected_template_id: preferred_id,
                    extracted_params: std::collections::HashMap::new(),
                    modified_where_clause: None,
                    detected_table: None,
                    confidence: 1.0,
                    reasoning: "User preferred template from feedback".to_string(),
                })
            } else {
                // Preferred template not found in matches, use LLM selection
                add_log(
                    &state.logs,
                    "WARN",
                    "SQL-RAG",
                    &format!(
                        "Preferred template {} not found in matches, using LLM selection",
                        preferred_id
                    ),
                );
                // Build schema context for LLM
                let schema_context = build_schema_context_for_llm(
                    &collection_config.selected_tables[0],
                    &conn_config
                        .selected_columns
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>(),
                    &conn_config.selected_columns,
                );
                select_template_with_llm(
                    &state.llm_client,
                    &llm_config,
                    &request.query,
                    &matched_templates,
                    &state.logs,
                    Some(&schema_context),
                )
                .await
            }
        } else {
            // No preferred template, use LLM to select best template and extract parameters
            // Build schema context for LLM
            let schema_context = build_schema_context_for_llm(
                &collection_config.selected_tables[0],
                &conn_config
                    .selected_columns
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>(),
                &conn_config.selected_columns,
            );
            select_template_with_llm(
                &state.llm_client,
                &llm_config,
                &request.query,
                &matched_templates,
                &state.logs,
                Some(&schema_context),
            )
            .await
        };

        if let Some(ref sel) = selection {
            // Find the selected template
            if let Some(template_match) = matched_templates
                .iter()
                .find(|tm| tm.template.id == sel.selected_template_id)
            {
                let template = &template_match.template;
                selected_template_id = Some(template.id);
                selected_template_name = Some(template.name.clone());
                llm_template_selection = selection.clone();

                // Get allowed columns for the table
                // For pattern-agnostic templates, prefer detected_table from LLM
                // This allows dynamic table selection based on user query
                let table_name = if template.is_pattern_agnostic {
                    // Use LLM detected table for pattern-agnostic templates
                    sel.detected_table
                        .as_ref()
                        .filter(|t| !t.is_empty() && conn_config.selected_columns.contains_key(t.as_str()))
                        .map(|t| t.as_str())
                        .unwrap_or_else(|| {
                            // Fallback: try to find table from selected_tables
                            template
                                .tables_used
                                .first()
                                .map(|t| t.as_str())
                                .unwrap_or(&collection_config.selected_tables[0])
                        })
                } else {
                    // For table-specific templates, use template's tables_used
                    template
                        .tables_used
                        .first()
                        .map(|t| t.as_str())
                        .unwrap_or(&collection_config.selected_tables[0])
                };

                // Log detected table for debugging
                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!(
                        "Table resolution: detected_table={:?}, template.tables_used={:?}, final={}",
                        sel.detected_table,
                        template.tables_used,
                        table_name
                    ),
                );

                // Get columns for the resolved table
                let allowed_columns: Vec<String> = conn_config
                    .selected_columns
                    .get(table_name)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Fallback: try to find columns from any matching table
                        add_log(
                            &state.logs,
                            "WARN",
                            "SQL-RAG",
                            &format!(
                                "Table '{}' not found in selected_columns, trying fallback. Available tables: {:?}",
                                table_name,
                                conn_config.selected_columns.keys().collect::<Vec<_>>()
                            ),
                        );
                        conn_config
                            .selected_columns
                            .values()
                            .next()
                            .cloned()
                            .unwrap_or_default()
                    });

                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!(
                        "Resolved {} columns for table '{}': {:?}",
                        allowed_columns.len(),
                        table_name,
                        allowed_columns.iter().take(5).collect::<Vec<_>>()
                    ),
                );

                // Build SQL from template
                if let Some((sql, description)) = build_sql_from_template(
                    template,
                    sel,
                    &allowed_columns,
                    table_name,
                    effective_limit,
                    &state.logs,
                ) {
                    add_log(
                        &state.logs,
                        "DEBUG",
                        "SQL-RAG",
                        &format!("Template SQL: {}", sql),
                    );

                    // Create a minimal plan for the response
                    let template_plan = QueryPlan {
                        mode: "template".to_string(),
                        table: table_name.to_string(),
                        select: allowed_columns.clone(),
                        filters: vec![],
                        limit: effective_limit,
                        order_by: None,
                        joins: None,
                    };

                    // Template SQL has values embedded, no params needed
                    (sql, description, template_plan, vec![])
                } else {
                    add_log(
                        &state.logs,
                        "WARN",
                        "SQL-RAG",
                        "Failed to build SQL from template, falling back to plan-based approach",
                    );
                    // Fallback to plan-based approach
                    let plan = router.generate_plan(&request.query, effective_limit)?;
                    let validator = AllowlistValidator::from_profile(&allowlist_profile)?
                        .with_selected_tables(collection_config.selected_tables.clone());
                    let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
                    let db_type = match db_conn.db_type.to_lowercase().as_str() {
                        "postgres" | "postgresql" => DbType::Postgres,
                        "sqlite" => DbType::Sqlite,
                        _ => DbType::Postgres,
                    };
                    let compiler = SqlCompiler::new(db_type);
                    let compiled = compiler.compile(&final_plan)?;
                    (
                        compiled.sql,
                        compiled.description,
                        final_plan,
                        compiled.params,
                    )
                }
            } else {
                // Template not found, fallback
                add_log(
                    &state.logs,
                    "WARN",
                    "SQL-RAG",
                    "Selected template not found, falling back to plan-based approach",
                );
                let plan = router.generate_plan(&request.query, effective_limit)?;
                let validator = AllowlistValidator::from_profile(&allowlist_profile)?
                    .with_selected_tables(collection_config.selected_tables.clone());
                let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
                let db_type = match db_conn.db_type.to_lowercase().as_str() {
                    "postgres" | "postgresql" => DbType::Postgres,
                    "sqlite" => DbType::Sqlite,
                    _ => DbType::Postgres,
                };
                let compiler = SqlCompiler::new(db_type);
                let compiled = compiler.compile(&final_plan)?;
                (
                    compiled.sql,
                    compiled.description,
                    final_plan,
                    compiled.params,
                )
            }
        } else {
            // LLM selection failed, fallback
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                "LLM template selection returned None, falling back to plan-based approach",
            );
            let plan = router.generate_plan(&request.query, effective_limit)?;
            let validator = AllowlistValidator::from_profile(&allowlist_profile)?
                .with_selected_tables(collection_config.selected_tables.clone());
            let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
            let db_type = match db_conn.db_type.to_lowercase().as_str() {
                "postgres" | "postgresql" => DbType::Postgres,
                "sqlite" => DbType::Sqlite,
                _ => DbType::Postgres,
            };
            let compiler = SqlCompiler::new(db_type);
            let compiled = compiler.compile(&final_plan)?;
            (
                compiled.sql,
                compiled.description,
                final_plan,
                compiled.params,
            )
        }
    } else {
        // No good template match - use traditional plan-based approach
        add_log(
            &state.logs,
            "INFO",
            "SQL-RAG",
            "Using PLAN-BASED approach (no good template match)",
        );

        let plan = router
            .generate_plan(&request.query, effective_limit)
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "SQL-RAG",
                    &format!("Failed to generate query plan: {}", e),
                );
                e
            })?;

        // Log the full query plan details for frontend visibility
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Generated plan: table={}, filters={}",
                plan.table,
                plan.filters.len()
            ),
        );

        // Log detailed query plan as JSON for debugging
        if let Ok(plan_json) = serde_json::to_string_pretty(&plan) {
            add_log(
                &state.logs,
                "DEBUG",
                "SQL-RAG",
                &format!("Query plan details: {}", plan_json),
            );
        }

        // Validate plan against allowlist
        let validator = AllowlistValidator::from_profile(&allowlist_profile)
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "SQL-RAG",
                    &format!("Failed to create validator: {}", e),
                );
                e
            })?
            .with_selected_tables(collection_config.selected_tables.clone());

        let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;

        // Compile query plan to SQL
        let db_type = match db_conn.db_type.to_lowercase().as_str() {
            "postgres" | "postgresql" => DbType::Postgres,
            "sqlite" => DbType::Sqlite,
            _ => DbType::Postgres,
        };

        let compiler = SqlCompiler::new(db_type);
        let compiled = compiler.compile(&final_plan).map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to compile SQL: {}", e),
            );
            e
        })?;

        // Validate compiled SQL
        validate_compiled_sql(&validator, &compiled, &state.logs)?;

        (
            compiled.sql,
            compiled.description,
            final_plan,
            compiled.params,
        )
    };

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Final SQL: {}", sql_description),
    );

    // Log the actual SQL query for debugging
    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Executing SQL: {}", sql_to_execute),
    );

    // Step 9: Execute query
    // Note: Template-based queries have parameters embedded in SQL, plan-based use params array
    let query_result = match state
        .db_connection_manager
        .execute_select(&db_conn, &sql_to_execute, &sql_params)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Query execution failed: {}", e),
            );
            // Record block for rate limiting on execution failure
            let _ = state.rate_limiter.record_block(request.collection_id).await;
            return Err(e);
        }
    };

    // Step 10: Convert DB rows to candidates for reranking
    let candidates = convert_db_rows_to_candidates(&query_result.rows, &final_plan.table);

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Converted {} rows to candidates for reranking",
            candidates.len()
        ),
    );

    // Step 11: Rerank candidates by relevance to user query
    let (reranked_candidates, rerank_init) = state
        .reranker_service
        .rerank_with_info(&request.query, candidates)
        .unwrap_or_else(|e| {
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                &format!("Reranking failed, using original order: {}", e),
            );
            // Fallback: return candidates in original order with no scores
            let mut fallback = convert_db_rows_to_candidates(&query_result.rows, &final_plan.table);
            for c in &mut fallback {
                c.score = Some(1.0); // Default score
            }
            (fallback, false)
        });

    if rerank_init {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "Reranker model initialized",
        );
    }

    // Step 12: Select top final_k results after reranking
    let final_rows = restore_rows_from_candidates(
        reranked_candidates
            .into_iter()
            .take(final_k as usize)
            .collect(),
        &query_result.rows,
    );

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Selected {} final results after reranking",
            final_rows.len()
        ),
    );

    // Step 13: Build citations from reranked results
    let citations: Vec<DbCitation> = final_rows
        .iter()
        .enumerate()
        .map(|(_rank, (original_idx, row, _score))| {
            let row_id = row
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("row_{}", original_idx));

            DbCitation {
                table_name: final_plan.table.clone(),
                row_id,
                columns: serde_json::json!(row),
            }
        })
        .collect();

    // Step 14: Generate answer summary with LLM
    let candidate_count = query_result.row_count;
    let final_count = final_rows.len();

    // Format results for LLM
    let results_context = format_sql_results_for_llm(&final_rows, &final_plan.table);

    // Get current LLM config
    let llm_config = state.last_config.lock().unwrap().clone();

    // Step 14: Generate natural language response using LLM
    // Use few-shot prompt if templates were matched, otherwise use standard prompt
    let answer = if !matched_templates.is_empty() {
        // Build few-shot NL examples (NOT SQL generation examples)
        // We need to create conversational examples, not SQL query examples
        let nl_examples = build_nl_few_shot_examples(&request.query, &matched_templates);

        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Using few-shot NL prompt for response generation ({} templates)",
                matched_templates.len()
            ),
        );

        generate_nl_response_with_few_shot(
            &state.llm_client,
            &llm_config,
            &request.query,
            &results_context,
            &nl_examples,
            &state.logs,
            // Format conversation history for NL response (SQL generation remains standalone)
            request.conversation_history.as_ref().map(|history| {
                history.iter()
                    .map(|msg| format!("{}: {}", msg.role, msg.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            }).as_deref(),
        )
        .await
    } else {
        generate_nl_response(
            &state.llm_client,
            &llm_config,
            &request.query,
            &results_context,
            &state.logs,
            // Format conversation history for NL response (SQL generation remains standalone)
            request.conversation_history.as_ref().map(|history| {
                history.iter()
                    .map(|msg| format!("{}: {}", msg.role, msg.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            }).as_deref(),
        )
        .await
    };

    let latency_ms = start.elapsed().as_millis() as i64;

    // Step 15: Determine LLM route (always local for DB collections)
    let llm_route = LlmRoute::Local;

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Query completed: {} candidates → {} final results in {}ms, route={}",
            candidate_count,
            final_count,
            latency_ms,
            llm_route.as_str()
        ),
    );

    // Step 16: Create audit log entry
    // Use selected_template_id/name if template-first approach was used
    let (audit_template_id, audit_template_name, audit_template_match_count) =
        if selected_template_id.is_some() {
            (
                selected_template_id,
                selected_template_name.clone(),
                Some(matched_templates.len() as i32),
            )
        } else if !matched_templates.is_empty() {
            let best_match = &matched_templates[0];
            (
                Some(best_match.template.id),
                Some(best_match.template.name.clone()),
                Some(matched_templates.len() as i32),
            )
        } else {
            (None, None, Some(0))
        };

    let audit_entry = AuditLogEntry {
        collection_id: request.collection_id,
        user_query_hash: AuditService::hash_query(&request.query),
        intent: final_plan.mode.clone(),
        plan_json: serde_json::to_string(&final_plan).ok(),
        compiled_sql: Some(sql_to_execute.clone()),
        params_json: AuditService::redact_params(&serde_json::json!(sql_params)),
        row_count: final_count as i32,
        latency_ms,
        llm_route: llm_route.as_str().to_string(),
        sent_context_chars: 0,
        template_id: audit_template_id,
        template_name: audit_template_name,
        template_match_count: audit_template_match_count,
    };

    // Log audit entry (non-blocking)
    if let Err(e) = state.audit_service.log_query(audit_entry).await {
        add_log(
            &state.logs,
            "WARN",
            "SQL-RAG",
            &format!("Failed to create audit log: {}", e),
        );
    }

    // Step 17: Record successful query in rate limiter
    if let Err(e) = state.rate_limiter.record_query(request.collection_id).await {
        add_log(
            &state.logs,
            "WARN",
            "SQL-RAG",
            &format!("Failed to record query in rate limiter: {}", e),
        );
    }

    // Convert matched_templates to TemplateMatchInfo for the response
    // Include example_question and query_pattern for UI display
    let matched_templates_info: Option<Vec<TemplateMatchInfo>> = if !matched_templates.is_empty() {
        Some(
            matched_templates
                .iter()
                .map(|m| TemplateMatchInfo {
                    template_id: m.template.id,
                    template_name: m.template.name.clone(),
                    score: m.score,
                    reason: m.reason.clone(),
                    example_question: Some(m.template.example_question.clone()),
                    query_pattern: Some(m.template.query_pattern.clone()),
                })
                .collect(),
        )
    } else {
        None
    };

    // Use selected_template_id/name if template-first was used, otherwise use best match
    let (response_template_id, response_template_name, response_template_match_count) =
        if selected_template_id.is_some() {
            (
                selected_template_id,
                selected_template_name,
                Some(matched_templates.len() as i32),
            )
        } else if !matched_templates.is_empty() {
            let best_match = &matched_templates[0];
            (
                Some(best_match.template.id),
                Some(best_match.template.name.clone()),
                Some(matched_templates.len() as i32),
            )
        } else {
            (None, None, None)
        };

    Ok(DbQueryResponse {
        answer,
        citations,
        telemetry: DbQueryTelemetry {
            row_count: final_count,
            latency_ms,
            llm_route: llm_route.as_str().to_string(),
            query_plan: Some(sql_description),
            executed_sql: Some(sql_to_execute.clone()),
            template_id: response_template_id,
            template_name: response_template_name,
            template_match_count: response_template_match_count,
            matched_templates: matched_templates_info,
            column_mappings: None, // Deprecated
            modified_where_clause: llm_template_selection
                .as_ref()
                .and_then(|s| s.modified_where_clause.clone()),
        },
        plan: Some(serde_json::to_value(&final_plan).unwrap_or_default()),
    })
}

// ============================================================================
// REGENERATE WITH SPECIFIC TEMPLATE (Feature 31 Enhancement)
// ============================================================================

/// Query with a specific template (for user-selected regeneration)
#[tauri::command]
pub async fn db_query_rag_with_template(
    state: State<'_, Arc<super::AppState>>,
    request: DbQueryWithTemplateRequest,
) -> Result<DbQueryResponse> {
    let start = Instant::now();

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Processing query with template {} for collection {}: {}",
            request.template_id,
            request.collection_id,
            truncate_query_for_log(&request.query)
        ),
    );

    // Get the collection and verify it's a DB collection
    let collection = state
        .rag_repository
        .get_collection(request.collection_id)
        .await?;

    if !matches!(
        collection.kind,
        crate::domain::rag_entities::CollectionKind::Db
    ) {
        return Err(crate::domain::error::AppError::ValidationError(
            "This collection is not a DB collection.".to_string(),
        ));
    }

    // Parse collection config
    let config_json = parse_collection_config(&collection.config_json, &state.logs)?;
    let collection_config = CollectionConfig::from_json(&config_json)?;

    // Get DB connection
    let db_conn = state
        .rag_repository
        .get_db_connection(collection_config.db_conn_id)
        .await?;

    // Get the specific template
    let template = state
        .rag_repository
        .get_query_template(request.template_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get template {}: {}", request.template_id, e),
            );
            e
        })?;

    // Get allowlist profile
    let _allowlist_profile = state
        .rag_repository
        .get_allowlist_profile(collection_config.allowlist_profile_id)
        .await?;

    // Parse DbConnection config_json
    let conn_config: DbConnectionConfig = if let Some(ref json) = db_conn.config_json {
        serde_json::from_str(json).unwrap_or_else(|_| DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        })
    } else {
        DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        }
    };

    let effective_limit = request
        .limit
        .unwrap_or(conn_config.default_limit.unwrap_or(DEFAULT_LIMIT));
    let final_k = request.final_k.unwrap_or(FINAL_K);

    // Get LLM config for parameter extraction
    let llm_config = state.last_config.lock().unwrap().clone();

    // Create a fake template match for LLM selection
    let template_match = TemplateMatch {
        template: template.clone(),
        score: 1.0,
        reason: "User selected".to_string(),
    };

    // Get table name and allowed columns first (needed for schema context)
    let table_name = template
        .tables_used
        .first()
        .map(|t| t.as_str())
        .unwrap_or(&collection_config.selected_tables[0]);

    let allowed_columns: Vec<String> = conn_config
        .selected_columns
        .get(table_name)
        .cloned()
        .unwrap_or_default();

    // Use LLM to extract parameters from query for this template
    // Build schema context for LLM
    let schema_context =
        build_schema_context_for_llm(table_name, &allowed_columns, &conn_config.selected_columns);
    let selection = select_template_with_llm(
        &state.llm_client,
        &llm_config,
        &request.query,
        &[template_match],
        &state.logs,
        Some(&schema_context),
    )
    .await
    .unwrap_or(LlmTemplateSelection {
        selected_template_id: template.id,
        extracted_params: std::collections::HashMap::new(),
        modified_where_clause: None,
        detected_table: None,
        confidence: 1.0,
        reasoning: "User-selected template".to_string(),
    });

    // Build SQL from template
    let (sql_to_execute, sql_description) = build_sql_from_template(
        &template,
        &selection,
        &allowed_columns,
        table_name,
        effective_limit,
        &state.logs,
    )
    .ok_or_else(|| {
        crate::domain::error::AppError::ValidationError(
            "Failed to build SQL from template".to_string(),
        )
    })?;

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Template SQL: {}", sql_to_execute),
    );

    // Execute query
    let empty_params: Vec<serde_json::Value> = vec![];
    let query_result = state
        .db_connection_manager
        .execute_select(&db_conn, &sql_to_execute, &empty_params)
        .await?;

    // Convert and rerank results
    let candidates = convert_db_rows_to_candidates(&query_result.rows, table_name);
    let (reranked_candidates, _) = state
        .reranker_service
        .rerank_with_info(&request.query, candidates)
        .unwrap_or_else(|_| {
            let mut fallback = convert_db_rows_to_candidates(&query_result.rows, table_name);
            for c in &mut fallback {
                c.score = Some(1.0);
            }
            (fallback, false)
        });

    let final_rows = restore_rows_from_candidates(
        reranked_candidates
            .into_iter()
            .take(final_k as usize)
            .collect(),
        &query_result.rows,
    );

    // Build citations
    let citations: Vec<DbCitation> = final_rows
        .iter()
        .enumerate()
        .map(|(_rank, (original_idx, row, _score))| {
            let row_id = row
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("row_{}", original_idx));

            DbCitation {
                table_name: table_name.to_string(),
                row_id,
                columns: serde_json::json!(row),
            }
        })
        .collect();

    // Generate NL response
    let results_context = format_sql_results_for_llm(&final_rows, table_name);
    let answer = generate_nl_response(
        &state.llm_client,
        &llm_config,
        &request.query,
        &results_context,
        &state.logs,
        // Format conversation history for NL response (SQL generation remains standalone)
        request.conversation_history.as_ref().map(|history| {
            history.iter()
                .map(|msg| format!("{}: {}", msg.role, msg.content))
                .collect::<Vec<_>>()
                .join("\n")
        }).as_deref(),
    )
    .await;

    let latency_ms = start.elapsed().as_millis() as i64;

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Template query completed: {} results in {}ms",
            final_rows.len(),
            latency_ms
        ),
    );

    // Build plan for response
    let plan = QueryPlan {
        mode: "template".to_string(),
        table: table_name.to_string(),
        select: allowed_columns,
        filters: vec![],
        limit: effective_limit,
        order_by: None,
        joins: None,
    };

    Ok(DbQueryResponse {
        answer,
        citations,
        telemetry: DbQueryTelemetry {
            row_count: final_rows.len(),
            latency_ms,
            llm_route: "local".to_string(),
            query_plan: Some(sql_description),
            executed_sql: Some(sql_to_execute.clone()),
            template_id: Some(template.id),
            template_name: Some(template.name.clone()),
            template_match_count: Some(1),
            matched_templates: Some(vec![TemplateMatchInfo {
                template_id: template.id,
                template_name: template.name.clone(),
                score: 1.0,
                reason: "User selected".to_string(),
                example_question: Some(template.example_question.clone()),
                query_pattern: Some(template.query_pattern.clone()),
            }]),
            column_mappings: None,
            modified_where_clause: None,
        },
        plan: Some(serde_json::to_value(&plan).unwrap_or_default()),
    })
}

// ============================================================================
// TEMPLATE FEEDBACK (Learning from user preferences)
// ============================================================================

/// Submit feedback when user selects a different template than auto-selected
#[tauri::command]
pub async fn submit_template_feedback(
    state: State<'_, Arc<super::AppState>>,
    request: TemplateFeedbackRequest,
) -> Result<TemplateFeedbackResponse> {
    let query_hash = hash_query(&request.query);

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Recording template feedback: query_hash={}, auto={:?}, user={}",
            query_hash, request.auto_selected_template_id, request.user_selected_template_id
        ),
    );

    // Record the feedback in the database
    // This will be used to prioritize user-preferred templates in future queries
    match state
        .rag_repository
        .record_template_feedback(
            &query_hash,
            request.collection_id,
            request.auto_selected_template_id,
            request.user_selected_template_id,
        )
        .await
    {
        Ok(_) => {
            add_log(
                &state.logs,
                "DEBUG",
                "SQL-RAG",
                "Template feedback recorded successfully",
            );
            Ok(TemplateFeedbackResponse {
                success: true,
                message: "Feedback recorded. Future similar queries will prioritize this template."
                    .to_string(),
            })
        }
        Err(e) => {
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                &format!("Failed to record template feedback: {}", e),
            );
            // Return success anyway - feedback is optional
            Ok(TemplateFeedbackResponse {
                success: false,
                message: format!("Could not record feedback: {}", e),
            })
        }
    }
}

// Get recent audit logs for a collection
