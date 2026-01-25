//! DB Query RAG Command
//!
//! This module provides the main SQL-RAG query command that:
//! - Validates queries against allowlist
//! - Compiles natural language to SQL
//! - Executes queries with rate limiting
//! - Returns citations and telemetry

use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::audit_service::{AuditLogEntry, AuditService};
use crate::application::use_cases::data_protection::{ExternalLlmPolicy, LlmRoute};
use crate::application::use_cases::rate_limiter::RateLimitResult;
use crate::application::use_cases::sql_compiler::{DbType, SqlCompiler};
use crate::application::use_cases::sql_rag_router::SqlRagRouter;
use crate::domain::error::Result;
use crate::domain::rag_entities::QueryPlan;
use crate::interfaces::http::add_log;
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
    default_limit: i32,
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

        let default_limit = config["default_limit"]
            .as_i64()
            .unwrap_or(DEFAULT_LIMIT as i64) as i32;

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
            default_limit,
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
    Ok(if let Some(adjusted_limit) = validation_result.adjusted_limit {
        let mut adjusted_plan = plan.clone();
        adjusted_plan.limit = adjusted_limit;
        adjusted_plan
    } else {
        plan.clone()
    })
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

    // Step 5: Create router and generate query plan
    let router = SqlRagRouter::from_profile(
        &allowlist_profile,
        collection_config.selected_tables.clone(),
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

    let effective_limit = request.limit.unwrap_or(collection_config.default_limit);
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

    // Step 6: Validate plan against allowlist
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

    // Step 7: Compile query plan to SQL
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

    // Step 8: Validate compiled SQL
    validate_compiled_sql(&validator, &compiled, &state.logs)?;

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Compiled SQL: {}", compiled.description),
    );

    // Step 9: Execute query
    let query_result = match state
        .db_connection_manager
        .execute_select(&db_conn, &compiled.sql, &compiled.params)
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

    // Step 10: Build citations from results
    let citations: Vec<DbCitation> = query_result
        .rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let row_id = row
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("row_{}", idx));

            DbCitation {
                table_name: final_plan.table.clone(),
                row_id,
                columns: serde_json::json!(row),
            }
        })
        .collect();

    // Step 11: Generate answer summary
    let answer = if query_result.rows.is_empty() {
        format!(
            "No results found for your query in table '{}'.",
            final_plan.table
        )
    } else {
        format!(
            "Found {} result(s) from table '{}'. {}",
            query_result.row_count,
            final_plan.table,
            compiled.description
        )
    };

    let latency_ms = start.elapsed().as_millis() as i64;

    // Step 12: Determine LLM route (always local for DB collections)
    let llm_route = LlmRoute::Local;

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Query completed: {} rows in {}ms, route={}",
            query_result.row_count,
            latency_ms,
            llm_route.as_str()
        ),
    );

    // Step 13: Create audit log entry
    let audit_entry = AuditLogEntry {
        collection_id: request.collection_id,
        user_query_hash: AuditService::hash_query(&request.query),
        intent: final_plan.mode.clone(),
        plan_json: serde_json::to_string(&final_plan).ok(),
        compiled_sql: Some(compiled.sql.clone()),
        params_json: AuditService::redact_params(&serde_json::json!(compiled.params)),
        row_count: query_result.row_count as i32,
        latency_ms,
        llm_route: llm_route.as_str().to_string(),
        sent_context_chars: 0,
        selected_tables: serde_json::to_string(&collection_config.selected_tables).ok(),
        table_selection_blocked: false,
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

    // Step 14: Record successful query in rate limiter
    if let Err(e) = state.rate_limiter.record_query(request.collection_id).await {
        add_log(
            &state.logs,
            "WARN",
            "SQL-RAG",
            &format!("Failed to record query in rate limiter: {}", e),
        );
    }

    Ok(DbQueryResponse {
        answer,
        citations,
        telemetry: DbQueryTelemetry {
            row_count: query_result.row_count,
            latency_ms,
            llm_route: llm_route.as_str().to_string(),
            query_plan: Some(compiled.description),
        },
        plan: Some(serde_json::to_value(&final_plan).unwrap_or_default()),
    })
}

// Get recent audit logs for a collection

