use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::Manager;
use tracing::error;

use crate::application::use_cases::audit_service::AuditService;
use crate::application::use_cases::conversation_service::ConversationService;
use crate::application::use_cases::data_protection::DataProtectionService;
use crate::application::use_cases::db_connection_manager::DbConnectionManager;
use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::qa_ai::QaAiUseCase;
use crate::application::use_cases::rate_limiter::RateLimiter;
use crate::application::use_cases::retrieval_service::RetrievalService;
use crate::application::use_cases::rag_analytics::SharedAnalyticsLogger;
use crate::application::use_cases::rag_config::{SharedConfigManager, SharedFeedbackCollector};
use crate::application::use_cases::rag_metrics::{SharedExperimentManager, SharedMetricsCollector};
use crate::application::{
    EnhanceUseCase, QaApiCallUseCase, QaEventUseCase, QaRunUseCase, QaSessionUseCase,
    RagIngestionUseCase, TranslateUseCase, TypeGenUseCase,
};
use crate::infrastructure::artifact_store::{
    ensure_daily_backup, BackupConfig, TrainingArtifactLayout,
};
use crate::infrastructure::config::ConfigService;
use crate::infrastructure::db::qa::init_qa_db;
use crate::infrastructure::db::qa_api_calls::QaApiCallRepository;
use crate::infrastructure::db::qa_checkpoints::QaCheckpointRepository;
use crate::infrastructure::db::qa_events::QaEventRepository;
use crate::infrastructure::db::qa_runs::QaRunRepository;
use crate::infrastructure::db::qa_sessions::QaRepository;
use crate::infrastructure::db::rag::connection::init_rag_db;
use crate::infrastructure::db::rag::repository::RagRepository;
use crate::infrastructure::db::sqlite::SqliteRepository;
use crate::infrastructure::db::training::connection::init_training_db;
use crate::infrastructure::llm_clients::{LLMClient, RouterClient};
use crate::infrastructure::storage::{ensure_qa_sessions_root, resolve_app_data_dir};
use crate::interfaces::http::add_log;
use crate::interfaces::mock_server::MockServerState;
use crate::interfaces::tauri::AppState;

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    let app_handle = app.handle().clone();

    let logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>> = Arc::new(Mutex::new(Vec::new()));

    let app_data_dir = resolve_app_data_dir(&app_handle).map_err(|err| {
        error!(error = %err, "Failed to resolve app data dir");
        err
    })?;

    let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir).map_err(|err| {
        error!(
            error = %err,
            qa_sessions_dir = %app_data_dir.join("qa_sessions").display(),
            "Failed to create qa_sessions dir"
        );
        err
    })?;

    configure_ocr(&app_handle, &logs);
    ensure_training_artifacts(&app_data_dir, &logs);
    bootstrap_databases_and_state(app_handle, app_data_dir, qa_sessions_dir, logs);

    Ok(())
}

fn configure_ocr(app_handle: &tauri::AppHandle, logs: &Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>) {
    let os_folder = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    };
    let tesseract_name = if std::env::consts::OS == "windows" {
        "tesseract.exe"
    } else {
        "tesseract"
    };

    let resource_dir = app_handle.path().resource_dir().ok();
    let mut ocr_root = resource_dir.as_ref().map(|dir| dir.join("ocr"));

    if ocr_root.as_ref().map(|dir| dir.exists()).unwrap_or(false) == false {
        ocr_root = Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("ocr"),
        );
    }

    let Some(ocr_root) = ocr_root else {
        add_log(logs, "INFO", "RAG", "OCR resources not configured");
        return;
    };

    add_log(
        logs,
        "INFO",
        "RAG",
        &format!("Using OCR resources at {}", ocr_root.display()),
    );

    let tesseract_path = ocr_root.join(os_folder).join(tesseract_name);
    if tesseract_path.exists() {
        std::env::set_var("TESSERACT_CMD", &tesseract_path);
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Using bundled Tesseract at {}", tesseract_path.display()),
        );
    } else {
        add_log(
            logs,
            "WARN",
            "RAG",
            &format!("Bundled Tesseract not found at {}", tesseract_path.display()),
        );
    }

    let pdftoppm_name = if std::env::consts::OS == "windows" {
        "pdftoppm.exe"
    } else {
        "pdftoppm"
    };
    let pdftoppm_path = ocr_root.join(os_folder).join(pdftoppm_name);
    if pdftoppm_path.exists() {
        std::env::set_var("PDFTOPPM_CMD", &pdftoppm_path);
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Using bundled pdftoppm at {}", pdftoppm_path.display()),
        );
    }

    let lib_dir = ocr_root.join(os_folder).join("lib");
    if lib_dir.exists() {
        let env_key = if std::env::consts::OS == "macos" {
            "DYLD_LIBRARY_PATH"
        } else if std::env::consts::OS == "linux" {
            "LD_LIBRARY_PATH"
        } else {
            ""
        };

        if !env_key.is_empty() {
            let separator = if std::env::consts::OS == "windows" { ";" } else { ":" };
            let new_value = match std::env::var(env_key) {
                Ok(existing) if !existing.is_empty() => {
                    format!("{}{}{}", lib_dir.display(), separator, existing)
                }
                _ => lib_dir.display().to_string(),
            };
            std::env::set_var(env_key, new_value);
            add_log(
                logs,
                "INFO",
                "RAG",
                &format!("Using bundled Tesseract libs at {}", lib_dir.display()),
            );
        }
    }

    let tessdata_path = ocr_root.join("tessdata");
    if tessdata_path.exists() {
        std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
        add_log(
            logs,
            "INFO",
            "RAG",
            &format!("Using bundled tessdata at {}", tessdata_path.display()),
        );
    }
}

fn ensure_training_artifacts(
    app_data_dir: &PathBuf,
    logs: &Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) {
    let training_artifacts = TrainingArtifactLayout::new(app_data_dir);
    if let Err(err) = training_artifacts.ensure() {
        add_log(
            logs,
            "ERROR",
            "Training",
            &format!("Failed to ensure training artifact dirs: {err}"),
        );
    } else {
        add_log(
            logs,
            "INFO",
            "Training",
            &format!(
                "Training artifacts root ready: {}",
                training_artifacts.root().display()
            ),
        );
    }
}

fn bootstrap_databases_and_state(
    app_handle: tauri::AppHandle,
    app_data_dir: PathBuf,
    qa_sessions_dir: PathBuf,
    logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) {
    let qa_db_path = app_data_dir.join("qa_recorder.db");
    let rag_db_path = app_data_dir.join("rag_sense.db");
    let training_db_path = app_data_dir.join("training.db");

    let db_path = app_data_dir.join("promptbridge.db");
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");
    let db_url = format!("sqlite://{}", db_path_str);

    tauri::async_runtime::block_on(async move {
        init_rag_db(&rag_db_path)
            .await
            .expect("Failed to initialize RAG database");
        init_qa_db(&qa_db_path)
            .await
            .expect("Failed to initialize QA database");
        init_training_db(&training_db_path)
            .await
            .expect("Failed to initialize Training database");

        spawn_daily_training_backup(app_data_dir.clone(), training_db_path.clone(), logs.clone());

        let qa_repo = QaRepository::connect(&qa_db_path)
            .await
            .expect("Failed to connect QA database");
        let qa_event_repo = QaEventRepository::connect(&qa_db_path)
            .await
            .expect("Failed to connect QA events database");
        let qa_checkpoint_repo = QaCheckpointRepository::connect(&qa_db_path)
            .await
            .expect("Failed to connect QA checkpoints database");
        let qa_run_repo = QaRunRepository::connect(&qa_db_path)
            .await
            .expect("Failed to connect QA runs database");
        let qa_api_call_repo = QaApiCallRepository::connect(&qa_db_path)
            .await
            .expect("Failed to connect QA API calls database");

        let rag_repo = RagRepository::connect(&rag_db_path)
            .await
            .expect("Failed to connect RAG database");

        let repository = SqliteRepository::init(&db_url)
            .await
            .expect("Failed to initialize database");

        let qa_repo_arc = Arc::new(qa_repo);
        let qa_event_repo_arc = Arc::new(qa_event_repo);
        let qa_checkpoint_repo_arc = Arc::new(qa_checkpoint_repo);
        let qa_run_repo_arc = Arc::new(qa_run_repo);
        let qa_api_call_repo_arc = Arc::new(qa_api_call_repo);
        let rag_repo_arc = Arc::new(rag_repo);
        let repository_arc = Arc::new(repository);

        let mock_server = Arc::new(MockServerState::new(
            app_data_dir.join("mock_server.json"),
            logs.clone(),
        ));

        let llm_client: Arc<dyn LLMClient + Send + Sync> = Arc::new(RouterClient::new());

        let translate_use_case = TranslateUseCase::new(llm_client.clone(), repository_arc.clone());
        let enhance_use_case = EnhanceUseCase::new(llm_client.clone(), repository_arc.clone());
        let typegen_use_case = TypeGenUseCase::new(llm_client.clone());

        let qa_session_use_case = QaSessionUseCase::new(qa_repo_arc.clone(), qa_sessions_dir);
        let qa_event_use_case = QaEventUseCase::new(qa_event_repo_arc.clone());
        let qa_run_use_case = QaRunUseCase::new(qa_run_repo_arc.clone());
        let qa_api_call_use_case = QaApiCallUseCase::new(qa_api_call_repo_arc.clone());
        let qa_ai_use_case = QaAiUseCase::new(
            qa_repo_arc.clone(),
            qa_event_repo_arc.clone(),
            qa_checkpoint_repo_arc.clone(),
            llm_client.clone(),
        );

        let embedding_config = crate::domain::llm_config::LLMConfig {
            provider: crate::domain::llm_config::LLMProvider::Local,
            base_url: String::new(),
            model: "all-minilm-l6-v2".to_string(),
            api_key: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };
        let embedding_service = Arc::new(EmbeddingService::new(embedding_config));
        let rag_ingestion_use_case = RagIngestionUseCase::with_embedding_service(
            rag_repo_arc.clone(),
            embedding_service.clone(),
        );
        let retrieval_service = Arc::new(RetrievalService::new(
            rag_repo_arc.clone(),
            embedding_service.clone(),
        ));

        let metrics_collector = SharedMetricsCollector::new();
        let experiment_manager = SharedExperimentManager::new();
        let analytics_logger = SharedAnalyticsLogger::new(2000);
        let config_manager = SharedConfigManager::new(app_data_dir.clone());
        let feedback_collector = SharedFeedbackCollector::new(1000);

        let conversation_service = Arc::new(ConversationService::new(rag_repo_arc.clone()));

        // SQL-RAG services expect a raw DB pool.
        let rag_pool = Arc::new(rag_repo_arc.pool().clone());
        let db_connection_manager = Arc::new(DbConnectionManager::new());
        let audit_service = Arc::new(AuditService::new(rag_pool.clone()));
        let rate_limiter = Arc::new(RateLimiter::new(rag_pool.clone()));
        let data_protection = Arc::new(DataProtectionService::new(rag_pool));

        let state = AppState {
            translate_use_case,
            enhance_use_case,
            typegen_use_case,
            qa_session_use_case,
            qa_event_use_case,
            qa_ai_use_case,
            qa_run_use_case,
            qa_api_call_use_case,
            rag_ingestion_use_case,
            retrieval_service,
            embedding_service,
            qa_session_id: Mutex::new(None),
            qa_recorder: Mutex::new(None),
            repository: repository_arc,
            rag_repository: rag_repo_arc,
            config_service: ConfigService::new(),
            llm_client: llm_client.clone(),
            mock_server,
            last_config: Mutex::new(crate::domain::llm_config::LLMConfig::default()),
            preferred_source: Mutex::new("Auto Detect".to_string()),
            preferred_target: Mutex::new("English".to_string()),
            logs: logs.clone(),
            distill_trainers: Mutex::new(HashMap::new()),
            distill_trainer_launches: Mutex::new(HashSet::new()),
            metrics_collector,
            experiment_manager,
            analytics_logger,
            config_manager,
            feedback_collector,
            conversation_service,
            db_connection_manager,
            audit_service,
            rate_limiter,
            data_protection,
        };
        let state_arc = Arc::new(state);

        app_handle.manage(state_arc.clone());

        // Start Actix server
        let logs_for_server = logs.clone();
        let server = crate::interfaces::http::start_server(state_arc.clone(), logs_for_server)
            .expect("Failed to start Actix server");
        tokio::spawn(server);

        add_log(
            &logs,
            "INFO",
            "System",
            "Backend initialized and HTTP server started on :3001",
        );

        if let Err(err) = crate::register_shortcuts(
            &app_handle,
            true,
            "Ctrl + Alt + T",
            "Ctrl + Alt + E",
            "Ctrl + Alt + P",
            "Ctrl + Alt + R",
        ) {
            add_log(
                &logs,
                "ERROR",
                "Shortcut",
                &format!("Failed to register default shortcuts: {err}"),
            );
        }
    });
}

fn spawn_daily_training_backup(
    app_data_dir: PathBuf,
    training_db_path: PathBuf,
    logs: Arc<Mutex<Vec<crate::interfaces::http::LogEntry>>>,
) {
    tauri::async_runtime::spawn(async move {
        let joined = tokio::task::spawn_blocking(move || {
            let cfg = BackupConfig::new(&app_data_dir);
            ensure_daily_backup(&training_db_path, &cfg)
        })
        .await;

        match joined {
            Ok(Ok(Some(result))) => add_log(
                &logs,
                "INFO",
                "Training",
                &format!(
                    "Daily training DB backup created: {} ({} bytes)",
                    result.backup_path.display(),
                    result.size_bytes
                ),
            ),
            Ok(Ok(None)) => add_log(
                &logs,
                "INFO",
                "Training",
                "Daily training DB backup already exists",
            ),
            Ok(Err(err)) => add_log(
                &logs,
                "ERROR",
                "Training",
                &format!("Failed to create daily training DB backup: {err}"),
            ),
            Err(err) => add_log(
                &logs,
                "ERROR",
                "Training",
                &format!("Daily backup worker failed: {err}"),
            ),
        }
    });
}
