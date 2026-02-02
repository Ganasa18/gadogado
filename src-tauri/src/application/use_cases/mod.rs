#![allow(dead_code)]
// TODO: tighten this allow once all use-cases are wired to commands/tests.

pub mod allowlist_validator;
pub mod audit_service;
pub mod chunking;
pub mod context_manager;
pub mod conversation_service;
pub mod csv_preprocessor;
pub mod data_protection;
pub mod db_connection_manager;
pub mod embedding_service;
pub mod enhance;
pub mod few_shot_prompt_builder;
pub mod prompt_engine;
pub mod qa_ai;
pub mod query_intent_enricher;
pub mod qa_api_call;
pub mod qa_event;
pub mod qa_run;
pub mod qa_session;
pub mod rag_analytics;
pub mod rag_config;
pub mod rag_ingestion;
pub mod rag_metrics;
pub mod rag_validation;
pub mod rate_limiter;
pub mod reranker_service;
pub mod retrieval_service;
pub mod semantic_matcher;
pub mod sql_compiler;
pub mod sql_rag_router;
pub mod structured_row_schema;
pub mod table_matcher;
pub mod template_matcher;
pub mod translate;
pub mod typegen;
pub mod web_crawler;
