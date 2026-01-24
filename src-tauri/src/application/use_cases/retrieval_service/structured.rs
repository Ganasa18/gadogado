use super::{QueryResult, RetrievalService, StructuredQueryHints};
use crate::domain::error::Result;
use crate::infrastructure::db::rag::repository::StructuredRowWithDoc;

impl RetrievalService {
    pub(super) async fn retrieve_structured_rows(
        &self,
        collection_id: i64,
        _query_text: &str,
        hints: &StructuredQueryHints,
        top_k: usize,
    ) -> Result<Vec<QueryResult>> {
        // For aggregate queries we return a compact "row summary" list.
        // The actual answer synthesis happens in the LLM prompt stage.

        // Build filter description for context clarity
        let filter_desc = build_filter_description(hints);

        if hints.wants_count {
            let count = self
                .rag_repository
                .count_structured_rows_by_collection(
                    collection_id,
                    hints.category.as_deref(),
                    hints.source.as_deref(),
                    hints.keyword.as_deref(),
                )
                .await?;

            let mut label_parts: Vec<String> = Vec::new();
            if let Some(cat) = &hints.category {
                label_parts.push(format!("category={}", cat));
            }
            if let Some(src) = &hints.source {
                label_parts.push(format!("source={}", src));
            }

            let label = if label_parts.is_empty() {
                format!("count(all)={}", count)
            } else {
                format!("count({})={}", label_parts.join(","), count)
            };

            return Ok(vec![QueryResult {
                content: format!(
                    "Total matching rows in structured data: {} | {}",
                    count, label
                ),
                source_type: "structured_count".to_string(),
                source_id: 0,
                // Structured counts are exact DB results; treat as high confidence.
                score: Some(1.0),
                page_number: None,
                page_offset: None,
                doc_name: None,
            }]);
        }

        let limit = hints.requested_limit.unwrap_or(top_k);
        let rows: Vec<StructuredRowWithDoc> = self
            .rag_repository
            .query_structured_rows_by_collection(
                collection_id,
                hints.category.as_deref(),
                hints.source.as_deref(),
                hints.keyword.as_deref(),
                limit as i64,
            )
            .await?;

        let mut results = Vec::new();

        // Add a filter context header as the first result if we have filters and results
        if !rows.is_empty() && !filter_desc.is_empty() {
            results.push(QueryResult {
                content: format!(
                    "[SEARCH CONTEXT] The following {} results were found {}. These are exact database matches for the requested filter criteria.",
                    rows.len(),
                    filter_desc
                ),
                source_type: "search_context".to_string(),
                source_id: 0,
                score: Some(1.0),
                page_number: None,
                page_offset: None,
                doc_name: None,
            });
        }

        for r in rows {
            results.push(QueryResult {
                content: format_structured_row_summary(&r, hints),
                source_type: "structured_row".to_string(),
                source_id: r.id,
                // Structured rows are exact DB matches (not similarity scores).
                // Use a high normalized score so UI confidence/thresholds behave sensibly.
                score: Some(1.0),
                page_number: None,
                page_offset: None,
                doc_name: Some(r.doc_name),
            });
        }

        Ok(results)
    }
}

/// Build a human-readable description of what filters were applied
fn build_filter_description(hints: &StructuredQueryHints) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(cat) = &hints.category {
        parts.push(format!("filtering by category \"{}\"", cat));
    }
    if let Some(src) = &hints.source {
        parts.push(format!("filtering by source \"{}\"", src));
    }
    if let Some(kw) = &hints.keyword {
        parts.push(format!("matching keyword \"{}\"", kw));
    }

    if parts.is_empty() {
        String::new()
    } else {
        parts.join(" and ")
    }
}

fn format_structured_row_summary(row: &StructuredRowWithDoc, _hints: &StructuredQueryHints) -> String {
    // Keep it readable for the LLM; use clear field labels
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("[Row #{}]", row.row_index));

    if let Some(cat) = &row.category {
        parts.push(format!("Category: {}", cat));
    }

    // Always show source field to avoid confusion
    if let Some(source) = &row.source {
        parts.push(format!("Data Source: {}", source));
    }

    if let Some(title) = &row.title {
        parts.push(format!("Title: {}", title));
    }

    if let Some(created) = row.created_at.as_ref().or(row.created_at_text.as_ref()) {
        parts.push(format!("Created: {}", created));
    }

    if let Some(content) = &row.content {
        // Avoid huge content.
        let snippet = if content.len() > 240 {
            format!("{}...", &content[..240])
        } else {
            content.clone()
        };
        parts.push(format!("Content: {}", snippet.replace('\n', " ")));
    }

    parts.join(" | ")
}
