use crate::domain::error::Result;
use crate::domain::rag_entities::DbConnectionConfig;
use crate::interfaces::http::add_log;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

use super::constants::{DEFAULT_LIMIT, FINAL_K};
use super::helpers::{parse_collection_config, truncate_query_for_log, CollectionConfig};
use super::nl::generate_nl_response;
use super::results::{convert_db_rows_to_candidates, format_sql_results_for_llm, restore_rows_from_candidates};
use super::template_llm::build_schema_context_for_llm;
use super::template_llm::select_template_with_llm;
use super::template_sql::build_sql_from_template;
use crate::application::use_cases::template_matcher::TemplateMatch;
use super::super::types::{DbCitation, DbQueryResponse, DbQueryTelemetry, DbQueryWithTemplateRequest, LlmTemplateSelection, TemplateMatchInfo};
use crate::domain::rag_entities::QueryPlan;

pub async fn db_query_rag_with_template_impl(
    state: State<'_, Arc<super::super::AppState>>,
    request: DbQueryWithTemplateRequest,
) -> Result<DbQueryResponse> {
    let start = Instant::now();

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Processing query with template {} for collection {}: {}",
            request.template_id,
            request.collection_id,
            truncate_query_for_log(&request.query)
        ),
    );

    let collection = state.rag_repository.get_collection(request.collection_id).await?;
    if !matches!(
        collection.kind,
        crate::domain::rag_entities::CollectionKind::Db
    ) {
        return Err(crate::domain::error::AppError::ValidationError(
            "This collection is not a DB collection.".to_string(),
        ));
    }

    let config_json = parse_collection_config(&collection.config_json, &state.logs)?;
    let collection_config = CollectionConfig::from_json(&config_json)?;

    let db_conn = state
        .rag_repository
        .get_db_connection(collection_config.db_conn_id)
        .await?;

    let template = state
        .rag_repository
        .get_query_template(request.template_id)
        .await
        .map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to get template {}: {}", request.template_id, e),
            );
            e
        })?;

    let _allowlist_profile = state
        .rag_repository
        .get_allowlist_profile(collection_config.allowlist_profile_id)
        .await?;

    let conn_config: DbConnectionConfig = if let Some(ref json) = db_conn.config_json {
        serde_json::from_str(json).unwrap_or_else(|_| DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        })
    } else {
        DbConnectionConfig {
            profile_id: None,
            selected_tables: collection_config.selected_tables.clone(),
            selected_columns: std::collections::HashMap::new(),
            default_limit: None,
            updated_at: None,
        }
    };

    let effective_limit = request
        .limit
        .unwrap_or(conn_config.default_limit.unwrap_or(DEFAULT_LIMIT));
    let final_k = request.final_k.unwrap_or(FINAL_K);
    let llm_config = state.last_config.lock().unwrap().clone();

    let template_match = TemplateMatch {
        template: template.clone(),
        score: 1.0,
        reason: "User selected".to_string(),
    };

    let table_name = template
        .tables_used
        .first()
        .map(|t| t.as_str())
        .unwrap_or(&collection_config.selected_tables[0]);

    let allowed_columns: Vec<String> = conn_config
        .selected_columns
        .get(table_name)
        .cloned()
        .unwrap_or_default();

    let schema_context =
        build_schema_context_for_llm(table_name, &allowed_columns, &conn_config.selected_columns);
    let selection = select_template_with_llm(
        &state.llm_client,
        &llm_config,
        &request.query,
        &[template_match],
        &state.logs,
        Some(&schema_context),
    )
    .await
    .unwrap_or(LlmTemplateSelection {
        selected_template_id: template.id,
        extracted_params: std::collections::HashMap::new(),
        modified_where_clause: None,
        detected_table: None,
        related_table: None,
        foreign_key_column: None,
        main_table_columns: None,
        related_table_columns: None,
        confidence: 1.0,
        reasoning: "User-selected template".to_string(),
    });

    let (sql_to_execute, sql_description) = build_sql_from_template(
        &template,
        &selection,
        &allowed_columns,
        table_name,
        effective_limit,
        &state.logs,
    )
    .ok_or_else(|| {
        crate::domain::error::AppError::ValidationError(
            "Failed to build SQL from template".to_string(),
        )
    })?;

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Template SQL: {}", sql_to_execute),
    );

    let empty_params: Vec<serde_json::Value> = vec![];
    let query_result = state
        .db_connection_manager
        .execute_select(&db_conn, &sql_to_execute, &empty_params)
        .await?;

    let candidates = convert_db_rows_to_candidates(&query_result.rows, table_name);
    let (reranked_candidates, _) = state
        .reranker_service
        .rerank_with_info(&request.query, candidates)
        .unwrap_or_else(|_| {
            let mut fallback = convert_db_rows_to_candidates(&query_result.rows, table_name);
            for c in &mut fallback {
                c.score = Some(1.0);
            }
            (fallback, false)
        });

    let final_rows = restore_rows_from_candidates(
        reranked_candidates
            .into_iter()
            .take(final_k as usize)
            .collect(),
        &query_result.rows,
    );

    let citations: Vec<DbCitation> = final_rows
        .iter()
        .enumerate()
        .map(|(_rank, (original_idx, row, _score))| {
            let row_id = row
                .get("id")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("row_{}", original_idx));

            DbCitation {
                table_name: table_name.to_string(),
                row_id,
                columns: serde_json::json!(row),
            }
        })
        .collect();

    let results_context = format_sql_results_for_llm(&final_rows, table_name);
    let answer = generate_nl_response(
        &state.llm_client,
        &llm_config,
        &request.query,
        &results_context,
        &state.logs,
        request
            .conversation_history
            .as_ref()
            .map(|history| {
                history
                    .iter()
                    .map(|msg| format!("{}: {}", msg.role, msg.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .as_deref(),
    )
    .await;

    let latency_ms = start.elapsed().as_millis() as i64;

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Template query completed: {} results in {}ms",
            final_rows.len(),
            latency_ms
        ),
    );

    let plan = QueryPlan {
        mode: "template".to_string(),
        table: table_name.to_string(),
        select: allowed_columns,
        filters: vec![],
        limit: effective_limit,
        order_by: None,
        joins: None,
    };

    Ok(DbQueryResponse {
        answer,
        citations,
        telemetry: DbQueryTelemetry {
            row_count: final_rows.len(),
            latency_ms,
            llm_route: "local".to_string(),
            query_plan: Some(sql_description),
            executed_sql: Some(sql_to_execute.clone()),
            template_id: Some(template.id),
            template_name: Some(template.name.clone()),
            template_match_count: Some(1),
            matched_templates: Some(vec![TemplateMatchInfo {
                template_id: template.id,
                template_name: template.name.clone(),
                score: 1.0,
                reason: "User selected".to_string(),
                example_question: Some(template.example_question.clone()),
                query_pattern: Some(template.query_pattern.clone()),
            }]),
            column_mappings: None,
            modified_where_clause: None,
            enriched_query: None,
            detected_intent: None,
        },
        plan: Some(serde_json::to_value(&plan).unwrap_or_default()),
    })
}
