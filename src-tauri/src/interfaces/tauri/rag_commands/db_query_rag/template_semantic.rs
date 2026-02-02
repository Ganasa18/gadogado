use crate::application::use_cases::semantic_matcher::{fuse_keyword_semantic_matches, SemanticMatcher};
use crate::application::use_cases::template_matcher::{TemplateMatch, TemplateMatcher};
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::LLMConfig;
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::{MAX_TEMPLATES_FOR_USER, SEMANTIC_LLM_TIMEOUT_SECS};

/// Load templates using both keyword AND semantic matching (LLM-based).
/// Returns fused TemplateMatch results sorted by final score (highest first).
pub async fn load_templates_with_semantic_matching(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    profile_id: i64,
    query: &str,
    detected_tables: &[String],
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    llm_config: &crate::domain::llm_config::LLMConfig,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Vec<TemplateMatch> {
    use std::time::Duration;
    use tokio::time::timeout;

    let all_templates = match repository.list_query_templates(Some(profile_id)).await {
        Ok(templates) => templates
            .into_iter()
            .filter(|t| t.is_enabled)
            .collect::<Vec<_>>(),
        Err(e) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("Failed to load templates: {}", e),
            );
            return Vec::new();
        }
    };

    if all_templates.is_empty() {
        return Vec::new();
    }

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Loaded {} templates for semantic matching",
            all_templates.len()
        ),
    );

    struct LLMClientWrapper {
        client: Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
        config: crate::domain::llm_config::LLMConfig,
    }

    impl crate::application::use_cases::semantic_matcher::LLMClient for LLMClientWrapper {
        fn generate(
            &self,
            _config: &LLMConfig,
            system_prompt: &str,
            user_prompt: &str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + 'static>>
        {
            let client = self.client.clone();
            let config = self.config.clone();
            let system_prompt = system_prompt.to_string();
            let user_prompt = user_prompt.to_string();

            Box::pin(async move {
                client
                    .generate(&config, &system_prompt, &user_prompt)
                    .await
                    .map_err(|e| crate::domain::error::AppError::LLMError(e.to_string()))
            })
        }
    }

    let wrapper = LLMClientWrapper {
        client: llm_client.clone(),
        config: llm_config.clone(),
    };

    let semantic_matcher = SemanticMatcher::new(Arc::new(wrapper), llm_config.clone());

    let keyword_future = async {
        let matcher = TemplateMatcher::new(all_templates.clone());
        Ok::<_, AppError>(matcher.find_matches(query, detected_tables, usize::MAX))
    };

    let semantic_future = async {
        timeout(
            Duration::from_secs(SEMANTIC_LLM_TIMEOUT_SECS),
            semantic_matcher.match_templates_batched(&all_templates, query, detected_tables),
        )
        .await
    };

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        "Running keyword and semantic matching in parallel...",
    );

    let (keyword_result, semantic_result) = tokio::join!(keyword_future, semantic_future);

    let keyword_matches = keyword_result.unwrap_or_default();

    let semantic_matches = match semantic_result {
        Ok(Ok(matches)) => {
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("LLM semantic matching succeeded: {} matches", matches.len()),
            );
            matches
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM semantic matching failed: {}, using keyword-only", e),
            );
            Vec::new()
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "LLM semantic matching timed out after {}s, falling back to keyword-only",
                    SEMANTIC_LLM_TIMEOUT_SECS
                ),
            );
            Vec::new()
        }
    };

    let fused_matches = fuse_keyword_semantic_matches(&all_templates, keyword_matches, semantic_matches);

    let mut matches = fused_matches;
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.template.priority.cmp(&a.template.priority))
    });

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Semantic matching complete: {} total matches, best score: {:.2}",
            matches.len(),
            matches.first().map(|m| m.score).unwrap_or(0.0)
        ),
    );

    matches.truncate(MAX_TEMPLATES_FOR_USER);
    matches
}
