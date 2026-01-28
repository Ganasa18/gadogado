use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

// chrono is used for DB backup timestamp
use chrono;

const RAG_SCHEMA: &str = include_str!("../../../resources/rag/schema.sql");

const RAG_SCHEMA_VERSION: i32 = 3;  // Incremented for DB Connector feature
const ENV_RAG_DB_ALLOW_RECREATE: &str = "RAG_DB_ALLOW_RECREATE";

pub async fn init_rag_db(db_path: &Path) -> Result<(), String> {
    // NOTE:
    // - We use PRAGMA user_version for schema versioning.
    // - Recreate (drop) is only allowed when ENV_RAG_DB_ALLOW_RECREATE is set.
    //   This prevents accidental data loss once the DB becomes valuable.

    let current_version = get_user_version(db_path).await.unwrap_or(0);

    // If schema mismatches and recreate is explicitly allowed, rebuild the DB.
    // This is intended for early dev when the DB can be safely rebuilt.
    if current_version != 0 && current_version != RAG_SCHEMA_VERSION && current_version < RAG_SCHEMA_VERSION && allow_recreate() {
        recreate_db(db_path).await?;
        return Ok(());
    }

    // Default behavior:
    // - Create DB if missing
    // - Apply schema additively (CREATE IF NOT EXISTS + ensure_column)
    // - Set user_version to the target version
    let pool = connect_pool(db_path).await?;

    // If DB is newer than this app expects, fail fast (safety).
    let effective_version = read_user_version_from_pool(&pool).await?;
    if effective_version > RAG_SCHEMA_VERSION {
        return Err(format!(
            "RAG database schema too new: db user_version={} > app supported_version={}",
            effective_version, RAG_SCHEMA_VERSION
        ));
    }

    apply_schema(&pool).await?;

    // Backfill FTS index if needed (safe no-op when already synced)
    backfill_chunks_fts(&pool).await?;

    set_user_version(&pool, RAG_SCHEMA_VERSION).await?;

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
    ensure_fts5_available(pool).await?;

    // IMPORTANT: Ensure columns exist BEFORE applying schema statements that depend on them.
    // This is needed because indexes in schema.sql reference these columns.
    // For existing DBs, the table exists but may lack new columns from migrations.

    // Check if collections table exists before trying to add columns
    let collections_exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='collections'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check collections table existence: {e}"))?;

    if collections_exists.is_some() {
        // DB Connector feature migration (v2 -> v3) - must happen before schema applies indexes
        ensure_column(pool, "collections", "kind", "TEXT NOT NULL DEFAULT 'files'").await?;
        ensure_column(pool, "collections", "config_json", "TEXT NOT NULL DEFAULT '{}'").await?;
    }

    // Now apply all schema statements (including CREATE INDEX on collections.kind)
    let statements = split_sql_statements(RAG_SCHEMA);
    for stmt in statements {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to apply RAG schema statement: {e}"))?;
    }

    // Additive upgrades for existing DBs (columns without dependent indexes).
    ensure_column(pool, "documents", "meta_json", "TEXT NOT NULL DEFAULT '{}'" ).await?;
    ensure_column(pool, "document_chunks", "meta_json", "TEXT NOT NULL DEFAULT '{}'" ).await?;
    ensure_column(pool, "document_chunks", "content_hash", "TEXT").await?;
    ensure_column(pool, "document_chunks", "page_offset", "INTEGER").await?;

    // Rate limiter feature migration - add blocked_count column
    ensure_column(pool, "db_query_sessions", "blocked_count", "INTEGER NOT NULL DEFAULT 0").await?;

    // Initialize default allowlist profile for DB connections
    init_default_allowlist_profile(pool).await?;

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

async fn connect_pool(db_path: &Path) -> Result<SqlitePool, String> {
    connect_pool_with_create(db_path, true).await
}

async fn connect_pool_with_create(db_path: &Path, create_if_missing: bool) -> Result<SqlitePool, String> {
    let db_url = db_path_to_url(db_path)?;
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| format!("Failed to parse RAG database URL: {e}"))?
        .create_if_missing(create_if_missing)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| format!("Failed to connect to RAG database: {e}"))
}

fn allow_recreate() -> bool {
    match std::env::var(ENV_RAG_DB_ALLOW_RECREATE) {
        Ok(val) => {
            let v = val.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "y"
        }
        Err(_) => false,
    }
}

async fn get_user_version(db_path: &Path) -> Result<i32, String> {
    if !db_path.exists() {
        return Ok(0);
    }

    let pool = connect_pool_with_create(db_path, false).await?;
    read_user_version_from_pool(&pool).await
}

async fn read_user_version_from_pool(pool: &SqlitePool) -> Result<i32, String> {
    let version: i32 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to read PRAGMA user_version: {e}"))?;
    Ok(version)
}

async fn set_user_version(pool: &SqlitePool, version: i32) -> Result<(), String> {
    let sql = format!("PRAGMA user_version = {}", version);
    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to set PRAGMA user_version: {e}"))?;
    Ok(())
}

async fn recreate_db(db_path: &Path) -> Result<(), String> {
    // Rename the existing DB to a backup, then create a new one.
    let backup_name = format!(
        "{}.bak.{}",
        db_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("rag.db"),
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );

    let backup_path = db_path
        .parent()
        .ok_or_else(|| "RAG DB path has no parent directory".to_string())?
        .join(backup_name);

    if db_path.exists() {
        std::fs::rename(db_path, &backup_path)
            .map_err(|e| format!("Failed to backup RAG DB (rename): {e}"))?;
    }

    // Clean up WAL files if present (they won't match the new DB).
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| "RAG database path is not valid UTF-8".to_string())?;
    let _ = std::fs::remove_file(format!("{}-wal", db_path_str));
    let _ = std::fs::remove_file(format!("{}-shm", db_path_str));

    let pool = connect_pool(db_path).await?;
    apply_schema(&pool).await?;
    backfill_chunks_fts(&pool).await?;
    init_default_allowlist_profile(&pool).await?;
    set_user_version(&pool, RAG_SCHEMA_VERSION).await?;
    Ok(())
}

async fn ensure_fts5_available(pool: &SqlitePool) -> Result<(), String> {
    // Fail fast if SQLite is built without FTS5.
    // This creates a temporary virtual table; if it fails, FTS5 isn't enabled.
    sqlx::query("CREATE VIRTUAL TABLE IF NOT EXISTS _fts5_probe USING fts5(content);")
        .execute(pool)
        .await
        .map_err(|e| format!("FTS5 is required but not available in SQLite build: {e}"))?;
    let _ = sqlx::query("DROP TABLE IF EXISTS _fts5_probe;")
        .execute(pool)
        .await;
    Ok(())
}

async fn backfill_chunks_fts(pool: &SqlitePool) -> Result<(), String> {
    // If the FTS table exists, backfill any missing rows.
    let exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='document_chunks_fts'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check FTS table existence: {e}"))?;

    if exists.is_none() {
        return Ok(());
    }

    // Ensure every chunk row exists in the FTS index.
    sqlx::query(
        "INSERT INTO document_chunks_fts(rowid, content, doc_id)\n         SELECT dc.id, dc.content, dc.doc_id\n         FROM document_chunks dc\n         WHERE dc.id NOT IN (SELECT rowid FROM document_chunks_fts)",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to backfill document_chunks_fts: {e}"))?;

    Ok(())
}

/// Initialize the default allowlist profile if it doesn't exist.
/// This ensures that profile ID=1 is always available for DB collections.
async fn init_default_allowlist_profile(pool: &SqlitePool) -> Result<(), String> {
    // Check if db_allowlist_profiles table exists
    let table_exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='db_allowlist_profiles'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to check db_allowlist_profiles table existence: {e}"))?;

    if table_exists.is_none() {
        // Table doesn't exist yet, will be created by schema
        return Ok(());
    }

    // Check if any profiles exist
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM db_allowlist_profiles"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to check allowlist profiles: {e}"))?;

    if count > 0 {
        tracing::debug!("Allowlist profiles already exist (count={}), skipping initialization", count);
        return Ok(());
    }

    // Create default profile with security rules
    let default_rules = serde_json::json!({
        "allowed_tables": {},
        "require_filters": {},
        "max_limit": 200,
        "allow_joins": false,
        "deny_keywords": ["password", "token", "secret", "api_key", "credential"],
        "deny_statements": ["INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "PRAGMA", "ATTACH", "GRANT", "REVOKE"]
    });

    sqlx::query(
        "INSERT INTO db_allowlist_profiles (name, description, rules_json)
         VALUES (?, ?, ?)"
    )
    .bind("Default Profile")
    .bind("Default security profile for DB connections. Configure allowed tables before use.")
    .bind(default_rules.to_string())
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create default allowlist profile: {e}"))?;

    tracing::info!("Created default allowlist profile with ID=1");
    Ok(())
}

fn trim_leading_ws_and_comments(s: &str) -> &str {
    // Remove leading whitespace and SQL line comments so we can reliably detect
    // statements that start with comments (common in schema.sql).
    let mut rest = s;

    loop {
        // trim whitespace
        let trimmed = rest.trim_start_matches(|c: char| c == ' ' || c == '\t' || c == '\r' || c == '\n');
        rest = trimmed;

        // strip leading '-- ...\n' comments
        if rest.starts_with("--") {
            if let Some(pos) = rest.find('\n') {
                rest = &rest[pos + 1..];
                continue;
            }
            // comment to EOF
            return "";
        }

        // strip leading /* ... */ comments
        if rest.starts_with("/*") {
            if let Some(end) = rest.find("*/") {
                rest = &rest[end + 2..];
                continue;
            }
            // unterminated block comment
            return "";
        }

        return rest;
    }
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    // Statement splitter that keeps CREATE TRIGGER ... BEGIN ... END; intact.
    // It also ignores semicolons inside quotes and comments.
    let mut out = Vec::new();
    let mut buf = String::new();

    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    let mut word = String::new();
    let mut trigger_depth: i32 = 0;
    let mut in_trigger_stmt = false;

    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if in_line_comment {
            buf.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }

        if in_block_comment {
            buf.push(c);
            if c == '*' && next == Some('/') {
                buf.push('/');
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if !in_single && !in_double {
            if c == '-' && next == Some('-') {
                buf.push(c);
                buf.push('-');
                in_line_comment = true;
                i += 2;
                continue;
            }
            if c == '/' && next == Some('*') {
                buf.push(c);
                buf.push('*');
                in_block_comment = true;
                i += 2;
                continue;
            }
        }

        if c == '\'' && !in_double {
            in_single = !in_single;
            buf.push(c);
            i += 1;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            buf.push(c);
            i += 1;
            continue;
        }

        // Track trigger statement mode.
        // We must ignore leading comments, otherwise statements like:
        // "-- comment\nCREATE TRIGGER ..." won't be detected.
        if !in_single && !in_double && !in_trigger_stmt {
            let trimmed = trim_leading_ws_and_comments(buf.as_str()).to_ascii_lowercase();
            if trimmed.starts_with("create trigger") || trimmed.starts_with("create temp trigger") {
                in_trigger_stmt = true;
            }
        }

        // Tokenize BEGIN/END to keep trigger blocks intact.
        if !in_single && !in_double {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
            } else {
                if !word.is_empty() {
                    let w = word.to_ascii_uppercase();
                    if in_trigger_stmt {
                        if w == "BEGIN" {
                            trigger_depth += 1;
                        } else if w == "END" {
                            if trigger_depth > 0 {
                                trigger_depth -= 1;
                            }
                        }
                    }
                    word.clear();
                }
            }
        }

        if c == ';' && !in_single && !in_double {
            // Split on semicolon only when not inside a trigger BEGIN..END block.
            if !in_trigger_stmt || trigger_depth == 0 {
                buf.push(c);
                out.push(buf.clone());
                buf.clear();
                word.clear();
                in_trigger_stmt = false;
                trigger_depth = 0;
                i += 1;
                continue;
            }
        }

        buf.push(c);
        i += 1;
    }

    if !buf.trim().is_empty() {
        out.push(buf);
    }

    out
}
