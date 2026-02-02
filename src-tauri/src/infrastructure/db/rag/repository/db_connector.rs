use crate::domain::error::{AppError, Result};
use crate::domain::rag_entities::{
    DbAllowlistProfile, DbConnection, DbConnectionInput, QueryTemplate, QueryTemplateInput,
    QueryTemplateDuplicateInfo, QueryTemplateImportPreview, QueryTemplateImportPreviewItem,
    QueryTemplateImportResult,
};
use crate::infrastructure::db::rag::connection::{
    split_sql_statements, trim_leading_ws_and_comments,
};

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

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

    /// Import query templates from a SQL file.
    ///
    /// Safety: only allows INSERT statements targeting db_query_templates.
    pub async fn import_query_templates_from_sql_file(&self, file_path: &str) -> Result<i64> {
        let sql_script = std::fs::read_to_string(file_path).map_err(|e| {
            AppError::ValidationError(format!(
                "Failed to read SQL file ({}): {}",
                file_path, e
            ))
        })?;

        let statements = split_sql_statements(&sql_script);
        if statements.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to start import transaction: {}", e))
        })?;

        let mut executed: i64 = 0;

        for stmt in statements {
            let sql = trim_leading_ws_and_comments(&stmt).trim();
            if sql.is_empty() {
                continue;
            }

            let upper = sql.to_ascii_uppercase();
            let allowed = upper.starts_with("INSERT INTO DB_QUERY_TEMPLATES")
                || upper.starts_with("INSERT OR IGNORE INTO DB_QUERY_TEMPLATES")
                || upper.starts_with("INSERT OR REPLACE INTO DB_QUERY_TEMPLATES");

            if !allowed {
                return Err(AppError::ValidationError(format!(
                    "Unsupported SQL in import (only INSERT into db_query_templates allowed): {}",
                    sql.chars().take(80).collect::<String>()
                )));
            }

            sqlx::query(sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::DatabaseError(format!("Failed to execute import SQL statement: {}", e))
                })?;

            executed += 1;
        }

        tx.commit().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to commit import transaction: {}", e))
        })?;

        Ok(executed)
    }

    pub async fn preview_import_query_templates_from_sql_file(
        &self,
        file_path: &str,
        target_profile_id: i64,
    ) -> Result<QueryTemplateImportPreview> {
        let sql_script = std::fs::read_to_string(file_path).map_err(|e| {
            AppError::ValidationError(format!(
                "Failed to read SQL file ({}): {}",
                file_path, e
            ))
        })?;

        let statements = split_sql_statements(&sql_script);
        let statement_count = statements.len() as i64;

        // Use a single connection for TEMP table lifetime.
        let mut conn = self.pool.acquire().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to acquire DB connection for preview: {}", e))
        })?;

        // Build a temp table we can safely insert into.
        sqlx::query("DROP TABLE IF EXISTS temp_import_query_templates")
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                AppError::DatabaseError(format!("Failed to reset temp import table: {}", e))
            })?;

        sqlx::query(
            r#"
            CREATE TEMP TABLE temp_import_query_templates (
              allowlist_profile_id INTEGER NOT NULL,
              name TEXT NOT NULL,
              description TEXT,
              intent_keywords TEXT NOT NULL,
              example_question TEXT NOT NULL,
              query_pattern TEXT NOT NULL,
              pattern_type TEXT NOT NULL,
              tables_used TEXT NOT NULL,
              priority INTEGER DEFAULT 0,
              is_enabled INTEGER DEFAULT 1,
              is_pattern_agnostic INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create temp import table: {}", e)))?;

        // Execute only allowed INSERTs, but redirect them to the temp table.
        // If a statement fails, collect it as a preview error and continue.
        let mut statement_errors: Vec<String> = Vec::new();
        for stmt in &statements {
            let sql = trim_leading_ws_and_comments(stmt).trim();
            if sql.is_empty() {
                continue;
            }

            let upper = sql.to_ascii_uppercase();
            let allowed = upper.starts_with("INSERT INTO")
                || upper.starts_with("INSERT OR IGNORE INTO")
                || upper.starts_with("INSERT OR REPLACE INTO");
            if !allowed {
                continue;
            }

            let lower = sql.to_ascii_lowercase();
            let into_pos = lower.find("into");
            if into_pos.is_none() {
                continue;
            }

            let after_into = &lower[into_pos.unwrap()..];
            let table_pos_rel = after_into.find("db_query_templates");
            if table_pos_rel.is_none() {
                continue;
            }

            // Replace the first occurrence after INTO to avoid touching query_pattern payloads.
            let table_pos = into_pos.unwrap() + table_pos_rel.unwrap();
            let table_end = table_pos + "db_query_templates".len();

            let rewritten = format!(
                "{}{}{}",
                &sql[..table_pos],
                "temp_import_query_templates",
                &sql[table_end..]
            );

            // Execute rewritten statement.
            if let Err(e) = sqlx::query(&rewritten).execute(&mut *conn).await {
                let snippet: String = sql.chars().take(140).collect();
                statement_errors.push(format!("{} ... ({})", snippet, e));
            }
        }

        #[derive(sqlx::FromRow)]
        struct TempImportRow {
            rowid: i64,
            allowlist_profile_id: i64,
            name: String,
            description: Option<String>,
            intent_keywords: String,
            example_question: String,
            query_pattern: String,
            pattern_type: String,
            tables_used: String,
            priority: Option<i64>,
            is_enabled: Option<i64>,
            is_pattern_agnostic: Option<i64>,
        }

        let rows: Vec<TempImportRow> = sqlx::query_as(
            r#"
            SELECT
              rowid,
              allowlist_profile_id,
              name,
              description,
              intent_keywords,
              example_question,
              query_pattern,
              pattern_type,
              tables_used,
              priority,
              is_enabled,
              is_pattern_agnostic
            FROM temp_import_query_templates
            ORDER BY rowid ASC
            "#,
        )
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to read preview rows: {}", e)))?;

        let parsed_count = rows.len() as i64;

        // Existing templates for duplicate detection (target profile).
        let existing = self.list_query_templates(Some(target_profile_id)).await?;
        let mut existing_by_name: HashMap<String, &QueryTemplate> = HashMap::new();
        let mut existing_by_pattern: HashMap<String, &QueryTemplate> = HashMap::new();
        let mut existing_by_exact: HashMap<(String, String), &QueryTemplate> = HashMap::new();
        for t in &existing {
            let name_n = normalize_text(&t.name);
            let pattern_n = normalize_text(&t.query_pattern);
            existing_by_name.insert(name_n.clone(), t);
            existing_by_pattern.insert(pattern_n.clone(), t);
            existing_by_exact.insert((name_n, pattern_n), t);
        }

        // Detect duplicates inside the import file itself.
        let mut seen_exact: HashMap<(String, String), i64> = HashMap::new();
        let mut seen_name: HashMap<String, i64> = HashMap::new();

        let mut items: Vec<QueryTemplateImportPreviewItem> = Vec::new();
        let mut ok_count: i64 = 0;
        let mut warning_count: i64 = 0;
        let mut error_count: i64 = 0;
        let mut duplicate_count: i64 = 0;

        for r in rows {
            let mut issues: Vec<String> = Vec::new();

            let pattern_type = r.pattern_type.trim().to_string();
            if !is_allowed_pattern_type(&pattern_type) {
                issues.push(format!("ERROR: Unsupported pattern_type: {}", pattern_type));
            }

            if r.name.trim().is_empty() {
                issues.push("ERROR: Missing name".to_string());
            }
            if r.example_question.trim().is_empty() {
                issues.push("ERROR: Missing example_question".to_string());
            }
            if r.query_pattern.trim().is_empty() {
                issues.push("ERROR: Missing query_pattern".to_string());
            }

            let intent_keywords: Vec<String> = match serde_json::from_str(&r.intent_keywords) {
                Ok(v) => v,
                Err(e) => {
                    issues.push(format!("ERROR: intent_keywords JSON invalid: {}", e));
                    Vec::new()
                }
            };

            let tables_used: Vec<String> = match serde_json::from_str(&r.tables_used) {
                Ok(v) => v,
                Err(e) => {
                    issues.push(format!("ERROR: tables_used JSON invalid: {}", e));
                    Vec::new()
                }
            };

            if intent_keywords.is_empty() {
                issues.push("WARN: intent_keywords empty".to_string());
            }

            let original_profile_id = r.allowlist_profile_id;
            if original_profile_id != target_profile_id {
                issues.push(format!(
                    "WARN: allowlist_profile_id mismatch (file={}, target={})",
                    original_profile_id, target_profile_id
                ));
            }

            let name_n = normalize_text(&r.name);
            let pattern_n = normalize_text(&r.query_pattern);

            if let Some(first_rowid) = seen_exact.get(&(name_n.clone(), pattern_n.clone())) {
                issues.push(format!(
                    "WARN: duplicate in file (same name+pattern as rowid={})",
                    first_rowid
                ));
            } else {
                seen_exact.insert((name_n.clone(), pattern_n.clone()), r.rowid);
            }

            if let Some(first_rowid) = seen_name.get(&name_n) {
                issues.push(format!(
                    "WARN: duplicate name in file (same name as rowid={})",
                    first_rowid
                ));
            } else {
                seen_name.insert(name_n.clone(), r.rowid);
            }

            let duplicate: Option<QueryTemplateDuplicateInfo> = if let Some(t) =
                existing_by_exact.get(&(name_n.clone(), pattern_n.clone()))
            {
                duplicate_count += 1;
                Some(QueryTemplateDuplicateInfo {
                    kind: "exact".to_string(),
                    existing_template_id: t.id,
                    existing_template_name: t.name.clone(),
                })
            } else if let Some(t) = existing_by_name.get(&name_n) {
                duplicate_count += 1;
                Some(QueryTemplateDuplicateInfo {
                    kind: "name".to_string(),
                    existing_template_id: t.id,
                    existing_template_name: t.name.clone(),
                })
            } else if let Some(t) = existing_by_pattern.get(&pattern_n) {
                duplicate_count += 1;
                Some(QueryTemplateDuplicateInfo {
                    kind: "pattern".to_string(),
                    existing_template_id: t.id,
                    existing_template_name: t.name.clone(),
                })
            } else {
                None
            };

            let key = compute_import_key(
                target_profile_id,
                &r.name,
                &r.query_pattern,
                &pattern_type,
                r.priority.unwrap_or(0) as i32,
            );

            let template = QueryTemplateInput {
                allowlist_profile_id: target_profile_id,
                name: r.name,
                description: r.description,
                intent_keywords,
                example_question: r.example_question,
                query_pattern: r.query_pattern,
                pattern_type,
                tables_used,
                priority: Some(r.priority.unwrap_or(0) as i32),
                is_enabled: Some(r.is_enabled.unwrap_or(1) != 0),
                is_pattern_agnostic: Some(r.is_pattern_agnostic.unwrap_or(0) != 0),
            };

            let has_error = issues.iter().any(|i| i.starts_with("ERROR:"));
            let has_warning = issues.iter().any(|i| i.starts_with("WARN:"));
            if has_error {
                error_count += 1;
            } else if has_warning || duplicate.is_some() {
                warning_count += 1;
            } else {
                ok_count += 1;
            }

            items.push(QueryTemplateImportPreviewItem {
                key,
                original_allowlist_profile_id: original_profile_id,
                template,
                issues,
                duplicate,
            });
        }

        Ok(QueryTemplateImportPreview {
            file_path: file_path.to_string(),
            target_profile_id,
            statement_count,
            parsed_count,
            ok_count,
            warning_count,
            error_count,
            duplicate_count,
            statement_errors,
            items,
        })
    }

    pub async fn import_query_templates_from_preview(
        &self,
        target_profile_id: i64,
        items: Vec<QueryTemplateInput>,
    ) -> Result<QueryTemplateImportResult> {
        if items.is_empty() {
            return Ok(QueryTemplateImportResult {
                requested: 0,
                imported: 0,
                skipped_duplicates: 0,
            });
        }

        // Re-check duplicates at import time for safety.
        let existing = self.list_query_templates(Some(target_profile_id)).await?;
        let mut existing_exact: HashSet<(String, String)> = HashSet::new();
        for t in &existing {
            existing_exact.insert((normalize_text(&t.name), normalize_text(&t.query_pattern)));
        }

        let requested = items.len() as i64;

        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to start import transaction: {}", e))
        })?;

        let mut imported: i64 = 0;
        let mut skipped_duplicates: i64 = 0;

        for mut item in items {
            item.allowlist_profile_id = target_profile_id;

            if !is_allowed_pattern_type(&item.pattern_type) {
                return Err(AppError::ValidationError(format!(
                    "Invalid pattern_type in import selection: {}",
                    item.pattern_type
                )));
            }

            let exact_key = (
                normalize_text(&item.name),
                normalize_text(&item.query_pattern),
            );
            if existing_exact.contains(&exact_key) {
                skipped_duplicates += 1;
                continue;
            }

            // Serialize JSON fields
            let keywords_json = serde_json::to_string(&item.intent_keywords).map_err(|e| {
                AppError::ValidationError(format!("Invalid intent keywords: {}", e))
            })?;
            let tables_json = serde_json::to_string(&item.tables_used).map_err(|e| {
                AppError::ValidationError(format!("Invalid tables_used: {}", e))
            })?;

            let _result = sqlx::query_as::<_, QueryTemplateEntity>(
                r#"
                INSERT INTO db_query_templates (
                    allowlist_profile_id, name, description, intent_keywords,
                    example_question, query_pattern, pattern_type, tables_used,
                    priority, is_enabled, is_pattern_agnostic
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING *
                "#,
            )
            .bind(item.allowlist_profile_id)
            .bind(&item.name)
            .bind(&item.description)
            .bind(&keywords_json)
            .bind(&item.example_question)
            .bind(&item.query_pattern)
            .bind(&item.pattern_type)
            .bind(&tables_json)
            .bind(item.priority.unwrap_or(0))
            .bind(item.is_enabled.unwrap_or(true) as i64)
            .bind(item.is_pattern_agnostic.unwrap_or(false) as i64)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to import template: {}", e)))?;

            existing_exact.insert(exact_key);
            imported += 1;
        }

        tx.commit().await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to commit import transaction: {}", e))
        })?;

        Ok(QueryTemplateImportResult {
            requested,
            imported,
            skipped_duplicates,
        })
    }
}

fn normalize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch.to_ascii_lowercase());
            prev_space = false;
        }
    }
    out
}

fn is_allowed_pattern_type(s: &str) -> bool {
    matches!(
        s,
        "select_where_in" | "select_where_eq" | "select_with_join" | "aggregate" | "custom"
    )
}

fn compute_import_key(
    profile_id: i64,
    name: &str,
    query_pattern: &str,
    pattern_type: &str,
    priority: i32,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    profile_id.hash(&mut hasher);
    normalize_text(name).hash(&mut hasher);
    normalize_text(query_pattern).hash(&mut hasher);
    pattern_type.hash(&mut hasher);
    priority.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
