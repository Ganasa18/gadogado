-- ============================================================
-- SKEMA DATABASE UNTUK LOCAL-SENSE RAG
-- ============================================================

-- 1. Tabel Folder/Koleksi (Untuk Context Isolation)
CREATE TABLE collections (
id INTEGER PRIMARY KEY AUTOINCREMENT,
name TEXT NOT NULL UNIQUE,
description TEXT,
created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Tabel Master Dokumen
CREATE TABLE documents (
id INTEGER PRIMARY KEY AUTOINCREMENT,
collection_id INTEGER,
file_name TEXT NOT NULL,
file_path TEXT UNIQUE, -- Bisa berupa path lokal atau URL untuk Web
file_type TEXT NOT NULL, -- 'pdf', 'docx', 'xlsx', 'txt', 'web'
total_pages INTEGER DEFAULT 1,
created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- 3. Tabel Chunk Teks (Data Tak Terstruktur: PDF, DOCX, TXT, WEB)
CREATE TABLE document_chunks (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
content TEXT NOT NULL,
page_number INTEGER,
chunk_index INTEGER,
FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- 4. Tabel Vektor (Pencarian Semantik via sqlite-vss)
-- Menggunakan model all-MiniLM-L6-v2 (384 dimensi)
CREATE VIRTUAL TABLE vss_chunks USING vss0(
content_embedding(384)
);

-- 5. Tabel Transaksi (Data Terstruktur: EXCEL)
-- Dibuat fleksibel untuk query presisi (SQL-to-Text)
CREATE TABLE excel_data (
id INTEGER PRIMARY KEY AUTOINCREMENT,
doc_id INTEGER,
row_index INTEGER,
data_json TEXT, -- Menyimpan seluruh baris dalam format JSON untuk fleksibilitas
val_a TEXT, -- Kolom bantu untuk filter cepat
val_b TEXT,
val_c REAL,
FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);
