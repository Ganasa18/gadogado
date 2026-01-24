use crate::application::use_cases::audit_service::AuditService;
use crate::application::use_cases::data_protection::DataProtectionService;
use crate::application::use_cases::db_connection_manager::DbConnectionManager;
use crate::application::use_cases::enhance::EnhanceUseCase;
use crate::application::use_cases::qa_ai::{ExploreResult, QaAiUseCase};
use crate::application::use_cases::qa_api_call::QaApiCallUseCase;
use crate::application::use_cases::qa_event::QaEventUseCase;
use crate::application::use_cases::qa_run::QaRunUseCase;
use crate::application::use_cases::qa_session::QaSessionUseCase;
use crate::application::use_cases::rag_ingestion::RagIngestionUseCase;
use crate::application::use_cases::rate_limiter::RateLimiter;
use crate::application::use_cases::retrieval_service::RetrievalService;
use crate::application::use_cases::translate::TranslateUseCase;
use crate::application::use_cases::typegen::TypeGenUseCase;
use crate::domain::error::{AppError, Result};
use crate::domain::llm_config::{ChatMessage, LLMConfig};
use crate::domain::prompt::Prompt;
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use crate::domain::qa_event::{QaEvent, QaEventInput, QaEventPage};
use crate::domain::qa_run::{QaRunStreamEvent, QaRunStreamInput, QaSessionRun};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::rag::repository::RagRepository;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::storage::{
    ensure_qa_sessions_root, ensure_session_dir, ensure_session_screenshots_dir,
    resolve_app_data_dir,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tracing::error;
use uuid::Uuid;

use crate::application::use_cases::conversation_service::ConversationService;
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::rag_analytics::SharedAnalyticsLogger;
use crate::application::use_cases::rag_config::{SharedConfigManager, SharedFeedbackCollector};
use crate::application::use_cases::rag_metrics::{SharedExperimentManager, SharedMetricsCollector};
use crate::infrastructure::config::ConfigService;
use crate::infrastructure::llm_clients::LLMClient;
use crate::interfaces::http::{add_log, add_log_entry, LogEntry};
use crate::interfaces::mock_server::{
    build_status as build_mock_status, save_config as save_mock_server_config, start_mock_server,
    stop_mock_server, MockServerConfig, MockServerState, MockServerStatus,
};

use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::Mutex as AsyncMutex;


pub struct AppState {
    pub translate_use_case: TranslateUseCase,
    pub enhance_use_case: EnhanceUseCase,
    pub typegen_use_case: TypeGenUseCase,
    pub qa_session_use_case: QaSessionUseCase,
    pub qa_event_use_case: QaEventUseCase,
    pub qa_ai_use_case: QaAiUseCase,
    pub qa_run_use_case: QaRunUseCase,
    pub qa_api_call_use_case: QaApiCallUseCase,
    pub rag_ingestion_use_case: RagIngestionUseCase,
    pub retrieval_service: Arc<RetrievalService>,
    pub embedding_service: Arc<EmbeddingService>,
    pub qa_session_id: Mutex<Option<String>>,
    pub qa_recorder: Mutex<Option<QaRecorderHandle>>,
    pub repository: Arc<SqliteRepository>,
    pub rag_repository: Arc<RagRepository>,
    pub config_service: ConfigService,
    pub llm_client: Arc<dyn LLMClient + Send + Sync>,
    pub mock_server: Arc<MockServerState>,
    pub last_config: Mutex<LLMConfig>,
    pub preferred_source: Mutex<String>,
    pub preferred_target: Mutex<String>,
    pub logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    pub distill_trainers: Mutex<HashMap<String, DistillTrainerHandle>>,
    pub distill_trainer_launches: Mutex<HashSet<String>>,
    /// RAG metrics collector for performance tracking
    pub metrics_collector: SharedMetricsCollector,
    /// A/B experiment manager for RAG experiments
    pub experiment_manager: SharedExperimentManager,
    /// Analytics logger for RAG operations
    pub analytics_logger: SharedAnalyticsLogger,
    /// RAG configuration manager
    pub config_manager: SharedConfigManager,
    /// User feedback collector
    pub feedback_collector: SharedFeedbackCollector,
    /// Conversation service for chat persistence
    pub conversation_service: Arc<ConversationService>,
    /// Database connection manager for SQL-RAG
    pub db_connection_manager: Arc<DbConnectionManager>,
    /// Audit service for SQL-RAG query logging
    pub audit_service: Arc<AuditService>,
    /// Rate limiter for SQL-RAG queries
    pub rate_limiter: Arc<RateLimiter>,
    /// Data protection service for SQL-RAG
    pub data_protection: Arc<DataProtectionService>,
}

pub(crate) struct QaRecorderHandle {
    pub(crate) child: Arc<AsyncMutex<Child>>,
    pub(crate) session_id: String,
    pub(crate) run_id: String,
    pub(crate) mode: String,
}

pub(crate) struct DistillTrainerHandle {
    pub(crate) child: Arc<AsyncMutex<Child>>,
    pub(crate) run_id: String,
    pub(crate) run_dir: PathBuf,
}

/// Cleanup all child processes when the app is closing.
/// This function kills the QA browser recorder and all distill trainers.
pub async fn cleanup_child_processes(state: &AppState) {
    // Kill QA recorder if running
    if let Some(handle) = state.qa_recorder.lock().unwrap().take() {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        tracing::info!("Killed QA browser recorder (run_id: {})", handle.run_id);
    }

    // Kill all distill trainers
    let handles: Vec<_> = {
        let mut trainers = state.distill_trainers.lock().unwrap();
        trainers.drain().collect()
    };

    for (run_id, handle) in handles {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        tracing::info!("Killed distill trainer (run_id: {})", run_id);
    }
}
