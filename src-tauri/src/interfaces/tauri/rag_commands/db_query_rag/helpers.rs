use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::data_protection::ExternalLlmPolicy;
use crate::application::use_cases::rate_limiter::RateLimitResult;
use crate::domain::error::Result;
use crate::domain::rag_entities::QueryPlan;
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::DEFAULT_ALLOWLIST_PROFILE_ID;
use super::constants::MAX_QUERY_LOG_LENGTH;

pub fn log_and_return_error(
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    context: &str,
    message: &str,
    error: crate::domain::error::AppError,
) -> crate::domain::error::AppError {
    add_log(logs, "ERROR", context, message);
    error
}

pub fn format_validation_errors<E>(errors: &[E]) -> String
where
    E: std::fmt::Debug,
{
    errors
        .iter()
        .map(|e| format!("{:?}", e))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn truncate_query_for_log(query: &str) -> String {
    if query.len() > MAX_QUERY_LOG_LENGTH {
        format!("{}...", &query[..MAX_QUERY_LOG_LENGTH])
    } else {
        query.to_string()
    }
}

pub fn check_rate_limit(
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

pub fn parse_collection_config(
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

pub struct CollectionConfig {
    pub db_conn_id: i64,
    pub allowlist_profile_id: i64,
    pub selected_tables: Vec<String>,
    pub external_llm_policy: ExternalLlmPolicy,
}

impl CollectionConfig {
    pub fn from_json(config: &serde_json::Value) -> Result<Self> {
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

pub fn validate_query_plan(
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

pub fn validate_compiled_sql(
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
