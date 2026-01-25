-- ============================================================
-- RAG DATABASE SCHEMA
-- ============================================================

-- 1. Tabel Folder/Koleksi (Untuk Context Isolation)
CREATE TABLE IF NOT EXISTS collections (
id INTEGER PRIMARY KEY AUTOINCREMENT,
name TEXT NOT NULL UNIQUE,
description TEXT,

-- 'files' | 'db' - Collection kind for routing
kind TEXT NOT NULL DEFAULT 'files',

-- Configuration depends on kind:
-- files: { }
-- db: { db_conn_id, allowlist_profile_id, selected_tables, default_limit, max_limit, external_llm_policy }
config_json TEXT NOT NULL DEFAULT '{}',

created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Tabel Master Dokumen
CREATE TABLE IF NOT EXISTS documents (
id INTEGER PRIMARY KEY AUTOINCREMENT,
collection_id INTEGER,
file_name TEXT NOT NULL,
file_path TEXT UNIQUE,
file_type TEXT NOT NULL,
language TEXT DEFAULT 'auto',
total_pages INTEGER DEFAULT 1,
quality_score REAL DEFAULT NULL,          -- Overall document quality (0.0-1.0)
ocr_confidence REAL DEFAULT NULL,         -- Average OCR confidence (0.0-1.0)
chunk_count INTEGER DEFAULT 0,            -- Total number of chunks
warning_count INTEGER DEFAULT 0,          -- Number of quality warnings

-- NEW: document-level metadata (JSON)
meta_json TEXT NOT NULL DEFAULT '{}',

created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- 3. Tabel Chunk Teks (Data Tak Terstruktur: PDF, DOCX, TXT, WEB)
CREATE TABLE IF NOT EXISTS document_chunks (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
content TEXT NOT NULL,

-- NEW: optional for dedupe
content_hash TEXT,

page_number INTEGER,
page_offset INTEGER,
chunk_index INTEGER,
token_count INTEGER,
chunk_quality REAL DEFAULT NULL,          -- Chunk quality score (0.0-1.0)
content_type TEXT DEFAULT NULL,           -- Detected content type (header, paragraph, list, table, code)

-- NEW: chunk-scoped metadata (JSON)
meta_json TEXT NOT NULL DEFAULT '{}',

embedding_api BLOB,
created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- 4. Tabel Vektor (Pencarian Semantik via sqlite-vss)
-- Menggunakan model all-MiniLM-L6-v2 (384 dimensi)
-- NOTE: sqlite-vss doesn't support Windows. Uncomment when alternative is available.
-- CREATE VIRTUAL TABLE IF NOT EXISTS vss_chunks USING vss0(
-- content_embedding(384)
-- );

-- 5. Tabel Transaksi (Data Terstruktur: EXCEL)
-- Dibuat fleksibel untuk query presisi (SQL-to-Text)
CREATE TABLE IF NOT EXISTS excel_data (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
row_index INTEGER,
data_json TEXT,
val_a TEXT,
val_b TEXT,
val_c REAL,
FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- 6. Tabel Structured Rows (CSV/XLSX queryable rows)
CREATE TABLE IF NOT EXISTS structured_rows (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  doc_id INTEGER NOT NULL,

  row_index INTEGER NOT NULL,

  -- Common filter columns (fast WHERE)
  category TEXT,
  source TEXT,
  title TEXT,
  created_at_text TEXT,
  created_at DATETIME,

  -- Main text for retrieval/rerank if needed
  content TEXT,

  -- Full row payload
  data_json TEXT NOT NULL,

  created_at_ingested DATETIME DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- 7. Telemetry table for retrieval debugging
CREATE TABLE IF NOT EXISTS retrieval_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,

  collection_id INTEGER NOT NULL,
  query_text TEXT NOT NULL,
  query_hash TEXT,
  intent TEXT NOT NULL,
  retrieval_mode TEXT NOT NULL,

  candidate_count INTEGER DEFAULT 0,
  reranked_count INTEGER DEFAULT 0,
  final_context_count INTEGER DEFAULT 0,

  confidence REAL,
  latency_ms INTEGER,

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- 8. Full-text search (FTS5) for chunk content
CREATE VIRTUAL TABLE IF NOT EXISTS document_chunks_fts USING fts5(
  content,
  doc_id UNINDEXED
);

-- Keep FTS in sync with document_chunks via triggers
CREATE TRIGGER IF NOT EXISTS document_chunks_ai AFTER INSERT ON document_chunks BEGIN
  INSERT INTO document_chunks_fts(rowid, content, doc_id)
  VALUES (new.id, new.content, new.doc_id);
END;

CREATE TRIGGER IF NOT EXISTS document_chunks_ad AFTER DELETE ON document_chunks BEGIN
  INSERT INTO document_chunks_fts(document_chunks_fts, rowid, content, doc_id)
  VALUES('delete', old.id, old.content, old.doc_id);
END;

CREATE TRIGGER IF NOT EXISTS document_chunks_au AFTER UPDATE OF content, doc_id ON document_chunks BEGIN
  INSERT INTO document_chunks_fts(document_chunks_fts, rowid, content, doc_id)
  VALUES('delete', old.id, old.content, old.doc_id);
  INSERT INTO document_chunks_fts(rowid, content, doc_id)
  VALUES (new.id, new.content, new.doc_id);
END;

-- ============================================================
-- PERFORMANCE INDEXES
-- ============================================================

-- Index for collection-based queries (context isolation)
CREATE INDEX IF NOT EXISTS idx_documents_collection_id ON documents(collection_id);

-- Index for document chunk lookups
CREATE INDEX IF NOT EXISTS idx_document_chunks_doc_id ON document_chunks(doc_id);

-- Index for Excel data lookups
CREATE INDEX IF NOT EXISTS idx_excel_data_doc_id ON excel_data(doc_id);

-- Index for Excel column filtering
CREATE INDEX IF NOT EXISTS idx_excel_data_val_a ON excel_data(val_a);
CREATE INDEX IF NOT EXISTS idx_excel_data_val_b ON excel_data(val_b);

-- Composite index for chunk retrieval with page info
CREATE INDEX IF NOT EXISTS idx_document_chunks_doc_page ON document_chunks(doc_id, page_number);

-- Structured rows indexes
CREATE INDEX IF NOT EXISTS idx_structured_rows_doc_id ON structured_rows(doc_id);
CREATE INDEX IF NOT EXISTS idx_structured_rows_category ON structured_rows(category);
CREATE INDEX IF NOT EXISTS idx_structured_rows_source ON structured_rows(source);
CREATE INDEX IF NOT EXISTS idx_structured_rows_doc_category ON structured_rows(doc_id, category);

-- Retrieval telemetry indexes
CREATE INDEX IF NOT EXISTS idx_retrieval_events_collection_time ON retrieval_events(collection_id, created_at);
CREATE INDEX IF NOT EXISTS idx_retrieval_events_intent ON retrieval_events(intent);

-- ============================================================
-- CONVERSATION MEMORY TABLES
-- ============================================================

-- Conversation sessions for multi-turn chat
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER,
    title TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Individual messages in a conversation
CREATE TABLE IF NOT EXISTS conversation_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    sources TEXT, -- JSON array of cited chunk IDs
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- Index for conversation message retrieval
CREATE INDEX IF NOT EXISTS idx_conversation_messages_conv_id ON conversation_messages(conversation_id);

-- ============================================================
-- QUALITY ANALYTICS TABLES
-- ============================================================

-- Document quality warnings for actionable feedback
CREATE TABLE IF NOT EXISTS document_warnings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id INTEGER NOT NULL,
    warning_type TEXT NOT NULL,           -- ocr_low_confidence, table_structure_lost, short_chunk, etc.
    page_number INTEGER,
    chunk_index INTEGER,
    severity TEXT DEFAULT 'warning',      -- info, warning, error
    message TEXT NOT NULL,                -- Human-readable message
    suggestion TEXT,                      -- Actionable suggestion
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Index for warning retrieval by document
CREATE INDEX IF NOT EXISTS idx_document_warnings_doc_id ON document_warnings(doc_id);

-- Collection-level quality metrics (aggregated for analytics)
CREATE TABLE IF NOT EXISTS collection_quality_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER NOT NULL,
    computed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    avg_quality_score REAL,               -- Average document quality
    avg_ocr_confidence REAL,              -- Average OCR confidence
    total_documents INTEGER DEFAULT 0,
    documents_with_warnings INTEGER DEFAULT 0,
    total_chunks INTEGER DEFAULT 0,
    avg_chunk_quality REAL,
    best_reranker TEXT,                   -- Which reranker performed best
    reranker_score REAL,                  -- Best reranker's score
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Index for metrics lookup by collection
CREATE INDEX IF NOT EXISTS idx_collection_quality_metrics_collection_id ON collection_quality_metrics(collection_id);

-- Retrieval gaps for analytics (low-confidence retrievals)
CREATE TABLE IF NOT EXISTS retrieval_gaps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER NOT NULL,
    query_hash TEXT NOT NULL,             -- Hashed query for privacy
    query_length INTEGER,
    result_count INTEGER,
    max_confidence REAL,
    avg_confidence REAL,
    gap_type TEXT,                        -- no_results, low_confidence, partial_match
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Index for gap analysis by collection
CREATE INDEX IF NOT EXISTS idx_retrieval_gaps_collection_id ON retrieval_gaps(collection_id);

-- ============================================================
-- DB CONNECTOR TABLES (SQL-RAG Feature)
-- ============================================================

-- Database Connections for DB Connector Collections
CREATE TABLE IF NOT EXISTS db_connections (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  db_type TEXT NOT NULL,              -- postgres | sqlite
  host TEXT,
  port INTEGER,
  database_name TEXT,
  username TEXT,
  password_ref TEXT,                  -- Reference to secure storage (NOT plaintext password)
  ssl_mode TEXT DEFAULT 'require',
  is_enabled INTEGER NOT NULL DEFAULT 1,
  config_json TEXT,                   -- JSON configuration for selected tables/columns
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- DB Allowlist Profiles (Security Boundary)
-- Defines exactly what AI is allowed to query
CREATE TABLE IF NOT EXISTS db_allowlist_profiles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  rules_json TEXT NOT NULL DEFAULT '{}',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Example rules_json structure:
-- {
--   "allowed_tables": {
--     "users_view": ["id", "username", "status", "created_at"],
--     "orders_view": ["id", "user_id", "total", "created_at"]
--   },
--   "require_filters": {
--     "users_view": ["id", "username"],
--     "orders_view": ["user_id"]
--   },
--   "max_limit": 200,
--   "allow_joins": false,
--   "deny_keywords": ["password", "token", "secret"],
--   "deny_statements": ["INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "PRAGMA", "ATTACH"]
-- }

-- Data Classification Rules (Prevent External Leakage)
CREATE TABLE IF NOT EXISTS data_classification_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  allowlist_profile_id INTEGER NOT NULL,
  match_json TEXT NOT NULL,           -- {table, column, level}
  action TEXT NOT NULL,               -- redact | block_external | block_all
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (allowlist_profile_id) REFERENCES db_allowlist_profiles(id) ON DELETE CASCADE
);

-- SQL-RAG Query Sessions (Rate Limiting Support)
CREATE TABLE IF NOT EXISTS db_query_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER NOT NULL,
  queries_count INTEGER NOT NULL DEFAULT 0,
  started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  last_used_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- SQL-RAG Query Audit Log (OWASP Logging)
-- No sensitive payloads stored
CREATE TABLE IF NOT EXISTS db_query_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER NOT NULL,
  user_query_hash TEXT,
  intent TEXT NOT NULL,               -- sql_rag
  plan_json TEXT NOT NULL,            -- structured query plan
  compiled_sql TEXT NOT NULL,         -- parameterized SQL
  params_json TEXT NOT NULL,          -- redacted if needed
  row_count INTEGER DEFAULT 0,
  latency_ms INTEGER,
  llm_route TEXT NOT NULL,            -- local | external | blocked
  sent_context_chars INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- ============================================================
-- DB CONNECTOR INDEXES
-- ============================================================

-- Index for collection kind filtering
CREATE INDEX IF NOT EXISTS idx_collections_kind ON collections(kind);

-- Index for DB audit queries
CREATE INDEX IF NOT EXISTS idx_db_audit_collection ON db_query_audit(collection_id);
CREATE INDEX IF NOT EXISTS idx_db_audit_time ON db_query_audit(created_at);
