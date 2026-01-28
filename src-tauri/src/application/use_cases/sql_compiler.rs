//! SQL Compiler for SQL-RAG
//!
//! This module compiles QueryPlan structures into parameterized SQL queries.
//! Key security features:
//! - Always uses parameter binding (no string concatenation)
//! - Explicit column lists (no SELECT *)
//! - Always enforces LIMIT
//! - SELECT-only verification
//!
//! Supports both PostgreSQL and SQLite with appropriate syntax.

use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{QueryFilter, QueryPlan};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Target database type for SQL generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DbType {
    Postgres,
    Sqlite,
}

/// Compiled SQL query with parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledQuery {
    /// The parameterized SQL query
    pub sql: String,
    /// Parameter values in order
    pub params: Vec<serde_json::Value>,
    /// Human-readable description of the query
    pub description: String,
}

/// SQL Compiler for generating parameterized queries from plans
pub struct SqlCompiler {
    db_type: DbType,
}

impl SqlCompiler {
    /// Create a new SQL compiler for the specified database type
    pub fn new(db_type: DbType) -> Self {
        Self { db_type }
    }

    /// Create a PostgreSQL compiler
    pub fn postgres() -> Self {
        Self::new(DbType::Postgres)
    }

    /// Create a SQLite compiler
    pub fn sqlite() -> Self {
        Self::new(DbType::Sqlite)
    }

    /// Compile a query plan into a parameterized SQL query
    pub fn compile(&self, plan: &QueryPlan) -> Result<CompiledQuery> {
        // Validate plan before compilation
        self.validate_plan(plan)?;

        let mut params: Vec<serde_json::Value> = Vec::new();
        let mut param_index = 1;

        // Build SELECT clause (explicit columns, no SELECT *)
        let select_clause = if plan.select.is_empty() || plan.select.contains(&"*".to_string()) {
            return Err(AppError::ValidationError(format!(
                "Explicit column list required for table '{}' (found empty or SELECT *)",
                plan.table
            )));
        } else {
            plan.select
                .iter()
                .map(|c| self.quote_identifier(c))
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Build FROM clause
        let from_clause = self.quote_identifier(&plan.table);

        // Build WHERE clause
        let (where_clause, where_params) = self.build_where_clause(&plan.filters, &mut param_index);
        params.extend(where_params);

        // Build ORDER BY clause
        let order_clause = plan.order_by.as_ref().map(|o| {
            let direction = if o.direction.to_lowercase() == "desc" {
                "DESC"
            } else {
                "ASC"
            };
            format!(
                "ORDER BY {} {}",
                self.quote_identifier(&o.column),
                direction
            )
        });

        // Build LIMIT clause (always required)
        let limit_clause = format!("LIMIT {}", plan.limit);

        // Assemble the full query
        let mut sql_parts = vec![
            format!("SELECT {}", select_clause),
            format!("FROM {}", from_clause),
        ];

        if !where_clause.is_empty() {
            sql_parts.push(format!("WHERE {}", where_clause));
        }

        if let Some(order) = order_clause {
            sql_parts.push(order);
        }

        sql_parts.push(limit_clause);

        let sql = sql_parts.join(" ");

        // Final security check
        self.verify_select_only(&sql)?;

        let description = self.generate_description(plan);

        debug!("Compiled SQL: {} with {} params", sql, params.len());

        Ok(CompiledQuery {
            sql,
            params,
            description,
        })
    }

    /// Validate the query plan before compilation
    fn validate_plan(&self, plan: &QueryPlan) -> Result<()> {
        // Table name must be valid identifier
        if plan.table.is_empty() || !self.is_valid_identifier(&plan.table) {
            return Err(AppError::ValidationError(format!(
                "Invalid table name: {}",
                plan.table
            )));
        }

        // Select columns must be valid
        for col in &plan.select {
            if col != "*" && !self.is_valid_identifier(col) {
                return Err(AppError::ValidationError(format!(
                    "Invalid column name: {}",
                    col
                )));
            }
        }

        // Limit must be positive
        if plan.limit <= 0 {
            return Err(AppError::ValidationError(
                "Limit must be a positive number".to_string(),
            ));
        }

        // Validate filter columns
        for filter in &plan.filters {
            if !self.is_valid_identifier(&filter.column) {
                return Err(AppError::ValidationError(format!(
                    "Invalid filter column: {}",
                    filter.column
                )));
            }
        }

        // Validate order by column
        if let Some(order) = &plan.order_by {
            if !self.is_valid_identifier(&order.column) {
                return Err(AppError::ValidationError(format!(
                    "Invalid order by column: {}",
                    order.column
                )));
            }
        }

        Ok(())
    }

    /// Check if a string is a valid SQL identifier
    fn is_valid_identifier(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Must start with letter or underscore
        let first = s.chars().next().unwrap();
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Quote an identifier (table or column name) for the target database
    fn quote_identifier(&self, name: &str) -> String {
        match self.db_type {
            DbType::Postgres => format!("\"{}\"", name.replace('"', "\"\"")),
            DbType::Sqlite => format!("\"{}\"", name.replace('"', "\"\"")),
        }
    }

    /// Build WHERE clause from filters
    fn build_where_clause(
        &self,
        filters: &[QueryFilter],
        param_index: &mut usize,
    ) -> (String, Vec<serde_json::Value>) {
        if filters.is_empty() {
            return (String::new(), Vec::new());
        }

        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for filter in filters {
            let (condition, filter_params) = self.build_filter_condition(filter, param_index);
            conditions.push(condition);
            params.extend(filter_params);
        }

        (conditions.join(" AND "), params)
    }

    /// Build a single filter condition
    fn build_filter_condition(
        &self,
        filter: &QueryFilter,
        param_index: &mut usize,
    ) -> (String, Vec<serde_json::Value>) {
        let column = self.quote_identifier(&filter.column);
        let mut params = Vec::new();

        let condition = match filter.operator.to_lowercase().as_str() {
            "eq" | "=" | "equals" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} = {}", column, placeholder)
            }
            "neq" | "!=" | "not_equals" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} != {}", column, placeholder)
            }
            "in" => {
                match self.db_type {
                    DbType::Postgres => {
                        // PostgreSQL: use ANY($1) with array
                        let placeholder = self.get_placeholder(*param_index);
                        *param_index += 1;
                        params.push(serde_json::Value::Array(
                            filter
                                .values
                                .iter()
                                .map(|v| self.value_to_json(v))
                                .collect(),
                        ));
                        format!("{} = ANY({})", column, placeholder)
                    }
                    DbType::Sqlite => {
                        // SQLite: use (?, ?, ?) placeholders
                        let placeholders: Vec<String> = filter
                            .values
                            .iter()
                            .map(|v| {
                                let p = self.get_placeholder(*param_index);
                                *param_index += 1;
                                params.push(self.value_to_json(v));
                                p
                            })
                            .collect();
                        format!("{} IN ({})", column, placeholders.join(", "))
                    }
                }
            }
            "like" | "contains" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    // Add wildcards for LIKE
                    params.push(serde_json::Value::String(format!("%{}%", val)));
                }
                format!("{} LIKE {}", column, placeholder)
            }
            "gte" | ">=" | "greater_or_equal" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} >= {}", column, placeholder)
            }
            "lte" | "<=" | "less_or_equal" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} <= {}", column, placeholder)
            }
            "gt" | ">" | "greater" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} > {}", column, placeholder)
            }
            "lt" | "<" | "less" => {
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} < {}", column, placeholder)
            }
            "between" => {
                let p1 = self.get_placeholder(*param_index);
                *param_index += 1;
                let p2 = self.get_placeholder(*param_index);
                *param_index += 1;
                if filter.values.len() >= 2 {
                    params.push(self.value_to_json(&filter.values[0]));
                    params.push(self.value_to_json(&filter.values[1]));
                }
                format!("{} BETWEEN {} AND {}", column, p1, p2)
            }
            "is_null" => {
                format!("{} IS NULL", column)
            }
            "is_not_null" => {
                format!("{} IS NOT NULL", column)
            }
            _ => {
                // Default to equals
                let placeholder = self.get_placeholder(*param_index);
                *param_index += 1;
                if let Some(val) = filter.values.first() {
                    params.push(self.value_to_json(val));
                }
                format!("{} = {}", column, placeholder)
            }
        };

        (condition, params)
    }

    /// Get placeholder for the given parameter index
    fn get_placeholder(&self, index: usize) -> String {
        match self.db_type {
            DbType::Postgres => format!("${}", index),
            DbType::Sqlite => "?".to_string(),
        }
    }

    /// Convert a string value to JSON, attempting to preserve type
    fn value_to_json(&self, value: &str) -> serde_json::Value {
        // Try to parse as number
        if let Ok(n) = value.parse::<i64>() {
            return serde_json::Value::Number(n.into());
        }
        if let Ok(n) = value.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(n) {
                return serde_json::Value::Number(num);
            }
        }
        // Try to parse as boolean
        if value.to_lowercase() == "true" {
            return serde_json::Value::Bool(true);
        }
        if value.to_lowercase() == "false" {
            return serde_json::Value::Bool(false);
        }
        // Default to string
        serde_json::Value::String(value.to_string())
    }

    /// Verify that the compiled SQL is SELECT-only
    fn verify_select_only(&self, sql: &str) -> Result<()> {
        let sql_upper = sql.trim().to_uppercase();

        // Must start with SELECT
        if !sql_upper.starts_with("SELECT") {
            return Err(AppError::ValidationError(
                "Query must start with SELECT".to_string(),
            ));
        }

        // Check for forbidden keywords using word boundary matching
        // This prevents false positives like "CREATED_AT" matching "CREATE"
        let forbidden = [
            "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE", "GRANT", "REVOKE",
            "PRAGMA", "ATTACH", "DETACH",
        ];

        for keyword in &forbidden {
            if self.contains_whole_word(&sql_upper, keyword) {
                return Err(AppError::ValidationError(format!(
                    "SQL contains forbidden keyword: {}",
                    keyword
                )));
            }
        }

        Ok(())
    }

    /// Check if a string contains a keyword as a whole word (not as substring)
    fn contains_whole_word(&self, text: &str, keyword: &str) -> bool {
        let keyword_len = keyword.len();
        let text_len = text.len();

        if keyword_len > text_len {
            return false;
        }

        let text_bytes = text.as_bytes();
        let keyword_bytes = keyword.as_bytes();

        for i in 0..=(text_len - keyword_len) {
            // Check if the substring matches
            if &text_bytes[i..i + keyword_len] == keyword_bytes {
                // Check if it's a whole word (surrounded by non-alphanumeric chars or boundaries)
                let before_ok = i == 0 || !text_bytes[i - 1].is_ascii_alphanumeric();
                let after_ok = i + keyword_len == text_len
                    || !text_bytes[i + keyword_len].is_ascii_alphanumeric();

                if before_ok && after_ok {
                    return true;
                }
            }
        }

        false
    }

    /// Generate a human-readable description of the query
    fn generate_description(&self, plan: &QueryPlan) -> String {
        let mut desc = format!("Query {} rows from {}", plan.mode, plan.table);

        if !plan.filters.is_empty() {
            let filter_desc: Vec<String> = plan
                .filters
                .iter()
                .map(|f| {
                    if f.values.len() == 1 {
                        format!("{} {} '{}'", f.column, f.operator, f.values[0])
                    } else {
                        format!("{} {} {:?}", f.column, f.operator, f.values)
                    }
                })
                .collect();
            desc.push_str(&format!(" where {}", filter_desc.join(" and ")));
        }

        if let Some(order) = &plan.order_by {
            desc.push_str(&format!(" ordered by {} {}", order.column, order.direction));
        }

        desc.push_str(&format!(" (limit {})", plan.limit));

        desc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rag_entities::OrderBy;

    #[test]
    fn test_simple_select_postgres() {
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "exact".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string(), "username".to_string()],
            filters: vec![QueryFilter {
                column: "id".to_string(),
                operator: "eq".to_string(),
                values: vec!["123".to_string()],
            }],
            limit: 1,
            order_by: None,
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok());
        let query = result.unwrap();
        assert!(query.sql.contains("SELECT"));
        assert!(query.sql.contains("FROM \"users_view\""));
        assert!(query.sql.contains("WHERE"));
        assert!(query.sql.contains("$1"));
        assert!(query.sql.contains("LIMIT 1"));
        assert_eq!(query.params.len(), 1);
    }

    #[test]
    fn test_in_clause_postgres() {
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string(), "username".to_string()],
            filters: vec![QueryFilter {
                column: "status".to_string(),
                operator: "in".to_string(),
                values: vec!["active".to_string(), "pending".to_string()],
            }],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok());
        let query = result.unwrap();
        assert!(query.sql.contains("ANY($1)"));
    }

    #[test]
    fn test_in_clause_sqlite() {
        let compiler = SqlCompiler::sqlite();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string(), "username".to_string()],
            filters: vec![QueryFilter {
                column: "status".to_string(),
                operator: "in".to_string(),
                values: vec!["active".to_string(), "pending".to_string()],
            }],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok());
        let query = result.unwrap();
        assert!(query.sql.contains("IN (?, ?)"));
    }

    #[test]
    fn test_order_by() {
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users_view".to_string(),
            select: vec!["id".to_string()],
            filters: vec![],
            limit: 10,
            order_by: Some(OrderBy {
                column: "created_at".to_string(),
                direction: "desc".to_string(),
            }),
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok());
        let query = result.unwrap();
        assert!(query.sql.contains("ORDER BY \"created_at\" DESC"));
    }

    #[test]
    fn test_reject_select_star() {
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users_view".to_string(),
            select: vec!["*".to_string()],
            filters: vec![],
            limit: 10,
            order_by: None,
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_err());
    }

    #[test]
    fn test_between_operator() {
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "orders_view".to_string(),
            select: vec!["id".to_string(), "total".to_string()],
            filters: vec![QueryFilter {
                column: "total".to_string(),
                operator: "between".to_string(),
                values: vec!["100".to_string(), "500".to_string()],
            }],
            limit: 50,
            order_by: None,
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok());
        let query = result.unwrap();
        assert!(query.sql.contains("BETWEEN $1 AND $2"));
        assert_eq!(query.params.len(), 2);
    }

    #[test]
    fn test_created_at_column_not_blocked() {
        // Regression test: "created_at" column should not be blocked
        // because it contains "CREATE" as a substring
        let compiler = SqlCompiler::postgres();
        let plan = QueryPlan {
            mode: "list".to_string(),
            table: "users".to_string(),
            select: vec![
                "id".to_string(),
                "name".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
            ],
            filters: vec![],
            limit: 5,
            order_by: Some(OrderBy {
                column: "created_at".to_string(),
                direction: "desc".to_string(),
            }),
            joins: None,
        };

        let result = compiler.compile(&plan);
        assert!(result.is_ok(), "Query with created_at column should compile successfully");
        let query = result.unwrap();
        assert!(query.sql.contains("\"created_at\""));
    }

    #[test]
    fn test_whole_word_keyword_detection() {
        let compiler = SqlCompiler::postgres();

        // Test that whole word detection works correctly
        assert!(compiler.contains_whole_word("DROP TABLE users", "DROP"));
        assert!(compiler.contains_whole_word("SELECT * FROM users; DROP TABLE users", "DROP"));
        assert!(!compiler.contains_whole_word("DROPDOWN", "DROP"));
        assert!(!compiler.contains_whole_word("CREATED_AT", "CREATE"));
        assert!(!compiler.contains_whole_word("\"created_at\"", "CREATE"));
        assert!(compiler.contains_whole_word("CREATE TABLE", "CREATE"));
        assert!(!compiler.contains_whole_word("UPDATED_BY", "UPDATE"));
        assert!(compiler.contains_whole_word("UPDATE users SET", "UPDATE"));
    }
}
