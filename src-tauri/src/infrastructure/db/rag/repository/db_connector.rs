use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    DbAllowlistProfile, DbConnection, DbConnectionInput,
};

use super::entities::{DbAllowlistProfileEntity, DbConnectionEntity};
use super::RagRepository;

impl RagRepository {
    /// Create a new database connection
    pub async fn create_db_connection(
        &self,
        input: &DbConnectionInput,
        password_ref: &str,
    ) -> Result<DbConnection> {
        let result = sqlx::query_as::<_, DbConnectionEntity>(
            r#"
            INSERT INTO db_connections
            (name, db_type, host, port, database_name, username, password_ref, ssl_mode)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&input.name)
        .bind(&input.db_type)
        .bind(&input.host)
        .bind(input.port)
        .bind(&input.database_name)
        .bind(&input.username)
        .bind(password_ref)
        .bind(input.ssl_mode.as_deref().unwrap_or("require"))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create DB connection: {}", e)))?;

        Ok(result.into())
    }

    /// Get database connection by ID
    pub async fn get_db_connection(&self, id: i64) -> Result<DbConnection> {
        let conn = sqlx::query_as::<_, DbConnectionEntity>(
            "SELECT * FROM db_connections WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch DB connection: {}", e)))?;

        match conn {
            Some(conn) => Ok(conn.into()),
            None => Err(AppError::NotFound(format!("DB connection not found: {}", id))),
        }
    }

    /// List all database connections
    pub async fn list_db_connections(&self) -> Result<Vec<DbConnection>> {
        let connections = sqlx::query_as::<_, DbConnectionEntity>(
            "SELECT * FROM db_connections ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list DB connections: {}", e)))?;

        Ok(connections.into_iter().map(|c| c.into()).collect())
    }

    /// Delete database connection
    pub async fn delete_db_connection(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM db_connections WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to delete DB connection: {}", e)))?;

        Ok(result.rows_affected())
    }

    /// Get allowlist profile by ID
    pub async fn get_allowlist_profile(&self, id: i64) -> Result<DbAllowlistProfile> {
        let profile = sqlx::query_as::<_, DbAllowlistProfileEntity>(
            "SELECT * FROM db_allowlist_profiles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to fetch allowlist profile: {}", e))
        })?;

        match profile {
            Some(profile) => Ok(profile.into()),
            None => Err(AppError::NotFound(format!(
                "Allowlist profile not found: {}",
                id
            ))),
        }
    }

    /// List all allowlist profiles
    pub async fn list_allowlist_profiles(&self) -> Result<Vec<DbAllowlistProfile>> {
        let profiles = sqlx::query_as::<_, DbAllowlistProfileEntity>(
            "SELECT * FROM db_allowlist_profiles ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to list allowlist profiles: {}", e))
        })?;

        Ok(profiles.into_iter().map(|p| p.into()).collect())
    }

    /// Create a default allowlist profile for new DB connections
    pub async fn ensure_default_allowlist_profile(&self) -> Result<DbAllowlistProfile> {
        // Try to get the first profile, or create a default one
        let profiles = self.list_allowlist_profiles().await?;
        if !profiles.is_empty() {
            return Ok(profiles[0].clone());
        }

        // Create default profile
        let default_rules = serde_json::json!({
            "allowed_tables": {},
            "require_filters": {},
            "max_limit": 200,
            "allow_joins": false,
            "deny_keywords": ["password", "token", "secret", "api_key"],
            "deny_statements": ["INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "PRAGMA", "ATTACH"]
        });

        let result = sqlx::query_as::<_, DbAllowlistProfileEntity>(
            "INSERT INTO db_allowlist_profiles (name, description, rules_json)\n             VALUES (?, ?, ?) RETURNING *",
        )
        .bind("Default Profile")
        .bind("Default security profile for DB connections")
        .bind(default_rules.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to create default allowlist profile: {}", e))
        })?;

        Ok(result.into())
    }

    /// Create a new allowlist profile
    pub async fn create_allowlist_profile(
        &self,
        name: &str,
        description: Option<&str>,
        rules_json: &str,
    ) -> Result<DbAllowlistProfile> {
        let result = sqlx::query_as::<_, DbAllowlistProfileEntity>(
            r#"
            INSERT INTO db_allowlist_profiles (name, description, rules_json)
            VALUES (?, ?, ?)
            RETURNING *
            "#
        )
        .bind(name)
        .bind(description)
        .bind(rules_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to create allowlist profile: {}", e))
        })?;

        Ok(result.into())
    }

    /// Update an existing allowlist profile
    pub async fn update_allowlist_profile(
        &self,
        id: i64,
        name: Option<&str>,
        description: Option<Option<&str>>,
        rules_json: Option<&str>,
    ) -> Result<DbAllowlistProfile> {
        // Build dynamic query based on what's being updated
        let mut query_parts = Vec::new();
        let mut param_index = 1;

        if name.is_some() {
            query_parts.push(format!("name = ?{}", param_index));
            param_index += 1;
        }
        if description.is_some() {
            query_parts.push(format!("description = ?{}", param_index));
            param_index += 1;
        }
        if rules_json.is_some() {
            query_parts.push(format!("rules_json = ?{}", param_index));
            param_index += 1;
        }

        if query_parts.is_empty() {
            return Err(AppError::ValidationError("No fields to update".to_string()));
        }

        let set_clause = query_parts.join(", ");
        let query_str = format!(
            "UPDATE db_allowlist_profiles SET {} WHERE id = ? RETURNING *",
            set_clause
        );

        let mut query = sqlx::query_as::<_, DbAllowlistProfileEntity>(&query_str);

        if let Some(n) = name {
            query = query.bind(n);
        }
        if let Some(d) = description {
            query = query.bind(d);
        }
        if let Some(r) = rules_json {
            query = query.bind(r);
        }
        query = query.bind(id);

        let result = query
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to update allowlist profile: {}", e))
            })?;

        match result {
            Some(profile) => Ok(profile.into()),
            None => Err(AppError::NotFound(format!(
                "Allowlist profile not found: {}",
                id
            ))),
        }
    }

    /// Delete an allowlist profile (protects default profile id=1)
    pub async fn delete_allowlist_profile(&self, id: i64) -> Result<u64> {
        // Prevent deletion of default profile
        if id == 1 {
            return Err(AppError::ValidationError(
                "Cannot delete default profile (id=1)".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM db_allowlist_profiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to delete allowlist profile: {}", e))
            })?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Allowlist profile not found: {}",
                id
            )));
        }

        Ok(result.rows_affected())
    }

    /// Update connection configuration JSON
    pub async fn update_db_connection_config(
        &self,
        id: i64,
        config_json: &str,
    ) -> Result<DbConnection> {
        let result = sqlx::query_as::<_, DbConnectionEntity>(
            "UPDATE db_connections SET config_json = ? WHERE id = ? RETURNING *"
        )
        .bind(config_json)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to update connection config: {}", e))
        })?;

        match result {
            Some(conn) => Ok(conn.into()),
            None => Err(AppError::NotFound(format!("DB connection not found: {}", id))),
        }
    }
}
