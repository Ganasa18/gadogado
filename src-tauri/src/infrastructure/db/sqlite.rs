use crate::domain::error::{AppError, Result};
use crate::domain::prompt::Prompt;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Pool, Sqlite,
};
use std::str::FromStr;

pub struct SqliteRepository {
    pool: Pool<Sqlite>,
}

impl SqliteRepository {
    pub async fn init(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to parse connection string: {}", e))
            })?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to connect: {}", e)))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS prompts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                source_lang TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                result TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create table: {}", e)))?;

        Ok(Self { pool })
    }

    pub async fn save_prompt(&self, prompt: &mut Prompt) -> Result<()> {
        let result = sqlx::query(
            "INSERT INTO prompts (content, source_lang, target_lang, result)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&prompt.content)
        .bind(&prompt.source_lang)
        .bind(&prompt.target_lang)
        .bind(&prompt.result)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to save prompt: {}", e)))?;

        prompt.id = Some(result.last_insert_rowid());
        Ok(())
    }

    pub async fn get_history(&self, limit: i64) -> Result<Vec<Prompt>> {
        sqlx::query_as::<_, PromptEntity>(
            "SELECT id, content, source_lang, target_lang, result, created_at FROM prompts ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch history: {}", e)))
        .map(|entities| entities.into_iter().map(|e| e.into()).collect())
    }
}

// Internal entity for database mapping
#[derive(sqlx::FromRow)]
struct PromptEntity {
    id: i64,
    content: String,
    source_lang: String,
    target_lang: String,
    result: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PromptEntity> for Prompt {
    fn from(e: PromptEntity) -> Self {
        Self {
            id: Some(e.id),
            content: e.content,
            source_lang: e.source_lang,
            target_lang: e.target_lang,
            result: e.result,
            created_at: Some(e.created_at),
        }
    }
}
