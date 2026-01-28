use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::MAX_TEMPLATES_FOR_USER;
use super::super::types::LlmTemplateSelection;
use crate::application::use_cases::template_matcher::TemplateMatch;

/// Build schema context for LLM parameter extraction.
pub fn build_schema_context_for_llm(
    _default_table_name: &str,
    _selected_columns: &[String],
    all_selected_columns: &std::collections::HashMap<String, Vec<String>>,
) -> String {
    let mut context = String::from("AVAILABLE TABLES AND COLUMNS:\n");
    context.push_str("(You must detect which table the user is querying from their question)\n\n");

    for (table_name, cols) in all_selected_columns {
        context.push_str(&format!("Table: {}\n", table_name));
        context.push_str("  Columns:\n");
        if cols.is_empty() {
            context.push_str("    (all columns accessible)\n");
        } else {
            for col in cols {
                context.push_str(&format!("    - {}\n", col));
            }
        }
        context.push('\n');
    }

    context.push_str("IMPORTANT:\n");
    context.push_str("- Detect the correct table from user's query keywords\n");
    context.push_str("- Match user's filter column to actual column names in that table\n");
    context.push_str("- Examples: 'merchant id' → merchant_id, 'user role' → role, 'loan channel' → loan_channel_id\n");

    context
}

/// Use LLM to select the best template and extract parameters from user query.
pub async fn select_template_with_llm(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    matched_templates: &[TemplateMatch],
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    schema_context: Option<&str>,
) -> Option<LlmTemplateSelection> {
    if matched_templates.is_empty() {
        return None;
    }

    let template_options: String = matched_templates
        .iter()
        .take(MAX_TEMPLATES_FOR_USER)
        .enumerate()
        .map(|(idx, tm)| {
            let pattern_type_label = if tm.template.is_pattern_agnostic {
                " [PATTERN-AGNOSTIC]".to_string()
            } else {
                String::new()
            };
            format!(
                "{}. Template: \"{}\"{} (ID: {})\n   Example: \"{}\"\n   Pattern: {}",
                idx + 1,
                tm.template.name,
                pattern_type_label,
                tm.template.id,
                tm.template.example_question,
                tm.template.query_pattern
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = if let Some(schema) = schema_context {
        format!(
            r#"You are an intelligent SQL query adapter that transforms template patterns into actual queries.

DATABASE SCHEMA:
{schema}

PATTERN-AGNOSTIC TEMPLATE ADAPTATION:
Templates marked [PATTERN-AGNOSTIC] are abstract SQL patterns that work across ANY table.
Your task is to ADAPT these patterns to the ACTUAL table and columns from the schema.

ADAPTATION RULES:
1. Templates are ABSTRACT PATTERNS - not tied to specific tables
2. Extract filter values from the user's query
3. DETECT which table the user is querying
4. If template WHERE uses a generic column (e.g., role, id), replace it with the RELEVANT column from detected schema
5. For CTE queries (start with WITH), DO NOT modify WHERE clause; extract ALL placeholders into extracted_params

Respond in JSON format only:
{{
  \"selected_template_id\": <id>,
  \"extracted_params\": {{ \"<param>\": \"<raw value>\" }},
  \"modified_where_clause\": \"<complete modified WHERE clause OR null>\",
  \"detected_table\": \"<table name>\",
  \"confidence\": <0.0-1.0>,
  \"reasoning\": \"<brief explanation>\"
}}"#,
            schema = schema
        )
    } else {
        r#"You are a SQL query template matcher.
1. Select the BEST matching template for the user's query
2. Extract parameter values from the user's query
3. Detect which table the user is querying

Respond in JSON format only:
{
  \"selected_template_id\": <id>,
  \"extracted_params\": { \"<param>\": \"<value>\" },
  \"modified_where_clause\": null,
  \"detected_table\": \"<table name or null>\",
  \"confidence\": <0.0-1.0>,
  \"reasoning\": \"<brief explanation>\"
}

CRITICAL: extracted_params values MUST be RAW VALUES ONLY - NO curly braces."#
            .to_string()
    };

    let user_prompt = format!(
        r#"User Query: \"{}\"

Available Templates:
{}

Select the best template and extract any parameter values from the user's query."#,
        user_query,
        template_options
    );

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Asking LLM to select from {} templates",
            matched_templates.len().min(MAX_TEMPLATES_FOR_USER)
        ),
    );

    match llm_client.generate(config, &system_prompt, &user_prompt).await {
        Ok(response) => {
            let cleaned = response
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            match serde_json::from_str::<LlmTemplateSelection>(cleaned) {
                Ok(selection) => {
                    add_log(
                        logs,
                        "DEBUG",
                        "SQL-RAG",
                        &format!(
                            "LLM selected template {} with confidence {:.2}",
                            selection.selected_template_id, selection.confidence
                        ),
                    );
                    Some(selection)
                }
                Err(e) => {
                    add_log(
                        logs,
                        "WARN",
                        "SQL-RAG",
                        &format!("Failed to parse LLM template selection: {}", e),
                    );
                    Some(LlmTemplateSelection {
                        selected_template_id: matched_templates[0].template.id,
                        extracted_params: std::collections::HashMap::new(),
                        modified_where_clause: None,
                        detected_table: None,
                        confidence: matched_templates[0].score,
                        reasoning: "Fallback to highest matching template".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM template selection failed: {}", e),
            );
            Some(LlmTemplateSelection {
                selected_template_id: matched_templates[0].template.id,
                extracted_params: std::collections::HashMap::new(),
                modified_where_clause: None,
                detected_table: None,
                confidence: matched_templates[0].score,
                reasoning: "Fallback due to LLM error".to_string(),
            })
        }
    }
}
