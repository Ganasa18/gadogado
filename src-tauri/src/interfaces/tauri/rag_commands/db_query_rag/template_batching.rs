use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::{MAX_TEMPLATE_BATCHES, MAX_TEMPLATES_FOR_USER, TEMPLATE_BATCH_SIZE, TEMPLATE_STOP_THRESHOLD};

/// Load templates with batching to improve efficiency when there are many templates.
/// Returns matched templates sorted by score (highest first).
pub async fn load_templates_with_batching(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    profile_id: i64,
    query: &str,
    detected_tables: &[String],
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Vec<crate::application::use_cases::template_matcher::TemplateMatch> {
    use crate::application::use_cases::template_matcher::{TemplateMatch, TemplateMatcher};
    use std::collections::HashSet;

    let mut all_matches: Vec<TemplateMatch> = Vec::new();
    let mut offset = 0i64;
    let mut batch_count = 0;

    let query_lower = query.to_lowercase();
    let query_words: HashSet<&str> = query_lower.split_whitespace().collect();
    let detected_table_set: HashSet<String> =
        detected_tables.iter().map(|s| s.to_lowercase()).collect();

    loop {
        let batch = match repository
            .list_query_templates_batched(
                Some(profile_id),
                offset,
                TEMPLATE_BATCH_SIZE,
                true,
            )
            .await
        {
            Ok(b) => b,
            Err(e) => {
                add_log(
                    logs,
                    "WARN",
                    "SQL-RAG",
                    &format!("Failed to load template batch {}: {}", batch_count + 1, e),
                );
                break;
            }
        };

        if batch.is_empty() {
            break;
        }

        batch_count += 1;
        if batch_count > MAX_TEMPLATE_BATCHES {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Reached max batch limit ({}), stopping retrieval",
                    MAX_TEMPLATE_BATCHES
                ),
            );
            break;
        }

        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Loaded template batch {} with {} templates",
                batch_count,
                batch.len()
            ),
        );

        let matcher = TemplateMatcher::new(batch.clone());

        let batch_matches: Vec<TemplateMatch> = batch
            .iter()
            .map(|template| {
                let (score, reason) = matcher.score_template(
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
            .filter(|m| m.score > 0.0)
            .collect();

        all_matches.extend(batch_matches);

        let best_score = all_matches
            .iter()
            .map(|m| m.score)
            .fold(0.0f32, f32::max);

        if best_score >= TEMPLATE_STOP_THRESHOLD {
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!(
                    "Found good match (score: {:.2}) after {} batches, stopping retrieval",
                    best_score, batch_count
                ),
            );
            break;
        }

        offset += TEMPLATE_BATCH_SIZE;
    }

    all_matches.sort_by(|a, b| {
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
            "Batched retrieval complete: {} batches, {} total matches, best score: {:.2}",
            batch_count,
            all_matches.len(),
            all_matches.first().map(|m| m.score).unwrap_or(0.0)
        ),
    );

    all_matches.truncate(MAX_TEMPLATES_FOR_USER);
    all_matches
}
