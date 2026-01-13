use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const QA_SCHEMA: &str = include_str!("../../resources/qa/schema.sql");

pub async fn init_qa_db(db_path: &Path) -> Result<(), String> {
    let db_url = db_path_to_url(db_path)?;
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| format!("Failed to parse QA database URL: {e}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| format!("Failed to connect to QA database: {e}"))?;

    apply_schema(&pool).await?;

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| format!("QA database health check failed: {e}"))?;

    Ok(())
}

fn db_path_to_url(db_path: &Path) -> Result<String, String> {
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| "QA database path is not valid UTF-8".to_string())?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

async fn apply_schema(pool: &SqlitePool) -> Result<(), String> {
    for statement in QA_SCHEMA.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to apply QA schema statement: {e}"))?;
    }
    Ok(())
}
