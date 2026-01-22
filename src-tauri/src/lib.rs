mod application;
mod domain;
mod infrastructure;
mod interfaces;

use crate::application::use_cases::embedding_service::EmbeddingService;
use crate::application::use_cases::qa_ai::QaAiUseCase;
use crate::application::use_cases::retrieval_service::RetrievalService;
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
use crate::infrastructure::llm_clients::LLMClient;
use crate::infrastructure::llm_clients::RouterClient;
use crate::infrastructure::storage::{
    ensure_qa_sessions_root, ensure_session_dir, resolve_app_data_dir,
};
use crate::interfaces::mock_server::MockServerState;
use crate::interfaces::tauri::distillation_commands::{
    distill_add_dataset_item, distill_cancel_python_training, distill_cleanup_old_backups,
    distill_create_backup, distill_create_dataset, distill_create_model_version,
    distill_create_training_run, distill_delete_correction, distill_delete_dataset,
    distill_delete_soft_label, distill_download_default_model, distill_evaluate_version,
    distill_generate_soft_labels, distill_get_active_version, distill_get_artifact_layout,
    distill_get_correction, distill_get_dataset, distill_get_model, distill_get_model_version,
    distill_get_soft_label, distill_get_training_run, distill_get_version_history,
    distill_import_base_model, distill_import_dataset_jsonl, distill_link_soft_labels_to_run,
    distill_list_backups, distill_list_base_models, distill_list_corrections,
    distill_list_dataset_items, distill_list_datasets, distill_list_model_versions,
    distill_list_models, distill_list_run_artifacts, distill_list_soft_labels, distill_list_tags,
    distill_list_training_logs, distill_list_training_runs, distill_list_version_metrics,
    distill_log_training_step, distill_promote_version, distill_record_artifact,
    distill_record_metric, distill_register_model, distill_restore_backup,
    distill_rollback_version, distill_save_correction, distill_start_python_training,
    distill_update_correction_tags, distill_update_run_status,
};
use crate::interfaces::tauri::rag_commands::{
    rag_add_conversation_message,
    rag_analyze_document_quality,
    rag_assign_experiment_variant,
    rag_build_correction_prompt,
    rag_build_verification_prompt,
    rag_chat_with_context,
    rag_clear_analytics,
    rag_clear_cache,
    rag_clear_feedback,
    rag_clear_metrics,
    rag_clear_retrieval_cache,
    rag_compute_collection_quality,
    rag_create_collection,
    // Conversation persistence
    rag_create_conversation,
    rag_create_document_warning,
    rag_deactivate_experiment,
    rag_delete_chunk,
    rag_delete_collection,
    rag_delete_conversation,
    rag_delete_document,
    rag_enhanced_ocr,
    rag_filter_low_quality_chunks,
    rag_get_analytics_summary,
    rag_get_chunks_with_quality,
    rag_get_collection,
    // Quality analytics
    rag_get_collection_quality,
    rag_get_config,
    rag_get_conversation_messages,
    rag_get_document,
    rag_get_document_quality_summary,
    rag_get_document_warnings,
    rag_get_feedback_stats,
    rag_get_low_quality_documents,
    rag_get_metrics,
    rag_get_recent_analytics,
    rag_get_recent_feedback,
    rag_get_retrieval_cache_stats,
    rag_get_retrieval_gaps,
    rag_get_system_stats,
    rag_health_check,
    rag_hybrid_retrieval,
    rag_hybrid_search,
    rag_import_file,
    rag_import_web,
    rag_invalidate_collection_cache,
    rag_list_chunks,
    rag_list_collections,
    rag_list_conversations,
    rag_list_documents,
    rag_list_excel_data,
    rag_list_experiments,
    rag_query,
    rag_record_document_quality,
    rag_record_metric,
    rag_record_retrieval_gap,
    rag_reembed_chunk,
    rag_reindex_collection,
    rag_reindex_document,
    rag_register_experiment,
    rag_reset_config,
    rag_run_validation_suite,
    rag_smart_chunking,
    rag_submit_feedback,
    rag_update_cache_config,
    rag_update_chat_config,
    rag_update_chunk_content,
    rag_update_chunking_config,
    rag_update_config,
    rag_update_embedding_config,
    rag_update_ocr_config,
    rag_update_retrieval_config,
    rag_validate_config,
    csv_preprocess_file,
    csv_preview_rows,
    csv_analyze,
};
use crate::interfaces::tauri::{
    add_log_message, delete_api_key, enhance_prompt, get_api_key, get_llm_models, get_logs,
    get_translation_history, llm_chat, mock_server_get_config, mock_server_start,
    mock_server_status, mock_server_stop, mock_server_update_config, qa_append_run_stream_event,
    qa_capture_native_screenshot, qa_capture_screenshot, qa_create_checkpoint, qa_delete_events,
    qa_delete_session, qa_end_run, qa_end_session, qa_execute_api_request, qa_explore_session,
    qa_generate_checkpoint_summary, qa_generate_test_cases, qa_get_session,
    qa_list_checkpoint_summaries, qa_list_checkpoints, qa_list_events, qa_list_events_page,
    qa_list_llm_runs, qa_list_run_stream_events, qa_list_screenshots, qa_list_sessions,
    qa_list_test_cases, qa_open_devtools, qa_record_event, qa_replay_browser,
    qa_start_browser_recorder, qa_start_run, qa_start_session, qa_stop_browser_recorder,
    save_api_key, sync_config, sync_embedding_config, sync_languages, sync_shortcuts,
    translate_prompt, AppState,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::{Clipboard, ClipboardExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::error;
use uuid::Uuid;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let app_data_dir = resolve_app_data_dir(&app_handle).map_err(|err| {
                error!(error = %err, "Failed to resolve app data dir");
                err
            })?;

            let mut tesseract_log: Option<String> = None;
            let mut tesseract_lib_log: Option<String> = None;
            let mut pdftoppm_log: Option<String> = None;
            let mut tessdata_log: Option<String> = None;
            let mut ocr_root_log: Option<String> = None;

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

            if let Some(ocr_root) = &ocr_root {
                ocr_root_log = Some(format!("Using OCR resources at {}", ocr_root.display()));
                let tesseract_path = ocr_root.join(os_folder).join(tesseract_name);
                if tesseract_path.exists() {
                    std::env::set_var("TESSERACT_CMD", &tesseract_path);
                    tesseract_log = Some(format!(
                        "Using bundled Tesseract at {}",
                        tesseract_path.display()
                    ));
                } else {
                    tesseract_log = Some(format!(
                        "Bundled Tesseract not found at {}",
                        tesseract_path.display()
                    ));
                }

                let pdftoppm_name = if std::env::consts::OS == "windows" {
                    "pdftoppm.exe"
                } else {
                    "pdftoppm"
                };
                let pdftoppm_path = ocr_root.join(os_folder).join(pdftoppm_name);
                if pdftoppm_path.exists() {
                    std::env::set_var("PDFTOPPM_CMD", &pdftoppm_path);
                    pdftoppm_log = Some(format!(
                        "Using bundled pdftoppm at {}",
                        pdftoppm_path.display()
                    ));
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
                        let separator = if std::env::consts::OS == "windows" {
                            ";"
                        } else {
                            ":"
                        };
                        let new_value = match std::env::var(env_key) {
                            Ok(existing) if !existing.is_empty() => {
                                format!("{}{}{}", lib_dir.display(), separator, existing)
                            }
                            _ => lib_dir.display().to_string(),
                        };
                        std::env::set_var(env_key, new_value);
                        tesseract_lib_log = Some(format!(
                            "Using bundled Tesseract libs at {}",
                            lib_dir.display()
                        ));
                    }
                }

                let tessdata_path = ocr_root.join("tessdata");
                if tessdata_path.exists() {
                    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
                    tessdata_log = Some(format!(
                        "Using bundled tessdata at {}",
                        tessdata_path.display()
                    ));
                }
            }

            let ocr_root_log = ocr_root_log.clone();
            let tesseract_log = tesseract_log.clone();
            let tesseract_lib_log = tesseract_lib_log.clone();
            let pdftoppm_log = pdftoppm_log.clone();
            let tessdata_log = tessdata_log.clone();
            let logs_for_init = Arc::new(Mutex::new(Vec::new()));
            let qa_sessions_dir = ensure_qa_sessions_root(&app_data_dir).map_err(|err| {
                error!(
                    error = %err,
                    qa_sessions_dir = %app_data_dir.join("qa_sessions").display(),
                    "Failed to create qa_sessions dir"
                );
                err
            })?;
            let bootstrap_session_id = Uuid::new_v4().to_string();
            let _bootstrap_session_dir =
                ensure_session_dir(&qa_sessions_dir, &bootstrap_session_id).map_err(|err| {
                    error!(
                        error = %err,
                        session_id = %bootstrap_session_id,
                        session_dir = %qa_sessions_dir.join(&bootstrap_session_id).display(),
                        "Failed to create QA session dir"
                    );
                    err
                })?;

            let qa_db_path = app_data_dir.join("qa_recorder.db");
            println!("Initializing QA database at: {}", qa_db_path.display());

            let rag_db_path = app_data_dir.join("rag_sense.db");
            println!("Initializing RAG database at: {}", rag_db_path.display());

            let training_db_path = app_data_dir.join("training.db");
            println!(
                "Initializing Training database at: {}",
                training_db_path.display()
            );

            let training_artifacts = TrainingArtifactLayout::new(&app_data_dir);
            if let Err(err) = training_artifacts.ensure() {
                crate::interfaces::http::add_log(
                    &logs_for_init,
                    "ERROR",
                    "Training",
                    &format!("Failed to ensure training artifact dirs: {err}"),
                );
            } else {
                crate::interfaces::http::add_log(
                    &logs_for_init,
                    "INFO",
                    "Training",
                    &format!(
                        "Training artifacts root ready: {}",
                        training_artifacts.root().display()
                    ),
                );
            }

            let db_path = app_data_dir.join("promptbridge.db");
            let db_path_str = db_path.to_string_lossy().replace("\\", "/");
            let db_url = format!("sqlite://{}", db_path_str);

            println!("Initializing database at: {}", db_url);

            tauri::async_runtime::block_on(async move {
                init_rag_db(&rag_db_path)
                    .await
                    .expect("Failed to initialize RAG database");
                println!("Initialized RAG database at: {}", rag_db_path.display());
                match std::fs::metadata(&rag_db_path) {
                    Ok(meta) => {
                        crate::interfaces::http::add_log(
                            &logs_for_init,
                            "INFO",
                            "RAG",
                            &format!(
                                "RAG database file created: {} ({} bytes)",
                                rag_db_path.display(),
                                meta.len()
                            ),
                        );
                        println!(
                            "RAG database file created: {} ({} bytes)",
                            rag_db_path.display(),
                            meta.len()
                        );
                    }
                    Err(err) => {
                        crate::interfaces::http::add_log(
                            &logs_for_init,
                            "ERROR",
                            "RAG",
                            &format!(
                                "RAG database file missing after init: {} ({})",
                                rag_db_path.display(),
                                err
                            ),
                        );
                        println!(
                            "RAG database file missing after init: {} ({})",
                            rag_db_path.display(),
                            err
                        );
                    }
                }
                println!("RAG database ready, proceeding to QA database init");

                init_qa_db(&qa_db_path)
                    .await
                    .expect("Failed to initialize QA database");
                println!("Initialized QA database at: {}", qa_db_path.display());

                init_training_db(&training_db_path)
                    .await
                    .expect("Failed to initialize Training database");
                println!(
                    "Initialized Training database at: {}",
                    training_db_path.display()
                );
                match std::fs::metadata(&qa_db_path) {
                    Ok(meta) => println!(
                        "QA database file created: {} ({} bytes)",
                        qa_db_path.display(),
                        meta.len()
                    ),
                    Err(err) => println!(
                        "QA database file missing after init: {} ({})",
                        qa_db_path.display(),
                        err
                    ),
                }

                match std::fs::metadata(&training_db_path) {
                    Ok(meta) => {
                        crate::interfaces::http::add_log(
                            &logs_for_init,
                            "INFO",
                            "Training",
                            &format!(
                                "Training database file created: {} ({} bytes)",
                                training_db_path.display(),
                                meta.len()
                            ),
                        );
                        println!(
                            "Training database file created: {} ({} bytes)",
                            training_db_path.display(),
                            meta.len()
                        );
                    }
                    Err(err) => {
                        crate::interfaces::http::add_log(
                            &logs_for_init,
                            "ERROR",
                            "Training",
                            &format!(
                                "Training database file missing after init: {} ({})",
                                training_db_path.display(),
                                err
                            ),
                        );
                        println!(
                            "Training database file missing after init: {} ({})",
                            training_db_path.display(),
                            err
                        );
                    }
                }

                // Best-effort daily backup + retention cleanup (async, non-blocking).
                let logs_for_backup = logs_for_init.clone();
                let training_db_for_backup = training_db_path.clone();
                let app_data_dir_for_backup = app_data_dir.clone();
                tauri::async_runtime::spawn(async move {
                    let joined = tokio::task::spawn_blocking(move || {
                        let cfg = BackupConfig::new(&app_data_dir_for_backup);
                        ensure_daily_backup(&training_db_for_backup, &cfg)
                    })
                    .await;

                    match joined {
                        Ok(Ok(Some(result))) => {
                            crate::interfaces::http::add_log(
                                &logs_for_backup,
                                "INFO",
                                "Training",
                                &format!(
                                    "Daily training DB backup created: {} ({} bytes)",
                                    result.backup_path.display(),
                                    result.size_bytes
                                ),
                            );
                        }
                        Ok(Ok(None)) => {
                            crate::interfaces::http::add_log(
                                &logs_for_backup,
                                "INFO",
                                "Training",
                                "Daily training DB backup already exists",
                            );
                        }
                        Ok(Err(err)) => {
                            crate::interfaces::http::add_log(
                                &logs_for_backup,
                                "ERROR",
                                "Training",
                                &format!("Failed to create daily training DB backup: {err}"),
                            );
                        }
                        Err(err) => {
                            crate::interfaces::http::add_log(
                                &logs_for_backup,
                                "ERROR",
                                "Training",
                                &format!("Daily backup worker failed: {err}"),
                            );
                        }
                    }
                });

                println!("Training database ready, proceeding to app database init");
                let qa_repo = QaRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA database");
                let qa_repo_arc = Arc::new(qa_repo);
                let qa_event_repo = QaEventRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA events database");
                let qa_event_repo_arc = Arc::new(qa_event_repo);
                let qa_checkpoint_repo = QaCheckpointRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA checkpoints database");
                let qa_checkpoint_repo_arc = Arc::new(qa_checkpoint_repo);
                let qa_run_repo = QaRunRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA runs database");
                let qa_run_repo_arc = Arc::new(qa_run_repo);
                let qa_api_call_repo = QaApiCallRepository::connect(&qa_db_path)
                    .await
                    .expect("Failed to connect QA API calls database");
                let qa_api_call_repo_arc = Arc::new(qa_api_call_repo);

                let rag_repo = RagRepository::connect(&rag_db_path)
                    .await
                    .expect("Failed to connect RAG database");
                let rag_repo_arc = Arc::new(rag_repo);

                let repo = SqliteRepository::init(&db_url)
                    .await
                    .expect("Failed to initialize database");
                let repo_arc = Arc::new(repo);

                let logs = Arc::new(Mutex::new(Vec::new()));
                let logs_for_server = logs.clone();
                let logs_for_setup = logs.clone();
                let mock_server_state = Arc::new(MockServerState::new(
                    app_data_dir.join("mock_server.json"),
                    logs.clone(),
                ));

                if let Some(message) = &ocr_root_log {
                    crate::interfaces::http::add_log(&logs, "INFO", "RAG", message);
                }
                if let Some(message) = &tesseract_log {
                    crate::interfaces::http::add_log(&logs, "INFO", "RAG", message);
                }
                if let Some(message) = &tesseract_lib_log {
                    crate::interfaces::http::add_log(&logs, "INFO", "RAG", message);
                }
                if let Some(message) = &pdftoppm_log {
                    crate::interfaces::http::add_log(&logs, "INFO", "RAG", message);
                }
                if let Some(message) = &tessdata_log {
                    crate::interfaces::http::add_log(&logs, "INFO", "RAG", message);
                }

                let llm_client: Arc<dyn LLMClient + Send + Sync> = Arc::new(RouterClient::new());

                let translate_use_case =
                    TranslateUseCase::new(llm_client.clone(), repo_arc.clone());
                let enhance_use_case = EnhanceUseCase::new(llm_client.clone(), repo_arc.clone());

                let typegen_use_case = TypeGenUseCase::new(llm_client.clone());
                let qa_session_use_case =
                    QaSessionUseCase::new(qa_repo_arc.clone(), qa_sessions_dir.clone());
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
                    base_url: "".to_string(),
                    model: "all-minilm-l6-v2".to_string(),
                    api_key: None,
                    max_tokens: Some(1024),
                    temperature: Some(0.7),
                };
                let embedding_service = Arc::new(EmbeddingService::new(embedding_config.clone()));
                let rag_ingestion_use_case = RagIngestionUseCase::with_embedding_service(
                    rag_repo_arc.clone(),
                    embedding_service.clone(),
                );
                let retrieval_service = Arc::new(RetrievalService::new(
                    rag_repo_arc.clone(),
                    embedding_service.clone(),
                ));

                // Initialize metrics collector and experiment manager
                let metrics_collector =
                    crate::application::use_cases::rag_metrics::SharedMetricsCollector::new();
                let experiment_manager =
                    crate::application::use_cases::rag_metrics::SharedExperimentManager::new();

                // Initialize config manager and feedback collector
                let config_dir = app_data_dir.clone();
                let config_manager =
                    crate::application::use_cases::rag_config::SharedConfigManager::new(config_dir);
                let feedback_collector =
                    crate::application::use_cases::rag_config::SharedFeedbackCollector::new(1000);
                let analytics_logger =
                    crate::application::use_cases::rag_analytics::SharedAnalyticsLogger::new(2000);

                // Initialize conversation service for chat persistence
                let conversation_service = std::sync::Arc::new(
                    crate::application::use_cases::conversation_service::ConversationService::new(
                        rag_repo_arc.clone(),
                    ),
                );

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
                    repository: repo_arc,
                    rag_repository: rag_repo_arc,
                    config_service: ConfigService::new(),
                    llm_client: llm_client.clone(),
                    mock_server: mock_server_state.clone(),
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
                };
                let state_arc = Arc::new(state);

                app_handle.manage(state_arc.clone());

                // Start Actix server
                let state_for_server = state_arc.clone();
                let server =
                    crate::interfaces::http::start_server(state_for_server, logs_for_server)
                        .expect("Failed to start Actix server");

                tokio::spawn(server);

                crate::interfaces::http::add_log(
                    &logs_for_setup,
                    "INFO",
                    "System",
                    "Backend initialized and HTTP server started on :3001",
                );

                if let Err(e) = register_shortcuts(
                    &app_handle,
                    true,
                    "Ctrl + Alt + T",
                    "Ctrl + Alt + E",
                    "Ctrl + Alt + P",
                    "Ctrl + Alt + R",
                ) {
                    eprintln!("Failed to register default shortcuts: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate_prompt,
            enhance_prompt,
            llm_chat,
            get_translation_history,
            save_api_key,
            get_api_key,
            delete_api_key,
            get_llm_models,
            sync_config,
            sync_embedding_config,
            sync_languages,
            sync_shortcuts,
            get_logs,
            add_log_message,
            mock_server_get_config,
            mock_server_update_config,
            mock_server_start,
            mock_server_stop,
            mock_server_status,
            qa_start_session,
            qa_end_session,
            qa_start_run,
            qa_end_run,
            qa_start_browser_recorder,
            qa_stop_browser_recorder,
            qa_append_run_stream_event,
            qa_list_run_stream_events,
            qa_execute_api_request,
            qa_replay_browser,
            qa_record_event,
            qa_open_devtools,
            qa_list_sessions,
            qa_list_events,
            qa_list_screenshots,
            qa_capture_screenshot,
            qa_capture_native_screenshot,
            qa_list_events_page,
            qa_delete_events,
            qa_delete_session,
            qa_get_session,
            qa_create_checkpoint,
            qa_list_checkpoints,
            qa_generate_checkpoint_summary,
            qa_generate_test_cases,
            qa_list_checkpoint_summaries,
            qa_list_test_cases,
            qa_list_llm_runs,
            qa_explore_session,
            rag_create_collection,
            rag_get_collection,
            rag_list_collections,
            rag_delete_collection,
            rag_get_document,
            rag_delete_document,
            rag_list_documents,
            rag_import_file,
            rag_list_chunks,
            rag_list_excel_data,
            rag_hybrid_search,
            rag_query,
            rag_import_web,
            rag_enhanced_ocr,
            rag_smart_chunking,
            rag_hybrid_retrieval,
            rag_run_validation_suite,
            rag_get_analytics_summary,
            rag_get_recent_analytics,
            rag_clear_analytics,
            rag_chat_with_context,
            rag_build_verification_prompt,
            rag_build_correction_prompt,
            rag_health_check,
            rag_clear_cache,
            rag_get_metrics,
            rag_record_metric,
            rag_record_document_quality,
            rag_get_document_quality_summary,
            rag_clear_metrics,
            rag_get_retrieval_cache_stats,
            rag_clear_retrieval_cache,
            rag_invalidate_collection_cache,
            rag_register_experiment,
            rag_list_experiments,
            rag_assign_experiment_variant,
            rag_deactivate_experiment,
            rag_get_system_stats,
            rag_analyze_document_quality,
            // Phase 5: Configuration management
            rag_get_config,
            rag_update_config,
            rag_update_chunking_config,
            rag_update_retrieval_config,
            rag_update_embedding_config,
            rag_update_ocr_config,
            rag_update_cache_config,
            rag_update_chat_config,
            rag_reset_config,
            rag_validate_config,
            // Phase 5: User feedback
            rag_submit_feedback,
            rag_get_feedback_stats,
            rag_get_recent_feedback,
            rag_clear_feedback,
            // Phase 5: Chunk management
            rag_get_chunks_with_quality,
            rag_delete_chunk,
            rag_update_chunk_content,
            rag_reembed_chunk,
            rag_reindex_document,
            rag_reindex_collection,
            rag_filter_low_quality_chunks,
            // Phase 9: Conversation persistence
            rag_create_conversation,
            rag_add_conversation_message,
            rag_get_conversation_messages,
            rag_list_conversations,
            rag_delete_conversation,
            // Phase 10: Quality analytics
            rag_get_collection_quality,
            rag_compute_collection_quality,
            rag_get_document_warnings,
            rag_create_document_warning,
            rag_get_low_quality_documents,
            rag_record_retrieval_gap,
            rag_get_retrieval_gaps,
            // CSV preprocessing commands
            csv_preprocess_file,
            csv_preview_rows,
            csv_analyze,
            // Model Distillation commands
            distill_save_correction,
            distill_get_correction,
            distill_list_corrections,
            distill_delete_correction,
            distill_update_correction_tags,
            distill_list_tags,
            distill_create_dataset,
            distill_get_dataset,
            distill_list_datasets,
            distill_delete_dataset,
            distill_add_dataset_item,
            distill_list_dataset_items,
            distill_import_dataset_jsonl,
            distill_register_model,
            distill_list_base_models,
            distill_import_base_model,
            distill_download_default_model,
            distill_list_models,
            distill_get_model,
            distill_create_training_run,
            distill_update_run_status,
            distill_get_training_run,
            distill_list_training_runs,
            distill_log_training_step,
            distill_list_training_logs,
            distill_create_model_version,
            distill_list_model_versions,
            distill_get_model_version,
            distill_promote_version,
            distill_get_active_version,
            distill_rollback_version,
            distill_get_version_history,
            distill_record_metric,
            distill_list_version_metrics,
            distill_record_artifact,
            distill_list_run_artifacts,
            distill_create_backup,
            distill_list_backups,
            distill_restore_backup,
            distill_cleanup_old_backups,
            distill_get_artifact_layout,
            distill_start_python_training,
            distill_cancel_python_training,
            distill_evaluate_version,
            // Soft Labels commands (Phase 1: Data Preparation)
            distill_generate_soft_labels,
            distill_list_soft_labels,
            distill_get_soft_label,
            distill_delete_soft_label,
            distill_link_soft_labels_to_run
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Only handle close for the main window
                if window.label() == "main" {
                    let app_handle = window.app_handle().clone();

                    // Get the app state and cleanup child processes
                    if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                        // Run cleanup in a blocking task to ensure it completes
                        tauri::async_runtime::block_on(async {
                            crate::interfaces::tauri::cleanup_child_processes(&state).await;
                        });
                    }

                    // Exit the app after cleanup
                    app_handle.exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use enigo::{Enigo, Key, KeyboardControllable};
use std::time::Duration;

fn log_shortcut(state: &tauri::State<'_, Arc<AppState>>, level: &str, message: &str) {
    crate::interfaces::http::add_log(&state.logs, level, "Shortcut", message);
}

fn emit_shortcut_event(app: &tauri::AppHandle, event: &str, payload: &str) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("Failed to emit event: {}", e);
    }
}

fn parse_shortcut(input: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = input
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return Err("Shortcut is empty.".to_string());
    }

    let key_part = parts[parts.len() - 1];
    let mut modifiers = Modifiers::empty();

    for modifier in &parts[..parts.len() - 1] {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "cmd" | "meta" | "win" | "super" => modifiers |= Modifiers::META,
            other => {
                return Err(format!("Unknown modifier: {}", other));
            }
        }
    }

    let code = parse_code(key_part)?;
    let modifiers = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Ok(Shortcut::new(modifiers, code))
}

fn parse_code(key: &str) -> Result<Code, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Shortcut key is empty.".to_string());
    }

    let normalized = match key {
        "Esc" => "Escape".to_string(),
        "Space" => "Space".to_string(),
        "Enter" => "Enter".to_string(),
        "Tab" => "Tab".to_string(),
        "Backspace" => "Backspace".to_string(),
        "Delete" => "Delete".to_string(),
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => key.to_string(),
        "PageUp" | "PageDown" | "Home" | "End" | "Insert" => key.to_string(),
        "Minus" | "Equal" | "Comma" | "Period" | "Slash" | "Semicolon" | "Quote"
        | "BracketLeft" | "BracketRight" | "Backslash" | "Backquote" => key.to_string(),
        _ if key.len() == 1 => {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii_alphabetic() {
                format!("Key{}", ch.to_ascii_uppercase())
            } else if ch.is_ascii_digit() {
                format!("Digit{}", ch)
            } else {
                match ch {
                    '-' => "Minus".to_string(),
                    '=' => "Equal".to_string(),
                    ',' => "Comma".to_string(),
                    '.' => "Period".to_string(),
                    '/' => "Slash".to_string(),
                    ';' => "Semicolon".to_string(),
                    '\'' => "Quote".to_string(),
                    '[' => "BracketLeft".to_string(),
                    ']' => "BracketRight".to_string(),
                    '\\' => "Backslash".to_string(),
                    '`' => "Backquote".to_string(),
                    _ => return Err(format!("Unsupported key: {}", key)),
                }
            }
        }
        _ if key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit()) => {
            key.to_string()
        }
        _ => key.to_string(),
    };

    Code::from_str(&normalized).map_err(|_| format!("Unsupported key: {}", key))
}

pub(crate) fn register_shortcuts(
    app: &tauri::AppHandle,
    enabled: bool,
    translate: &str,
    enhance: &str,
    popup: &str,
    terminal: &str,
) -> Result<(), String> {
    let _ = app.global_shortcut().unregister_all();
    if !enabled {
        return Ok(());
    }

    let translate_shortcut = parse_shortcut(translate)?;
    let enhance_shortcut = parse_shortcut(enhance)?;
    let popup_shortcut = parse_shortcut(popup)?;
    let terminal_shortcut = parse_shortcut(terminal)?;

    let h_t = app.clone();
    app.global_shortcut()
        .on_shortcut(translate_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_t.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_translate(h).await {
                    eprintln!("Global translate error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register translate shortcut: {}", e))?;

    let h_e = app.clone();
    app.global_shortcut()
        .on_shortcut(enhance_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_e.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_enhance(h).await {
                    eprintln!("Global enhance error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register enhance shortcut: {}", e))?;

    let h_p = app.clone();
    app.global_shortcut()
        .on_shortcut(popup_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_p.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_popup(h).await {
                    eprintln!("Global popup error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register popup shortcut: {}", e))?;

    let h_term = app.clone();
    app.global_shortcut()
        .on_shortcut(terminal_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                return;
            }
            let h = h_term.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_global_terminal(h).await {
                    eprintln!("Global terminal error: {}", e);
                }
            });
        })
        .map_err(|e| format!("Failed to register terminal shortcut: {}", e))?;

    Ok(())
}

fn should_restore_main_window(app: &tauri::AppHandle) -> bool {
    if let Some(main_window) = app.get_webview_window("main") {
        let was_minimized = main_window.is_minimized().unwrap_or(false);
        let was_visible = main_window.is_visible().unwrap_or(true);
        if was_visible && !was_minimized {
            let _ = main_window.minimize();
            return true;
        }
    }
    false
}

fn show_loading_window(app: &tauri::AppHandle, payload: &str) {
    if let Some(window) = app.get_webview_window("loading") {
        let _ = window.emit("loading-update", payload);
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = window.center();
    }
}

fn hide_loading_window(app: &tauri::AppHandle, restore_main_window: bool) {
    if let Some(window) = app.get_webview_window("loading") {
        let _ = window.hide();
    }

    if restore_main_window {
        if let Some(main_window) = app.get_webview_window("main") {
            let _ = main_window.unminimize();
            let _ = main_window.set_focus();
        }
    }
}

fn current_config(state: &tauri::State<'_, Arc<AppState>>) -> crate::domain::llm_config::LLMConfig {
    state.last_config.lock().unwrap().clone()
}

fn current_languages(state: &tauri::State<'_, Arc<AppState>>) -> (String, String) {
    let source = state.preferred_source.lock().unwrap().clone();
    let target = state.preferred_target.lock().unwrap().clone();
    let source = if source.trim().is_empty() {
        "Auto Detect".to_string()
    } else {
        source
    };
    let target = if target.trim().is_empty() {
        "English".to_string()
    } else {
        target
    };
    (source, target)
}

async fn perform_robust_copy(enigo: &mut Enigo) {
    #[cfg(target_os = "windows")]
    {
        enigo.key_up(Key::Alt);
        enigo.key_up(Key::Control);
        tokio::time::sleep(Duration::from_millis(50)).await;

        enigo.key_down(Key::Control);
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_down(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Control);
    }
    #[cfg(target_os = "macos")]
    {
        enigo.key_up(Key::Option);
        enigo.key_up(Key::Meta);
        tokio::time::sleep(Duration::from_millis(50)).await;

        enigo.key_down(Key::Meta);
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_down(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Layout('c'));
        tokio::time::sleep(Duration::from_millis(100)).await;
        enigo.key_up(Key::Meta);
    }
    let _ = enigo;
}

async fn capture_selection<R: tauri::Runtime>(
    clipboard: &Clipboard<R>,
    enigo: &mut Enigo,
) -> Result<String, String> {
    let _ = clipboard.write_text("");
    perform_robust_copy(enigo).await;

    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(content) = clipboard.read_text() {
            if !content.is_empty() {
                return Ok(content);
            }
        }
    }

    Err("Clipboard is empty after auto-copy retries.".to_string())
}

async fn auto_paste(enigo: &mut Enigo) {
    #[cfg(target_os = "windows")]
    {
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Control);
    }
    #[cfg(target_os = "macos")]
    {
        enigo.key_down(Key::Meta);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Meta);
    }
    let _ = enigo;
}

async fn handle_global_translate(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "translate");

    emit_shortcut_event(&app, "shortcut-start", "translate");
    log_shortcut(&state, "INFO", "Processing translate shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let (source_lang, target_lang) = current_languages(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!("Shortcut languages: {} -> {}", source_lang, target_lang),
    );
    let result = match state
        .translate_use_case
        .execute(&config, text, source_lang, target_lang)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(translated) = result.result {
        clipboard
            .write_text(translated)
            .map_err(|e| e.to_string())?;

        auto_paste(&mut enigo).await;
        log_shortcut(&state, "INFO", "Translation complete.");
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}

async fn handle_global_enhance(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "enhance");

    emit_shortcut_event(&app, "shortcut-start", "enhance");
    log_shortcut(&state, "INFO", "Processing enhance shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let result = match state.enhance_use_case.execute(&config, text, None).await {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(enhanced) = result.result {
        clipboard.write_text(enhanced).map_err(|e| e.to_string())?;

        auto_paste(&mut enigo).await;
        log_shortcut(&state, "INFO", "Enhancement complete.");
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}

async fn handle_global_popup(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    log_shortcut(&state, "INFO", "Processing popup shortcut...");
    let restore_main_window = should_restore_main_window(&app);
    emit_shortcut_event(&app, "shortcut-start", "popup");

    let mut enigo = Enigo::new();

    let text = capture_selection(&clipboard, &mut enigo).await.ok();

    if let Some(text) = text {
        log_shortcut(&state, "INFO", "Captured text for popup.");
        emit_shortcut_event(&app, "shortcut-capture", &text);
    } else {
        log_shortcut(&state, "WARN", "No text captured for popup.");
    }

    if restore_main_window {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    emit_shortcut_event(&app, "shortcut-end", "success");

    Ok(())
}

async fn handle_global_terminal(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let clipboard = app.clipboard();

    let restore_main_window = should_restore_main_window(&app);
    show_loading_window(&app, "terminal");

    emit_shortcut_event(&app, "shortcut-start", "terminal");
    log_shortcut(&state, "INFO", "Processing terminal shortcut...");

    let mut enigo = Enigo::new();

    let text = match capture_selection(&clipboard, &mut enigo).await {
        Ok(text) => text,
        Err(message) => {
            log_shortcut(&state, "WARN", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    let config = current_config(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Terminal shortcut config: provider={:?} base_url={} model={}",
            config.provider, config.base_url, config.model
        ),
    );

    let (source_lang, target_lang) = current_languages(&state);
    log_shortcut(
        &state,
        "INFO",
        &format!(
            "Terminal shortcut languages: {} -> {}",
            source_lang, target_lang
        ),
    );
    let result = match state
        .translate_use_case
        .execute(&config, text, source_lang, target_lang)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let message = e.to_string();
            log_shortcut(&state, "ERROR", &message);
            emit_shortcut_event(&app, "shortcut-end", "error");
            hide_loading_window(&app, restore_main_window);
            return Ok(());
        }
    };

    if let Some(translated) = result.result {
        clipboard
            .write_text(translated)
            .map_err(|e| e.to_string())?;

        // NOTE: No auto-paste for terminal mode - user manually pastes when ready
        log_shortcut(
            &state,
            "INFO",
            "Terminal translation complete (clipboard ready, no auto-paste).",
        );
    }

    emit_shortcut_event(&app, "shortcut-end", "success");
    hide_loading_window(&app, restore_main_window);
    Ok(())
}
