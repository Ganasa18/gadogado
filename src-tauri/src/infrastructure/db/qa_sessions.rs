use crate::domain::error::{AppError, Result};
use crate::domain::qa_session::QaSession;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct QaRepository {
    pool: SqlitePool,
}

impl QaRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = db_path_to_url(db_path)?;
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse QA DB URL: {e}")))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect QA DB: {e}")))?;

        Ok(Self { pool })
    }

    pub async fn insert_session(&self, session: &QaSession) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (id, title, goal, session_type, is_positive_case, target_url, api_base_url, auth_profile_json, source_session_id, app_version, os, started_at, ended_at, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(&session.goal)
        .bind(&session.session_type)
        .bind(if session.is_positive_case { 1 } else { 0 })
        .bind(&session.target_url)
        .bind(&session.api_base_url)
        .bind(&session.auth_profile_json)
        .bind(&session.source_session_id)
        .bind(&session.app_version)
        .bind(&session.os)
        .bind(session.started_at)
        .bind(session.ended_at)
        .bind(&session.notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert QA session: {e}")))?;

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<QaSession> {
        let session = sqlx::query_as::<_, QaSessionEntity>(
            "SELECT id, title, goal, session_type, is_positive_case, target_url, api_base_url, auth_profile_json, source_session_id, app_version, os, started_at, ended_at, notes
             FROM sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch QA session: {e}")))?;

        match session {
            Some(session) => Ok(session.into()),
            None => Err(AppError::NotFound(format!(
                "QA session not found: {}",
                session_id
            ))),
        }
    }

    pub async fn end_session(&self, session_id: &str, ended_at: i64) -> Result<QaSession> {
        let result = sqlx::query(
            "UPDATE sessions SET ended_at = ? WHERE id = ? AND ended_at IS NULL",
        )
        .bind(ended_at)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to end QA session: {e}")))?;

        if result.rows_affected() == 0 {
            match self.get_session(session_id).await {
                Ok(_) => Err(AppError::ValidationError(
                    "QA session already ended.".to_string(),
                )),
                Err(AppError::NotFound(_)) => Err(AppError::NotFound(format!(
                    "QA session not found: {}",
                    session_id
                ))),
                Err(err) => Err(err),
            }
        } else {
            self.get_session(session_id).await
        }
    }

    pub async fn list_sessions(&self, limit: i64) -> Result<Vec<QaSession>> {
        let sessions = sqlx::query_as::<_, QaSessionEntity>(
            "SELECT id, title, goal, session_type, is_positive_case, target_url, api_base_url, auth_profile_json, source_session_id, app_version, os, started_at, ended_at, notes
             FROM sessions ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA sessions: {e}")))?;

        Ok(sessions.into_iter().map(|session| session.into()).collect())
    }

    pub async fn delete_session_cascade(&self, session_id: &str) -> Result<u64> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to start QA delete txn: {e}")))?;

        sqlx::query(
            "DELETE FROM checkpoint_summaries WHERE checkpoint_id IN (
                SELECT id FROM checkpoints WHERE session_id = ?
            )",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete checkpoint summaries: {e}")))?;

        sqlx::query("DELETE FROM test_cases WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete test cases: {e}")))?;

        sqlx::query(
            "DELETE FROM llm_runs WHERE scope_id IN (
                SELECT id FROM checkpoints WHERE session_id = ?
            )",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete LLM runs: {e}")))?;

        sqlx::query("DELETE FROM artifacts WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete artifacts: {e}")))?;

        sqlx::query("DELETE FROM api_calls WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete api calls: {e}")))?;

        sqlx::query("DELETE FROM ai_actions WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete ai actions: {e}")))?;

        sqlx::query("DELETE FROM test_case_runs WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete test case runs: {e}")))?;

        sqlx::query("DELETE FROM replay_runs WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete replay runs: {e}")))?;

        sqlx::query(
            "DELETE FROM run_stream_events WHERE run_id IN (
                SELECT id FROM session_runs WHERE session_id = ?
            )",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete run stream events: {e}")))?;

        sqlx::query("DELETE FROM session_runs WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete session runs: {e}")))?;

        sqlx::query("DELETE FROM events WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete events: {e}")))?;

        sqlx::query("DELETE FROM checkpoints WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete checkpoints: {e}")))?;

        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete session: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to commit delete: {e}")))?;

        Ok(result.rows_affected())
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("QA database path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

#[derive(sqlx::FromRow)]
struct QaSessionEntity {
    id: String,
    title: String,
    goal: String,
    session_type: String,
    is_positive_case: i64,
    target_url: Option<String>,
    api_base_url: Option<String>,
    auth_profile_json: Option<String>,
    source_session_id: Option<String>,
    app_version: Option<String>,
    os: Option<String>,
    started_at: i64,
    ended_at: Option<i64>,
    notes: Option<String>,
}

impl From<QaSessionEntity> for QaSession {
    fn from(entity: QaSessionEntity) -> Self {
        Self {
            id: entity.id,
            title: entity.title,
            goal: entity.goal,
            session_type: entity.session_type,
            is_positive_case: entity.is_positive_case != 0,
            target_url: entity.target_url,
            api_base_url: entity.api_base_url,
            auth_profile_json: entity.auth_profile_json,
            source_session_id: entity.source_session_id,
            app_version: entity.app_version,
            os: entity.os,
            started_at: entity.started_at,
            ended_at: entity.ended_at,
            notes: entity.notes,
        }
    }
}
