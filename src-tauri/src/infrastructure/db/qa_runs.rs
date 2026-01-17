use crate::domain::error::{AppError, Result};
use crate::domain::qa_run::{QaRunStreamEvent, QaSessionRun};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct QaRunRepository {
    pool: SqlitePool,
}

impl QaRunRepository {
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

    pub async fn insert_run(&self, run: &QaSessionRun) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_runs (id, session_id, run_type, mode, status, triggered_by, source_run_id, checkpoint_id, started_at, ended_at, meta_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&run.id)
        .bind(&run.session_id)
        .bind(&run.run_type)
        .bind(&run.mode)
        .bind(&run.status)
        .bind(&run.triggered_by)
        .bind(&run.source_run_id)
        .bind(&run.checkpoint_id)
        .bind(run.started_at)
        .bind(run.ended_at)
        .bind(&run.meta_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert QA run: {e}")))?;

        Ok(())
    }

    pub async fn update_run_status(
        &self,
        run_id: &str,
        status: &str,
        ended_at: Option<i64>,
    ) -> Result<()> {
        sqlx::query("UPDATE session_runs SET status = ?, ended_at = ? WHERE id = ?")
            .bind(status)
            .bind(ended_at)
            .bind(run_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update QA run: {e}")))?;

        Ok(())
    }

    pub async fn get_run(&self, run_id: &str) -> Result<QaSessionRun> {
        let run = sqlx::query_as::<_, QaRunEntity>(
            "SELECT id, session_id, run_type, mode, status, triggered_by, source_run_id, checkpoint_id, started_at, ended_at, meta_json
             FROM session_runs WHERE id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch QA run: {e}")))?;

        match run {
            Some(run) => Ok(run.into()),
            None => Err(AppError::NotFound(format!("QA run not found: {}", run_id))),
        }
    }

    #[allow(dead_code)]
    pub async fn list_runs(&self, session_id: &str) -> Result<Vec<QaSessionRun>> {
        let runs = sqlx::query_as::<_, QaRunEntity>(
            "SELECT id, session_id, run_type, mode, status, triggered_by, source_run_id, checkpoint_id, started_at, ended_at, meta_json
             FROM session_runs WHERE session_id = ? ORDER BY started_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA runs: {e}")))?;

        Ok(runs.into_iter().map(|run| run.into()).collect())
    }

    pub async fn insert_stream_event(&self, event: &QaRunStreamEvent) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_stream_events (id, run_id, seq, ts, channel, level, message, payload_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event.run_id)
        .bind(event.seq)
        .bind(event.ts)
        .bind(&event.channel)
        .bind(&event.level)
        .bind(&event.message)
        .bind(&event.payload_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert QA stream event: {e}")))?;

        Ok(())
    }

    pub async fn next_stream_seq(&self, run_id: &str) -> Result<i64> {
        let seq = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM run_stream_events WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch stream seq: {e}")))?;

        Ok(seq)
    }

    pub async fn list_stream_events(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<QaRunStreamEvent>> {
        let events = sqlx::query_as::<_, QaRunStreamEntity>(
            "SELECT id, run_id, seq, ts, channel, level, message, payload_json
             FROM run_stream_events WHERE run_id = ? ORDER BY seq DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list run stream events: {e}")))?;

        Ok(events.into_iter().map(|event| event.into()).collect())
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("QA database path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

#[derive(sqlx::FromRow)]
struct QaRunEntity {
    id: String,
    session_id: String,
    run_type: String,
    mode: String,
    status: String,
    triggered_by: String,
    source_run_id: Option<String>,
    checkpoint_id: Option<String>,
    started_at: i64,
    ended_at: Option<i64>,
    meta_json: Option<String>,
}

impl From<QaRunEntity> for QaSessionRun {
    fn from(entity: QaRunEntity) -> Self {
        Self {
            id: entity.id,
            session_id: entity.session_id,
            run_type: entity.run_type,
            mode: entity.mode,
            status: entity.status,
            triggered_by: entity.triggered_by,
            source_run_id: entity.source_run_id,
            checkpoint_id: entity.checkpoint_id,
            started_at: entity.started_at,
            ended_at: entity.ended_at,
            meta_json: entity.meta_json,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QaRunStreamEntity {
    id: String,
    run_id: String,
    seq: i64,
    ts: i64,
    channel: String,
    level: String,
    message: String,
    payload_json: Option<String>,
}

impl From<QaRunStreamEntity> for QaRunStreamEvent {
    fn from(entity: QaRunStreamEntity) -> Self {
        Self {
            id: entity.id,
            run_id: entity.run_id,
            seq: entity.seq,
            ts: entity.ts,
            channel: entity.channel,
            level: entity.level,
            message: entity.message,
            payload_json: entity.payload_json,
        }
    }
}
