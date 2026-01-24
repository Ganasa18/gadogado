use super::QaAiUseCase;
use crate::domain::error::{AppError, Result};
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use crate::domain::qa_event::{QaEvent, QaEventSummary};

const IDLE_THRESHOLD_MS: i64 = 15000;

impl QaAiUseCase {
    pub async fn create_checkpoint(
        &self,
        session_id: &str,
        title: Option<String>,
    ) -> Result<QaCheckpoint> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError("Session id is required.".to_string()));
        }

        let latest_event = self
            .event_repository
            .latest_event_summary(session_id)
            .await?
            .ok_or_else(|| AppError::ValidationError("No events recorded yet.".to_string()))?;

        let latest_checkpoint = self
            .checkpoint_repository
            .latest_checkpoint(session_id)
            .await?;
        let start_seq = latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.end_event_seq + 1)
            .unwrap_or(1);

        if start_seq > latest_event.seq {
            return Err(AppError::ValidationError(
                "No new events to create a checkpoint.".to_string(),
            ));
        }

        self.insert_checkpoint(session_id, title, start_seq, latest_event.seq)
            .await
    }

    pub async fn maybe_create_checkpoint_from_event(
        &self,
        session_id: &str,
        recorded: &QaEvent,
        previous_event: Option<QaEventSummary>,
    ) -> Result<Vec<QaCheckpoint>> {
        let mut created = Vec::new();
        let mut latest_checkpoint = self
            .checkpoint_repository
            .latest_checkpoint(session_id)
            .await?;
        let mut start_seq = latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.end_event_seq + 1)
            .unwrap_or(1);

        if let Some(previous) = previous_event {
            if recorded.ts - previous.ts >= IDLE_THRESHOLD_MS && previous.seq >= start_seq {
                let checkpoint = self
                    .insert_checkpoint(
                        session_id,
                        Some("Idle gap checkpoint".to_string()),
                        start_seq,
                        previous.seq,
                    )
                    .await?;
                created.push(checkpoint);
                latest_checkpoint = self
                    .checkpoint_repository
                    .latest_checkpoint(session_id)
                    .await?;
                start_seq = latest_checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.end_event_seq + 1)
                    .unwrap_or(1);
            }
        }

        let event_type = recorded.event_type.as_str();
        let is_submit = event_type == "submit";
        let is_navigation = event_type == "navigation";
        if (is_submit || is_navigation) && recorded.seq >= start_seq {
            let title = if is_submit {
                "Form submit checkpoint".to_string()
            } else {
                "Navigation checkpoint".to_string()
            };
            let checkpoint = self
                .insert_checkpoint(session_id, Some(title), start_seq, recorded.seq)
                .await?;
            created.push(checkpoint);
        }

        Ok(created)
    }

    pub async fn list_checkpoints(&self, session_id: &str) -> Result<Vec<QaCheckpoint>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError("Session id is required.".to_string()));
        }
        self.checkpoint_repository
            .list_checkpoints(session_id)
            .await
    }

    pub async fn list_checkpoint_summaries(
        &self,
        session_id: &str,
    ) -> Result<Vec<QaCheckpointSummary>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError("Session id is required.".to_string()));
        }
        self.checkpoint_repository
            .list_checkpoint_summaries(session_id)
            .await
    }

    pub async fn list_test_cases(&self, session_id: &str) -> Result<Vec<QaTestCase>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError("Session id is required.".to_string()));
        }
        self.checkpoint_repository.list_test_cases(session_id).await
    }

    pub async fn list_llm_runs(&self, session_id: &str) -> Result<Vec<QaLlmRun>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError("Session id is required.".to_string()));
        }
        self.checkpoint_repository.list_llm_runs(session_id).await
    }
}
