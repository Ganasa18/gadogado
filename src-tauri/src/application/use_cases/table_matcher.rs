//! Dynamic Table and Column Matcher for SQL-RAG
//!
//! This module provides dynamic matching logic for:
//! - Finding the best table from user query based on selected_tables config
//! - Resolving table aliases and fuzzy matching
//! - Getting available columns for a matched table
//! - Handling multi-table scenarios
//!
//! The matcher is dynamically configured from DbConnection.config_json:
//! ```json
//! {
//!   "selected_tables": ["users", "addresses"],
//!   "selected_columns": {
//!     "users": ["id", "name", "email", ...],
//!     "addresses": ["id", "address", "city", ...]
//!   }
//! }
//! ```

use crate::domain::error::{AppError, Result};
use std::collections::{HashMap, HashSet};

/// Match result with confidence score
#[derive(Debug, Clone)]
pub struct TableMatch {
    pub table_name: String,
    pub confidence: f32,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// Direct exact match (user typed exact table name)
    Exact,
    /// Alias match (user typed "user" → "users_view")
    Alias,
    /// Partial/fuzzy match (user typed "addr" → "addresses")
    Fuzzy,
    /// Default fallback (only one table available)
    Default,
}

/// Dynamic table and column matcher configured from connection config
pub struct TableMatcher {
    /// Tables selected by user for this collection
    selected_tables: Vec<String>,
    /// Columns selected by user for each table
    selected_columns: HashMap<String, Vec<String>>,
    /// Table name aliases for fuzzy matching
    table_aliases: HashMap<String, String>,
    /// Common word aliases that map to tables
    word_aliases: HashMap<String, String>,
}

impl TableMatcher {
    /// Create a new matcher from DbConnection config
    pub fn from_config(
        selected_tables: Vec<String>,
        selected_columns: HashMap<String, Vec<String>>,
    ) -> Self {
        let mut table_aliases = HashMap::new();
        let mut word_aliases = HashMap::new();

        // Build table aliases from selected tables with column-aware matching
        for table in &selected_tables {
            // Get columns for this table (empty vec if not configured)
            let columns = selected_columns
                .get(table)
                .cloned()
                .unwrap_or_default();

            // Build dynamic aliases using both table name and column names
            Self::build_dynamic_aliases(table, &columns, &mut table_aliases, &mut word_aliases);
        }

        Self {
            selected_tables,
            selected_columns,
            table_aliases,
            word_aliases,
        }
    }

    /// Extract domain term from column name
    /// Examples: "customer_id" → "customer", "username" → "user", "province_code" → "province"
    fn extract_domain_term(column: &str) -> Option<String> {
        let column_lower = column.to_lowercase();
        let parts: Vec<&str> = column_lower.split('_').collect();

        // Common non-domain terms to skip
        let skip_terms = [
            "id", "at", "by", "is", "has", "created", "updated", "deleted",
            "date", "time", "timestamp", "status", "type", "active", "enabled",
            "version", "seq", "no", "num", "count", "total", "sum",
        ];

        for part in parts {
            // Skip empty parts and common non-domain terms
            if !part.is_empty() && !skip_terms.contains(&part) {
                return Some(part.to_string());
            }
        }
        None
    }

    /// Build dynamic aliases from table name and column names
    /// This replaces the hardcoded pattern matching with fully dynamic generation
    fn build_dynamic_aliases(
        table: &str,
        columns: &[String],
        table_aliases: &mut HashMap<String, String>,
        word_aliases: &mut HashMap<String, String>,
    ) {
        // 1. Extract base name from table (remove suffixes)
        let base = table
            .replace("_view", "")
            .replace("_table", "")
            .to_lowercase();

        // 2. Table aliases - exact match
        table_aliases.insert(table.to_lowercase(), table.to_string());

        // 3. Base name alias (korwil_view → korwil)
        if !base.is_empty() {
            table_aliases.insert(base.clone(), table.to_string());
            word_aliases.insert(base.clone(), table.to_string());

            // 4. Plural/singular variants
            if base.ends_with('s') {
                // korwils → korwil
                let singular = base.trim_end_matches('s');
                table_aliases.insert(singular.to_string(), table.to_string());
                word_aliases.insert(singular.to_string(), table.to_string());
            } else {
                // korwil → korwils
                let plural = format!("{}s", base);
                table_aliases.insert(plural, table.to_string());
            }
        }

        // 5. Extract domain terms from column names
        for column in columns {
            if let Some(domain_term) = Self::extract_domain_term(column) {
                // Only add if not already a table alias (avoid duplicates)
                if !table_aliases.contains_key(&domain_term) {
                    word_aliases.insert(domain_term, table.to_string());
                }
            }
        }
    }

    /// Find the best matching table from a user query
    /// Returns the matched table name with confidence score
    pub fn find_table(&self, query: &str) -> Result<TableMatch> {
        let query_lower = query.to_lowercase();

        // If only one table selected, use it as default
        if self.selected_tables.len() == 1 {
            return Ok(TableMatch {
                table_name: self.selected_tables[0].clone(),
                confidence: 1.0,
                match_type: MatchType::Default,
            });
        }

        // Try exact match first
        if let Some(m) = self.try_exact_match(&query_lower) {
            return Ok(m);
        }

        // Try alias match
        if let Some(m) = self.try_alias_match(&query_lower) {
            return Ok(m);
        }

        // Try word alias match
        if let Some(m) = self.try_word_alias_match(&query_lower) {
            return Ok(m);
        }

        // Try fuzzy match (contains)
        if let Some(m) = self.try_fuzzy_match(&query_lower) {
            return Ok(m);
        }

        // No match found - return error with suggestions
        Err(AppError::ValidationError(format!(
            "Could not determine which table to query from: '{}'. \
            Available tables: {:?}. \
            Try specifying the table name explicitly.",
            query,
            self.get_table_display_names()
        )))
    }

    /// Try exact match with selected table names
    fn try_exact_match(&self, query: &str) -> Option<TableMatch> {
        for table in &self.selected_tables {
            if query == table.to_lowercase() {
                return Some(TableMatch {
                    table_name: table.clone(),
                    confidence: 1.0,
                    match_type: MatchType::Exact,
                });
            }
        }
        None
    }

    /// Try alias match (user → users_view)
    fn try_alias_match(&self, query: &str) -> Option<TableMatch> {
        if let Some(table) = self.table_aliases.get(query) {
            if self.selected_tables.contains(table) {
                return Some(TableMatch {
                    table_name: table.clone(),
                    confidence: 0.95,
                    match_type: MatchType::Alias,
                });
            }
        }
        None
    }

    /// Try word alias match (alamat → addresses)
    fn try_word_alias_match(&self, query: &str) -> Option<TableMatch> {
        // Split query into words and check each
        for word in query.split_whitespace() {
            if let Some(table) = self.word_aliases.get(word) {
                if self.selected_tables.contains(table) {
                    return Some(TableMatch {
                        table_name: table.clone(),
                        confidence: 0.85,
                        match_type: MatchType::Alias,
                    });
                }
            }
        }
        None
    }

    /// Try fuzzy match (partial string match)
    fn try_fuzzy_match(&self, query: &str) -> Option<TableMatch> {
        let mut best_match: Option<(String, f32)> = None;

        for table in &self.selected_tables {
            let table_lower = table.to_lowercase();

            // Check if query is substring of table name
            if table_lower.contains(query) {
                let confidence = query.len() as f32 / table_lower.len() as f32;
                if best_match.as_ref().map_or(true, |(_, c)| *c < confidence) {
                    best_match = Some((table.clone(), confidence));
                }
            }

            // Check if table is substring of query
            if query.contains(&table_lower) {
                let confidence = table_lower.len() as f32 / query.len() as f32;
                if best_match.as_ref().map_or(true, |(_, c)| *c < confidence) {
                    best_match = Some((table.clone(), confidence));
                }
            }
        }

        // Only return if confidence is above threshold
        if let Some((table, confidence)) = best_match {
            if confidence >= 0.3 {
                return Some(TableMatch {
                    table_name: table,
                    confidence,
                    match_type: MatchType::Fuzzy,
                });
            }
        }

        None
    }

    /// Get selected columns for a specific table
    /// Returns the columns from config, or empty vec if table not configured
    pub fn get_columns_for_table(&self, table_name: &str) -> Vec<String> {
        // Try exact match first
        if let Some(columns) = self.selected_columns.get(table_name) {
            return columns.clone();
        }

        // Try with common suffixes
        let variants = vec![
            table_name.to_string(),
            format!("{}_view", table_name),
            format!("{}_table", table_name),
        ];

        for variant in variants {
            if let Some(columns) = self.selected_columns.get(&variant) {
                return columns.clone();
            }
        }

        // Table not configured, return empty
        Vec::new()
    }

    /// Get all available tables with their column counts
    pub fn get_available_tables(&self) -> Vec<(String, usize)> {
        self.selected_tables
            .iter()
            .map(|table| {
                let columns = self.get_columns_for_table(table);
                (table.clone(), columns.len())
            })
            .collect()
    }

    /// Get display names for tables (strips _view, _table suffixes)
    pub fn get_table_display_names(&self) -> Vec<String> {
        self.selected_tables
            .iter()
            .map(|t| {
                t.replace("_view", "")
                    .replace("_table", "")
                    .to_string()
            })
            .collect()
    }

    /// Extract table mentions from query using common patterns
    pub fn extract_table_mentions(&self, query: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        let query_lower = query.to_lowercase();

        // Check for exact table name matches
        for table in &self.selected_tables {
            let table_lower = table.to_lowercase();
            if query_lower.contains(&table_lower) {
                mentions.push(table.clone());
            }
        }

        // Check for alias matches
        for (alias, table) in &self.table_aliases {
            if query_lower.contains(alias) && !mentions.contains(table) {
                mentions.push(table.clone());
            }
        }

        // Check for word alias matches
        for (word, table) in &self.word_aliases {
            if query_lower.contains(word) && !mentions.contains(table) {
                mentions.push(table.clone());
            }
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        mentions.retain(|t| seen.insert(t.clone()));

        mentions
    }

    /// Score how well a query matches a table
    /// Returns 0.0 - 1.0, higher is better match
    pub fn score_match(&self, query: &str, table_name: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let table_lower = table_name.to_lowercase();
        let mut score = 0.0f32;

        // Exact match = highest score
        if query_lower == table_lower {
            return 1.0;
        }

        // Contains (query in table)
        if table_lower.contains(&query_lower) {
            score += 0.7;
        }

        // Contains (table in query)
        if query_lower.contains(&table_lower) {
            score += 0.7;
        }

        // Alias match
        if let Some(alias_table) = self.table_aliases.get(&query_lower) {
            if alias_table == table_name {
                score += 0.9;
            }
        }

        // Word alias match
        for word in query_lower.split_whitespace() {
            if let Some(alias_table) = self.word_aliases.get(word) {
                if alias_table == table_name {
                    score += 0.8;
                }
            }
        }

        // Cap at 1.0
        score.min(1.0)
    }

    /// Find all tables mentioned in a query, sorted by relevance
    pub fn find_all_mentioned_tables(&self, query: &str) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self
            .selected_tables
            .iter()
            .map(|table| {
                let score = self.score_match(query, table);
                (table.clone(), score)
            })
            .filter(|(_, score)| *score > 0.3) // Minimum threshold
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores
    }

    /// Check if a column exists in a table's selected columns
    pub fn has_column(&self, table_name: &str, column: &str) -> bool {
        let columns = self.get_columns_for_table(table_name);
        columns.contains(&column.to_string())
            || columns.contains(&format!("{}_view", column))
            || columns.contains(&format!("{}_id", column))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_matcher() -> TableMatcher {
        let selected_tables = vec!["users_view".to_string(), "addresses_view".to_string()];
        let mut selected_columns = HashMap::new();

        selected_columns.insert(
            "users_view".to_string(),
            vec![
                "id".to_string(),
                "name".to_string(),
                "email".to_string(),
                "role".to_string(),
                "created_at".to_string(),
            ],
        );

        selected_columns.insert(
            "addresses_view".to_string(),
            vec![
                "id".to_string(),
                "address".to_string(),
                "city".to_string(),
                "user_id".to_string(),
            ],
        );

        TableMatcher::from_config(selected_tables, selected_columns)
    }

    #[test]
    fn test_exact_match() {
        let matcher = create_test_matcher();
        let result = matcher.find_table("users_view");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "users_view");
        assert_eq!(m.match_type, MatchType::Exact);
        assert_eq!(m.confidence, 1.0);
    }

    #[test]
    fn test_alias_match() {
        let matcher = create_test_matcher();
        let result = matcher.find_table("user");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "users_view");
        assert_eq!(m.match_type, MatchType::Alias);
    }

    #[test]
    fn test_dynamic_table_korwil() {
        // Test that any new table works without hardcoded patterns
        let selected_tables = vec!["korwil_view".to_string(), "other_view".to_string()];
        let mut selected_columns = HashMap::new();
        selected_columns.insert(
            "korwil_view".to_string(),
            vec!["id".to_string(), "korwil_name".to_string(), "korwil_code".to_string()],
        );
        selected_columns.insert(
            "other_view".to_string(),
            vec!["id".to_string()],
        );

        let matcher = TableMatcher::from_config(selected_tables, selected_columns);

        // Test base name match
        let result = matcher.find_table("korwil");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "korwil_view");
        assert_eq!(m.match_type, MatchType::Alias);

        // Test exact match
        let result = matcher.find_table("korwil_view");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "korwil_view");
        assert_eq!(m.match_type, MatchType::Exact);

        // Test single table scenario returns Default
        let single_tables = vec!["korwil_view".to_string()];
        let single_matcher = TableMatcher::from_config(single_tables, HashMap::new());
        let result = single_matcher.find_table("korwil");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "korwil_view");
        assert_eq!(m.match_type, MatchType::Default);
    }

    #[test]
    fn test_fuzzy_match() {
        let matcher = create_test_matcher();
        let result = matcher.find_table("users");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "users_view");
        // "users" is now an Alias (from plural generation), not Fuzzy
        assert_eq!(m.match_type, MatchType::Alias);
    }

    #[test]
    fn test_get_columns_for_table() {
        let matcher = create_test_matcher();
        let columns = matcher.get_columns_for_table("users_view");
        assert_eq!(columns.len(), 5);
        assert!(columns.contains(&"id".to_string()));
        assert!(columns.contains(&"name".to_string()));
    }

    #[test]
    fn test_extract_table_mentions() {
        let matcher = create_test_matcher();
        // Use English keywords instead of hardcoded Indonesian
        let mentions = matcher.extract_table_mentions("cari user dan address");
        assert_eq!(mentions.len(), 2);
        assert!(mentions.contains(&"users_view".to_string()));
        assert!(mentions.contains(&"addresses_view".to_string()));
    }

    #[test]
    fn test_single_table_default() {
        let selected_tables = vec!["users_view".to_string()];
        let selected_columns = HashMap::new();
        let matcher = TableMatcher::from_config(selected_tables, selected_columns);

        let result = matcher.find_table("any random query");
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.table_name, "users_view");
        assert_eq!(m.match_type, MatchType::Default);
    }

    #[test]
    fn test_no_match_error() {
        let matcher = create_test_matcher();
        let result = matcher.find_table("products");
        assert!(result.is_err());
    }

    #[test]
    fn test_score_match() {
        let matcher = create_test_matcher();

        // Exact match
        let score1 = matcher.score_match("users_view", "users_view");
        assert_eq!(score1, 1.0);

        // Alias match
        let score2 = matcher.score_match("user", "users_view");
        assert!(score2 > 0.8);

        // Fuzzy match
        let score3 = matcher.score_match("use", "users_view");
        assert!(score3 > 0.0 && score3 < 0.8);

        // No match
        let score4 = matcher.score_match("product", "users_view");
        assert_eq!(score4, 0.0);
    }

    #[test]
    fn test_find_all_mentioned_tables() {
        let matcher = create_test_matcher();
        let results = matcher.find_all_mentioned_tables("user dan address");

        // Should find both tables via dynamic alias matching
        assert_eq!(results.len(), 2);

        // Both should have high scores (> 0.3 threshold)
        assert!(results[0].1 > 0.3);
        assert!(results[1].1 > 0.3);

        // Should contain both tables (order may vary)
        let table_names: Vec<&String> = results.iter().map(|r| &r.0).collect();
        assert!(table_names.contains(&&"users_view".to_string()));
        assert!(table_names.contains(&&"addresses_view".to_string()));
    }
}
