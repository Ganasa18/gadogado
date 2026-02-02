//! SQL-RAG Router for Query Plan Generation
//!
//! This module handles:
//! - Intent detection from natural language queries
//! - Entity extraction (usernames, IDs, date ranges)
//! - Query plan generation
//! - Plan validation before compilation
//!
//! The router uses rule-based pattern matching for Stage 1.
//! Future versions may use LLM-based intent detection.

use super::table_matcher::TableMatcher;
use super::template_matcher::TemplateMatch;
use crate::application::use_cases::allowlist_validator::AllowlistRules;
use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{DbAllowlistProfile, OrderBy, QueryFilter, QueryPlan, QueryTemplate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Query mode indicating the type of result expected
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Single row lookup (WHERE id = ?)
    Exact,
    /// Multiple rows (list/search)
    List,
    /// Aggregation (COUNT, SUM, etc.) - Stage 2
    Aggregate,
}

impl ToString for QueryMode {
    fn to_string(&self) -> String {
        match self {
            QueryMode::Exact => "exact".to_string(),
            QueryMode::List => "list".to_string(),
            QueryMode::Aggregate => "aggregate".to_string(),
        }
    }
}

/// Detected intent from the natural language query
#[derive(Debug, Clone)]
pub struct DetectedIntent {
    pub mode: QueryMode,
    pub table_hint: Option<String>,
    pub entities: Vec<ExtractedEntity>,
    pub order_hint: Option<(String, String)>,
    pub limit_hint: Option<i32>,
}

/// Entity extracted from the query
#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub entity_type: EntityType,
    pub value: String,
    pub column_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityType {
    Username,
    Email,
    Id,
    DateRange,
    Status,
    Text,
    Number,
}

/// SQL-RAG Router for converting natural language to query plans
pub struct SqlRagRouter {
    /// Available tables and their columns from allowlist
    available_tables: HashMap<String, Vec<String>>,
    /// Selected tables for this collection
    selected_tables: Vec<String>,
    /// Table name aliases for fuzzy matching
    table_aliases: HashMap<String, String>,
    /// Column name aliases for fuzzy matching
    column_aliases: HashMap<String, String>,
    /// Explicitly selected columns per table (from DbConnection config)
    selected_columns: HashMap<String, Vec<String>>,
    /// Dynamic table matcher for resolving tables from user queries
    table_matcher: TableMatcher,
    /// Optional template matcher for few-shot learning
    template_matcher: Option<super::template_matcher::TemplateMatcher>,
}

impl SqlRagRouter {
    /// Create a new router from an allowlist profile
    pub fn from_profile(
        profile: &DbAllowlistProfile,
        selected_tables: Vec<String>,
        selected_columns: HashMap<String, Vec<String>>,
    ) -> Result<Self> {
        let rules: AllowlistRules = serde_json::from_str(&profile.rules_json).map_err(|e| {
            AppError::ValidationError(format!("Invalid allowlist rules JSON: {}", e))
        })?;

        Ok(Self::from_rules(rules, selected_tables, selected_columns))
    }

    /// Create a new router from allowlist rules directly
    pub fn from_rules(
        rules: AllowlistRules,
        selected_tables: Vec<String>,
        selected_columns: HashMap<String, Vec<String>>,
    ) -> Self {
        let mut table_aliases = HashMap::new();
        let mut column_aliases = HashMap::new();

        // Build table aliases (e.g., "user" -> "users_view", "users" -> "users_view")
        for table in rules.allowed_tables.keys() {
            let base = table.replace("_view", "").replace("_table", "");
            table_aliases.insert(base.clone(), table.clone());
            table_aliases.insert(format!("{}s", base), table.clone());
            table_aliases.insert(table.clone(), table.clone());
        }

        // Build common column aliases
        column_aliases.insert("user".to_string(), "username".to_string());
        column_aliases.insert("name".to_string(), "username".to_string());
        column_aliases.insert("date".to_string(), "created_at".to_string());
        column_aliases.insert("created".to_string(), "created_at".to_string());

        // Create dynamic table matcher
        let table_matcher = TableMatcher::from_config(selected_tables.clone(), selected_columns.clone());

        Self {
            available_tables: rules.allowed_tables,
            selected_tables,
            table_aliases,
            column_aliases,
            selected_columns,
            table_matcher,
            template_matcher: None, // Will be set separately
        }
    }

    /// Set template matcher for few-shot learning
    pub fn with_templates(mut self, templates: Vec<QueryTemplate>) -> Self {
        use super::template_matcher::TemplateMatcher;
        self.template_matcher = Some(TemplateMatcher::new(templates));
        self
    }

    /// Get matched templates for a query (for few-shot prompting)
    pub fn get_matched_templates(
        &self,
        query: &str,
        detected_tables: &[String],
        max_templates: usize,
    ) -> Vec<TemplateMatch> {
        if let Some(ref matcher) = self.template_matcher {
            matcher.find_matches(query, detected_tables, max_templates)
        } else {
            Vec::new()
        }
    }

    /// Generate a query plan from a natural language query
    pub fn generate_plan(&self, query: &str, default_limit: i32) -> Result<QueryPlan> {
        let query_lower = query.to_lowercase();

        // Step 1: Detect intent
        let intent = self.detect_intent(&query_lower);
        debug!("Detected intent: {:?}", intent);

        // Step 2: Determine target table
        let table = self.resolve_table(&intent, &query_lower)?;

        // Verify table is in selected tables
        if !self.selected_tables.is_empty() && !self.selected_tables.contains(&table) {
            return Err(AppError::ValidationError(format!(
                "Table '{}' is not selected for this collection. Selected tables: {:?}",
                table, self.selected_tables
            )));
        }

        // Step 3: Get allowed columns for the table
        let allowed_columns = self
            .available_tables
            .get(&table)
            .ok_or_else(|| AppError::ValidationError(format!("Table '{}' not found", table)))?;

        // Step 4: Build filters from entities
        let filters = self.build_filters(&intent, allowed_columns);

        // Step 5: Determine select columns
        let select = self.determine_select_columns(&intent, allowed_columns)?;

        // Step 6: Build order by if hinted
        let order_by = intent.order_hint.and_then(|(col, dir)| {
            let resolved_col = self.column_aliases.get(&col).unwrap_or(&col).clone();
            if allowed_columns.contains(&resolved_col) {
                Some(OrderBy {
                    column: resolved_col,
                    direction: dir,
                })
            } else {
                None
            }
        });

        // Step 7: Determine limit
        let limit = intent.limit_hint.unwrap_or(default_limit);

        let plan = QueryPlan {
            mode: intent.mode.to_string(),
            table,
            select,
            filters,
            limit,
            order_by,
            joins: None,
        };

        info!("Generated query plan: {:?}", plan);
        Ok(plan)
    }

    /// Detect intent from the query
    fn detect_intent(&self, query: &str) -> DetectedIntent {
        let mut mode = QueryMode::List;
        let mut entities = Vec::new();
        let mut table_hint = None;
        let mut order_hint = None;
        let mut limit_hint = None;

        // Detect exact lookup patterns
        let exact_patterns = [
            r#"find\s+(\w+)\s+(?:with\s+)?(?:id|username|email)\s*[=:]\s*['"]?(\w+)['"]?"#,
            r#"get\s+(\w+)\s+(?:where\s+)?(?:id|username)\s*[=:]\s*['"]?(\w+)['"]?"#,
            r#"show\s+(\w+)\s+['"]?(\w+)['"]?$"#,
            r#"(?:id|username|email)\s*[=:]\s*['"]?(\w+)['"]?"#,
        ];

        for pattern in &exact_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(query) {
                    mode = QueryMode::Exact;
                    break;
                }
            }
        }

        // Detect list patterns
        let list_patterns = [
            "list all",
            "show all",
            "find all",
            "get all",
            "search for",
            "where",
        ];
        for pattern in &list_patterns {
            if query.contains(pattern) {
                mode = QueryMode::List;
            }
        }

        // Detect table hints from query
        for (alias, table) in &self.table_aliases {
            if query.contains(alias) {
                table_hint = Some(table.clone());
                break;
            }
        }

        // Extract entities
        entities.extend(self.extract_entities(query));

        // Detect ordering
        if query.contains("latest") || query.contains("newest") || query.contains("recent") {
            order_hint = Some(("created_at".to_string(), "desc".to_string()));
        } else if query.contains("oldest") || query.contains("first") {
            order_hint = Some(("created_at".to_string(), "asc".to_string()));
        }

        // Extract ORDER BY clause
        if let Ok(re) = Regex::new(r"order\s+by\s+(\w+)\s*(asc|desc)?") {
            if let Some(caps) = re.captures(query) {
                let col = caps.get(1).map(|m| m.as_str().to_string()).unwrap();
                let dir = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or("asc".to_string());
                order_hint = Some((col, dir));
            }
        }

        // Extract limit
        if let Ok(re) = Regex::new(r"(?:limit|top|first)\s+(\d+)") {
            if let Some(caps) = re.captures(query) {
                if let Some(n) = caps.get(1) {
                    limit_hint = n.as_str().parse().ok();
                }
            }
        }

        DetectedIntent {
            mode,
            table_hint,
            entities,
            order_hint,
            limit_hint,
        }
    }

    /// Extract entities from the query
    fn extract_entities(&self, query: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();

        // Extract usernames (quoted or after keywords)
        if let Ok(re) = Regex::new(r#"(?:user(?:name)?|name)\s*[=:]\s*['"]?(\w+)['"]?"#) {
            for caps in re.captures_iter(query) {
                if let Some(val) = caps.get(1) {
                    entities.push(ExtractedEntity {
                        entity_type: EntityType::Username,
                        value: val.as_str().to_string(),
                        column_hint: Some("username".to_string()),
                    });
                }
            }
        }

        // Extract emails
        if let Ok(re) = Regex::new(r"[\w.+-]+@[\w-]+\.[\w.-]+") {
            for cap in re.find_iter(query) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Email,
                    value: cap.as_str().to_string(),
                    column_hint: Some("email".to_string()),
                });
            }
        }

        // Extract IDs
        if let Ok(re) = Regex::new(r"(?:id)\s*[=:]\s*(\d+)") {
            for caps in re.captures_iter(query) {
                if let Some(val) = caps.get(1) {
                    entities.push(ExtractedEntity {
                        entity_type: EntityType::Id,
                        value: val.as_str().to_string(),
                        column_hint: Some("id".to_string()),
                    });
                }
            }
        }

        // Extract status values
        let status_keywords = ["active", "inactive", "pending", "disabled", "enabled"];
        for status in &status_keywords {
            if query.contains(status) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Status,
                    value: status.to_string(),
                    column_hint: Some("status".to_string()),
                });
            }
        }

        // Extract quoted strings as text entities
        if let Ok(re) = Regex::new(r#"['"]([\w\s]+)['"]"#) {
            for caps in re.captures_iter(query) {
                if let Some(val) = caps.get(1) {
                    let val_str = val.as_str().to_string();
                    // Don't add if already captured as another type
                    if !entities.iter().any(|e| e.value == val_str) {
                        entities.push(ExtractedEntity {
                            entity_type: EntityType::Text,
                            value: val_str,
                            column_hint: None,
                        });
                    }
                }
            }
        }

        // Extract IN clause values
        if let Ok(re) = Regex::new(r"(?:in|among)\s*\(([^)]+)\)") {
            for caps in re.captures_iter(query) {
                if let Some(vals) = caps.get(1) {
                    let values: Vec<String> = vals
                        .as_str()
                        .split(',')
                        .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for val in values {
                        entities.push(ExtractedEntity {
                            entity_type: EntityType::Text,
                            value: val,
                            column_hint: None,
                        });
                    }
                }
            }
        }

        entities
    }

    /// Resolve the target table from intent and query
    /// Now uses dynamic TableMatcher for flexible table resolution
    fn resolve_table(&self, intent: &DetectedIntent, query: &str) -> Result<String> {
        // First, use table hint if available from intent detection
        if let Some(ref table) = intent.table_hint {
            // Verify table is in selected tables
            if self.selected_tables.contains(table) {
                return Ok(table.clone());
            }
        }

        // Use dynamic table matcher for intelligent table resolution
        let query_lower = query.to_lowercase();
        match self.table_matcher.find_table(&query_lower) {
            Ok(table_match) => Ok(table_match.table_name),
            Err(e) => {
                // Fallback: try to infer from entity types
                for entity in &intent.entities {
                    match entity.entity_type {
                        EntityType::Username | EntityType::Email => {
                            if let Some(table) = self.table_aliases.get("user") {
                                if self.selected_tables.contains(table) {
                                    return Ok(table.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Default to first selected table if only one
                if self.selected_tables.len() == 1 {
                    return Ok(self.selected_tables[0].clone());
                }

                // Return the original matcher error with more context
                Err(AppError::ValidationError(format!(
                    "{}. Available tables in this collection: {:?}. Hint: Try specifying the table name explicitly in your query.",
                    e,
                    self.table_matcher.get_table_display_names()
                )))
            }
        }
    }

    /// Build filters from extracted entities
    fn build_filters(
        &self,
        intent: &DetectedIntent,
        allowed_columns: &[String],
    ) -> Vec<QueryFilter> {
        let mut filters = Vec::new();

        // Group entities by column hint
        let mut by_column: HashMap<String, Vec<String>> = HashMap::new();

        for entity in &intent.entities {
            let column = entity
                .column_hint
                .as_ref()
                .map(|c| self.column_aliases.get(c).unwrap_or(c).clone())
                .unwrap_or_else(|| {
                    // Try to infer column from entity type
                    match entity.entity_type {
                        EntityType::Username => "username".to_string(),
                        EntityType::Email => "email".to_string(),
                        EntityType::Id => "id".to_string(),
                        EntityType::Status => "status".to_string(),
                        _ => "id".to_string(), // default fallback
                    }
                });

            if allowed_columns.contains(&column) {
                by_column
                    .entry(column)
                    .or_default()
                    .push(entity.value.clone());
            }
        }

        // Convert to filters
        for (column, values) in by_column {
            let operator = if values.len() == 1 { "eq" } else { "in" };
            filters.push(QueryFilter {
                column,
                operator: operator.to_string(),
                values,
            });
        }

        filters
    }

    /// Determine which columns to select
    fn determine_select_columns(
        &self,
        intent: &DetectedIntent,
        allowed_columns: &[String],
    ) -> Result<Vec<String>> {
        // Basis for selection: either user-selected columns or the allowlist columns
        let table_name = intent.table_hint.clone().unwrap_or_default();
        let base_columns = if let Some(columns) = self.selected_columns.get(&table_name) {
            // If we have specific columns selected for this table, use them
            columns.clone()
        } else if let Some(alias) = self.table_aliases.get(&table_name) {
            // Try via alias
            self.selected_columns
                .get(alias)
                .cloned()
                .unwrap_or_else(|| allowed_columns.to_vec())
        } else {
            // Fallback to allowed columns (allowlist)
            allowed_columns.to_vec()
        };

        // Filter out wildcard and ensure they exist in allowed_columns
        let filtered_allowed: Vec<String> = base_columns
            .iter()
            .filter(|c| *c != "*" && allowed_columns.contains(c))
            .cloned()
            .collect();

        // For exact lookup, return all filtered allowed columns
        if intent.mode == QueryMode::Exact {
            if filtered_allowed.is_empty() {
                return Err(AppError::ValidationError(format!(
                    "No accessible columns found for table '{}'",
                    intent.table_hint.as_ref().unwrap_or(&"unknown".to_string())
                )));
            }
            return Ok(filtered_allowed);
        }

        // For list mode, return a subset of useful columns
        let priority_cols = ["id", "username", "name", "status", "created_at", "title"];
        let mut selected: Vec<String> = Vec::new();

        for col in &priority_cols {
            if filtered_allowed.contains(&col.to_string()) {
                selected.push(col.to_string());
            }
        }

        // Add any remaining columns up to a reasonable limit (8)
        // Increased from 5 to 8 since the user has a long list of selected columns
        for col in &filtered_allowed {
            if !selected.contains(col) && selected.len() < 8 {
                selected.push(col.clone());
            }
        }

        if selected.is_empty() {
            if filtered_allowed.is_empty() {
                return Err(AppError::ValidationError(format!(
                    "No accessible columns found for table '{}'",
                    intent.table_hint.as_ref().unwrap_or(&"unknown".to_string())
                )));
            }
            // Fallback to all filtered columns
            Ok(filtered_allowed)
        } else {
            Ok(selected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_router() -> SqlRagRouter {
        let mut allowed_tables = HashMap::new();
        allowed_tables.insert(
            "users_view".to_string(),
            vec![
                "id".to_string(),
                "username".to_string(),
                "email".to_string(),
                "status".to_string(),
                "created_at".to_string(),
            ],
        );
        allowed_tables.insert(
            "orders_view".to_string(),
            vec![
                "id".to_string(),
                "user_id".to_string(),
                "total".to_string(),
                "status".to_string(),
                "created_at".to_string(),
            ],
        );

        let rules = AllowlistRules {
            allowed_tables,
            ..Default::default()
        };

        SqlRagRouter::from_rules(
            rules,
            vec!["users_view".to_string(), "orders_view".to_string()],
            HashMap::new(),
        )
    }

    #[test]
    fn test_exact_lookup() {
        let router = create_test_router();
        let plan = router.generate_plan("find user with username = admin", 50);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.table, "users_view");
        assert_eq!(plan.mode, "exact");
        assert!(plan.filters.iter().any(|f| f.column == "username"));
    }

    #[test]
    fn test_list_query() {
        let router = create_test_router();
        let plan = router.generate_plan("list all users with status = active", 50);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.mode, "list");
    }

    #[test]
    fn test_order_detection() {
        let router = create_test_router();
        let plan = router.generate_plan("show latest users", 50);
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert!(plan.order_by.is_some());
        let order = plan.order_by.unwrap();
        assert_eq!(order.column, "created_at");
        assert_eq!(order.direction, "desc");
    }

    #[test]
    fn test_entity_extraction() {
        let router = create_test_router();
        let intent = router.detect_intent("find user with username = admin");
        assert!(!intent.entities.is_empty());
        assert!(intent
            .entities
            .iter()
            .any(|e| e.entity_type == EntityType::Username));
    }
}
