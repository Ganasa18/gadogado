use crate::domain::rag_entities::QueryTemplate;
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::super::types::LlmTemplateSelection;

/// Placeholder replacement with type information
struct PlaceholderReplacement {
    placeholder: &'static str,
    description: &'static str,
}

/// Known SQL placeholders with their descriptions
const KNOWN_PLACEHOLDERS: &[PlaceholderReplacement] = &[
    PlaceholderReplacement { placeholder: "{columns}", description: "Column list from allowlist" },
    PlaceholderReplacement { placeholder: "{table}", description: "Table name" },
    PlaceholderReplacement { placeholder: "{id}", description: "Single ID value" },
    PlaceholderReplacement { placeholder: "{id_list}", description: "Multiple IDs for IN clause" },
    PlaceholderReplacement { placeholder: "{id_column}", description: "ID column name" },
    PlaceholderReplacement { placeholder: "{date_start}", description: "Start date for BETWEEN" },
    PlaceholderReplacement { placeholder: "{date_end}", description: "End date for BETWEEN" },
    PlaceholderReplacement { placeholder: "{search_term}", description: "Search text for LIKE" },
    PlaceholderReplacement { placeholder: "{filter_column}", description: "Column for WHERE filter" },
    PlaceholderReplacement { placeholder: "{order_by_column}", description: "Column for ORDER BY" },
    PlaceholderReplacement { placeholder: "{sort_direction}", description: "ASC or DESC" },
    PlaceholderReplacement { placeholder: "{group_by_column}", description: "Column for GROUP BY" },
    PlaceholderReplacement { placeholder: "{numeric_column}", description: "Numeric column for aggregation" },
    PlaceholderReplacement { placeholder: "{date_column}", description: "Date column for filtering" },
    PlaceholderReplacement { placeholder: "{related_table}", description: "Related table for JOIN" },
    PlaceholderReplacement { placeholder: "{foreign_key_column}", description: "Foreign key column for JOIN" },
    PlaceholderReplacement { placeholder: "{main_table_columns}", description: "Main table columns for JOIN" },
    PlaceholderReplacement { placeholder: "{related_table_columns}", description: "Related table columns for JOIN" },
    PlaceholderReplacement { placeholder: "{main_table_prefix}", description: "Main table alias (e.g., m, t1)" },
    PlaceholderReplacement { placeholder: "{related_table_prefix}", description: "Related table alias (e.g., r, t2)" },
    PlaceholderReplacement { placeholder: "{main_table}", description: "Main table name" },
    PlaceholderReplacement { placeholder: "{filter_column_1}", description: "First filter column for multi-condition WHERE" },
    PlaceholderReplacement { placeholder: "{search_term_1}", description: "First search value for multi-condition WHERE" },
    PlaceholderReplacement { placeholder: "{filter_column_2}", description: "Second filter column for multi-condition WHERE" },
    PlaceholderReplacement { placeholder: "{search_term_2}", description: "Second search value for multi-condition WHERE" },
    PlaceholderReplacement { placeholder: "{text_column}", description: "Text column for LIKE search" },
];

/// Check if a placeholder is a known SQL placeholder
fn is_known_placeholder(placeholder: &str) -> bool {
    KNOWN_PLACEHOLDERS.iter().any(|p| p.placeholder == placeholder)
}

/// Get description for a known placeholder
fn get_placeholder_description(placeholder: &str) -> Option<&'static str> {
    KNOWN_PLACEHOLDERS.iter()
        .find(|p| p.placeholder == placeholder)
        .map(|p| p.description)
}

/// Strip any table alias prefixes from LLM-generated column lists.
/// LLM may return "mlm.col1, mlm.col2" but we need "m."col1", m."col2""
/// or just bare column names depending on the template.
fn strip_column_prefixes(columns: &str, correct_alias: &str) -> String {
    columns
        .split(',')
        .map(|col| {
            let col = col.trim();
            // Strip any "prefix." from the column (e.g., "mlm.merchant_id" → "merchant_id")
            let bare_col = if let Some(dot_pos) = col.find('.') {
                col[dot_pos + 1..].trim()
            } else {
                col
            };
            // Re-add the correct alias prefix and quote the column name
            // But preserve "AS" aliases if present
            if let Some(as_pos) = bare_col.to_uppercase().find(" AS ") {
                let col_name = bare_col[..as_pos].trim().trim_matches('"');
                let alias = bare_col[as_pos..].trim();
                format!("{}.\"{}\" {}", correct_alias, col_name, alias)
            } else {
                let col_name = bare_col.trim_matches('"');
                format!("{}.\"{}\"", correct_alias, col_name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

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

    // Collect columns that appear as separate placeholders to avoid duplicates
    let mut exclude_from_columns: Vec<String> = Vec::new();
    for param_name in ["group_by_column", "numeric_column", "date_column"] {
        if sql.contains(&format!("{{{}}}", param_name)) {
            if let Some(val) = selection.extracted_params.get(param_name) {
                let cleaned = val.trim().trim_matches('\'').trim_matches('"')
                    .trim_start_matches('{').trim_end_matches('}');
                exclude_from_columns.push(cleaned.to_string());
            }
        }
    }

    let columns_str = if allowed_columns.is_empty() {
        "*".to_string()
    } else {
        allowed_columns
            .iter()
            .filter(|c| !exclude_from_columns.contains(c))
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

    // Replace {main_table} with the same resolved table name
    if sql.contains("{main_table}") {
        sql = sql.replace("{main_table}", &format!("\"{}\"", final_table));
        add_log(logs, "DEBUG", "SQL-RAG", &format!("Replaced {{main_table}} with: {}", final_table));
    }

    // Replace {main_table_prefix} and {related_table_prefix} with fixed aliases
    if sql.contains("{main_table_prefix}") {
        sql = sql.replace("{main_table_prefix}", "m");
        add_log(logs, "DEBUG", "SQL-RAG", "Replaced {main_table_prefix} with: m");
    }
    if sql.contains("{related_table_prefix}") {
        sql = sql.replace("{related_table_prefix}", "r");
        add_log(logs, "DEBUG", "SQL-RAG", "Replaced {related_table_prefix} with: r");
    }

    // Replace {id_column} with default "id" if not provided by LLM params
    if sql.contains("{id_column}") && !selection.extracted_params.contains_key("id_column") {
        sql = sql.replace("{id_column}", "\"id\"");
        add_log(logs, "DEBUG", "SQL-RAG", "Auto-replaced {id_column} with default: id");
    }

    // =========================================================================
    // JOIN-SPECIFIC PLACEHOLDERS (for LEFT JOIN templates)
    // =========================================================================

    // Replace {related_table}
    if let Some(ref related) = selection.related_table {
        if !related.is_empty() {
            sql = sql.replace("{related_table}", &format!("\"{}\"", related));
            add_log(logs, "DEBUG", "SQL-RAG", &format!("Replaced {{related_table}} with: {}", related));
        }
    }

    // Replace {foreign_key_column}
    if let Some(ref fk_col) = selection.foreign_key_column {
        if !fk_col.is_empty() {
            sql = sql.replace("{foreign_key_column}", &format!("\"{}\"", fk_col));
            add_log(logs, "DEBUG", "SQL-RAG", &format!("Replaced {{foreign_key_column}} with: {}", fk_col));
        }
    }

    // Replace {main_table_columns}
    // Strip any table alias prefixes the LLM may have added (e.g., "mlm.col" → "m."col"")
    if let Some(ref main_cols) = selection.main_table_columns {
        if !main_cols.is_empty() {
            let cleaned = strip_column_prefixes(main_cols, "m");
            sql = sql.replace("{main_table_columns}", &cleaned);
            add_log(logs, "DEBUG", "SQL-RAG", &format!("Replaced {{main_table_columns}} with: {}", cleaned));
        }
    }

    // Replace {related_table_columns}
    if let Some(ref related_cols) = selection.related_table_columns {
        if !related_cols.is_empty() {
            let cleaned = strip_column_prefixes(related_cols, "r");
            sql = sql.replace("{related_table_columns}", &cleaned);
            add_log(logs, "DEBUG", "SQL-RAG", &format!("Replaced {{related_table_columns}} with: {}", cleaned));
        }
    }

    // =========================================================================
    // END JOIN PLACEHOLDERS
    // =========================================================================

    if !is_cte_query {
        if let Some(where_clause) = &selection.modified_where_clause {
            let where_pos = sql.find("WHERE").or_else(|| sql.find("where"));
            if let Some(pos) = where_pos {
                let before_where = sql[..pos].trim_end();
                let after_where = &sql[pos..];
                let upper_after = after_where.to_uppercase();

                // Find the first SQL clause keyword that follows WHERE at a word boundary
                // (preceded by whitespace or newline to avoid matching inside identifiers/values)
                let clause_keywords = ["LIMIT ", "ORDER BY ", "GROUP BY ", "HAVING "];
                let mut earliest: Option<usize> = None;
                for keyword in &clause_keywords {
                    // Search for keyword preceded by whitespace (word boundary)
                    let mut search_start = 0;
                    while let Some(found) = upper_after[search_start..].find(keyword) {
                        let abs_pos = search_start + found;
                        // Must be preceded by whitespace or newline (word boundary)
                        if abs_pos == 0 || upper_after.as_bytes()[abs_pos - 1].is_ascii_whitespace() {
                            match earliest {
                                Some(e) if abs_pos < e => earliest = Some(abs_pos),
                                None => earliest = Some(abs_pos),
                                _ => {}
                            }
                            break;
                        }
                        search_start = abs_pos + 1;
                    }
                }

                if let Some(end_offset) = earliest {
                    let suffix = &after_where[end_offset..];
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
            // All column/identifier params must be quoted as SQL identifiers
            "order_by_column" | "group_by_column" | "id_column" | "filter_column"
            | "filter_column_1" | "filter_column_2" | "date_column" | "numeric_column"
            | "text_column" => {
                let cleaned = value
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"')
                    .trim_start_matches('{')
                    .trim_end_matches('}');
                format!("\"{}\"", cleaned)
            }
            "sort_direction" => {
                let upper = value.trim().to_uppercase();
                if upper == "ASC" || upper == "DESC" {
                    upper
                } else {
                    add_log(logs, "WARN", "SQL-RAG", &format!(
                        "Invalid sort_direction '{}', defaulting to ASC", value
                    ));
                    "ASC".to_string()
                }
            }
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
        // Classify unreplaced placeholders into known and unknown
        let known: Vec<String> = unreplaced.iter()
            .filter(|p| is_known_placeholder(p))
            .cloned()
            .collect();

        let unknown: Vec<String> = unreplaced.iter()
            .filter(|p| !is_known_placeholder(p))
            .cloned()
            .collect();

        if !known.is_empty() {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Template has unreplaced KNOWN placeholders: {:?}. These should be extracted by LLM.",
                    known
                ),
            );
        }

        if !unknown.is_empty() {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Template has unreplaced UNKNOWN placeholders: {:?}. These may be custom or typos. Known placeholders: {:?}",
                    unknown,
                    KNOWN_PLACEHOLDERS.iter().map(|p| p.placeholder).collect::<Vec<_>>()
                ),
            );
        }
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
