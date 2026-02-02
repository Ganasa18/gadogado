//! Allowlist Validator for SQL-RAG Security
//!
//! This module enforces security policies for SQL-RAG queries:
//! - Table/view allowlisting
//! - Column allowlisting
//! - Required filter enforcement (prevent full table scans)
//! - Keyword blocking (password, token, etc.)
//! - Statement blocking (INSERT, UPDATE, DELETE, etc.)
//! - Limit clamping
//!
//! Security philosophy: Deny by default, allow by explicit rule

use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{DbAllowlistProfile, QueryPlan};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Parsed allowlist rules from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllowlistRules {
    /// Map of table name -> allowed columns
    #[serde(default)]
    pub allowed_tables: HashMap<String, Vec<String>>,
    /// Map of table name -> required filter columns (at least one must be present)
    #[serde(default)]
    pub require_filters: HashMap<String, Vec<String>>,
    /// Maximum rows to return
    #[serde(default = "default_max_limit")]
    pub max_limit: i32,
    /// Whether JOINs are allowed
    #[serde(default)]
    pub allow_joins: bool,
    /// Keywords that cannot appear in queries (case-insensitive)
    #[serde(default)]
    pub deny_keywords: Vec<String>,
    /// SQL statements that are forbidden
    #[serde(default)]
    pub deny_statements: Vec<String>,
    /// Maximum number of filters allowed in a query
    #[serde(default = "default_max_filters")]
    pub max_filters: i32,
    /// Maximum size of IN clause list
    #[serde(default = "default_max_in_list")]
    pub max_in_list_size: i32,
}

fn default_max_limit() -> i32 {
    200
}

fn default_max_filters() -> i32 {
    5
}

fn default_max_in_list() -> i32 {
    50
}

impl Default for AllowlistRules {
    fn default() -> Self {
        Self {
            allowed_tables: HashMap::new(),
            require_filters: HashMap::new(),
            max_limit: 200,
            allow_joins: false,
            deny_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "api_key".to_string(),
                "private_key".to_string(),
                "credential".to_string(),
            ],
            deny_statements: vec![
                "INSERT".to_string(),
                "UPDATE".to_string(),
                "DELETE".to_string(),
                "DROP".to_string(),
                "ALTER".to_string(),
                "TRUNCATE".to_string(),
                "CREATE".to_string(),
                "GRANT".to_string(),
                "REVOKE".to_string(),
                "PRAGMA".to_string(),
                "ATTACH".to_string(),
                "DETACH".to_string(),
            ],
            max_filters: 5,
            max_in_list_size: 50,
        }
    }
}

/// Validation result with detailed error information
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    /// Clamped limit if original exceeded max
    pub adjusted_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            adjusted_limit: None,
        }
    }

    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: vec![],
            adjusted_limit: None,
        }
    }

    pub fn add_error(&mut self, code: &str, message: &str, field: Option<&str>) {
        self.is_valid = false;
        self.errors.push(ValidationError {
            code: code.to_string(),
            message: message.to_string(),
            field: field.map(|s| s.to_string()),
        });
    }

    pub fn add_warning(&mut self, message: &str) {
        self.warnings.push(message.to_string());
    }
}

/// Allowlist validator for SQL-RAG queries
pub struct AllowlistValidator {
    rules: AllowlistRules,
    selected_tables: HashSet<String>,
}

impl AllowlistValidator {
    /// Create a new validator from an allowlist profile
    pub fn from_profile(profile: &DbAllowlistProfile) -> Result<Self> {
        let rules: AllowlistRules = serde_json::from_str(&profile.rules_json).map_err(|e| {
            AppError::ValidationError(format!("Invalid allowlist rules JSON: {}", e))
        })?;

        Ok(Self {
            rules,
            selected_tables: HashSet::new(),
        })
    }

    /// Create a new validator from rules directly
    pub fn from_rules(rules: AllowlistRules) -> Self {
        Self {
            rules,
            selected_tables: HashSet::new(),
        }
    }

    /// Set the selected tables for this validator instance
    /// This is used to enforce the table selection requirement
    pub fn with_selected_tables(mut self, tables: Vec<String>) -> Self {
        self.selected_tables = tables.into_iter().collect();
        self
    }

    /// Validate a query plan against the allowlist rules
    pub fn validate_plan(&self, plan: &QueryPlan) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // 1. Validate table is in selected tables (CRITICAL - Rule 2)
        if !self.selected_tables.is_empty() && !self.selected_tables.contains(&plan.table) {
            result.add_error(
                "TABLE_NOT_SELECTED",
                &format!(
                    "Table '{}' is not selected for this collection. Selected tables: {:?}",
                    plan.table,
                    self.selected_tables.iter().collect::<Vec<_>>()
                ),
                Some("table"),
            );
            // Return early - this is a critical error
            return result;
        }

        // 2. Validate table is in allowlist
        if !self.rules.allowed_tables.contains_key(&plan.table) {
            result.add_error(
                "TABLE_NOT_ALLOWED",
                &format!(
                    "Table '{}' is not in the allowlist. Allowed tables: {:?}",
                    plan.table,
                    self.rules.allowed_tables.keys().collect::<Vec<_>>()
                ),
                Some("table"),
            );
            return result;
        }

        // 3. Validate columns are allowed
        let allowed_columns = self.rules.allowed_tables.get(&plan.table).unwrap();
        let allowed_set: HashSet<_> = allowed_columns.iter().collect();

        for col in &plan.select {
            if col != "*" && !allowed_set.contains(col) {
                result.add_error(
                    "COLUMN_NOT_ALLOWED",
                    &format!(
                        "Column '{}' is not allowed for table '{}'. Allowed columns: {:?}",
                        col, plan.table, allowed_columns
                    ),
                    Some("select"),
                );
            }
        }

        // 4. Check for SELECT * (should use explicit columns)
        if plan.select.contains(&"*".to_string()) {
            result.add_warning("Consider using explicit column names instead of SELECT *");
        }

        // 5. Validate required filters
        if let Some(required) = self.rules.require_filters.get(&plan.table) {
            if !required.is_empty() {
                let filter_columns: HashSet<_> = plan.filters.iter().map(|f| &f.column).collect();
                let has_required = required.iter().any(|r| filter_columns.contains(r));

                if !has_required {
                    result.add_error(
                        "MISSING_REQUIRED_FILTER",
                        &format!(
                            "Table '{}' requires at least one of these filters: {:?}",
                            plan.table, required
                        ),
                        Some("filters"),
                    );
                }
            }
        }

        // 6. Validate filter columns are allowed
        for filter in &plan.filters {
            if !allowed_set.contains(&filter.column) {
                result.add_error(
                    "FILTER_COLUMN_NOT_ALLOWED",
                    &format!(
                        "Filter column '{}' is not allowed for table '{}'",
                        filter.column, plan.table
                    ),
                    Some("filters"),
                );
            }
        }

        // 7. Check filter count limit
        if plan.filters.len() as i32 > self.rules.max_filters {
            result.add_error(
                "TOO_MANY_FILTERS",
                &format!(
                    "Query has {} filters, maximum allowed is {}",
                    plan.filters.len(),
                    self.rules.max_filters
                ),
                Some("filters"),
            );
        }

        // 8. Check IN list sizes
        for filter in &plan.filters {
            if filter.operator == "in" && filter.values.len() as i32 > self.rules.max_in_list_size {
                result.add_error(
                    "IN_LIST_TOO_LARGE",
                    &format!(
                        "IN clause for '{}' has {} values, maximum allowed is {}",
                        filter.column,
                        filter.values.len(),
                        self.rules.max_in_list_size
                    ),
                    Some(&filter.column),
                );
            }
        }

        // 9. Validate JOINs
        if let Some(joins) = &plan.joins {
            if !joins.is_empty() && !self.rules.allow_joins {
                result.add_error(
                    "JOINS_NOT_ALLOWED",
                    "JOIN operations are not allowed in this profile",
                    Some("joins"),
                );
            }

            // Validate joined tables are in allowlist
            for join in joins {
                if !self.rules.allowed_tables.contains_key(&join.table) {
                    result.add_error(
                        "JOIN_TABLE_NOT_ALLOWED",
                        &format!("Join table '{}' is not in the allowlist", join.table),
                        Some("joins"),
                    );
                }
            }
        }

        // 10. Validate and clamp limit
        if plan.limit > self.rules.max_limit {
            result.adjusted_limit = Some(self.rules.max_limit);
            result.add_warning(&format!(
                "Limit {} exceeds maximum {}, will be clamped",
                plan.limit, self.rules.max_limit
            ));
        }

        // 11. Validate order_by column if present
        if let Some(order) = &plan.order_by {
            if !allowed_set.contains(&order.column) {
                result.add_error(
                    "ORDER_COLUMN_NOT_ALLOWED",
                    &format!(
                        "Order by column '{}' is not allowed for table '{}'",
                        order.column, plan.table
                    ),
                    Some("order_by"),
                );
            }
        }

        result
    }

    /// Validate raw SQL string for forbidden patterns
    /// This is a secondary check after query plan validation
    pub fn validate_sql(&self, sql: &str) -> ValidationResult {
        let mut result = ValidationResult::valid();
        let sql_upper = sql.to_uppercase();

        // Check for denied statements
        for stmt in &self.rules.deny_statements {
            let stmt_upper = stmt.to_uppercase();
            // Check for statement at start or after whitespace/semicolon
            if sql_upper.starts_with(&stmt_upper)
                || sql_upper.contains(&format!(" {} ", stmt_upper))
                || sql_upper.contains(&format!(";{}", stmt_upper))
                || sql_upper.contains(&format!("({}", stmt_upper))
            {
                result.add_error(
                    "FORBIDDEN_STATEMENT",
                    &format!("SQL statement '{}' is not allowed", stmt),
                    None,
                );
            }
        }

        // Check for denied keywords in column names or values
        for keyword in &self.rules.deny_keywords {
            let keyword_lower = keyword.to_lowercase();
            let sql_lower = sql.to_lowercase();

            // Check if keyword appears as a column name or in WHERE clause
            // Be careful not to block legitimate uses in string values
            if sql_lower.contains(&format!(".{}", keyword_lower))
                || sql_lower.contains(&format!("{} =", keyword_lower))
                || sql_lower.contains(&format!("{},", keyword_lower))
                || sql_lower.contains(&format!("select {}", keyword_lower))
            {
                result.add_error(
                    "FORBIDDEN_KEYWORD",
                    &format!(
                        "Column or field containing '{}' is not allowed to be queried",
                        keyword
                    ),
                    None,
                );
            }
        }

        // Check for subqueries (potential security risk in Stage 1)
        if sql_upper.contains("SELECT") && sql_upper.matches("SELECT").count() > 1 {
            result.add_error(
                "SUBQUERY_NOT_ALLOWED",
                "Subqueries are not allowed in Stage 1",
                None,
            );
        }

        // Check for UNION (potential security risk)
        if sql_upper.contains("UNION") {
            result.add_error("UNION_NOT_ALLOWED", "UNION queries are not allowed", None);
        }

        // Check for comments (could be used to bypass security)
        if sql.contains("--") || sql.contains("/*") {
            result.add_error("COMMENTS_NOT_ALLOWED", "SQL comments are not allowed", None);
        }

        result
    }

    /// Get the effective limit (clamped to max if necessary)
    pub fn get_effective_limit(&self, requested: i32) -> i32 {
        std::cmp::min(requested, self.rules.max_limit)
    }

    /// Get list of allowed tables
    pub fn get_allowed_tables(&self) -> Vec<String> {
        self.rules.allowed_tables.keys().cloned().collect()
    }

    /// Get allowed columns for a table
    pub fn get_allowed_columns(&self, table: &str) -> Option<&Vec<String>> {
        self.rules.allowed_tables.get(table)
    }

    /// Check if a specific table-column combination is allowed
    pub fn is_column_allowed(&self, table: &str, column: &str) -> bool {
        if let Some(columns) = self.rules.allowed_tables.get(table) {
            columns.contains(&column.to_string())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rules() -> AllowlistRules {
        AllowlistRules {
            allowed_tables: {
                let mut m = HashMap::new();
                m.insert(
                    "users_view".to_string(),
                    vec![
                        "id".to_string(),
                        "username".to_string(),
                        "status".to_string(),
                        "created_at".to_string(),
                    ],
                );
                m.insert(
                    "orders_view".to_string(),
                    vec![
                        "id".to_string(),
                        "user_id".to_string(),
                        "total".to_string(),
                        "created_at".to_string(),
                    ],
                );
                m
            },
            require_filters: {
                let mut m = HashMap::new();
                m.insert(
                    "users_view".to_string(),
                    vec!["id".to_string(), "username".to_string()],
                );
                m
            },
            max_limit: 100,
            allow_joins: false,
            deny_keywords: vec!["password".to_string(), "token".to_string()],
            deny_statements: vec!["INSERT".to_string(), "DELETE".to_string()],
            max_filters: 3,
            max_in_list_size: 10,
        }
    }

    #[test]
    fn test_valid_plan() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules)
            .with_selected_tables(vec!["users_view".to_string()]);

        let plan = QueryPlan {
            mode: "exact".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string(), "username".to_string()],
            filters: vec![QueryFilter {
                column: "username".to_string(),
                operator: "eq".to_string(),
                values: vec!["admin".to_string()],
            }],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = validator.validate_plan(&plan);
        assert!(result.is_valid, "Expected valid: {:?}", result.errors);
    }

    #[test]
    fn test_table_not_selected() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules)
            .with_selected_tables(vec!["orders_view".to_string()]);

        let plan = QueryPlan {
            mode: "exact".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string()],
            filters: vec![],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = validator.validate_plan(&plan);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "TABLE_NOT_SELECTED"));
    }

    #[test]
    fn test_table_not_allowed() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules);

        let plan = QueryPlan {
            mode: "exact".to_string(),
            table: "secret_table".to_string(),
            select: vec!["id".to_string()],
            filters: vec![],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = validator.validate_plan(&plan);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "TABLE_NOT_ALLOWED"));
    }

    #[test]
    fn test_missing_required_filter() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules)
            .with_selected_tables(vec!["users_view".to_string()]);

        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string(), "username".to_string()],
            filters: vec![], // No filters - should fail
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = validator.validate_plan(&plan);
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "MISSING_REQUIRED_FILTER"));
    }

    #[test]
    fn test_limit_clamping() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules)
            .with_selected_tables(vec!["users_view".to_string()]);

        let plan = QueryPlan {
            mode: "exact".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string()],
            filters: vec![QueryFilter {
                column: "id".to_string(),
                operator: "eq".to_string(),
                values: vec!["1".to_string()],
            }],
            limit: 500, // Exceeds max of 100
            order_by: None,
            joins: None,
        };

        let result = validator.validate_plan(&plan);
        assert!(result.is_valid); // Still valid, just clamped
        assert_eq!(result.adjusted_limit, Some(100));
    }

    #[test]
    fn test_sql_forbidden_statement() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules);

        let result = validator.validate_sql("DELETE FROM users WHERE id = 1");
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "FORBIDDEN_STATEMENT"));
    }

    #[test]
    fn test_sql_forbidden_keyword() {
        let rules = create_test_rules();
        let validator = AllowlistValidator::from_rules(rules);

        let result = validator.validate_sql("SELECT password FROM users WHERE id = 1");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "FORBIDDEN_KEYWORD"));
    }
}
