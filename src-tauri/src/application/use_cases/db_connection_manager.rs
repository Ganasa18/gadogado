//! Database Connection Manager for SQL-RAG
//!
//! Manages connections to external databases (PostgreSQL/SQLite) for the DB Connector feature.
//! This module handles:
//! - Connection pooling with configurable timeouts
//! - SSL mode support for PostgreSQL
//! - Health check operations
//! - Password resolution from environment variables
//!
//! Security considerations:
//! - Only SELECT queries are allowed
//! - Credentials are resolved from env vars, never stored in plaintext
//! - All connections use read-only access where possible

use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{DbConnection, DbConnectionInput, TestConnectionResult};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Column, Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Configuration for the connection manager
#[derive(Debug, Clone)]
pub struct DbConnectionConfig {
    /// Maximum connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Query timeout in seconds
    pub query_timeout_secs: u64,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for DbConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 5,
            connect_timeout_secs: 10,
            query_timeout_secs: 30,
            idle_timeout_secs: 300,
        }
    }
}

/// Result of executing a query
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub row_count: usize,
}

/// Manages database connections for SQL-RAG queries
pub struct DbConnectionManager {
    /// Active PostgreSQL connection pools keyed by connection ID
    pools: Arc<RwLock<HashMap<i64, Pool<Postgres>>>>,
    /// Configuration for connection pooling
    config: DbConnectionConfig,
}

impl DbConnectionManager {
    /// Create a new connection manager with default configuration
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            config: DbConnectionConfig::default(),
        }
    }

    /// Create a new connection manager with custom configuration
    pub fn with_config(config: DbConnectionConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Resolve password from environment variable reference
    /// Format: "env:DB_PASSWORD_MYDB" -> reads from DB_PASSWORD_MYDB env var
    fn resolve_password(password_ref: &str) -> Result<String> {
        if password_ref.starts_with("env:") {
            let env_key = &password_ref[4..];
            std::env::var(env_key).map_err(|_| {
                AppError::ValidationError(format!(
                    "Environment variable '{}' not found for password",
                    env_key
                ))
            })
        } else if password_ref.starts_with("keychain:") {
            // TODO: Implement keychain integration
            Err(AppError::ValidationError(
                "Keychain password storage not yet implemented".to_string(),
            ))
        } else {
            // Direct password (for testing only, not recommended)
            Ok(password_ref.to_string())
        }
    }

    /// Parse SSL mode string to PgSslMode
    fn parse_ssl_mode(ssl_mode: &str) -> PgSslMode {
        match ssl_mode.to_lowercase().as_str() {
            "disable" => PgSslMode::Disable,
            "allow" => PgSslMode::Allow,
            "prefer" => PgSslMode::Prefer,
            "require" => PgSslMode::Require,
            "verify-ca" => PgSslMode::VerifyCa,
            "verify-full" => PgSslMode::VerifyFull,
            _ => PgSslMode::Prefer,
        }
    }

    /// Build PostgreSQL connection options from DbConnection
    fn build_pg_options(&self, conn: &DbConnection, password: &str) -> Result<PgConnectOptions> {
        let host = conn.host.as_deref().ok_or_else(|| {
            AppError::ValidationError("PostgreSQL host is required".to_string())
        })?;

        let port = conn.port.unwrap_or(5432) as u16;

        let database = conn.database_name.as_deref().ok_or_else(|| {
            AppError::ValidationError("PostgreSQL database name is required".to_string())
        })?;

        let username = conn.username.as_deref().ok_or_else(|| {
            AppError::ValidationError("PostgreSQL username is required".to_string())
        })?;

        let ssl_mode = Self::parse_ssl_mode(&conn.ssl_mode);

        let options = PgConnectOptions::new()
            .host(host)
            .port(port)
            .database(database)
            .username(username)
            .password(password)
            .ssl_mode(ssl_mode);

        Ok(options)
    }

    /// Create a connection pool for a PostgreSQL database
    async fn create_pg_pool(&self, conn: &DbConnection) -> Result<Pool<Postgres>> {
        let password_ref = conn.password_ref.as_deref().unwrap_or("");
        let password = Self::resolve_password(password_ref)?;

        let options = self.build_pg_options(conn, &password)?;

        let pool = PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .acquire_timeout(Duration::from_secs(self.config.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(self.config.idle_timeout_secs))
            .connect_with(options)
            .await
            .map_err(|e| {
                error!("Failed to connect to PostgreSQL: {}", e);
                AppError::DatabaseError(format!("Failed to connect to PostgreSQL: {}", e))
            })?;

        info!(
            "Created PostgreSQL connection pool for '{}' (host: {})",
            conn.name,
            conn.host.as_deref().unwrap_or("unknown")
        );

        Ok(pool)
    }

    /// Get or create a connection pool for the specified connection
    pub async fn get_pool(&self, conn: &DbConnection) -> Result<Pool<Postgres>> {
        // Check if pool already exists
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&conn.id) {
                // Verify pool is still healthy
                if pool.is_closed() {
                    warn!(
                        "Connection pool for '{}' is closed, will recreate",
                        conn.name
                    );
                } else {
                    return Ok(pool.clone());
                }
            }
        }

        // Create new pool
        let pool = self.create_pg_pool(conn).await?;

        // Store in cache
        {
            let mut pools = self.pools.write().await;
            pools.insert(conn.id, pool.clone());
        }

        Ok(pool)
    }

    /// Test connection without persisting the pool
    pub async fn test_connection_input(&self, input: &DbConnectionInput) -> TestConnectionResult {
        // Validate required fields
        if input.host.as_deref().unwrap_or("").is_empty() {
            return TestConnectionResult {
                success: false,
                message: "Host is required for PostgreSQL connection".to_string(),
            };
        }

        if input.database_name.as_deref().unwrap_or("").is_empty() {
            return TestConnectionResult {
                success: false,
                message: "Database name is required".to_string(),
            };
        }

        if input.username.as_deref().unwrap_or("").is_empty() {
            return TestConnectionResult {
                success: false,
                message: "Username is required".to_string(),
            };
        }

        if input.password.is_empty() {
            return TestConnectionResult {
                success: false,
                message: "Password is required".to_string(),
            };
        }

        // Build connection options
        let host = input.host.as_deref().unwrap();
        let port = input.port.unwrap_or(5432) as u16;
        let database = input.database_name.as_deref().unwrap();
        let username = input.username.as_deref().unwrap();
        let ssl_mode = Self::parse_ssl_mode(input.ssl_mode.as_deref().unwrap_or("prefer"));

        let options = PgConnectOptions::new()
            .host(host)
            .port(port)
            .database(database)
            .username(username)
            .password(&input.password)
            .ssl_mode(ssl_mode);

        // Try to connect with timeout
        let connect_result = tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_secs),
            PgPoolOptions::new()
                .max_connections(1)
                .connect_with(options),
        )
        .await;

        match connect_result {
            Ok(Ok(pool)) => {
                // Try a simple health check query
                match sqlx::query("SELECT 1 as health_check")
                    .fetch_one(&pool)
                    .await
                {
                    Ok(_) => {
                        // Close the test pool
                        pool.close().await;
                        TestConnectionResult {
                            success: true,
                            message: format!(
                                "Successfully connected to PostgreSQL at {}:{}/{}",
                                host, port, database
                            ),
                        }
                    }
                    Err(e) => TestConnectionResult {
                        success: false,
                        message: format!("Connected but health check failed: {}", e),
                    },
                }
            }
            Ok(Err(e)) => TestConnectionResult {
                success: false,
                message: format!("Connection failed: {}", e),
            },
            Err(_) => TestConnectionResult {
                success: false,
                message: format!(
                    "Connection timed out after {} seconds",
                    self.config.connect_timeout_secs
                ),
            },
        }
    }

    /// Test an existing connection configuration
    pub async fn test_connection(&self, conn: &DbConnection) -> TestConnectionResult {
        if !conn.is_enabled {
            return TestConnectionResult {
                success: false,
                message: "Connection is disabled".to_string(),
            };
        }

        // Resolve password
        let password_ref = conn.password_ref.as_deref().unwrap_or("");
        let password = match Self::resolve_password(password_ref) {
            Ok(p) => p,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: format!("Failed to resolve password: {}", e),
                }
            }
        };

        // Build connection options
        let options = match self.build_pg_options(conn, &password) {
            Ok(o) => o,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: format!("Invalid connection configuration: {}", e),
                }
            }
        };

        // Try to connect with timeout
        let connect_result = tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_secs),
            PgPoolOptions::new()
                .max_connections(1)
                .connect_with(options),
        )
        .await;

        match connect_result {
            Ok(Ok(pool)) => {
                // Try a simple health check query
                match sqlx::query("SELECT 1 as health_check")
                    .fetch_one(&pool)
                    .await
                {
                    Ok(_) => {
                        pool.close().await;
                        TestConnectionResult {
                            success: true,
                            message: format!(
                                "Successfully connected to PostgreSQL at {}:{}/{}",
                                conn.host.as_deref().unwrap_or("unknown"),
                                conn.port.unwrap_or(5432),
                                conn.database_name.as_deref().unwrap_or("unknown")
                            ),
                        }
                    }
                    Err(e) => TestConnectionResult {
                        success: false,
                        message: format!("Connected but health check failed: {}", e),
                    },
                }
            }
            Ok(Err(e)) => TestConnectionResult {
                success: false,
                message: format!("Connection failed: {}", e),
            },
            Err(_) => TestConnectionResult {
                success: false,
                message: format!(
                    "Connection timed out after {} seconds",
                    self.config.connect_timeout_secs
                ),
            },
        }
    }

    /// Execute a SELECT query and return results as JSON
    /// SECURITY: Only SELECT statements are allowed
    pub async fn execute_select(
        &self,
        conn: &DbConnection,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<QueryResult> {
        // Security check: only allow SELECT
        let sql_upper = sql.trim().to_uppercase();
        if !sql_upper.starts_with("SELECT") {
            return Err(AppError::ValidationError(
                "Only SELECT queries are allowed".to_string(),
            ));
        }

        // Block dangerous keywords
        let blocked_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE", "GRANT", "REVOKE",
            "PRAGMA", "ATTACH", "DETACH",
        ];
        for keyword in blocked_keywords {
            if sql_upper.contains(keyword) {
                return Err(AppError::ValidationError(format!(
                    "Query contains forbidden keyword: {}",
                    keyword
                )));
            }
        }

        let pool = self.get_pool(conn).await?;

        // Build parameterized query
        let mut query = sqlx::query(sql);
        for param in params {
            query = match param {
                serde_json::Value::String(s) => query.bind(s.clone()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query.bind(f)
                    } else {
                        query.bind(n.to_string())
                    }
                }
                serde_json::Value::Bool(b) => query.bind(*b),
                serde_json::Value::Null => query.bind(Option::<String>::None),
                _ => query.bind(param.to_string()),
            };
        }

        // Execute with timeout
        let result = tokio::time::timeout(
            Duration::from_secs(self.config.query_timeout_secs),
            query.fetch_all(&pool),
        )
        .await
        .map_err(|_| {
            AppError::DatabaseError(format!(
                "Query timed out after {} seconds",
                self.config.query_timeout_secs
            ))
        })?
        .map_err(|e| AppError::DatabaseError(format!("Query execution failed: {}", e)))?;

        // Convert rows to JSON
        let mut rows_json: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let mut columns: Vec<String> = Vec::new();

        for row in &result {
            if columns.is_empty() {
                columns = row
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect();
            }

            let mut row_map = HashMap::new();
            for (i, column) in row.columns().iter().enumerate() {
                let col_name = column.name().to_string();
                let value: serde_json::Value = Self::extract_column_value(row, i);
                row_map.insert(col_name, value);
            }
            rows_json.push(row_map);
        }

        Ok(QueryResult {
            columns,
            row_count: rows_json.len(),
            rows: rows_json,
        })
    }

    /// Extract a column value from a row as serde_json::Value
    fn extract_column_value(row: &sqlx::postgres::PgRow, index: usize) -> serde_json::Value {
        // Try different types in order of likelihood
        if let Ok(v) = row.try_get::<Option<String>, _>(index) {
            return v
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<i64>, _>(index) {
            return v
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<i32>, _>(index) {
            return v
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<f64>, _>(index) {
            return v
                .and_then(|n| serde_json::Number::from_f64(n))
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<bool>, _>(index) {
            return v
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(index) {
            return v
                .map(|dt| serde_json::Value::String(dt.to_rfc3339()))
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(index) {
            return v
                .map(|d| serde_json::Value::String(d.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        // Default to null for unsupported types
        serde_json::Value::Null
    }

    /// Close all connection pools
    pub async fn close_all(&self) {
        let mut pools = self.pools.write().await;
        for (id, pool) in pools.drain() {
            info!("Closing connection pool for connection ID {}", id);
            pool.close().await;
        }
    }

    /// Close a specific connection pool
    pub async fn close_pool(&self, conn_id: i64) {
        let mut pools = self.pools.write().await;
        if let Some(pool) = pools.remove(&conn_id) {
            info!("Closing connection pool for connection ID {}", conn_id);
            pool.close().await;
        }
    }

    /// Get pool statistics for monitoring
    pub async fn get_pool_stats(&self, conn_id: i64) -> Option<PoolStats> {
        let pools = self.pools.read().await;
        pools.get(&conn_id).map(|pool| PoolStats {
            size: pool.size(),
            num_idle: pool.num_idle(),
            is_closed: pool.is_closed(),
        })
    }
}

/// Pool statistics for monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    pub size: u32,
    pub num_idle: usize,
    pub is_closed: bool,
}

impl Default for DbConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssl_mode() {
        assert!(matches!(
            DbConnectionManager::parse_ssl_mode("disable"),
            PgSslMode::Disable
        ));
        assert!(matches!(
            DbConnectionManager::parse_ssl_mode("require"),
            PgSslMode::Require
        ));
        assert!(matches!(
            DbConnectionManager::parse_ssl_mode("PREFER"),
            PgSslMode::Prefer
        ));
        assert!(matches!(
            DbConnectionManager::parse_ssl_mode("unknown"),
            PgSslMode::Prefer
        ));
    }

    #[test]
    fn test_resolve_password_env() {
        std::env::set_var("TEST_DB_PASSWORD", "secret123");
        let result = DbConnectionManager::resolve_password("env:TEST_DB_PASSWORD");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "secret123");
        std::env::remove_var("TEST_DB_PASSWORD");
    }

    #[test]
    fn test_resolve_password_direct() {
        let result = DbConnectionManager::resolve_password("direct_password");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "direct_password");
    }
}
