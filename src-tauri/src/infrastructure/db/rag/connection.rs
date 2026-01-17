use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const RAG_SCHEMA: &str = include_str!("../../../resources/rag/schema.sql");

pub async fn init_rag_db(db_path: &Path) -> Result<(), String> {
    let db_url = db_path_to_url(db_path)?;
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| format!("Failed to parse RAG database URL: {e}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| format!("Failed to connect to RAG database: {e}"))?;

    apply_schema(&pool).await?;

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| format!("RAG database health check failed: {e}"))?;

    Ok(())
}

fn db_path_to_url(db_path: &Path) -> Result<String, String> {
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| "RAG database path is not valid UTF-8".to_string())?;
    Ok(format!("sqlite://{}", db_path_str.replace("\\", "/")))
}

async fn apply_schema(pool: &SqlitePool) -> Result<(), String> {
    for statement in RAG_SCHEMA.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to apply RAG schema statement: {e}"))?;
    }
    ensure_column(pool, "document_chunks", "page_offset", "INTEGER").await?;
    Ok(())
}

async fn ensure_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let pragma_query = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&pragma_query)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to inspect {table} schema: {e}"))?;
    let mut exists = false;
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|e| format!("Failed to read {table} schema: {e}"))?;
        if name == column {
            exists = true;
            break;
        }
    }

    if !exists {
        let alter_stmt = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
        sqlx::query(&alter_stmt)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to add {column} column to {table}: {e}"))?;
    }

    Ok(())
}
