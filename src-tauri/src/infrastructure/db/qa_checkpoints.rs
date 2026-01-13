use crate::domain::error::{AppError, Result};
use crate::domain::qa_checkpoint::{QaCheckpoint, QaCheckpointSummary, QaLlmRun, QaTestCase};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

pub struct QaCheckpointRepository {
    pool: SqlitePool,
}

impl QaCheckpointRepository {
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

    pub async fn insert_checkpoint(
        &self,
        session_id: &str,
        title: Option<String>,
        start_event_seq: i64,
        end_event_seq: i64,
        created_at: i64,
        id: String,
    ) -> Result<QaCheckpoint> {
        let seq = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM checkpoints WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch checkpoint seq: {e}")))?;

        sqlx::query(
            "INSERT INTO checkpoints (id, session_id, seq, title, start_event_seq, end_event_seq, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(seq)
        .bind(&title)
        .bind(start_event_seq)
        .bind(end_event_seq)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert checkpoint: {e}")))?;

        Ok(QaCheckpoint {
            id,
            session_id: session_id.to_string(),
            seq,
            title,
            start_event_seq,
            end_event_seq,
            created_at,
        })
    }

    pub async fn list_checkpoints(&self, session_id: &str) -> Result<Vec<QaCheckpoint>> {
        let checkpoints = sqlx::query_as::<_, QaCheckpointEntity>(
            "SELECT id, session_id, seq, title, start_event_seq, end_event_seq, created_at
             FROM checkpoints WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list checkpoints: {e}")))?;

        Ok(checkpoints.into_iter().map(|checkpoint| checkpoint.into()).collect())
    }

    pub async fn latest_checkpoint(&self, session_id: &str) -> Result<Option<QaCheckpoint>> {
        let checkpoint = sqlx::query_as::<_, QaCheckpointEntity>(
            "SELECT id, session_id, seq, title, start_event_seq, end_event_seq, created_at
             FROM checkpoints WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch latest checkpoint: {e}")))?;

        Ok(checkpoint.map(|entry| entry.into()))
    }

    pub async fn get_checkpoint(&self, checkpoint_id: &str) -> Result<QaCheckpoint> {
        let checkpoint = sqlx::query_as::<_, QaCheckpointEntity>(
            "SELECT id, session_id, seq, title, start_event_seq, end_event_seq, created_at
             FROM checkpoints WHERE id = ?",
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch checkpoint: {e}")))?;

        checkpoint
            .map(|entry| entry.into())
            .ok_or_else(|| AppError::NotFound("Checkpoint not found.".to_string()))
    }

    pub async fn insert_checkpoint_summary(
        &self,
        id: String,
        checkpoint_id: &str,
        summary_text: String,
        entities_json: Option<String>,
        risks_json: Option<String>,
        created_at: i64,
    ) -> Result<QaCheckpointSummary> {
        sqlx::query(
            "INSERT INTO checkpoint_summaries (id, checkpoint_id, summary_text, entities_json, risks_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(checkpoint_id)
        .bind(&summary_text)
        .bind(&entities_json)
        .bind(&risks_json)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert checkpoint summary: {e}")))?;

        Ok(QaCheckpointSummary {
            id,
            checkpoint_id: checkpoint_id.to_string(),
            summary_text,
            entities_json,
            risks_json,
            created_at,
        })
    }

    pub async fn list_checkpoint_summaries(
        &self,
        session_id: &str,
    ) -> Result<Vec<QaCheckpointSummary>> {
        let summaries = sqlx::query_as::<_, QaCheckpointSummaryEntity>(
            "SELECT s.id, s.checkpoint_id, s.summary_text, s.entities_json, s.risks_json, s.created_at
             FROM checkpoint_summaries s
             JOIN checkpoints c ON s.checkpoint_id = c.id
             WHERE c.session_id = ?
             ORDER BY s.created_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list checkpoint summaries: {e}")))?;

        Ok(summaries.into_iter().map(|summary| summary.into()).collect())
    }

    pub async fn get_checkpoint_summary(
        &self,
        checkpoint_id: &str,
    ) -> Result<Option<QaCheckpointSummary>> {
        let summary = sqlx::query_as::<_, QaCheckpointSummaryEntity>(
            "SELECT id, checkpoint_id, summary_text, entities_json, risks_json, created_at
             FROM checkpoint_summaries WHERE checkpoint_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch checkpoint summary: {e}")))?;

        Ok(summary.map(|entry| entry.into()))
    }

    pub async fn insert_test_case(&self, test_case: &QaTestCase) -> Result<()> {
        sqlx::query(
            "INSERT INTO test_cases (id, session_id, checkpoint_id, type, title, steps_json, expected, priority, status, dedup_hash, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&test_case.id)
        .bind(&test_case.session_id)
        .bind(&test_case.checkpoint_id)
        .bind(&test_case.case_type)
        .bind(&test_case.title)
        .bind(&test_case.steps_json)
        .bind(&test_case.expected)
        .bind(&test_case.priority)
        .bind(&test_case.status)
        .bind(&test_case.dedup_hash)
        .bind(test_case.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert test case: {e}")))?;

        Ok(())
    }

    pub async fn list_test_cases(&self, session_id: &str) -> Result<Vec<QaTestCase>> {
        let cases = sqlx::query_as::<_, QaTestCaseEntity>(
            "SELECT id, session_id, checkpoint_id, type, title, steps_json, expected, priority, status, dedup_hash, created_at
             FROM test_cases WHERE session_id = ? ORDER BY created_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list test cases: {e}")))?;

        Ok(cases.into_iter().map(|case| case.into()).collect())
    }

    pub async fn list_test_cases_for_checkpoint(
        &self,
        checkpoint_id: &str,
    ) -> Result<Vec<QaTestCase>> {
        let cases = sqlx::query_as::<_, QaTestCaseEntity>(
            "SELECT id, session_id, checkpoint_id, type, title, steps_json, expected, priority, status, dedup_hash, created_at
             FROM test_cases WHERE checkpoint_id = ? ORDER BY created_at DESC",
        )
        .bind(checkpoint_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list checkpoint test cases: {e}")))?;

        Ok(cases.into_iter().map(|case| case.into()).collect())
    }

    pub async fn insert_llm_run(&self, run: &QaLlmRun) -> Result<()> {
        sqlx::query(
            "INSERT INTO llm_runs (id, scope, scope_id, model, prompt_version, input_digest, input_summary, output_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&run.id)
        .bind(&run.scope)
        .bind(&run.scope_id)
        .bind(&run.model)
        .bind(&run.prompt_version)
        .bind(&run.input_digest)
        .bind(&run.input_summary)
        .bind(&run.output_json)
        .bind(run.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert LLM run: {e}")))?;

        Ok(())
    }

    pub async fn list_llm_runs(&self, session_id: &str) -> Result<Vec<QaLlmRun>> {
        let runs = sqlx::query_as::<_, QaLlmRunEntity>(
            "SELECT lr.id, lr.scope, lr.scope_id, lr.model, lr.prompt_version, lr.input_digest, lr.input_summary, lr.output_json, lr.created_at
             FROM llm_runs lr
             JOIN checkpoints c ON lr.scope_id = c.id
             WHERE c.session_id = ?
             ORDER BY lr.created_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list LLM runs: {e}")))?;

        Ok(runs.into_iter().map(|run| run.into()).collect())
    }
}

fn db_path_to_url(db_path: &Path) -> Result<String> {
    let db_path_str = db_path.to_str().ok_or_else(|| {
        AppError::DatabaseError("QA database path is not valid UTF-8".to_string())
    })?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

#[derive(sqlx::FromRow)]
struct QaCheckpointEntity {
    id: String,
    session_id: String,
    seq: i64,
    title: Option<String>,
    start_event_seq: i64,
    end_event_seq: i64,
    created_at: i64,
}

impl From<QaCheckpointEntity> for QaCheckpoint {
    fn from(entity: QaCheckpointEntity) -> Self {
        Self {
            id: entity.id,
            session_id: entity.session_id,
            seq: entity.seq,
            title: entity.title,
            start_event_seq: entity.start_event_seq,
            end_event_seq: entity.end_event_seq,
            created_at: entity.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QaCheckpointSummaryEntity {
    id: String,
    checkpoint_id: String,
    summary_text: String,
    entities_json: Option<String>,
    risks_json: Option<String>,
    created_at: i64,
}

impl From<QaCheckpointSummaryEntity> for QaCheckpointSummary {
    fn from(entity: QaCheckpointSummaryEntity) -> Self {
        Self {
            id: entity.id,
            checkpoint_id: entity.checkpoint_id,
            summary_text: entity.summary_text,
            entities_json: entity.entities_json,
            risks_json: entity.risks_json,
            created_at: entity.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QaTestCaseEntity {
    id: String,
    session_id: String,
    checkpoint_id: Option<String>,
    #[sqlx(rename = "type")]
    case_type: String,
    title: String,
    steps_json: String,
    expected: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    dedup_hash: String,
    created_at: i64,
}

impl From<QaTestCaseEntity> for QaTestCase {
    fn from(entity: QaTestCaseEntity) -> Self {
        Self {
            id: entity.id,
            session_id: entity.session_id,
            checkpoint_id: entity.checkpoint_id,
            case_type: entity.case_type,
            title: entity.title,
            steps_json: entity.steps_json,
            expected: entity.expected,
            priority: entity.priority,
            status: entity.status,
            dedup_hash: entity.dedup_hash,
            created_at: entity.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct QaLlmRunEntity {
    id: String,
    scope: String,
    scope_id: String,
    model: String,
    prompt_version: Option<String>,
    input_digest: Option<String>,
    input_summary: Option<String>,
    output_json: String,
    created_at: i64,
}

impl From<QaLlmRunEntity> for QaLlmRun {
    fn from(entity: QaLlmRunEntity) -> Self {
        Self {
            id: entity.id,
            scope: entity.scope,
            scope_id: entity.scope_id,
            model: entity.model,
            prompt_version: entity.prompt_version,
            input_digest: entity.input_digest,
            input_summary: entity.input_summary,
            output_json: entity.output_json,
            created_at: entity.created_at,
        }
    }
}
