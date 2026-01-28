use crate::domain::rag_entities::QueryTemplate;
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::super::types::LlmTemplateSelection;

pub fn build_sql_from_template(
    template: &QueryTemplate,
    selection: &LlmTemplateSelection,
    allowed_columns: &[String],
    table_name: &str,
    limit: i32,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) -> Option<(String, String)> {
    let mut sql = template.query_pattern.clone();
    let is_cte_query = sql.trim().to_uppercase().starts_with("WITH ");

    if is_cte_query {
        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            "Detected CTE query - using CTE-safe parameter replacement",
        );
    }

    let columns_str = if allowed_columns.is_empty() {
        "*".to_string()
    } else {
        allowed_columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if sql.contains("{columns}") {
        sql = sql.replace("{columns}", &columns_str);
    }

    let final_table = selection
        .detected_table
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or(table_name);
    sql = sql.replace("{table}", &format!("\"{}\"", final_table));

    if !is_cte_query {
        if let Some(where_clause) = &selection.modified_where_clause {
            let where_pos = sql.find("WHERE").or_else(|| sql.find("where"));
            if let Some(pos) = where_pos {
                let before_where = sql[..pos].trim_end();
                let after_where = &sql[pos..];
                let where_end = after_where
                    .find(|c: char| {
                        c.to_ascii_uppercase() == 'L'
                            || c.to_ascii_uppercase() == 'O'
                            || c.to_ascii_uppercase() == 'G'
                    })
                    .and_then(|p| {
                        let rest = &after_where[p..];
                        if rest.to_uppercase().starts_with("LIMIT ")
                            || rest.to_uppercase().starts_with("ORDER BY ")
                            || rest.to_uppercase().starts_with("GROUP BY ")
                        {
                            Some(pos + p)
                        } else {
                            None
                        }
                    });

                if let Some(end) = where_end {
                    let suffix = &after_where[end..];
                    sql = format!("{} {} {}", before_where, where_clause, suffix);
                } else {
                    sql = format!("{} {}", before_where, where_clause);
                }
            }
        }
    } else if selection.modified_where_clause.is_some() {
        add_log(
            logs,
            "DEBUG",
            "SQL-RAG",
            "Skipping WHERE modification for CTE query - use parameter placeholders instead",
        );
    }

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Replacing params: {:?}", selection.extracted_params),
    );

    for (param, value) in &selection.extracted_params {
        let replacement = match param.as_str() {
            "order_by_column" | "group_by_column" => {
                let cleaned = value
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"')
                    .trim_start_matches('{')
                    .trim_end_matches('}');
                format!("\"{}\"", cleaned)
            }
            "sort_direction" => value.to_uppercase(),
            _ => {
                let cleaned = value
                    .trim()
                    .trim_start_matches('{')
                    .trim_end_matches('}');
                cleaned.replace('\'', "''")
            }
        };

        let single_brace = format!("{{{}}}", param);
        let double_brace = format!("{{{{{}}}}}", param);

        if sql.contains(&double_brace) {
            sql = sql.replace(&double_brace, &replacement);
        } else if sql.contains(&single_brace) {
            sql = sql.replace(&single_brace, &replacement);
        }
    }

    let mut unreplaced: Vec<String> = Vec::new();
    let mut start = 0;
    while let Some(open_pos) = sql[start..].find('{') {
        let abs_open = start + open_pos;
        if let Some(close_offset) = sql[abs_open..].find('}') {
            let placeholder = &sql[abs_open..abs_open + close_offset + 1];
            let inner = &placeholder[1..placeholder.len() - 1];

            if !inner.is_empty()
                && inner.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !placeholder.contains(':')
                && !placeholder.contains('"')
                && !placeholder.contains(' ')
                && placeholder.len() < 30
            {
                unreplaced.push(placeholder.to_string());
            }
            start = abs_open + close_offset + 1;
        } else {
            break;
        }
    }

    if !unreplaced.is_empty() {
        add_log(
            logs,
            "WARN",
            "SQL-RAG",
            &format!(
                "Template has unreplaced placeholders: {:?}. LLM may need to extract these values.",
                unreplaced
            ),
        );
    }

    if !sql.to_lowercase().contains("limit") {
        sql = format!("{} LIMIT {}", sql.trim_end_matches(';').trim(), limit);
    }

    let description = format!(
        "Template query: {} ({}){}",
        template.name,
        if selection.modified_where_clause.is_some() && !is_cte_query {
            format!(
                "modified WHERE: {}",
                selection.modified_where_clause.as_ref().unwrap()
            )
        } else if selection.extracted_params.is_empty() {
            "no params".to_string()
        } else {
            selection
                .extracted_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        },
        if is_cte_query { " [CTE]" } else { "" }
    );

    Some((sql, description))
}

pub fn hash_query(query: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let normalized = query.to_lowercase().trim().to_string();
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub async fn get_user_template_preference(
    repository: &crate::infrastructure::db::rag::repository::RagRepository,
    query_hash: &str,
    collection_id: i64,
) -> Option<i64> {
    repository
        .get_preferred_template(query_hash, collection_id)
        .await
        .ok()
        .flatten()
}
