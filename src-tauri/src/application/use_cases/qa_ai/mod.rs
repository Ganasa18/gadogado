mod checkpoints;
mod event_text;
mod explore;
mod hashing;
mod llm_output;
mod prompts;
mod success_detection;
mod summaries;
mod tests;
mod types;

use crate::domain::error::{AppError, Result};
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::qa_checkpoints::QaCheckpointRepository;
use crate::infrastructure::db::qa_events::QaEventRepository;
use crate::infrastructure::db::qa_sessions::QaRepository;
use crate::infrastructure::llm_clients::LLMClient;
use std::sync::Arc;
use uuid::Uuid;

use hashing::hash_value;
use types::TestCaseInput;

const PROMPT_VERSION: &str = "v1";

pub struct QaAiUseCase {
    session_repository: Arc<QaRepository>,
    event_repository: Arc<QaEventRepository>,
    checkpoint_repository: Arc<QaCheckpointRepository>,
    llm_client: Arc<dyn LLMClient + Send + Sync>,
}

impl QaAiUseCase {
    pub fn new(
        session_repository: Arc<QaRepository>,
        event_repository: Arc<QaEventRepository>,
        checkpoint_repository: Arc<QaCheckpointRepository>,
        llm_client: Arc<dyn LLMClient + Send + Sync>,
    ) -> Self {
        Self {
            session_repository,
            event_repository,
            checkpoint_repository,
            llm_client,
        }
    }

    async fn insert_checkpoint(
        &self,
        session_id: &str,
        title: Option<String>,
        start_event_seq: i64,
        end_event_seq: i64,
    ) -> Result<QaCheckpoint> {
        if start_event_seq <= 0 || end_event_seq <= 0 {
            return Err(AppError::ValidationError(
                "Checkpoint event range is invalid.".to_string(),
            ));
        }
        if start_event_seq > end_event_seq {
            return Err(AppError::ValidationError(
                "Checkpoint start sequence must be before end.".to_string(),
            ));
        }

        let created_at = chrono::Utc::now().timestamp_millis();
        let id = Uuid::new_v4().to_string();
        self.checkpoint_repository
            .insert_checkpoint(
                session_id,
                title
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                start_event_seq,
                end_event_seq,
                created_at,
                id,
            )
            .await
    }

    async fn store_test_cases(
        &self,
        checkpoint_id: &str,
        session: &QaSession,
        items: &[TestCaseInput],
        case_type: &str,
        created_at: i64,
    ) -> Result<Vec<QaTestCase>> {
        let mut stored = Vec::new();
        for item in items {
            let title = item.title.trim();
            if title.is_empty() {
                continue;
            }

            let steps_json = serde_json::to_string(&item.steps).unwrap_or_else(|_| "[]".to_string());
            let dedup_source = format!("{}:{}:{}", case_type, title, steps_json);
            let test_case = QaTestCase {
                id: Uuid::new_v4().to_string(),
                session_id: session.id.clone(),
                checkpoint_id: Some(checkpoint_id.to_string()),
                case_type: case_type.to_string(),
                title: title.to_string(),
                steps_json,
                expected: item.expected.clone().filter(|value| !value.trim().is_empty()),
                priority: item.priority.clone().filter(|value| !value.trim().is_empty()),
                status: None,
                dedup_hash: hash_value(&dedup_source),
                created_at,
            };
            self.checkpoint_repository
                .insert_test_case(&test_case)
                .await?;
            stored.push(test_case);
        }

        Ok(stored)
    }
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResult {
    pub checkpoints: Vec<QaCheckpoint>,
    pub summaries: Vec<QaCheckpointSummary>,
    pub test_cases: Vec<QaTestCase>,
    pub llm_runs: Vec<QaLlmRun>,
    pub post_submit_detected: bool,
    pub detected_patterns: Vec<String>,
}
