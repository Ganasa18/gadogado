use crate::application::QueryResult;

pub fn convert_db_rows_to_candidates(
    rows: &[std::collections::HashMap<String, serde_json::Value>],
    table_name: &str,
) -> Vec<QueryResult> {
    rows.iter()
        .enumerate()
        .map(|(idx, row)| {
            let content = row
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ");

            QueryResult {
                content,
                source_type: "db_row".to_string(),
                source_id: idx as i64,
                score: None,
                page_number: None,
                page_offset: None,
                doc_name: Some(table_name.to_string()),
            }
        })
        .collect()
}

pub fn restore_rows_from_candidates(
    candidates: Vec<QueryResult>,
    original_rows: &[std::collections::HashMap<String, serde_json::Value>],
) -> Vec<(
    usize,
    std::collections::HashMap<String, serde_json::Value>,
    Option<f32>,
)> {
    candidates
        .into_iter()
        .filter_map(|c| {
            let idx = c.source_id as usize;
            if idx < original_rows.len() {
                Some((idx, original_rows[idx].clone(), c.score))
            } else {
                None
            }
        })
        .collect()
}

pub fn format_sql_results_for_llm(
    rows: &[(
        usize,
        std::collections::HashMap<String, serde_json::Value>,
        Option<f32>,
    )],
    table_name: &str,
) -> String {
    if rows.is_empty() {
        return "No results found.".to_string();
    }

    let mut all_columns: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_row_idx, row, _score) in rows.iter() {
        for key in row.keys() {
            all_columns.insert(key.clone());
        }
    }

    let columns: Vec<String> = all_columns.into_iter().collect();

    let mut table = String::new();
    table.push_str(&format!(
        "Found {} result(s) from table '{}':\n\n",
        rows.len(),
        table_name
    ));

    table.push_str("| ");
    for col in &columns {
        table.push_str(&format!("{} | ", col));
    }
    table.push('\n');

    table.push_str("|");
    for _ in &columns {
        table.push_str("---|");
    }
    table.push('\n');

    for (_row_idx, row, _score) in rows.iter() {
        table.push_str("| ");
        for col in &columns {
            let value = row
                .get(col)
                .and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| {
                            let cleaned = s.replace('|', "\\|");
                            cleaned.replace(|c: char| c == '\n' || c == '\r', " ")
                        })
                    } else if v.is_null() {
                        Some("NULL".to_string())
                    } else {
                        Some(v.to_string().replace('|', "\\|"))
                    }
                })
                .unwrap_or_else(|| "NULL".to_string());

            table.push_str(&format!("{} | ", value));
        }
        table.push('\n');
    }

    table
}
