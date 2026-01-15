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
created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- 3. Tabel Chunk Teks (Data Tak Terstruktur: PDF, DOCX, TXT, WEB)
CREATE TABLE IF NOT EXISTS document_chunks (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
content TEXT NOT NULL,
page_number INTEGER,
chunk_index INTEGER,
token_count INTEGER,
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
