use crate::domain::error::{AppError, Result};
use crate::domain::qa_session::QaSession;
use crate::infrastructure::db::qa_sessions::QaRepository;
use crate::infrastructure::storage::ensure_session_dir;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

pub struct QaSessionUseCase {
    repository: Arc<QaRepository>,
    qa_sessions_dir: PathBuf,
}

impl QaSessionUseCase {
    pub fn new(repository: Arc<QaRepository>, qa_sessions_dir: PathBuf) -> Self {
        Self {
            repository,
            qa_sessions_dir,
        }
    }

    pub async fn start_session(
        &self,
        title: String,
        goal: String,
        is_positive_case: bool,
        app_version: Option<String>,
        os: Option<String>,
        notes: Option<String>,
    ) -> Result<QaSession> {
        if goal.trim().is_empty() {
            return Err(AppError::ValidationError("Goal is required.".to_string()));
        }

        let title = if title.trim().is_empty() {
            "Untitled Session".to_string()
        } else {
            title
        };

        let session_id = Uuid::new_v4().to_string();
        ensure_session_dir(&self.qa_sessions_dir, &session_id).map_err(|e| {
            let session_dir = self.qa_sessions_dir.join(&session_id);
            error!(
                error = %e,
                session_id = %session_id,
                session_dir = %session_dir.display(),
                "Failed to create QA session directory"
            );
            AppError::Internal(format!("Failed to create QA session dir: {}", e))
        })?;

        let started_at = chrono::Utc::now().timestamp_millis();
        let session = QaSession {
            id: session_id,
            title,
            goal,
            is_positive_case,
            app_version,
            os,
            started_at,
            ended_at: None,
            notes,
        };

        self.repository.insert_session(&session).await?;
        Ok(session)
    }

    pub async fn end_session(&self, session_id: &str) -> Result<QaSession> {
        let ended_at = chrono::Utc::now().timestamp_millis();
        self.repository.end_session(session_id, ended_at).await
    }

    pub async fn get_session(&self, session_id: &str) -> Result<QaSession> {
        self.repository.get_session(session_id).await
    }

    pub async fn list_sessions(&self, limit: Option<i64>) -> Result<Vec<QaSession>> {
        let limit = match limit {
            Some(value) if value > 0 => value,
            _ => 50,
        };
        self.repository.list_sessions(limit).await
    }
}
