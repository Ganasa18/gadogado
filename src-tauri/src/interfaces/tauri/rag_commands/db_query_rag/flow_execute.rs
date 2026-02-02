use crate::domain::error::Result;
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::FINAL_K;
use super::nl::{build_nl_few_shot_examples, generate_nl_response, generate_nl_response_with_few_shot};
use super::results::{convert_db_rows_to_candidates, format_sql_results_for_llm, restore_rows_from_candidates};
use super::flow_resolve::ResolvedQuery;
use super::super::types::{DbCitation, DbQueryRequest, DbQueryResponse, DbQueryTelemetry, TemplateMatchInfo};
use crate::application::use_cases::audit_service::{AuditLogEntry, AuditService};
use crate::application::use_cases::data_protection::LlmRoute;

pub async fn execute_and_build_response(
    state: &Arc<super::super::AppState>,
    request: &DbQueryRequest,
    db_conn: &crate::domain::rag_entities::DbConnection,
    resolved: ResolvedQuery,
    start: std::time::Instant,
) -> Result<DbQueryResponse> {
    let final_k = request.final_k.unwrap_or(FINAL_K);

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Final SQL: {}", resolved.sql_description),
    );
    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!("Executing SQL: {}", resolved.sql_to_execute),
    );

    let query_result = match state
        .db_connection_manager
        .execute_select(db_conn, &resolved.sql_to_execute, &resolved.sql_params)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Query execution failed: {}", e),
            );
            let _ = state.rate_limiter.record_block(request.collection_id).await;
            return Err(e);
        }
    };

    let candidates = convert_db_rows_to_candidates(&query_result.rows, &resolved.final_plan.table);
    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Converted {} rows to candidates for reranking",
            candidates.len()
        ),
    );

    let (reranked_candidates, rerank_init) = state
        .reranker_service
        .rerank_with_info(&request.query, candidates)
        .unwrap_or_else(|e| {
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                &format!("Reranking failed, using original order: {}", e),
            );
            let mut fallback =
                convert_db_rows_to_candidates(&query_result.rows, &resolved.final_plan.table);
            for c in &mut fallback {
                c.score = Some(1.0);
            }
            (fallback, false)
        });

    if rerank_init {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "Reranker model initialized",
        );
    }

    let final_rows = restore_rows_from_candidates(
        reranked_candidates
            .into_iter()
            .take(final_k as usize)
            .collect(),
        &query_result.rows,
    );

    add_log(
        &state.logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Selected {} final results after reranking",
            final_rows.len()
        ),
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
                table_name: resolved.final_plan.table.clone(),
                row_id,
                columns: serde_json::json!(row),
            }
        })
        .collect();

    let candidate_count = query_result.row_count;
    let final_count = final_rows.len();

    let results_context = format_sql_results_for_llm(&final_rows, &resolved.final_plan.table);
    let llm_config = state.last_config.lock().unwrap().clone();

    let answer = if !resolved.matched_templates.is_empty() {
        let nl_examples = build_nl_few_shot_examples(&request.query, &resolved.matched_templates);
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Using few-shot NL prompt for response generation ({} templates)",
                resolved.matched_templates.len()
            ),
        );
        generate_nl_response_with_few_shot(
            &state.llm_client,
            &llm_config,
            &request.query,
            &results_context,
            &nl_examples,
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
        .await
    } else {
        generate_nl_response(
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
        .await
    };

    let latency_ms = start.elapsed().as_millis() as i64;
    let llm_route = LlmRoute::Local;

    add_log(
        &state.logs,
        "INFO",
        "SQL-RAG",
        &format!(
            "Query completed: {} candidates → {} final results in {}ms, route={}",
            candidate_count,
            final_count,
            latency_ms,
            llm_route.as_str()
        ),
    );

    let (audit_template_id, audit_template_name, audit_template_match_count) =
        if resolved.selected_template_id.is_some() {
            (
                resolved.selected_template_id,
                resolved.selected_template_name.clone(),
                Some(resolved.matched_templates.len() as i32),
            )
        } else if !resolved.matched_templates.is_empty() {
            let best_match = &resolved.matched_templates[0];
            (
                Some(best_match.template.id),
                Some(best_match.template.name.clone()),
                Some(resolved.matched_templates.len() as i32),
            )
        } else {
            (None, None, Some(0))
        };

    let audit_entry = AuditLogEntry {
        collection_id: request.collection_id,
        user_query_hash: AuditService::hash_query(&request.query),
        intent: resolved.final_plan.mode.clone(),
        plan_json: serde_json::to_string(&resolved.final_plan).ok(),
        compiled_sql: Some(resolved.sql_to_execute.clone()),
        params_json: AuditService::redact_params(&serde_json::json!(resolved.sql_params)),
        row_count: final_count as i32,
        latency_ms,
        llm_route: llm_route.as_str().to_string(),
        sent_context_chars: 0,
        template_id: audit_template_id,
        template_name: audit_template_name,
        template_match_count: audit_template_match_count,
    };

    if let Err(e) = state.audit_service.log_query(audit_entry).await {
        add_log(
            &state.logs,
            "WARN",
            "SQL-RAG",
            &format!("Failed to create audit log: {}", e),
        );
    }

    if let Err(e) = state.rate_limiter.record_query(request.collection_id).await {
        add_log(
            &state.logs,
            "WARN",
            "SQL-RAG",
            &format!("Failed to record query in rate limiter: {}", e),
        );
    }

    let matched_templates_info: Option<Vec<TemplateMatchInfo>> = if !resolved.matched_templates.is_empty() {
        Some(
            resolved
                .matched_templates
                .iter()
                .map(|m| TemplateMatchInfo {
                    template_id: m.template.id,
                    template_name: m.template.name.clone(),
                    score: m.score,
                    reason: m.reason.clone(),
                    example_question: Some(m.template.example_question.clone()),
                    query_pattern: Some(m.template.query_pattern.clone()),
                })
                .collect(),
        )
    } else {
        None
    };

    let (response_template_id, response_template_name, response_template_match_count) =
        if resolved.selected_template_id.is_some() {
            (
                resolved.selected_template_id,
                resolved.selected_template_name,
                Some(resolved.matched_templates.len() as i32),
            )
        } else if !resolved.matched_templates.is_empty() {
            let best_match = &resolved.matched_templates[0];
            (
                Some(best_match.template.id),
                Some(best_match.template.name.clone()),
                Some(resolved.matched_templates.len() as i32),
            )
        } else {
            (None, None, None)
        };

    Ok(DbQueryResponse {
        answer,
        citations,
        telemetry: DbQueryTelemetry {
            row_count: final_count,
            latency_ms,
            llm_route: llm_route.as_str().to_string(),
            query_plan: Some(resolved.sql_description),
            executed_sql: Some(resolved.sql_to_execute.clone()),
            template_id: response_template_id,
            template_name: response_template_name,
            template_match_count: response_template_match_count,
            matched_templates: matched_templates_info,
            column_mappings: None,
            modified_where_clause: resolved
                .llm_template_selection
                .as_ref()
                .and_then(|s| s.modified_where_clause.clone()),
            enriched_query: resolved
                .enriched_query
                .as_ref()
                .map(|e| e.rewritten_query.clone()),
            detected_intent: resolved
                .enriched_query
                .as_ref()
                .map(|e| e.detected_operation.clone()),
        },
        plan: Some(serde_json::to_value(&resolved.final_plan).unwrap_or_default()),
    })
}
