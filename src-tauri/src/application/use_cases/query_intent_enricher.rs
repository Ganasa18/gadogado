//! Query Intent Enricher
//!
//! Pre-processes user queries using LLM to clarify ambiguous intent
//! before template matching. This handles cases like:
//! - "tampilkan data user ada juga data korwil" → JOIN query
//! - "tampilkan data user dan korwil terbaru limit 10" → JOIN + ORDER BY
//! - "lihat semua loan sama channel-nya" → JOIN query

use crate::domain::llm_config::LLMConfig;
use crate::interfaces::http::add_log;
use crate::interfaces::tauri::rag_commands::types::{EnrichedQuery, QueryIntent};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// LLM timeout for query enrichment (must be fast)
const ENRICHER_TIMEOUT_SECS: u64 = 5;

/// Build the schema context string for the enricher prompt
fn build_schema_context(
    all_selected_columns: &std::collections::HashMap<String, Vec<String>>,
) -> String {
    let mut ctx = String::new();
    for (table, cols) in all_selected_columns {
        ctx.push_str(&format!("- Table: {}", table));
        if !cols.is_empty() {
            let col_list: Vec<&str> = cols.iter().take(10).map(|c| c.as_str()).collect();
            ctx.push_str(&format!(" (columns: {})", col_list.join(", ")));
            if cols.len() > 10 {
                ctx.push_str(&format!(" ... +{} more", cols.len() - 10));
            }
        }
        ctx.push('\n');
    }
    ctx
}

/// Build the system prompt for the enricher
fn build_system_prompt(schema_context: &str) -> String {
    format!(
        r#"You are a SQL query intent classifier. Your task is to understand the user's intent and rewrite their query to be clear and unambiguous.

DATABASE SCHEMA (available tables):
{schema}

REWRITING RULES:
1. If user mentions 2+ entities/tables (e.g., "user dan korwil", "loan sama channel", "data X ada juga Y"), this is a JOIN query. Rewrite with explicit "JOIN" keyword and mention both table names from schema.
2. If user says "terbaru", "terakhir", "latest", "newest", add "ORDER BY ... DESC" intent.
3. If user says "berapa", "jumlah", "total", "count", "hitung", "rata-rata", "average", this is an aggregate query.
4. If user says "limit N", preserve the limit value.
5. If user mentions searching text content ("mengandung", "contains", "mirip", "like"), this is a text search.
6. If user mentions date range ("antara", "between", "dari ... sampai"), this is a date filter.
7. If user mentions "top", "ranking", "tertinggi", "terendah", this is a ranked query.
8. Map user's entity references to ACTUAL table names from the schema above.
9. DO NOT invent tables that don't exist in the schema.
10. Keep the rewritten query in the SAME LANGUAGE as the original (Indonesian/English).
11. This is ALWAYS a single query operation, never split into multiple queries.

INTENT CATEGORIES:
- simple_select: Basic data retrieval
- filter_by_value: WHERE column = value
- filter_by_multiple_values: WHERE IN (list)
- join_tables: Combining data from 2+ tables
- aggregate: GROUP BY, COUNT, SUM, AVG
- date_filter: Date/time based filtering
- ranked_query: TOP N, ranking, sorting
- text_search: LIKE, text pattern matching

Respond in JSON only:
{{
  "rewritten_query": "<clear rewritten query>",
  "detected_intent": "<intent category>",
  "detected_tables": ["<table1>", "<table2>"],
  "detected_operation": "<select|join|aggregate|filter|search|rank>",
  "confidence": <0.0-1.0>
}}"#,
        schema = schema_context
    )
}

/// Enrich a user query using LLM to clarify intent
pub async fn enrich_query(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &LLMConfig,
    raw_query: &str,
    all_selected_columns: &std::collections::HashMap<String, Vec<String>>,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> EnrichedQuery {
    let start = std::time::Instant::now();

    let schema_context = build_schema_context(all_selected_columns);
    let system_prompt = build_system_prompt(&schema_context);
    let user_prompt = format!(r#"User Query: "{}""#, raw_query);

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG-ENRICHER",
        &format!("Enriching query: {}", if raw_query.len() > 80 { &raw_query[..80] } else { raw_query }),
    );

    let result = timeout(
        Duration::from_secs(ENRICHER_TIMEOUT_SECS),
        llm_client.generate(config, &system_prompt, &user_prompt),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(response)) => {
            let cleaned = response
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            match serde_json::from_str::<EnrichedQueryLlmResponse>(cleaned) {
                Ok(parsed) => {
                    let intent = match parsed.detected_intent.as_str() {
                        "simple_select" => QueryIntent::SimpleSelect,
                        "filter_by_value" => QueryIntent::FilterByValue,
                        "filter_by_multiple_values" => QueryIntent::FilterByMultipleValues,
                        "join_tables" => QueryIntent::JoinTables,
                        "aggregate" => QueryIntent::Aggregate,
                        "date_filter" => QueryIntent::DateFilter,
                        "ranked_query" => QueryIntent::RankedQuery,
                        "text_search" => QueryIntent::TextSearch,
                        _ => QueryIntent::SimpleSelect,
                    };

                    let enriched = EnrichedQuery {
                        original_query: raw_query.to_string(),
                        rewritten_query: parsed.rewritten_query,
                        detected_intent: intent,
                        detected_tables: parsed.detected_tables,
                        detected_operation: parsed.detected_operation,
                        confidence: parsed.confidence.clamp(0.0, 1.0),
                    };

                    add_log(
                        logs,
                        "DEBUG",
                        "SQL-RAG-ENRICHER",
                        &format!(
                            "Enriched in {}ms: intent={}, operation={}, tables={:?}, confidence={:.2}",
                            elapsed_ms,
                            parsed.detected_intent,
                            enriched.detected_operation,
                            enriched.detected_tables,
                            enriched.confidence,
                        ),
                    );

                    if enriched.was_enriched() {
                        add_log(
                            logs,
                            "INFO",
                            "SQL-RAG-ENRICHER",
                            &format!(
                                "Query rewritten: '{}' → '{}'",
                                raw_query, enriched.rewritten_query
                            ),
                        );
                    }

                    enriched
                }
                Err(e) => {
                    add_log(
                        logs,
                        "WARN",
                        "SQL-RAG-ENRICHER",
                        &format!("Failed to parse enricher response ({}ms): {}", elapsed_ms, e),
                    );
                    EnrichedQuery::passthrough(raw_query)
                }
            }
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG-ENRICHER",
                &format!("LLM enricher call failed ({}ms): {}", elapsed_ms, e),
            );
            EnrichedQuery::passthrough(raw_query)
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG-ENRICHER",
                &format!("LLM enricher timed out after {}ms", elapsed_ms),
            );
            EnrichedQuery::passthrough(raw_query)
        }
    }
}

/// Internal struct for parsing LLM JSON response
#[derive(Debug, serde::Deserialize)]
struct EnrichedQueryLlmResponse {
    pub rewritten_query: String,
    pub detected_intent: String,
    #[serde(default)]
    pub detected_tables: Vec<String>,
    #[serde(default = "default_operation")]
    pub detected_operation: String,
    #[serde(default)]
    pub confidence: f64,
}

fn default_operation() -> String {
    "unknown".to_string()
}
