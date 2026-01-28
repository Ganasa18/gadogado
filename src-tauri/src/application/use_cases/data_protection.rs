//! Data Protection Service for SQL-RAG
//!
//! This module implements data leakage prevention and field-level redaction:
//! - External LLM policy enforcement
//! - Data classification rule loading
//! - Field-level redaction for sensitive data
//! - LLM routing decisions (local/external/blocked)

use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// External LLM policy for DB collections
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalLlmPolicy {
    /// Block all external LLM access for this collection
    AlwaysBlock,
    /// Use local LLM only (no external API calls)
    LocalOnly,
    /// Allow external LLM for non-sensitive data
    AllowExternal,
}

impl Default for ExternalLlmPolicy {
    fn default() -> Self {
        ExternalLlmPolicy::AlwaysBlock
    }
}

impl From<String> for ExternalLlmPolicy {
    fn from(s: String) -> Self {
        match s.as_str() {
            "local_only" => ExternalLlmPolicy::LocalOnly,
            "allow_external" => ExternalLlmPolicy::AllowExternal,
            _ => ExternalLlmPolicy::AlwaysBlock,
        }
    }
}

impl From<&str> for ExternalLlmPolicy {
    fn from(s: &str) -> Self {
        match s {
            "local_only" => ExternalLlmPolicy::LocalOnly,
            "allow_external" => ExternalLlmPolicy::AllowExternal,
            _ => ExternalLlmPolicy::AlwaysBlock,
        }
    }
}

impl ExternalLlmPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternalLlmPolicy::AlwaysBlock => "always_block",
            ExternalLlmPolicy::LocalOnly => "local_only",
            ExternalLlmPolicy::AllowExternal => "allow_external",
        }
    }
}

/// Data sensitivity level for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSensitivity {
    /// Confidential data - PII, financial, secrets (LOCAL ONLY)
    Confidential,
    /// Internal data - business metrics, internal docs (LOCAL ONLY)
    Internal,
    /// Public data - safe to send to external LLM
    Public,
}

/// LLM routing decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmRoute {
    /// Use local LLM only
    Local,
    /// Use external LLM (allowed for public data)
    External,
    /// Block request completely
    Blocked,
}

impl LlmRoute {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmRoute::Local => "local",
            LlmRoute::External => "external",
            LlmRoute::Blocked => "blocked",
        }
    }
}

/// Data classification rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    /// Table this rule applies to (optional - if None, applies to all)
    pub table: Option<String>,
    /// Column this rule applies to (optional - if None, applies to all columns in table)
    pub column: Option<String>,
    /// Match pattern in JSON format
    pub match_json: Value,
    /// Action to take: redact, block_external, block_all
    pub action: String,
    /// Sensitivity level for this rule
    pub sensitivity: Option<String>,
}

/// Query row for redaction
#[derive(Debug, Clone)]
pub struct QueryRow {
    pub table_name: String,
    pub columns: HashMap<String, String>,
}

impl QueryRow {
    /// Apply redaction to sensitive columns
    pub fn redact_column(&mut self, column: &str, replacement: &str) {
        if let Some(value) = self.columns.get_mut(column) {
            *value = replacement.to_string();
        }
    }

    /// Check if column exists
    pub fn has_column(&self, column: &str) -> bool {
        self.columns.contains_key(column)
    }

    /// Get column value
    pub fn get_column(&self, column: &str) -> Option<&String> {
        self.columns.get(column)
    }
}

/// Data protection service for SQL-RAG
pub struct DataProtectionService {
    db_pool: Arc<SqlitePool>,
}

impl DataProtectionService {
    /// Create a new data protection service
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self { db_pool }
    }

    /// Check if external LLM is allowed for a collection
    pub async fn check_external_llm_allowed(&self, collection_id: i64) -> Result<bool> {
        // Get collection config to check external_llm_policy
        let config_json: Value =
            sqlx::query_scalar("SELECT config_json FROM collections WHERE id = ?")
                .bind(collection_id)
                .fetch_one(self.db_pool.as_ref())
                .await
                .map_err(|e| {
                    AppError::DatabaseError(format!("Failed to fetch collection: {}", e))
                })?;

        let policy_str = config_json
            .get("external_llm_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("always_block");

        let policy: ExternalLlmPolicy = policy_str.into();

        Ok(matches!(policy, ExternalLlmPolicy::AllowExternal))
    }

    /// Load classification rules for an allowlist profile
    pub async fn load_classification_rules(
        &self,
        profile_id: i64,
    ) -> Result<Vec<ClassificationRule>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT match_json, action FROM data_classification_rules WHERE allowlist_profile_id = ?"
        )
        .bind(profile_id)
        .fetch_all(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to load classification rules: {}", e)))?;

        let mut rules = Vec::new();
        for (match_json, action) in rows {
            let match_value: Value = serde_json::from_str(&match_json)
                .map_err(|e| AppError::ValidationError(format!("Invalid match_json: {}", e)))?;

            // Extract table and column from match_json
            let table = match_value
                .get("table")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let column = match_value
                .get("column")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            rules.push(ClassificationRule {
                table,
                column,
                match_json: match_value.clone(),
                action,
                sensitivity: match_value
                    .get("sensitivity")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }

        Ok(rules)
    }

    /// Apply redaction to a query row based on classification rules
    pub fn apply_redaction(&self, row: &QueryRow, rules: &[ClassificationRule]) -> QueryRow {
        let mut redacted_row = row.clone();

        for rule in rules {
            // Check if rule applies to this table
            if let Some(ref rule_table) = rule.table {
                if rule_table != &row.table_name {
                    continue;
                }
            }

            // Check if rule applies to specific column
            if let Some(ref rule_column) = rule.column {
                if !row.has_column(rule_column) {
                    continue;
                }

                // Apply action based on rule type
                match rule.action.as_str() {
                    "redact" => {
                        let replacement = self.get_redaction_value(&row.table_name, rule_column);
                        redacted_row.redact_column(rule_column, &replacement);
                        info!(
                            "Redacted column '{}' in table '{}'",
                            rule_column, row.table_name
                        );
                    }
                    "block_external" | "block_all" => {
                        // These are handled at the routing level, not per-column
                        // But we still redact for safety
                        redacted_row.redact_column(rule_column, "[REDACTED]");
                    }
                    _ => {
                        warn!("Unknown action '{}' in classification rule", rule.action);
                    }
                }
            } else {
                // Rule applies to all columns in table (e.g., confidential table)
                if matches!(rule.action.as_str(), "block_external" | "block_all") {
                    // Mark entire row as sensitive - redact all column values
                    for col_name in redacted_row.columns.keys().cloned().collect::<Vec<_>>() {
                        if self.is_sensitive_column(&col_name) {
                            redacted_row.redact_column(&col_name, "[REDACTED]");
                        }
                    }
                }
            }
        }

        redacted_row
    }

    /// Get LLM route decision based on policy and sensitivity
    pub fn get_llm_route(
        &self,
        policy: &ExternalLlmPolicy,
        sensitivity: &DataSensitivity,
    ) -> LlmRoute {
        match (policy, sensitivity) {
            // Confidential data NEVER goes external
            (_, DataSensitivity::Confidential) => LlmRoute::Local,
            // Internal data stays local
            (_, DataSensitivity::Internal) => LlmRoute::Local,
            // Public data can go external if policy allows
            (ExternalLlmPolicy::AllowExternal, DataSensitivity::Public) => LlmRoute::External,
            // Public data but policy is restrictive
            (ExternalLlmPolicy::LocalOnly, DataSensitivity::Public) => LlmRoute::Local,
            (ExternalLlmPolicy::AlwaysBlock, _) => LlmRoute::Blocked,
        }
    }

    /// Determine sensitivity of a query row based on rules
    pub fn determine_row_sensitivity(
        &self,
        row: &QueryRow,
        rules: &[ClassificationRule],
    ) -> DataSensitivity {
        let mut max_sensitivity = DataSensitivity::Public;

        for rule in rules {
            // Check if rule applies to this row
            if let Some(ref rule_table) = rule.table {
                if rule_table != &row.table_name {
                    continue;
                }
            }

            // Check sensitivity from rule
            if let Some(ref sensitivity_str) = rule.sensitivity {
                let sensitivity = match sensitivity_str.as_str() {
                    "confidential" => DataSensitivity::Confidential,
                    "internal" => DataSensitivity::Internal,
                    _ => DataSensitivity::Public,
                };

                // Upgrade to highest sensitivity found
                if matches!(sensitivity, DataSensitivity::Confidential) {
                    return DataSensitivity::Confidential;
                }
                if matches!(sensitivity, DataSensitivity::Internal)
                    && max_sensitivity == DataSensitivity::Public
                {
                    max_sensitivity = DataSensitivity::Internal;
                }
            }
        }

        // Also check for sensitive column names as fallback
        for col_name in row.columns.keys() {
            if self.is_confidential_column(col_name) {
                return DataSensitivity::Confidential;
            }
            if self.is_sensitive_column(col_name) && max_sensitivity == DataSensitivity::Public {
                max_sensitivity = DataSensitivity::Internal;
            }
        }

        max_sensitivity
    }

    /// Check if a column name indicates confidential data
    fn is_confidential_column(&self, column: &str) -> bool {
        let column_lower = column.to_lowercase();
        let confidential_patterns = [
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
        confidential_patterns
            .iter()
            .any(|pattern| column_lower.contains(pattern))
    }

    /// Check if a column name indicates sensitive data
    fn is_sensitive_column(&self, column: &str) -> bool {
        let column_lower = column.to_lowercase();
        let sensitive_patterns = [
            "email", "phone", "mobile", "address", "zip", "postal", "salary", "income", "balance",
            "amount",
        ];
        sensitive_patterns
            .iter()
            .any(|pattern| column_lower.contains(pattern))
    }

    /// Get redaction placeholder value for a column
    fn get_redaction_value(&self, _table: &str, column: &str) -> String {
        if self.is_confidential_column(column) {
            "[CONFIDENTIAL]".to_string()
        } else if self.is_sensitive_column(column) {
            "[REDACTED]".to_string()
        } else {
            "[REDACTED]".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_llm_policy_from_string() {
        assert_eq!(
            ExternalLlmPolicy::from("always_block"),
            ExternalLlmPolicy::AlwaysBlock
        );
        assert_eq!(
            ExternalLlmPolicy::from("local_only"),
            ExternalLlmPolicy::LocalOnly
        );
        assert_eq!(
            ExternalLlmPolicy::from("allow_external"),
            ExternalLlmPolicy::AllowExternal
        );
        assert_eq!(
            ExternalLlmPolicy::from("unknown"),
            ExternalLlmPolicy::AlwaysBlock
        );
    }

    #[test]
    fn test_llm_route_decision() {
        let service = DataProtectionService::new(Arc::new(
            SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        ));

        // Confidential never goes external
        assert_eq!(
            service.get_llm_route(
                &ExternalLlmPolicy::AllowExternal,
                &DataSensitivity::Confidential
            ),
            LlmRoute::Local
        );

        // Public can go external if policy allows
        assert_eq!(
            service.get_llm_route(&ExternalLlmPolicy::AllowExternal, &DataSensitivity::Public),
            LlmRoute::External
        );

        // Always block policy
        assert_eq!(
            service.get_llm_route(&ExternalLlmPolicy::AlwaysBlock, &DataSensitivity::Public),
            LlmRoute::Blocked
        );
    }

    #[test]
    fn test_sensitive_column_detection() {
        let service = DataProtectionService::new(Arc::new(
            SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        ));

        assert!(service.is_confidential_column("password"));
        assert!(service.is_confidential_column("user_token"));
        assert!(service.is_sensitive_column("email"));
        assert!(service.is_sensitive_column("phone_number"));
        assert!(!service.is_sensitive_column("username"));
        assert!(!service.is_sensitive_column("id"));
    }

    #[test]
    fn test_row_redaction() {
        let row = QueryRow {
            table_name: "users".to_string(),
            columns: {
                let mut m = HashMap::new();
                m.insert("username".to_string(), "admin".to_string());
                m.insert("email".to_string(), "admin@example.com".to_string());
                m.insert("password".to_string(), "secret123".to_string());
                m
            },
        };

        let rules = vec![ClassificationRule {
            table: Some("users".to_string()),
            column: Some("password".to_string()),
            match_json: serde_json::json!({"table": "users", "column": "password"}),
            action: "redact".to_string(),
            sensitivity: Some("confidential".to_string()),
        }];

        let service = DataProtectionService::new(Arc::new(
            SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        ));
        let redacted = service.apply_redaction(&row, &rules);

        assert_eq!(redacted.get_column("username").unwrap(), "admin");
        assert_eq!(redacted.get_column("email").unwrap(), "admin@example.com");
        assert_eq!(redacted.get_column("password").unwrap(), "[CONFIDENTIAL]");
    }
}
