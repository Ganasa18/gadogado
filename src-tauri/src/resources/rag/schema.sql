-- ============================================================
-- RAG DATABASE SCHEMA
-- ============================================================

-- 1. Tabel Folder/Koleksi (Untuk Context Isolation)
CREATE TABLE IF NOT EXISTS collections (
id INTEGER PRIMARY KEY AUTOINCREMENT,
name TEXT NOT NULL UNIQUE,
description TEXT,
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
created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- 3. Tabel Chunk Teks (Data Tak Terstruktur: PDF, DOCX, TXT, WEB)
CREATE TABLE IF NOT EXISTS document_chunks (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
content TEXT NOT NULL,
page_number INTEGER,
page_offset INTEGER,
chunk_index INTEGER,
token_count INTEGER,
chunk_quality REAL DEFAULT NULL,          -- Chunk quality score (0.0-1.0)
content_type TEXT DEFAULT NULL,           -- Detected content type (header, paragraph, list, table, code)
embedding_api BLOB,
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
