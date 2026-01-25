//! Audit and Rate Limit Commands for SQL-RAG
//!
//! This module provides Tauri commands for:
//! - Fetching audit logs for DB collections
//! - Checking rate limit status per collection

use crate::application::use_cases::rate_limiter::RateLimitStatus;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Constants
// ============================================================================

/// Log context for SQL-RAG operations
const LOG_CONTEXT: &str = "SQL-RAG";

/// Default limit for audit log queries
const DEFAULT_AUDIT_LIMIT: i32 = 50;

/// Minimum allowed limit for audit log queries
const MIN_AUDIT_LIMIT: i32 = 1;

/// Maximum allowed limit for audit log queries
const MAX_AUDIT_LIMIT: i32 = 500;

#[tauri::command]
pub async fn db_get_audit_recent(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
    limit: Option<i32>,
) -> std::result::Result<Vec<crate::application::use_cases::audit_service::AuditLogRecord>, String>
{
    let limit = limit.unwrap_or(DEFAULT_AUDIT_LIMIT).clamp(MIN_AUDIT_LIMIT, MAX_AUDIT_LIMIT);

    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT,
        &format!(
            "Fetching audit logs for collection {}, limit={}",
            collection_id, limit
        ),
    );

    match state
        .audit_service
        .get_recent_audit_logs(collection_id, limit)
        .await
    {
        Ok(logs) => Ok(logs),
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                LOG_CONTEXT,
                &format!("Failed to fetch audit logs: {}", e),
            );
            Err(e.to_string())
        }
    }
}

/// Get current rate limit status for a collection

#[tauri::command]
pub async fn db_get_rate_limit_status(
    state: State<'_, Arc<super::AppState>>,
    collection_id: i64,
) -> std::result::Result<RateLimitStatus, String> {
    add_log(
        &state.logs,
        "INFO",
        LOG_CONTEXT,
        &format!(
            "Fetching rate limit status for collection {}",
            collection_id
        ),
    );

    match state.rate_limiter.get_status(collection_id).await {
        Ok(status) => Ok(status),
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                LOG_CONTEXT,
                &format!("Failed to fetch rate limit status: {}", e),
            );
            Err(e.to_string())
        }
    }
}

