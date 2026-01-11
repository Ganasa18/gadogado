use crate::domain::error::{AppError, Result};
use crate::domain::qa_session::QaSession;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::path::Path;
use std::str::FromStr;

pub struct QaRepository {
    pool: SqlitePool,
}

impl QaRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = db_path_to_url(db_path)?;
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse QA DB URL: {e}")))?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect QA DB: {e}")))?;

        Ok(Self { pool })
    }

    pub async fn insert_session(&self, session: &QaSession) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (id, title, goal, is_positive_case, app_version, os, started_at, ended_at, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(&session.goal)
        .bind(if session.is_positive_case { 1 } else { 0 })
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
            "SELECT id, title, goal, is_positive_case, app_version, os, started_at, ended_at, notes
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
            "SELECT id, title, goal, is_positive_case, app_version, os, started_at, ended_at, notes
             FROM sessions ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA sessions: {e}")))?;

        Ok(sessions.into_iter().map(|session| session.into()).collect())
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
    is_positive_case: i64,
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
            is_positive_case: entity.is_positive_case != 0,
            app_version: entity.app_version,
            os: entity.os,
            started_at: entity.started_at,
            ended_at: entity.ended_at,
            notes: entity.notes,
        }
    }
}
