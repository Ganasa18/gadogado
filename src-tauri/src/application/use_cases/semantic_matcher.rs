//! Semantic Template Matcher using LLM
//!
//! Matches user queries to relevant query templates using LLM-based semantic understanding.
//! This enables cross-language matching (e.g., Indonesian queries matching English templates)
//! without requiring manual keyword maintenance.

use crate::application::use_cases::template_matcher::TemplateMatch;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::domain::rag_entities::QueryTemplate;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// LLM client trait for generating responses
pub trait LLMClient: Send + Sync {
    fn generate(
        &self,
        config: &LLMConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'static>>;
}

/// LLM semantic match result
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub template_id: i64,
    pub semantic_score: f32,     // 0.0 to 1.0
    pub reasoning: String,        // Why LLM matched this
    pub confidence: f32,          // How confident is LLM
}

/// Source of the match (for logging/debugging)
#[derive(Debug, Clone, PartialEq)]
pub enum MatchSource {
    KeywordOnly,      // LLM failed, using keyword-only
    SemanticOnly,     // No keyword match, pure LLM
    Fused,            // Both scores combined
}

/// Semantic matcher using LLM for language-agnostic template matching
pub struct SemanticMatcher<C: LLMClient + ?Sized> {
    llm_client: Arc<C>,
    config: LLMConfig,
}

impl<C: LLMClient + ?Sized> SemanticMatcher<C> {
    /// Create a new semantic matcher
    pub fn new(llm_client: Arc<C>, config: LLMConfig) -> Self {
        Self { llm_client, config }
    }

    /// Match templates semantically using LLM
    pub async fn match_templates(
        &self,
        templates: &[QueryTemplate],
        user_query: &str,
        _detected_tables: &[String],
    ) -> Result<Vec<SemanticMatch>> {
        if templates.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Build LLM prompt
        let prompt = build_semantic_matching_prompt(user_query, templates);

        // 2. Call LLM with 15s timeout (increased from 5s for external LLM providers)
        let response = timeout(
            Duration::from_secs(15),
            self.llm_client.generate(&self.config, &prompt, ""),
        )
        .await
        .map_err(|_| AppError::LLMError("LLM semantic matching timed out after 15s".to_string()))??;

        // 3. Parse JSON response
        parse_llm_response(&response, templates)
    }

    /// Constants for batched semantic matching
    const SEMANTIC_BATCH_SIZE: usize = 10;  // Templates per LLM call
    const SEMANTIC_STOP_THRESHOLD: f32 = 0.85;  // Stop if confidence > 0.85
    const MAX_SEMANTIC_BATCHES: usize = 10;  // Max 10 batches = 100 templates

    /// Match templates using BATCHED LLM calls for efficiency
    /// Sends templates in batches, early-stopping if high-confidence match found
    pub async fn match_templates_batched(
        &self,
        templates: &[QueryTemplate],
        user_query: &str,
        detected_tables: &[String],
    ) -> Result<Vec<SemanticMatch>> {
        if templates.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_matches: Vec<SemanticMatch> = Vec::new();
        let mut best_score = 0.0f32;

        // Process templates in batches
        for (batch_idx, batch) in templates.chunks(Self::SEMANTIC_BATCH_SIZE).enumerate() {
            // Match this batch using the existing match_templates method
            let batch_matches = self.match_templates(batch, user_query, detected_tables).await?;

            // Track best score
            for match_result in &batch_matches {
                if match_result.semantic_score > best_score {
                    best_score = match_result.semantic_score;
                }
            }

            all_matches.extend(batch_matches);

            // Early stopping: if we found a very confident match, stop processing
            if best_score >= Self::SEMANTIC_STOP_THRESHOLD {
                break;
            }

            // Safety limit
            if batch_idx + 1 >= Self::MAX_SEMANTIC_BATCHES {
                break;
            }
        }

        Ok(all_matches)
    }
}

/// Build the LLM prompt for semantic template matching
fn build_semantic_matching_prompt(user_query: &str, templates: &[QueryTemplate]) -> String {
    let template_list = templates
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let pattern_label = if t.is_pattern_agnostic {
                " [PATTERN-AGNOSTIC]"
            } else {
                ""
            };
            format!(
                "{}. Template: \"{}\"{} (ID: {})\n   Example: \"{}\"\n   Pattern: {}",
                idx + 1,
                t.name,
                pattern_label,
                t.id,
                t.example_question,
                t.query_pattern
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"You are a semantic template matcher. Match the user's query to templates based on INTENT, not keywords.

USER QUERY: "{}"

AVAILABLE TEMPLATES:
{}

TASK:
For each template, rate how well it matches the USER'S INTENT (ignoring language).
Consider:
- What operation is being performed? (select, filter, aggregate, join)
- What type of filtering? (by ID, by date, by name, by status)
- What pattern best fits this query?

IMPORTANT:
- Ignore language differences (Indonesian "cari" = English "find", "data" = "data/records")
- For [PATTERN-AGNOSTIC] templates, focus on the pattern, not table names
- Score 0.0-1.0 based on semantic similarity

Respond in JSON format only:
{{
  "matches": [
    {{
      "template_id": <id>,
      "semantic_similarity": <0.0-1.0>,
      "confidence": <0.0-1.0>,
      "reasoning": "<brief explanation>"
    }}
  ]
}}"#,
        user_query, template_list
    )
}

/// LLM response structure for parsing
#[derive(Debug, Deserialize)]
struct LLMResponse {
    matches: Vec<LLMMatch>,
}

#[derive(Debug, Deserialize)]
struct LLMMatch {
    template_id: i64,
    #[serde(rename = "semantic_similarity")]
    semantic_similarity: f32,
    confidence: f32,
    reasoning: String,
}

/// Parse LLM JSON response into SemanticMatch structs
fn parse_llm_response(response: &str, templates: &[QueryTemplate]) -> Result<Vec<SemanticMatch>> {
    // Clean response (remove markdown code blocks)
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse JSON
    let parsed: LLMResponse = serde_json::from_str(cleaned).map_err(|e| {
        AppError::ParseError(format!("Failed to parse LLM response as JSON: {}", e))
    })?;

    // Validate and convert to SemanticMatch
    let valid_ids: std::collections::HashSet<i64> =
        templates.iter().map(|t| t.id).collect();

    let matches: Vec<SemanticMatch> = parsed
        .matches
        .into_iter()
        .filter(|m| valid_ids.contains(&m.template_id))
        .map(|m| SemanticMatch {
            template_id: m.template_id,
            semantic_score: m.semantic_similarity.clamp(0.0, 1.0),
            reasoning: m.reasoning,
            confidence: m.confidence.clamp(0.0, 1.0),
        })
        .collect();

    Ok(matches)
}

/// Fuse keyword and semantic scores into a hybrid score with reason
pub fn fuse_scores_with_reason(
    keyword_match: Option<&TemplateMatch>,
    semantic_match: Option<&SemanticMatch>,
) -> (f32, String, MatchSource) {
    match (keyword_match, semantic_match) {
        (Some(km), Some(sm)) => {
            // Both scores available - adaptive fusion
            let final_score = if sm.confidence > 0.8 {
                // High LLM confidence: trust LLM more (cross-language)
                sm.semantic_score * 0.7 + km.score * 0.3
            } else if km.score > 0.5 {
                // Good keyword match: trust keywords more
                km.score * 0.6 + sm.semantic_score * 0.4
            } else {
                // Low confidence: balance both
                (sm.semantic_score + km.score) / 2.0
            };
            let reason = format!(
                "{} (keyword: {:.2}, semantic: {:.2}, LLM confidence: {:.2})",
                sm.reasoning, km.score, sm.semantic_score, sm.confidence
            );
            (final_score, reason, MatchSource::Fused)
        }
        (Some(km), None) => {
            // Only keyword score available
            (km.score, km.reason.clone(), MatchSource::KeywordOnly)
        }
        (None, Some(sm)) => {
            // Only semantic score available
            (sm.semantic_score, format!("{} (LLM confidence: {:.2})", sm.reasoning, sm.confidence), MatchSource::SemanticOnly)
        }
        (None, None) => (0.0, "no match".to_string(), MatchSource::KeywordOnly),
    }
}

/// Fuse keyword and semantic matches into TemplateMatch results
pub fn fuse_keyword_semantic_matches(
    all_templates: &[QueryTemplate],
    keyword_matches: Vec<TemplateMatch>,
    semantic_matches: Vec<SemanticMatch>,
) -> Vec<TemplateMatch> {
    let keyword_map: std::collections::HashMap<i64, TemplateMatch> =
        keyword_matches.into_iter().map(|m| (m.template.id, m)).collect();

    let semantic_map: std::collections::HashMap<i64, SemanticMatch> =
        semantic_matches.into_iter().map(|m| (m.template_id, m)).collect();

    all_templates
        .iter()
        .map(|template| {
            let keyword_match = keyword_map.get(&template.id);
            let semantic_match = semantic_map.get(&template.id);

            let (final_score, reason, _match_source) = fuse_scores_with_reason(keyword_match, semantic_match);

            TemplateMatch {
                template: template.clone(),
                score: final_score,
                reason,
            }
        })
        .filter(|tm| tm.score > 0.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_template(
        id: i64,
        name: &str,
        pattern: &str,
        keywords: Vec<&str>,
        is_pattern_agnostic: bool,
    ) -> QueryTemplate {
        QueryTemplate {
            id,
            allowlist_profile_id: 1,
            name: name.to_string(),
            description: None,
            intent_keywords: keywords.into_iter().map(String::from).collect(),
            example_question: "Test question".to_string(),
            query_pattern: pattern.to_string(),
            pattern_type: "select_where_eq".to_string(),
            tables_used: vec!["users_view".to_string()],
            priority: 10,
            is_enabled: true,
            is_pattern_agnostic,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_keyword_match(id: i64, score: f32) -> TemplateMatch {
        TemplateMatch {
            template: create_test_template(id, "test", "SELECT *", vec![], false),
            score,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn test_fuse_scores_both_available_high_confidence() {
        let keyword_match = create_keyword_match(1, 0.3);
        let semantic_match = SemanticMatch {
            template_id: 1,
            semantic_score: 0.9,
            reasoning: "semantic match".to_string(),
            confidence: 0.85,
        };

        let (final_score, _reason, source) = fuse_scores_with_reason(Some(&keyword_match), Some(&semantic_match));

        // High LLM confidence → weighted toward semantic (0.9 * 0.7 + 0.3 * 0.3 = 0.72)
        assert!(final_score > 0.7);
        assert!(final_score < 1.0);
        assert_eq!(source, MatchSource::Fused);
    }

    #[test]
    fn test_fuse_scores_both_available_low_confidence() {
        let keyword_match = create_keyword_match(1, 0.3);
        let semantic_match = SemanticMatch {
            template_id: 1,
            semantic_score: 0.9,
            reasoning: "semantic match".to_string(),
            confidence: 0.6, // Low confidence
        };

        let (final_score, _reason, source) = fuse_scores_with_reason(Some(&keyword_match), Some(&semantic_match));

        // Low confidence → balance both (0.9 + 0.3) / 2 = 0.6
        assert!((final_score - 0.6).abs() < 0.01);
        assert_eq!(source, MatchSource::Fused);
    }

    #[test]
    fn test_fuse_scores_keyword_only() {
        let keyword_match = create_keyword_match(1, 0.8);

        let (final_score, _reason, source) = fuse_scores_with_reason(Some(&keyword_match), None);

        assert_eq!(final_score, 0.8);
        assert_eq!(source, MatchSource::KeywordOnly);
    }

    #[test]
    fn test_fuse_scores_semantic_only() {
        let semantic_match = SemanticMatch {
            template_id: 1,
            semantic_score: 0.7,
            reasoning: "semantic match".to_string(),
            confidence: 0.75,
        };

        let (final_score, _reason, source) = fuse_scores_with_reason(None, Some(&semantic_match));

        assert_eq!(final_score, 0.7);
        assert_eq!(source, MatchSource::SemanticOnly);
    }

    #[test]
    fn test_fuse_scores_none() {
        let (final_score, _reason, source) = fuse_scores_with_reason(None, None);

        assert_eq!(final_score, 0.0);
        assert_eq!(source, MatchSource::KeywordOnly);
    }

    #[test]
    fn test_parse_llm_response() {
        let response = r#"```json
        {
          "matches": [
            {
              "template_id": 1,
              "semantic_similarity": 0.95,
              "confidence": 0.90,
              "reasoning": "User wants to find by ID"
            }
          ]
        }
        ```"#;

        let templates = vec![create_test_template(1, "test", "SELECT *", vec![], false)];
        let matches = parse_llm_response(response, &templates).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].template_id, 1);
        assert_eq!(matches[0].semantic_score, 0.95);
        assert_eq!(matches[0].confidence, 0.90);
    }

    #[test]
    fn test_parse_llm_response_filters_invalid_ids() {
        let response = r#"{
          "matches": [
            {
              "template_id": 1,
              "semantic_similarity": 0.95,
              "confidence": 0.90,
              "reasoning": "Valid"
            },
            {
              "template_id": 999,
              "semantic_similarity": 0.80,
              "confidence": 0.70,
              "reasoning": "Invalid - not in template list"
            }
          ]
        }"#;

        let templates = vec![create_test_template(1, "test", "SELECT *", vec![], false)];
        let matches = parse_llm_response(response, &templates).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].template_id, 1);
    }
}
