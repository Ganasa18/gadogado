use crate::application::use_cases::allowlist_validator::AllowlistValidator;
use crate::application::use_cases::sql_compiler::{DbType, SqlCompiler};
use crate::application::use_cases::sql_rag_router::SqlRagRouter;
use crate::application::use_cases::template_matcher::TemplateMatch;
use crate::domain::error::Result;
use crate::domain::rag_entities::{DbAllowlistProfile, DbConnectionConfig, QueryPlan};
use crate::interfaces::http::add_log;
use std::sync::Arc;

use super::constants::{DEFAULT_LIMIT, TEMPLATE_MATCH_THRESHOLD};
use super::helpers::validate_compiled_sql;
use super::helpers::validate_query_plan;
use super::template_llm::{build_schema_context_for_llm, select_template_with_llm};
use super::template_semantic::load_templates_with_semantic_matching;
use super::template_sql::{build_sql_from_template, get_user_template_preference, hash_query};
use super::super::types::{DbQueryRequest, EnrichedQuery, LlmTemplateSelection, QueryIntent};
use crate::application::use_cases::query_intent_enricher;

pub struct ResolvedQuery {
    pub sql_to_execute: String,
    pub sql_description: String,
    pub final_plan: QueryPlan,
    pub sql_params: Vec<serde_json::Value>,
    pub matched_templates: Vec<TemplateMatch>,
    pub selected_template_id: Option<i64>,
    pub selected_template_name: Option<String>,
    pub llm_template_selection: Option<LlmTemplateSelection>,
    /// Enriched query from the intent enricher (for telemetry/debugging)
    pub enriched_query: Option<EnrichedQuery>,
}

pub fn resolve_limit_from_request(
    request_limit: Option<i32>,
    conn_default_limit: Option<i32>,
) -> i32 {
    request_limit.unwrap_or(conn_default_limit.unwrap_or(DEFAULT_LIMIT))
}

pub async fn resolve_sql_and_plan(
    state: &Arc<super::super::AppState>,
    request: &DbQueryRequest,
    collection_config: &super::helpers::CollectionConfig,
    conn_config: &DbConnectionConfig,
    allowlist_profile: &DbAllowlistProfile,
    router: &SqlRagRouter,
    db_conn: &crate::domain::rag_entities::DbConnection,
    effective_limit: i32,
) -> Result<ResolvedQuery> {
    let query_hash = hash_query(&request.query);
    let is_new_query = request.is_new_query.unwrap_or(false);
    let preferred_template_id = if is_new_query {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "New query detected - skipping template feedback lookup",
        );
        None
    } else {
        get_user_template_preference(&state.rag_repository, &query_hash, request.collection_id).await
    };

    if preferred_template_id.is_some() {
        add_log(
            &state.logs,
            "INFO",
            "SQL-RAG",
            &format!(
                "Found user preferred template from feedback: template_id={:?}",
                preferred_template_id
            ),
        );
    }

    let detected_tables = collection_config.selected_tables.clone();
    let llm_config = state.last_config.lock().unwrap().clone();

    // =========================================================================
    // STEP 1: Query Intent Enrichment (LLM-based, with silent fallback)
    // Rewrites ambiguous queries to clarify intent before template matching.
    // e.g., "tampilkan data user ada juga data korwil" → "tampilkan data user JOIN table_korwil"
    // =========================================================================
    let enriched = query_intent_enricher::enrich_query(
        &state.llm_client,
        &llm_config,
        &request.query,
        &conn_config.selected_columns,
        &state.logs,
    )
    .await;

    // Use the enriched (rewritten) query for template matching
    let query_for_matching = &enriched.rewritten_query;

    // =========================================================================
    // STEP 2: Template Matching (uses enriched query)
    // =========================================================================
    let matched_templates = load_templates_with_semantic_matching(
        &state.rag_repository,
        collection_config.allowlist_profile_id,
        query_for_matching,
        &detected_tables,
        &state.llm_client,
        &llm_config,
        &state.logs,
    )
    .await;

    let mut selected_template_id: Option<i64> = None;
    let mut selected_template_name: Option<String> = None;
    let mut llm_template_selection: Option<LlmTemplateSelection> = None;

    // =========================================================================
    // STEP 3: Intent Detection (uses enricher results + keyword fallback)
    // =========================================================================
    let has_join_intent = matches!(enriched.detected_intent, QueryIntent::JoinTables)
        || {
            // Fallback: keyword-based detection on BOTH original and rewritten query
            let query_lower = query_for_matching.to_lowercase();
            ["join", "gabung", "relasi", "beserta", "lookup"]
                .iter()
                .any(|kw| query_lower.contains(kw))
                || (query_lower.contains("dengan") && matched_templates.iter().any(|tm| {
                    tm.template.name.to_lowercase().contains("join")
                }))
        };

    let has_join_template = matched_templates.iter().any(|tm| {
        tm.template.name.to_lowercase().contains("join")
    });

    let has_date_intent = matches!(enriched.detected_intent, QueryIntent::DateFilter)
        || {
            let query_lower = query_for_matching.to_lowercase();
            [
                "hari", "terakhir", "recent", "latest", "today", "hari ini",
                "minggu", "bulan", "tahun", "week", "month", "year",
                "tanggal", "date", "terbaru", "baru",
            ].iter().any(|kw| query_lower.contains(kw))
        };

    let has_date_template = matched_templates.iter().any(|tm| {
        let name_lower = tm.template.name.to_lowercase();
        name_lower.contains("recent") || name_lower.contains("date") || name_lower.contains("time series")
    });

    let use_template_first = if preferred_template_id.is_some() {
        true
    } else if has_join_intent && has_join_template {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "Forcing TEMPLATE-FIRST: JOIN intent detected and JOIN template available",
        );
        true
    } else if has_date_intent && has_date_template {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            "Forcing TEMPLATE-FIRST: date/time intent detected and date template available",
        );
        true
    } else {
        !matched_templates.is_empty() && matched_templates[0].score >= TEMPLATE_MATCH_THRESHOLD
    };

    if !matched_templates.is_empty() {
        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!(
                "Found {} matching templates: {} (best score: {:.2})",
                matched_templates.len(),
                matched_templates
                    .iter()
                    .map(|m| m.template.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                matched_templates[0].score
            ),
        );
    }

    let llm_config = state.last_config.lock().unwrap().clone();

    let (sql_to_execute, sql_description, final_plan, sql_params) = if use_template_first {
        if preferred_template_id.is_some() {
            add_log(
                &state.logs,
                "INFO",
                "SQL-RAG",
                "Using TEMPLATE-FIRST approach (user preferred template from feedback)",
            );
        } else {
            add_log(
                &state.logs,
                "INFO",
                "SQL-RAG",
                "Using TEMPLATE-FIRST approach (good template match found)",
            );
        }

        let selection = if let Some(preferred_id) = preferred_template_id {
            if matched_templates.iter().any(|tm| tm.template.id == preferred_id) {
                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!("Using preferred template {} from feedback", preferred_id),
                );
                Some(LlmTemplateSelection {
                    selected_template_id: preferred_id,
                    extracted_params: std::collections::HashMap::new(),
                    modified_where_clause: None,
                    detected_table: None,
                    related_table: None,
                    foreign_key_column: None,
                    main_table_columns: None,
                    related_table_columns: None,
                    confidence: 1.0,
                    reasoning: "User preferred template from feedback".to_string(),
                })
            } else {
                add_log(
                    &state.logs,
                    "WARN",
                    "SQL-RAG",
                    &format!(
                        "Preferred template {} not found in matches, using LLM selection",
                        preferred_id
                    ),
                );
                let schema_context = build_schema_context_for_llm(
                    &collection_config.selected_tables[0],
                    &conn_config
                        .selected_columns
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>(),
                    &conn_config.selected_columns,
                );
                select_template_with_llm(
                    &state.llm_client,
                    &llm_config,
                    query_for_matching,
                    &matched_templates,
                    &state.logs,
                    Some(&schema_context),
                )
                .await
            }
        } else {
            let schema_context = build_schema_context_for_llm(
                &collection_config.selected_tables[0],
                &conn_config
                    .selected_columns
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>(),
                &conn_config.selected_columns,
            );
            select_template_with_llm(
                &state.llm_client,
                &llm_config,
                &request.query,
                &matched_templates,
                &state.logs,
                Some(&schema_context),
            )
            .await
        };

        if let Some(ref sel) = selection {
            if let Some(template_match) = matched_templates
                .iter()
                .find(|tm| tm.template.id == sel.selected_template_id)
            {
                let template = &template_match.template;
                selected_template_id = Some(template.id);
                selected_template_name = Some(template.name.clone());
                llm_template_selection = selection.clone();

                let table_name = if template.is_pattern_agnostic {
                    sel.detected_table
                        .as_ref()
                        .filter(|t| {
                            !t.is_empty() && conn_config.selected_columns.contains_key(t.as_str())
                        })
                        .map(|t| t.as_str())
                        .unwrap_or_else(|| {
                            template
                                .tables_used
                                .first()
                                .map(|t| t.as_str())
                                .unwrap_or(&collection_config.selected_tables[0])
                        })
                } else {
                    template
                        .tables_used
                        .first()
                        .map(|t| t.as_str())
                        .unwrap_or(&collection_config.selected_tables[0])
                };

                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!(
                        "Table resolution: detected_table={:?}, template.tables_used={:?}, final={}",
                        sel.detected_table, template.tables_used, table_name
                    ),
                );

                let allowed_columns: Vec<String> = conn_config
                    .selected_columns
                    .get(table_name)
                    .cloned()
                    .unwrap_or_else(|| {
                        add_log(
                            &state.logs,
                            "WARN",
                            "SQL-RAG",
                            &format!(
                                "Table '{}' not found in selected_columns, trying fallback. Available tables: {:?}",
                                table_name,
                                conn_config.selected_columns.keys().collect::<Vec<_>>()
                            ),
                        );
                        conn_config
                            .selected_columns
                            .values()
                            .next()
                            .cloned()
                            .unwrap_or_default()
                    });

                add_log(
                    &state.logs,
                    "DEBUG",
                    "SQL-RAG",
                    &format!(
                        "Resolved {} columns for table '{}': {:?}",
                        allowed_columns.len(),
                        table_name,
                        allowed_columns.iter().take(5).collect::<Vec<_>>()
                    ),
                );

                if let Some((sql, description)) = build_sql_from_template(
                    template,
                    sel,
                    &allowed_columns,
                    table_name,
                    effective_limit,
                    &state.logs,
                ) {
                    add_log(
                        &state.logs,
                        "DEBUG",
                        "SQL-RAG",
                        &format!("Template SQL: {}", sql),
                    );

                    let template_plan = QueryPlan {
                        mode: "template".to_string(),
                        table: table_name.to_string(),
                        select: allowed_columns.clone(),
                        filters: vec![],
                        limit: effective_limit,
                        order_by: None,
                        joins: None,
                    };

                    (sql, description, template_plan, vec![])
                } else {
                    add_log(
                        &state.logs,
                        "WARN",
                        "SQL-RAG",
                        "Failed to build SQL from template, falling back to plan-based approach",
                    );
                    let plan = router.generate_plan(&request.query, effective_limit)?;
                    let validator = AllowlistValidator::from_profile(allowlist_profile)?
                        .with_selected_tables(collection_config.selected_tables.clone());
                    let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
                    let db_type = match db_conn.db_type.to_lowercase().as_str() {
                        "postgres" | "postgresql" => DbType::Postgres,
                        "sqlite" => DbType::Sqlite,
                        _ => DbType::Postgres,
                    };
                    let compiler = SqlCompiler::new(db_type);
                    let compiled = compiler.compile(&final_plan)?;
                    (compiled.sql, compiled.description, final_plan, compiled.params)
                }
            } else {
                add_log(
                    &state.logs,
                    "WARN",
                    "SQL-RAG",
                    "Selected template not found, falling back to plan-based approach",
                );
                let plan = router.generate_plan(&request.query, effective_limit)?;
                let validator = AllowlistValidator::from_profile(allowlist_profile)?
                    .with_selected_tables(collection_config.selected_tables.clone());
                let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
                let db_type = match db_conn.db_type.to_lowercase().as_str() {
                    "postgres" | "postgresql" => DbType::Postgres,
                    "sqlite" => DbType::Sqlite,
                    _ => DbType::Postgres,
                };
                let compiler = SqlCompiler::new(db_type);
                let compiled = compiler.compile(&final_plan)?;
                (compiled.sql, compiled.description, final_plan, compiled.params)
            }
        } else {
            add_log(
                &state.logs,
                "WARN",
                "SQL-RAG",
                "LLM template selection returned None, falling back to plan-based approach",
            );
            let plan = router.generate_plan(&request.query, effective_limit)?;
            let validator = AllowlistValidator::from_profile(allowlist_profile)?
                .with_selected_tables(collection_config.selected_tables.clone());
            let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;
            let db_type = match db_conn.db_type.to_lowercase().as_str() {
                "postgres" | "postgresql" => DbType::Postgres,
                "sqlite" => DbType::Sqlite,
                _ => DbType::Postgres,
            };
            let compiler = SqlCompiler::new(db_type);
            let compiled = compiler.compile(&final_plan)?;
            (compiled.sql, compiled.description, final_plan, compiled.params)
        }
    } else {
        add_log(
            &state.logs,
            "INFO",
            "SQL-RAG",
            "Using PLAN-BASED approach (no good template match)",
        );

        let plan = router.generate_plan(&request.query, effective_limit).map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to generate query plan: {}", e),
            );
            e
        })?;

        add_log(
            &state.logs,
            "DEBUG",
            "SQL-RAG",
            &format!("Generated plan: table={}, filters={}", plan.table, plan.filters.len()),
        );

        if let Ok(plan_json) = serde_json::to_string_pretty(&plan) {
            add_log(
                &state.logs,
                "DEBUG",
                "SQL-RAG",
                &format!("Query plan details: {}", plan_json),
            );
        }

        let validator = AllowlistValidator::from_profile(allowlist_profile)
            .map_err(|e| {
                add_log(
                    &state.logs,
                    "ERROR",
                    "SQL-RAG",
                    &format!("Failed to create validator: {}", e),
                );
                e
            })?
            .with_selected_tables(collection_config.selected_tables.clone());

        let final_plan = validate_query_plan(&validator, &plan, &state.logs)?;

        let db_type = match db_conn.db_type.to_lowercase().as_str() {
            "postgres" | "postgresql" => DbType::Postgres,
            "sqlite" => DbType::Sqlite,
            _ => DbType::Postgres,
        };

        let compiler = SqlCompiler::new(db_type);
        let compiled = compiler.compile(&final_plan).map_err(|e| {
            add_log(
                &state.logs,
                "ERROR",
                "SQL-RAG",
                &format!("Failed to compile SQL: {}", e),
            );
            e
        })?;

        validate_compiled_sql(&validator, &compiled, &state.logs)?;

        (compiled.sql, compiled.description, final_plan, compiled.params)
    };

    Ok(ResolvedQuery {
        sql_to_execute,
        sql_description,
        final_plan,
        sql_params,
        matched_templates,
        selected_template_id,
        selected_template_name,
        llm_template_selection,
        enriched_query: if enriched.was_enriched() { Some(enriched) } else { None },
    })
}
