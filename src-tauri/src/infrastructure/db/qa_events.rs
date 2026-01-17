use crate::domain::error::{AppError, Result};
use crate::domain::qa_event::{QaEvent, QaEventSummary};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct QaEventRepository {
    pool: SqlitePool,
}

impl QaEventRepository {
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

    pub async fn insert_event(&self, mut event: QaEvent) -> Result<QaEvent> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                AppError::DatabaseError(format!("Failed to start QA event tx: {e}"))
            })?;

        let next_seq = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM events WHERE session_id = ?",
        )
        .bind(&event.session_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to read QA event seq: {e}")))?;

        event.seq = next_seq;

        sqlx::query(
            "INSERT INTO events (id, session_id, run_id, checkpoint_id, seq, ts, event_type, origin, recording_mode, selector, element_text, value, url, screenshot_id, meta_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event.session_id)
        .bind(&event.run_id)
        .bind(&event.checkpoint_id)
        .bind(event.seq)
        .bind(event.ts)
        .bind(&event.event_type)
        .bind(&event.origin)
        .bind(&event.recording_mode)
        .bind(&event.selector)
        .bind(&event.element_text)
        .bind(&event.value)
        .bind(&event.url)
        .bind(&event.screenshot_id)
        .bind(&event.meta_json)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert QA event: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to commit QA event tx: {e}")))?;

        Ok(event)
    }

    pub async fn list_events(&self, session_id: &str) -> Result<Vec<QaEvent>> {
        let events = sqlx::query_as::<_, QaEventEntity>(
            "SELECT e.id, e.session_id, e.run_id, e.checkpoint_id, e.seq, e.ts, e.event_type, e.origin, e.recording_mode, e.selector, e.element_text, e.value, e.url, e.screenshot_id, e.meta_json, a.path as screenshot_path
             FROM events e
             LEFT JOIN artifacts a ON e.screenshot_id = a.id
             WHERE e.session_id = ? ORDER BY e.seq ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA events: {e}")))?;

        Ok(events.into_iter().map(|event| event.into()).collect())
    }

    pub async fn list_screenshots(&self, session_id: &str) -> Result<Vec<QaEvent>> {
        let events = sqlx::query_as::<_, QaEventEntity>(
            "SELECT e.id, e.session_id, e.run_id, e.checkpoint_id, e.seq, e.ts, e.event_type, e.origin, e.recording_mode, e.selector, e.element_text, e.value, e.url, e.screenshot_id, e.meta_json, a.path as screenshot_path
             FROM events e
             INNER JOIN artifacts a ON e.screenshot_id = a.id
             WHERE e.session_id = ? ORDER BY e.seq ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list screenshots: {e}")))?;

        Ok(events.into_iter().map(|event| event.into()).collect())
    }

    pub async fn latest_event_id(&self, session_id: &str) -> Result<Option<String>> {
        let event_id = sqlx::query_scalar::<_, String>(
            "SELECT id FROM events WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest QA event id: {e}")))?;

        Ok(event_id)
    }

    pub async fn latest_event_summary(&self, session_id: &str) -> Result<Option<QaEventSummary>> {
        let summary = sqlx::query_as::<_, QaEventSummaryEntity>(
            "SELECT id, seq, ts, event_type FROM events WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest QA event summary: {e}")))?;

        Ok(summary.map(|entry| entry.into()))
    }

    pub async fn list_events_range(
        &self,
        session_id: &str,
        start_seq: i64,
        end_seq: i64,
    ) -> Result<Vec<QaEvent>> {
        let events = sqlx::query_as::<_, QaEventEntity>(
            "SELECT e.id, e.session_id, e.run_id, e.checkpoint_id, e.seq, e.ts, e.event_type, e.origin, e.recording_mode, e.selector, e.element_text, e.value, e.url, e.screenshot_id, e.meta_json, a.path as screenshot_path
             FROM events e
             LEFT JOIN artifacts a ON e.screenshot_id = a.id
             WHERE e.session_id = ? AND e.seq BETWEEN ? AND ? ORDER BY e.seq ASC",
        )
        .bind(session_id)
        .bind(start_seq)
        .bind(end_seq)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA events range: {e}")))?;

        Ok(events.into_iter().map(|event| event.into()).collect())
    }

    pub async fn attach_screenshot(
        &self,
        session_id: &str,
        event_id: &str,
        artifact_id: &str,
        path: &str,
        mime: Option<&str>,
        width: Option<i64>,
        height: Option<i64>,
        created_at: i64,
    ) -> Result<()> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                AppError::DatabaseError(format!("Failed to start QA artifact tx: {e}"))
            })?;

        sqlx::query(
            "INSERT INTO artifacts (id, session_id, event_id, type, path, mime, width, height, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(artifact_id)
        .bind(session_id)
        .bind(event_id)
        .bind("screenshot")
        .bind(path)
        .bind(mime)
        .bind(width)
        .bind(height)
        .bind(created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert QA artifact: {e}")))?;

        let result =
            sqlx::query("UPDATE events SET screenshot_id = ? WHERE id = ? AND session_id = ?")
                .bind(artifact_id)
                .bind(event_id)
                .bind(session_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::DatabaseError(format!("Failed to update QA event screenshot: {e}"))
                })?;

        if result.rows_affected() == 0 {
            return Err(AppError::DatabaseError(
                "No QA event updated for screenshot link.".to_string(),
            ));
        }

        tx.commit().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to commit QA artifact tx: {e}"))
        })?;

        Ok(())
    }

    pub async fn list_events_page(
        &self,
        session_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<QaEvent>> {
        let events = sqlx::query_as::<_, QaEventEntity>(
            "SELECT e.id, e.session_id, e.run_id, e.checkpoint_id, e.seq, e.ts, e.event_type, e.origin, e.recording_mode, e.selector, e.element_text, e.value, e.url, e.screenshot_id, e.meta_json, a.path as screenshot_path
             FROM events e
             LEFT JOIN artifacts a ON e.screenshot_id = a.id
             WHERE e.session_id = ? ORDER BY e.seq ASC LIMIT ? OFFSET ?",
        )
        .bind(session_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list QA events page: {e}")))?;

        Ok(events.into_iter().map(|event| event.into()).collect())
    }

    pub async fn count_events(&self, session_id: &str) -> Result<i64> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM events WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to count QA events: {e}")))?;

        Ok(count)
    }

    pub async fn delete_events(&self, session_id: &str, event_ids: &[String]) -> Result<u64> {
        if event_ids.is_empty() {
            return Ok(0);
        }

        let placeholders = vec!["?"; event_ids.len()].join(", ");
        let query = format!(
            "DELETE FROM events WHERE session_id = ? AND id IN ({})",
            placeholders
        );

        let mut statement = sqlx::query(&query).bind(session_id);
        for id in event_ids {
            statement = statement.bind(id);
        }

        let result = statement
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete QA events: {e}")))?;

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
struct QaEventEntity {
    id: String,
    session_id: String,
    run_id: Option<String>,
    checkpoint_id: Option<String>,
    seq: i64,
    ts: i64,
    event_type: String,
    origin: Option<String>,
    recording_mode: Option<String>,
    selector: Option<String>,
    element_text: Option<String>,
    value: Option<String>,
    url: Option<String>,
    screenshot_id: Option<String>,
    screenshot_path: Option<String>,
    meta_json: Option<String>,
}

impl From<QaEventEntity> for QaEvent {
    fn from(entity: QaEventEntity) -> Self {
        Self {
            id: entity.id,
            session_id: entity.session_id,
            run_id: entity.run_id,
            checkpoint_id: entity.checkpoint_id,
            seq: entity.seq,
            ts: entity.ts,
            event_type: entity.event_type,
            origin: entity.origin,
            recording_mode: entity.recording_mode,
            selector: entity.selector,
            element_text: entity.element_text,
            value: entity.value,
            url: entity.url,
            screenshot_id: entity.screenshot_id,
            screenshot_path: entity.screenshot_path,
            meta_json: entity.meta_json,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QaEventSummaryEntity {
    id: String,
    seq: i64,
    ts: i64,
    event_type: String,
}

impl From<QaEventSummaryEntity> for QaEventSummary {
    fn from(entity: QaEventSummaryEntity) -> Self {
        Self {
            id: entity.id,
            seq: entity.seq,
            ts: entity.ts,
            event_type: entity.event_type,
        }
    }
}
