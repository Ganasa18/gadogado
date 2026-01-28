//! Audit Service for SQL-RAG
//!
//! This module implements comprehensive audit logging for SQL-RAG queries:
//! - Query execution logging to db_query_audit table
//! - Query hashing for privacy (don't store raw queries)
//! - Parameter redaction before logging
//! - Session tracking and rate limit compliance

use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info};

/// Audit log entry for SQL-RAG queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub collection_id: i64,
    pub user_query_hash: String,
    pub intent: String,
    pub plan_json: Option<String>,
    pub compiled_sql: Option<String>,
    pub params_json: Option<String>,
    pub row_count: i32,
    pub latency_ms: i64,
    pub llm_route: String,
    pub sent_context_chars: i32,
    pub template_id: Option<i64>,
    pub template_name: Option<String>,
    pub template_match_count: Option<i32>,
}

/// Audit log entry as stored in database (with created_at)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub id: i64,
    pub collection_id: i64,
    pub user_query_hash: String,
    pub intent: Option<String>,
    pub plan_json: Option<String>,
    pub compiled_sql: Option<String>,
    pub params_json: Option<String>,
    pub row_count: i32,
    pub latency_ms: i64,
    pub llm_route: Option<String>,
    pub sent_context_chars: i32,
    pub template_id: Option<i64>,
    pub template_name: Option<String>,
    pub template_match_count: Option<i32>,
    pub created_at: String,
}

/// Query parameters for redaction

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam {
    pub value: String,
}

/// Audit service for SQL-RAG queries
pub struct AuditService {
    db_pool: Arc<SqlitePool>,
}

impl AuditService {
    /// Create a new audit service
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self { db_pool }
    }

    /// Log a query execution to the audit table
    pub async fn log_query(&self, entry: AuditLogEntry) -> Result<()> {
        info!(
            "Logging audit entry for collection {}: intent={}, rows={}, latency={}ms, route={}",
            entry.collection_id, entry.intent, entry.row_count, entry.latency_ms, entry.llm_route
        );

        let result = sqlx::query(
            r#"
            INSERT INTO db_query_audit (
                collection_id,
                user_query_hash,
                intent,
                plan_json,
                compiled_sql,
                params_json,
                row_count,
                latency_ms,
                llm_route,
                sent_context_chars,
                template_id,
                template_name,
                template_match_count,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(entry.collection_id)
        .bind(&entry.user_query_hash)
        .bind(&entry.intent)
        .bind(&entry.plan_json)
        .bind(&entry.compiled_sql)
        .bind(&entry.params_json)
        .bind(entry.row_count)
        .bind(entry.latency_ms)
        .bind(&entry.llm_route)
        .bind(entry.sent_context_chars)
        .bind(entry.template_id)
        .bind(&entry.template_name)
        .bind(entry.template_match_count)
        .execute(self.db_pool.as_ref())
        .await;

        match result {
            Ok(_) => {
                info!("Audit log entry created successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to create audit log entry: {}", e);
                Err(AppError::DatabaseError(format!(
                    "Failed to create audit log: {}",
                    e
                )))
            }
        }
    }

    /// Get recent audit logs for a collection
    pub async fn get_recent_audit_logs(
        &self,
        collection_id: i64,
        limit: i32,
    ) -> Result<Vec<AuditLogRecord>> {
        let limit = limit.clamp(1, 500); // Sanity check

        let rows = sqlx::query_as::<
            _,
            (
                i64,
                i64,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i32,
                i64,
                Option<String>,
                i32,
                Option<i64>,
                Option<String>,
                Option<i32>,
                String,
            ),
        >(
            r#"
            SELECT
                id, collection_id, user_query_hash, intent, plan_json, compiled_sql,
                params_json, row_count, latency_ms, llm_route, sent_context_chars,
                template_id, template_name, template_match_count, created_at
            FROM db_query_audit
            WHERE collection_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(collection_id)
        .bind(limit)
        .fetch_all(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch audit logs: {}", e)))?;

        let records = rows
            .into_iter()
            .map(
                |(
                    id,
                    collection_id,
                    user_query_hash,
                    intent,
                    plan_json,
                    compiled_sql,
                    params_json,
                    row_count,
                    latency_ms,
                    llm_route,
                    sent_context_chars,
                    template_id,
                    template_name,
                    template_match_count,
                    created_at,
                )| {
                    AuditLogRecord {
                        id,
                        collection_id,
                        user_query_hash,
                        intent,
                        plan_json,
                        compiled_sql,
                        params_json,
                        row_count,
                        latency_ms,
                        llm_route,
                        sent_context_chars,
                        template_id,
                        template_name,
                        template_match_count,
                        created_at,
                    }
                },
            )
            .collect();

        Ok(records)
    }

    /// Generate a hash for a query string (for privacy - don't store raw queries)
    pub fn hash_query(query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Normalize query: lowercase, trim whitespace
        let normalized = query.trim().to_lowercase();

        // Create hash
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        let hash = hasher.finish();

        // Return as hex string
        format!("{:x}", hash)
    }

    /// Redact sensitive parameters before logging
    pub fn redact_params(params: &Value) -> Option<String> {
        if !params.is_array() && !params.is_object() {
            return params.to_string().into();
        }

        let redacted = self::redact_value_recursive(params);
        redacted.to_string().into()
    }

    /// Get audit statistics for a collection
    pub async fn get_audit_stats(&self, collection_id: i64) -> Result<AuditStats> {
        let row: (i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_queries,
                COUNT(CASE WHEN row_count > 0 THEN 1 END) as successful_queries,
                AVG(latency_ms) as avg_latency_ms,
                MAX(latency_ms) as max_latency_ms
            FROM db_query_audit
            WHERE collection_id = ?
            "#,
        )
        .bind(collection_id)
        .fetch_one(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch audit stats: {}", e)))?;

        Ok(AuditStats {
            total_queries: row.0,
            successful_queries: row.1,
            avg_latency_ms: row.2,
            max_latency_ms: row.3,
        })
    }

    /// Clear old audit logs (maintenance function)
    pub async fn clear_old_logs(&self, days_old: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM db_query_audit
            WHERE created_at < datetime('now', '-' || ? || ' days')
            "#,
        )
        .bind(days_old)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to clear old logs: {}", e)))?;

        let deleted = result.rows_affected();
        info!(
            "Cleared {} old audit logs (older than {} days)",
            deleted, days_old
        );

        Ok(deleted)
    }
}

/// Audit statistics for a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_queries: i64,
    pub successful_queries: i64,
    pub avg_latency_ms: Option<i64>,
    pub max_latency_ms: Option<i64>,
}

/// Recursively redact sensitive values from JSON
fn redact_value_recursive(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            // Check if string looks like sensitive data
            if is_sensitive_string(s) {
                Value::String("[REDACTED]".to_string())
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(arr) => Value::Array(arr.iter().map(redact_value_recursive).collect()),
        Value::Object(obj) => {
            let mut redacted = serde_json::Map::new();
            for (key, val) in obj {
                // Check if key indicates sensitive data
                if is_sensitive_key(key) {
                    redacted.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_value_recursive(val));
                }
            }
            Value::Object(redacted)
        }
        _ => value.clone(),
    }
}

/// Check if a string key indicates sensitive data
fn is_sensitive_key(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    let sensitive_patterns = [
        "password",
        "passwd",
        "pwd",
        "token",
        "secret",
        "api_key",
        "private_key",
        "credential",
        "ssn",
        "social_security",
        "credit_card",
        "cc_number",
        "bank_account",
    ];
    sensitive_patterns
        .iter()
        .any(|pattern| key_lower.contains(pattern))
}

/// Check if a string value looks like sensitive data
fn is_sensitive_string(s: &str) -> bool {
    // Check for common patterns
    // Email pattern
    if s.contains('@') && s.contains('.') {
        return true;
    }
    // Phone number pattern (simple check)
    if s.len() > 7 && s.chars().filter(|c| c.is_numeric()).count() > 7 {
        return true;
    }
    // Credit card pattern (16 digits)
    if s.len() >= 13 && s.len() <= 19 && s.chars().all(|c| c.is_numeric() || c == ' ') {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_hash() {
        let hash1 = AuditService::hash_query("SELECT * FROM users");
        let hash2 = AuditService::hash_query("select * from users");
        let hash3 = AuditService::hash_query("SELECT * FROM orders");

        // Same query (normalized) should produce same hash
        assert_eq!(hash1, hash2);
        // Different query should produce different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_redact_sensitive_key() {
        let value = serde_json::json!({
            "username": "admin",
            "password": "secret123",
            "email": "admin@example.com"
        });

        let redacted = redact_value_recursive(&value);

        assert_eq!(redacted["username"], "admin");
        assert_eq!(redacted["password"], "[REDACTED]");
        assert_eq!(redacted["email"], "admin@example.com"); // Email not redacted by key
    }

    #[test]
    fn test_redact_sensitive_value() {
        let value = serde_json::json!(["normal_value", "admin@example.com", "1234567890123456"]);

        let redacted = redact_value_recursive(&value);

        assert_eq!(redacted[0], "normal_value");
        assert_eq!(redacted[1], "[REDACTED]"); // Email pattern
        assert_eq!(redacted[2], "[REDACTED]"); // Credit card pattern
    }

    #[test]
    fn test_sensitive_key_detection() {
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("user_token"));
        assert!(is_sensitive_key("api_key"));
        assert!(!is_sensitive_key("username"));
        assert!(!is_sensitive_key("id"));
    }

    #[test]
    fn test_sensitive_string_detection() {
        assert!(is_sensitive_string("admin@example.com"));
        assert!(is_sensitive_string("1234567890123456"));
        assert!(is_sensitive_string("+1234567890"));
        assert!(!is_sensitive_string("normal_value"));
        assert!(!is_sensitive_string("123"));
    }
}
