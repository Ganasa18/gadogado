use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    DbAllowlistProfile, DbConnection, DbConnectionInput, QueryTemplate, QueryTemplateInput,
};

use super::entities::{DbAllowlistProfileEntity, DbConnectionEntity, QueryTemplateEntity};
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

        if name.is_some() {
            query_parts.push("name = ?".to_string());
        }
        if description.is_some() {
            query_parts.push("description = ?".to_string());
        }
        if rules_json.is_some() {
            query_parts.push("rules_json = ?".to_string());
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

    // ============================================================
    // QUERY TEMPLATE CRUD (Feature 31: Few-Shot Learning)
    // ============================================================

    /// List query templates for a profile (or all if profile_id is None)
    pub async fn list_query_templates(
        &self,
        allowlist_profile_id: Option<i64>,
    ) -> Result<Vec<QueryTemplate>> {
        let templates = if let Some(profile_id) = allowlist_profile_id {
            sqlx::query_as::<_, QueryTemplateEntity>(
                "SELECT * FROM db_query_templates
             WHERE allowlist_profile_id = ?
             ORDER BY priority DESC, created_at DESC",
            )
            .bind(profile_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, QueryTemplateEntity>(
                "SELECT * FROM db_query_templates
             ORDER BY priority DESC, created_at DESC",
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to list query templates: {}", e))
        })?;

        Ok(templates.into_iter().map(|t| t.into()).collect())
    }

    /// List query templates in batches with pagination
    /// This is useful for efficient template matching when there are many templates
    pub async fn list_query_templates_batched(
        &self,
        allowlist_profile_id: Option<i64>,
        offset: i64,
        limit: i64,
        pattern_agnostic_first: bool,
    ) -> Result<Vec<QueryTemplate>> {
        let query = if pattern_agnostic_first {
            // Prioritize pattern-agnostic templates first
            if allowlist_profile_id.is_some() {
                "SELECT * FROM db_query_templates
                 WHERE allowlist_profile_id = ? AND is_enabled = 1
                 ORDER BY is_pattern_agnostic DESC, priority DESC
                 LIMIT ? OFFSET ?"
            } else {
                "SELECT * FROM db_query_templates
                 WHERE is_enabled = 1
                 ORDER BY is_pattern_agnostic DESC, priority DESC
                 LIMIT ? OFFSET ?"
            }
        } else {
            // Standard ordering by priority
            if allowlist_profile_id.is_some() {
                "SELECT * FROM db_query_templates
                 WHERE allowlist_profile_id = ? AND is_enabled = 1
                 ORDER BY priority DESC
                 LIMIT ? OFFSET ?"
            } else {
                "SELECT * FROM db_query_templates
                 WHERE is_enabled = 1
                 ORDER BY priority DESC
                 LIMIT ? OFFSET ?"
            }
        };

        let templates = if let Some(profile_id) = allowlist_profile_id {
            sqlx::query_as::<_, QueryTemplateEntity>(query)
                .bind(profile_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, QueryTemplateEntity>(query)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to list query templates (batched): {}", e))
        })?;

        Ok(templates.into_iter().map(|t| t.into()).collect())
    }

    /// Get a single query template by ID
    pub async fn get_query_template(&self, id: i64) -> Result<QueryTemplate> {
        let template = sqlx::query_as::<_, QueryTemplateEntity>(
            "SELECT * FROM db_query_templates WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to fetch query template: {}", e))
        })?;

        match template {
            Some(t) => Ok(t.into()),
            None => Err(AppError::NotFound(format!("Query template not found: {}", id))),
        }
    }

    /// Create a new query template
    pub async fn create_query_template(
        &self,
        input: &QueryTemplateInput,
    ) -> Result<QueryTemplate> {
        // Serialize JSON fields
        let keywords_json = serde_json::to_string(&input.intent_keywords).map_err(|e| {
            AppError::ValidationError(format!("Invalid intent keywords: {}", e))
        })?;
        let tables_json = serde_json::to_string(&input.tables_used).map_err(|e| {
            AppError::ValidationError(format!("Invalid tables_used: {}", e))
        })?;

        let result = sqlx::query_as::<_, QueryTemplateEntity>(
            r#"
        INSERT INTO db_query_templates (
            allowlist_profile_id, name, description, intent_keywords,
            example_question, query_pattern, pattern_type, tables_used,
            priority, is_enabled, is_pattern_agnostic
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
        )
        .bind(input.allowlist_profile_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&keywords_json)
        .bind(&input.example_question)
        .bind(&input.query_pattern)
        .bind(&input.pattern_type)
        .bind(&tables_json)
        .bind(input.priority.unwrap_or(0))
        .bind(input.is_enabled.unwrap_or(true) as i64)
        .bind(input.is_pattern_agnostic.unwrap_or(false) as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to create query template: {}", e))
        })?;

        Ok(result.into())
    }

    /// Update an existing query template
    pub async fn update_query_template(
        &self,
        id: i64,
        input: &QueryTemplateInput,
    ) -> Result<QueryTemplate> {
        let keywords_json = serde_json::to_string(&input.intent_keywords).map_err(|e| {
            AppError::ValidationError(format!("Invalid intent keywords: {}", e))
        })?;
        let tables_json = serde_json::to_string(&input.tables_used).map_err(|e| {
            AppError::ValidationError(format!("Invalid tables_used: {}", e))
        })?;

        let result = sqlx::query_as::<_, QueryTemplateEntity>(
            r#"
        UPDATE db_query_templates SET
            allowlist_profile_id = ?,
            name = ?,
            description = ?,
            intent_keywords = ?,
            example_question = ?,
            query_pattern = ?,
            pattern_type = ?,
            tables_used = ?,
            priority = ?,
            is_enabled = ?,
            is_pattern_agnostic = ?,
            updated_at = datetime('now')
        WHERE id = ?
        RETURNING *
        "#,
        )
        .bind(input.allowlist_profile_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&keywords_json)
        .bind(&input.example_question)
        .bind(&input.query_pattern)
        .bind(&input.pattern_type)
        .bind(&tables_json)
        .bind(input.priority.unwrap_or(0))
        .bind(input.is_enabled.unwrap_or(true) as i64)
        .bind(input.is_pattern_agnostic.unwrap_or(false) as i64)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to update query template: {}", e))
        })?;

        match result {
            Some(t) => Ok(t.into()),
            None => Err(AppError::NotFound(format!("Query template not found: {}", id))),
        }
    }

    /// Delete a query template
    pub async fn delete_query_template(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM db_query_templates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to delete query template: {}", e))
            })?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Query template not found: {}", id)));
        }

        Ok(result.rows_affected())
    }

    /// Toggle template enabled status
    pub async fn toggle_query_template(
        &self,
        id: i64,
        is_enabled: bool,
    ) -> Result<QueryTemplate> {
        let result = sqlx::query_as::<_, QueryTemplateEntity>(
            "UPDATE db_query_templates SET is_enabled = ?, updated_at = datetime('now')
         WHERE id = ? RETURNING *",
        )
        .bind(is_enabled as i64)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to toggle query template: {}", e))
        })?;

        match result {
            Some(t) => Ok(t.into()),
            None => Err(AppError::NotFound(format!("Query template not found: {}", id))),
        }
    }

    /// Record template feedback for learning user preferences
    /// If feedback for this query_hash already exists, update it
    pub async fn record_template_feedback(
        &self,
        query_hash: &str,
        collection_id: i64,
        auto_selected_template_id: Option<i64>,
        user_selected_template_id: i64,
    ) -> Result<()> {
        let is_override = auto_selected_template_id
            .map(|auto_id| auto_id != user_selected_template_id)
            .unwrap_or(false);

        sqlx::query(
            r#"
            INSERT INTO db_query_template_feedback
                (query_hash, collection_id, auto_selected_template_id, user_selected_template_id, is_user_override, feedback_count)
            VALUES (?, ?, ?, ?, ?, 1)
            ON CONFLICT(query_hash, collection_id) DO UPDATE SET
                auto_selected_template_id = excluded.auto_selected_template_id,
                user_selected_template_id = excluded.user_selected_template_id,
                is_user_override = excluded.is_user_override,
                feedback_count = db_query_template_feedback.feedback_count + 1,
                updated_at = datetime('now')
            "#,
        )
        .bind(query_hash)
        .bind(collection_id)
        .bind(auto_selected_template_id)
        .bind(user_selected_template_id)
        .bind(is_override as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to record template feedback: {}", e))
        })?;

        Ok(())
    }

    /// Get user's preferred template for a query (based on past feedback)
    pub async fn get_preferred_template(
        &self,
        query_hash: &str,
        collection_id: i64,
    ) -> Result<Option<i64>> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT user_selected_template_id FROM db_query_template_feedback
             WHERE query_hash = ? AND collection_id = ?",
        )
        .bind(query_hash)
        .bind(collection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(format!("Failed to get preferred template: {}", e))
        })?;

        Ok(result.map(|(id,)| id))
    }
}
