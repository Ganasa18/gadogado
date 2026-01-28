//! Template Matcher for Few-Shot Learning
//!
//! Matches user queries to relevant query templates using:
//! - Intent keyword matching (lexical similarity)
//! - Table overlap scoring (structural similarity)
//! - Priority-based ranking

use crate::domain::rag_entities::QueryTemplate;
use std::collections::HashSet;

/// Match score for a template
#[derive(Debug, Clone)]
pub struct TemplateMatch {
    pub template: QueryTemplate,
    pub score: f32,  // 0.0 to 1.0
    pub reason: String,  // Human-readable explanation
}

/// Template matcher for finding relevant examples
pub struct TemplateMatcher {
    templates: Vec<QueryTemplate>,
}

impl TemplateMatcher {
    /// Create a new template matcher from available templates
    pub fn new(templates: Vec<QueryTemplate>) -> Self {
        Self {
            templates: templates.into_iter().filter(|t| t.is_enabled).collect(),
        }
    }

    /// Find the best matching templates for a query
    /// Returns up to max_templates results, sorted by score (highest first)
    pub fn find_matches(
        &self,
        query: &str,
        detected_tables: &[String],
        max_templates: usize,
    ) -> Vec<TemplateMatch> {
        if self.templates.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower.split_whitespace().collect();
        let detected_table_set: HashSet<String> = detected_tables
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Score each template
        let mut matches: Vec<TemplateMatch> = self
            .templates
            .iter()
            .map(|template| {
                let (score, reason) = self.score_template(
                    &query_lower,
                    &query_words,
                    &detected_table_set,
                    template,
                );
                TemplateMatch {
                    template: template.clone(),
                    score,
                    reason,
                }
            })
            .filter(|m| m.score > 0.0)  // Only keep matches with positive scores
            .collect();

        // Sort by score (descending), then by priority (descending)
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.template.priority.cmp(&a.template.priority))
        });

        // Return top N
        matches.truncate(max_templates);
        matches
    }

    /// Score a single template against the query
    /// Returns (score, reason) where score is 0.0 to 1.0
    pub fn score_template(
        &self,
        query_lower: &str,
        query_words: &HashSet<&str>,
        detected_tables: &HashSet<String>,
        template: &QueryTemplate,
    ) -> (f32, String) {
        let mut score = 0.0f32;
        let mut reasons = Vec::new();

        if template.is_pattern_agnostic {
            // Pattern-agnostic mode: Ignore table overlap
            // Keywords (60%) + Pattern Type (40%)

            // 1. Intent keyword matching (60% weight)
            let keyword_score = self.score_keywords(query_lower, query_words, template);
            if keyword_score > 0.0 {
                score += keyword_score * 0.6;
                reasons.push(format!("keyword match: {:.2}", keyword_score));
            }

            // 2. Pattern type bonus (40% weight)
            let pattern_bonus = self.score_pattern_type(query_lower, template);
            if pattern_bonus > 0.0 {
                score += pattern_bonus * 0.4;
                reasons.push(format!("pattern match: {:.2}", pattern_bonus));
            }

            // Add "pattern-agnostic" label to reason
            if !reasons.is_empty() {
                reasons.push("pattern-agnostic".to_string());
            }
        } else {
            // Table-specific mode: Original scoring
            // Keywords (40%) + Tables (40%) + Pattern (20%)

            // 1. Intent keyword matching (40% weight)
            let keyword_score = self.score_keywords(query_lower, query_words, template);
            if keyword_score > 0.0 {
                score += keyword_score * 0.4;
                reasons.push(format!("keyword match: {:.2}", keyword_score));
            }

            // 2. Table overlap scoring (40% weight)
            let table_score = self.score_tables(detected_tables, template);
            if table_score > 0.0 {
                score += table_score * 0.4;
                reasons.push(format!("table overlap: {:.2}", table_score));
            }

            // 3. Pattern type bonus (20% weight)
            let pattern_bonus = self.score_pattern_type(query_lower, template);
            if pattern_bonus > 0.0 {
                score += pattern_bonus * 0.2;
                reasons.push(format!("pattern match: {:.2}", pattern_bonus));
            }
        }

        let reason = if reasons.is_empty() {
            "no significant match".to_string()
        } else {
            reasons.join(", ")
        };

        (score.min(1.0), reason)  // Cap at 1.0
    }

    /// Score based on intent keyword matching
    ///
    /// Scoring:
    /// - Exact phrase match: 2 points (highest)
    /// - All words present (any order): 1 point (word-set match)
    /// - Single word match: 0.5 points (partial match)
    fn score_keywords(
        &self,
        query_lower: &str,
        query_words: &HashSet<&str>,
        template: &QueryTemplate,
    ) -> f32 {
        let mut score = 0.0f32;
        let mut total_keywords = 0;

        for keyword in &template.intent_keywords {
            total_keywords += 1;
            let keyword_lower = keyword.to_lowercase();

            // Priority 1: Exact phrase match (highest score)
            if query_lower.contains(&keyword_lower) {
                score += 2.0;
            }
            // Priority 2: Word-set match - all words present but in different order
            // e.g., "merchant loan" matches "loan merchant dengan..."
            else {
                let keyword_words: Vec<&str> = keyword_lower.split_whitespace().collect();

                if keyword_words.len() > 1 {
                    // Multi-word keyword: check if all words present in query
                    let all_words_present = keyword_words.iter()
                        .all(|kw| query_words.contains(kw) || query_lower.contains(kw));

                    if all_words_present {
                        score += 1.5;  // Good match - all words present
                    } else {
                        // Partial match: count how many words are present
                        let present_count = keyword_words.iter()
                            .filter(|kw| query_words.contains(*kw) || query_lower.contains(*kw))
                            .count();

                        if present_count > 0 {
                            // Partial credit for partial word match
                            score += (present_count as f32 / keyword_words.len() as f32) * 0.5;
                        }
                    }
                } else {
                    // Single word keyword: check word-level match
                    if query_words.contains(keyword_lower.as_str()) {
                        score += 1.0;
                    }
                }
            }
        }

        if total_keywords == 0 {
            return 0.0;
        }

        // Normalize: score / (total_keywords * 2) for 0.0 to 1.0 range
        (score / (total_keywords as f32 * 2.0)).min(1.0)
    }

    /// Score based on table overlap
    fn score_tables(
        &self,
        detected_tables: &HashSet<String>,
        template: &QueryTemplate,
    ) -> f32 {
        if template.tables_used.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for template_table in &template.tables_used {
            let template_table_lower = template_table.to_lowercase();
            if detected_tables.contains(&template_table_lower) {
                matches += 1;
            }
        }

        // Score: overlap ratio
        (matches as f32) / (template.tables_used.len() as f32)
    }

    /// Score based on pattern type matching query intent
    fn score_pattern_type(&self, query_lower: &str, template: &QueryTemplate) -> f32 {
        match template.pattern_type.as_str() {
            "aggregate" => {
                // Check for aggregation keywords
                let agg_keywords = [
                    "count",
                    "sum",
                    "average",
                    "avg",
                    "total",
                    "maximum",
                    "minimum",
                    "max",
                    "min",
                    "how many",
                    "how much",
                    "jumlah",
                    "rata-rata",
                ];
                let has_agg = agg_keywords.iter().any(|kw| query_lower.contains(kw));
                if has_agg {
                    1.0
                } else {
                    0.0
                }
            }
            "select_where_in" => {
                // Check for IN clause indicators
                let in_keywords = ["in", "among", "list of", "following"];
                let has_in = in_keywords.iter().any(|kw| query_lower.contains(kw));
                if has_in {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,  // No bonus for other patterns
        }
    }

    /// Get all enabled templates
    pub fn get_templates(&self) -> &[QueryTemplate] {
        &self.templates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_template(
        id: i64,
        name: &str,
        keywords: Vec<&str>,
        tables: Vec<&str>,
        pattern: &str,
        priority: i32,
    ) -> QueryTemplate {
        QueryTemplate {
            id,
            allowlist_profile_id: 1,
            name: name.to_string(),
            description: None,
            intent_keywords: keywords.into_iter().map(String::from).collect(),
            example_question: "Test question".to_string(),
            query_pattern: "SELECT * FROM test".to_string(),
            pattern_type: pattern.to_string(),
            tables_used: tables.into_iter().map(String::from).collect(),
            priority,
            is_enabled: true,
            is_pattern_agnostic: false,  // Default to table-specific for tests
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_keyword_matching() {
        let templates = vec![create_test_template(
            1,
            "Find users",
            vec!["find", "user"],
            vec!["users_view"],
            "select_where_eq",
            10,
        )];
        let matcher = TemplateMatcher::new(templates);

        let matches = matcher.find_matches("Find user with id 123", &["users_view".to_string()], 5);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].score > 0.3); // Should have good keyword match
    }

    #[test]
    fn test_table_overlap() {
        let templates = vec![
            create_test_template(1, "Users query", vec![], vec!["users_view"], "select_where_eq", 10),
            create_test_template(2, "Orders query", vec![], vec!["orders_view"], "select_where_eq", 10),
        ];
        let matcher = TemplateMatcher::new(templates);

        let matches = matcher.find_matches("Get users", &["users_view".to_string()], 5);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].template.id, 1);
    }

    #[test]
    fn test_priority_ranking() {
        let templates = vec![
            create_test_template(1, "Low priority", vec!["users"], vec!["users_view"], "select_where_eq", 1),
            create_test_template(
                2,
                "High priority",
                vec!["users"],
                vec!["users_view"],
                "select_where_eq",
                100,
            ),
        ];
        let matcher = TemplateMatcher::new(templates);

        let matches = matcher.find_matches("Get users", &["users_view".to_string()], 5);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].template.id, 2); // High priority first
        assert_eq!(matches[1].template.id, 1);
    }

    #[test]
    fn test_no_matches() {
        let templates = vec![create_test_template(
            1,
            "Orders",
            vec!["orders"],
            vec!["orders_view"],
            "select_where_eq",
            10,
        )];
        let matcher = TemplateMatcher::new(templates);

        let matches = matcher.find_matches("Get products", &["products_view".to_string()], 5);

        assert_eq!(matches.len(), 0); // No relevant templates
    }
}
