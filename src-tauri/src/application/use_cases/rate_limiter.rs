//! Rate Limiter for SQL-RAG Queries
//!
//! This module implements rate limiting using db_query_sessions table:
//! - Query count tracking per collection
//! - Rate limit enforcement (queries per hour)
//! - Cooldown after repeated blocked queries
//! - Session expiration and cleanup

use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx::SqlitePool;
use tracing::{info, warn, debug};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum queries allowed per hour
    pub max_queries_per_hour: i32,
    /// Number of consecutive blocks before cooldown
    pub cooldown_after_blocks: i32,
    /// Cooldown duration in minutes
    pub block_duration_minutes: i32,
    /// Session expiration in minutes (default 60)
    pub session_expiration_minutes: i32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_queries_per_hour: 60,
            cooldown_after_blocks: 3,
            block_duration_minutes: 5,
            session_expiration_minutes: 60,
        }
    }
}

/// Rate limit check result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Query is allowed
    Allowed,
    /// Rate limit exceeded
    Exceeded { retry_after_seconds: i64 },
    /// Cooldown active after repeated blocks
    CooldownActive { retry_after_seconds: i64 },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }
}

/// Rate limit status for a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub collection_id: i64,
    pub queries_count: i32,
    pub max_queries_per_hour: i32,
    pub blocked_count: i32,
    pub is_rate_limited: bool,
    pub is_cooldown_active: bool,
    pub retry_after_seconds: Option<i64>,
    pub session_started_at: String,
    pub last_used_at: String,
}

/// Session data from database
#[derive(Debug, Clone)]
struct SessionData {
    collection_id: i64,
    queries_count: i32,
    blocked_count: i32,
    started_at: String,
    last_used_at: String,
}

/// Rate limiter for SQL-RAG queries
pub struct RateLimiter {
    db_pool: Arc<SqlitePool>,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with default config
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self {
            db_pool,
            config: RateLimitConfig::default(),
        }
    }

    /// Create a new rate limiter with custom config
    pub fn with_config(db_pool: Arc<SqlitePool>, config: RateLimitConfig) -> Self {
        Self { db_pool, config }
    }

    /// Check if a query is allowed under rate limit
    pub async fn check_rate_limit(
        &self,
        collection_id: i64,
    ) -> Result<RateLimitResult> {
        // Get or create session
        let session = self.get_or_create_session(collection_id).await?;

        // Check if session is expired
        if self.is_session_expired(&session) {
            debug!("Session expired for collection {}, resetting", collection_id);
            self.reset_session(collection_id).await?;
            return Ok(RateLimitResult::Allowed);
        }

        // Check cooldown status
        if session.blocked_count >= self.config.cooldown_after_blocks {
            let cooldown_end = self.calculate_cooldown_end(&session)?;
            if let Some(retry_after) = cooldown_end {
                debug!("Cooldown active for collection {}, retry after {}s", collection_id, retry_after);
                return Ok(RateLimitResult::CooldownActive { retry_after_seconds: retry_after });
            }
        }

        // Check rate limit
        if session.queries_count >= self.config.max_queries_per_hour {
            debug!("Rate limit exceeded for collection {}", collection_id);
            let retry_after = self.calculate_rate_limit_reset(&session)?;
            return Ok(RateLimitResult::Exceeded { retry_after_seconds: retry_after });
        }

        Ok(RateLimitResult::Allowed)
    }

    /// Record a successful query attempt
    pub async fn record_query(&self, collection_id: i64) -> Result<()> {
        debug!("Recording query for collection {}", collection_id);

        sqlx::query(
            r#"
            INSERT INTO db_query_sessions (collection_id, queries_count, blocked_count, started_at, last_used_at)
            VALUES (?, 1, 0, datetime('now'), datetime('now'))
            ON CONFLICT(collection_id) DO UPDATE SET
                queries_count = queries_count + 1,
                last_used_at = datetime('now')
            "#
        )
        .bind(collection_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to record query: {}", e)))?;

        info!("Query recorded for collection {}", collection_id);
        Ok(())
    }

    /// Record a blocked query (for cooldown tracking)
    pub async fn record_block(&self, collection_id: i64) -> Result<()> {
        warn!("Recording blocked query for collection {}", collection_id);

        sqlx::query(
            r#"
            INSERT INTO db_query_sessions (collection_id, queries_count, blocked_count, started_at, last_used_at)
            VALUES (?, 1, 1, datetime('now'), datetime('now'))
            ON CONFLICT(collection_id) DO UPDATE SET
                blocked_count = blocked_count + 1,
                last_used_at = datetime('now')
            "#
        )
        .bind(collection_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to record block: {}", e)))?;

        info!("Blocked query recorded for collection {}", collection_id);
        Ok(())
    }

    /// Reset session for a collection
    pub async fn reset_session(&self, collection_id: i64) -> Result<()> {
        info!("Resetting session for collection {}", collection_id);

        sqlx::query(
            r#"
            UPDATE db_query_sessions
            SET queries_count = 0, blocked_count = 0, started_at = datetime('now'), last_used_at = datetime('now')
            WHERE collection_id = ?
            "#
        )
        .bind(collection_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to reset session: {}", e)))?;

        Ok(())
    }

    /// Get current rate limit status for a collection
    pub async fn get_status(&self, collection_id: i64) -> Result<RateLimitStatus> {
        let session = self.get_or_create_session(collection_id).await?;

        let is_rate_limited = session.queries_count >= self.config.max_queries_per_hour;
        let is_cooldown_active = session.blocked_count >= self.config.cooldown_after_blocks;

        let retry_after = if is_rate_limited {
            Some(self.calculate_rate_limit_reset(&session)?)
        } else if is_cooldown_active {
            self.calculate_cooldown_end(&session)?
        } else {
            None
        };

        Ok(RateLimitStatus {
            collection_id,
            queries_count: session.queries_count,
            max_queries_per_hour: self.config.max_queries_per_hour,
            blocked_count: session.blocked_count,
            is_rate_limited,
            is_cooldown_active,
            retry_after_seconds: retry_after,
            session_started_at: session.started_at,
            last_used_at: session.last_used_at,
        })
    }

    /// Get or create session for a collection
    async fn get_or_create_session(&self, collection_id: i64) -> Result<SessionData> {
        let row: Option<(i32, i32, String, String)> = sqlx::query_as(
            r#"
            SELECT queries_count, blocked_count, started_at, last_used_at
            FROM db_query_sessions
            WHERE collection_id = ?
            "#
        )
        .bind(collection_id)
        .fetch_optional(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch session: {}", e)))?;

        if let Some((queries_count, blocked_count, started_at, last_used_at)) = row {
            Ok(SessionData {
                collection_id,
                queries_count,
                blocked_count,
                started_at,
                last_used_at,
            })
        } else {
            // Create new session
            sqlx::query(
                r#"
                INSERT INTO db_query_sessions (collection_id, queries_count, blocked_count, started_at, last_used_at)
                VALUES (?, 0, 0, datetime('now'), datetime('now'))
                "#
            )
            .bind(collection_id)
            .execute(self.db_pool.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create session: {}", e)))?;

            Ok(SessionData {
                collection_id,
                queries_count: 0,
                blocked_count: 0,
                started_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                last_used_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            })
        }
    }

    /// Check if a session is expired
    fn is_session_expired(&self, session: &SessionData) -> bool {
        if let Ok(last_used) = chrono::DateTime::parse_from_rfc3339(&session.last_used_at) {
            let elapsed = chrono::Utc::now().signed_duration_since(last_used);
            elapsed.num_minutes() > self.config.session_expiration_minutes as i64
        } else {
            // If we can't parse, assume not expired
            false
        }
    }

    /// Calculate seconds until rate limit resets
    fn calculate_rate_limit_reset(&self, session: &SessionData) -> Result<i64> {
        if let Ok(started_at) = chrono::DateTime::parse_from_rfc3339(&session.started_at) {
            let reset_time = started_at + chrono::Duration::hours(1);
            let now = chrono::Utc::now();
            let remaining = (reset_time.with_timezone(&chrono::Utc) - now).num_seconds();
            Ok(remaining.max(0))
        } else {
            // Default to 1 hour if we can't parse
            Ok(3600)
        }
    }

    /// Calculate seconds until cooldown ends
    fn calculate_cooldown_end(&self, session: &SessionData) -> Result<Option<i64>> {
        if session.blocked_count < self.config.cooldown_after_blocks {
            return Ok(None);
        }

        if let Ok(last_used) = chrono::DateTime::parse_from_rfc3339(&session.last_used_at) {
            let cooldown_end = last_used + chrono::Duration::minutes(self.config.block_duration_minutes as i64);
            let now = chrono::Utc::now();
            let remaining = (cooldown_end.with_timezone(&chrono::Utc) - now).num_seconds();
            if remaining > 0 {
                Ok(Some(remaining))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Clean up expired sessions (maintenance function)
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM db_query_sessions
            WHERE datetime(last_used_at) < datetime('now', '-' || ? || ' minutes')
            "#
        )
        .bind(self.config.session_expiration_minutes)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to cleanup sessions: {}", e)))?;

        let deleted = result.rows_affected();
        info!("Cleaned up {} expired sessions", deleted);

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_queries_per_hour, 60);
        assert_eq!(config.cooldown_after_blocks, 3);
        assert_eq!(config.block_duration_minutes, 5);
        assert_eq!(config.session_expiration_minutes, 60);
    }

    #[test]
    fn test_rate_limit_result_is_allowed() {
        assert!(RateLimitResult::Allowed.is_allowed());
        assert!(!RateLimitResult::Exceeded { retry_after_seconds: 60 }.is_allowed());
        assert!(!RateLimitResult::CooldownActive { retry_after_seconds: 60 }.is_allowed());
    }

    #[test]
    fn test_session_expiration() {
        let config = RateLimitConfig {
            session_expiration_minutes: 60,
            ..Default::default()
        };
        let limiter = RateLimiter::with_config(Arc::new(SqlitePool::connect_lazy("sqlite::memory:").unwrap()), config);

        let expired_session = SessionData {
            collection_id: 1,
            queries_count: 10,
            blocked_count: 0,
            started_at: "2024-01-01T00:00:00Z".to_string(),
            last_used_at: "2024-01-01T00:00:00Z".to_string(),
        };

        // Old session should be expired
        assert!(limiter.is_session_expired(&expired_session));
    }
}
