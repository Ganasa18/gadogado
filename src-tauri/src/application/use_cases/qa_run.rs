use crate::domain::error::{AppError, Result};
use crate::domain::qa_run::{QaRunStreamEvent, QaRunStreamInput, QaSessionRun};
use crate::infrastructure::db::qa_runs::QaRunRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct QaRunUseCase {
    repository: Arc<QaRunRepository>,
}

impl QaRunUseCase {
    pub fn new(repository: Arc<QaRunRepository>) -> Self {
        Self { repository }
    }

    pub async fn start_run(
        &self,
        session_id: &str,
        run_type: &str,
        mode: &str,
        triggered_by: &str,
        source_run_id: Option<String>,
        checkpoint_id: Option<String>,
        meta_json: Option<String>,
    ) -> Result<QaSessionRun> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        let run_type = normalize_required(run_type, "Run type is required.")?;
        let mode = normalize_required(mode, "Run mode is required.")?;
        let triggered_by = normalize_required(triggered_by, "Triggered by is required.")?;

        let run = QaSessionRun {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            run_type,
            mode,
            status: "running".to_string(),
            triggered_by,
            source_run_id: normalize_optional(source_run_id),
            checkpoint_id: normalize_optional(checkpoint_id),
            started_at: chrono::Utc::now().timestamp_millis(),
            ended_at: None,
            meta_json: normalize_optional(meta_json),
        };

        self.repository.insert_run(&run).await?;
        Ok(run)
    }

    pub async fn end_run(&self, run_id: &str, status: &str) -> Result<QaSessionRun> {
        let run_id = run_id.trim();
        if run_id.is_empty() {
            return Err(AppError::ValidationError("Run id is required.".to_string()));
        }
        let status = normalize_required(status, "Run status is required.")?;
        let ended_at = chrono::Utc::now().timestamp_millis();
        self.repository
            .update_run_status(run_id, &status, Some(ended_at))
            .await?;
        self.repository.get_run(run_id).await
    }

    #[allow(dead_code)]
    pub async fn list_runs(&self, session_id: &str) -> Result<Vec<QaSessionRun>> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Err(AppError::ValidationError(
                "Session id is required.".to_string(),
            ));
        }
        self.repository.list_runs(session_id).await
    }

    pub async fn append_stream_event(
        &self,
        run_id: &str,
        input: QaRunStreamInput,
    ) -> Result<QaRunStreamEvent> {
        let run_id = run_id.trim();
        if run_id.is_empty() {
            return Err(AppError::ValidationError("Run id is required.".to_string()));
        }
        let channel = normalize_required(&input.channel, "Channel is required.")?;
        let level = normalize_required(&input.level, "Level is required.")?;
        let message = normalize_required(&input.message, "Message is required.")?;

        let seq = self.repository.next_stream_seq(run_id).await?;
        let event = QaRunStreamEvent {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.to_string(),
            seq,
            ts: chrono::Utc::now().timestamp_millis(),
            channel,
            level,
            message,
            payload_json: normalize_optional(input.payload_json),
        };
        self.repository.insert_stream_event(&event).await?;
        Ok(event)
    }

    pub async fn list_stream_events(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<QaRunStreamEvent>> {
        let run_id = run_id.trim();
        if run_id.is_empty() {
            return Err(AppError::ValidationError("Run id is required.".to_string()));
        }
        let limit = if limit <= 0 { 50 } else { limit };
        self.repository.list_stream_events(run_id, limit).await
    }
}

fn normalize_required(value: &str, message: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(message.to_string()));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
