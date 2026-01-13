# Local‑Sense RAG — English Master Specification

This document is an **English, easy‑to‑read engineering spec** (not a literal translation) for **Local‑Sense RAG**.

Design goals:

- Clear **goal + exit criteria** for every feature
- Strict **checkpoint discipline** (no skipping)
- Friendly for **local / limited‑context LLMs**
- Can be split later into per‑feature `.md` files without rewriting

---

## 1. System Overview

Local‑Sense RAG is a **desktop‑first, offline‑capable RAG system** designed for developers who:

- Run **local LLMs** (LM Studio, Ollama, etc.)
- Work with **mixed data sources** (documents, spreadsheets, web docs)
- Need **strong context isolation** to avoid hallucination

The system treats the LLM as a **reasoning engine**, not as a database.

---

## 2. Core Design Rules (Non‑Negotiable)

1. **No Global Context**
   Every query MUST be scoped by `collection` or `document`.

2. **Small Chunks, Smart Context**
   Embeddings are small; prompt assembly is dynamic.

3. **Hybrid Retrieval**

   - Text → Vector Search
   - Tables → SQL first, LLM second

4. **One Feature = One Checkpoint**
   A feature is either DONE or NOT DONE.

---

## 3. Database Schema — With Purpose

### 3.1 Collections (Context Boundary)

**Goal**: Logical isolation of knowledge domains.

```sql
CREATE TABLE collections (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

Exit Criteria:

- Collections can be created, listed, deleted
- Deleting a collection removes all related data

---

### 3.2 Documents (Metadata Layer)

**Goal**: Track every knowledge source precisely.

```sql
CREATE TABLE documents (
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
```

Exit Criteria:

- Each file has exactly one document record
- Language metadata is stored

---

### 3.3 Document Chunks (Unstructured Text)

**Goal**: Store LLM‑friendly text units.

```sql
CREATE TABLE document_chunks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  doc_id INTEGER,
  content TEXT NOT NULL,
  page_number INTEGER,
  chunk_index INTEGER,
  token_count INTEGER,
  FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);
```

Exit Criteria:

- No chunk exceeds size limits
- Chunk order is deterministic

---

### 3.4 Vector Index (Semantic Search)

**Goal**: Fast similarity search with minimal RAM.

```sql
CREATE VIRTUAL TABLE vss_chunks USING vss0(
  content_embedding(384)
);
```

Rules:

- `rowid` MUST match `document_chunks.id`
- No raw text stored in vector table

Exit Criteria:

- Vector count equals chunk count

---

### 3.5 Excel / Structured Data

**Goal**: Precise filtering before LLM reasoning.

```sql
CREATE TABLE excel_data (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  doc_id INTEGER,
  row_index INTEGER,
  data_json TEXT,
  val_a TEXT,
  val_b TEXT,
  val_c REAL,
  FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);
```

Exit Criteria:

- SQL queries return exact rows
- LLM only summarizes results

---

## 4. Feature Roadmap (STRICT)

---

### Feature 01 — Project Bootstrap

**Goal**: Running app + ready database.

Exit Criteria:

- App launches without errors
- SQLite + sqlite‑vss loaded
- LLM endpoint reachable

Checkpoint:

- ✅ App boot log clean

---

### Feature 02 — Multi‑Format Ingestion

**Goal**: All supported formats enter the system.

Supported:

- PDF, DOCX, TXT, XLSX, Web

Exit Criteria:

- One valid document per format ingested
- Metadata stored correctly

Checkpoint:

- ✅ Documents visible in DB

---

### Feature 03 — Chunking Engine

**Goal**: Predictable, embedding‑safe chunks.

Rules:

- Size ≤ 500 chars
- Overlap = 50
- Token aware

Exit Criteria:

- No overflow chunks
- token_count populated

Checkpoint:

- ✅ Chunk inspection passes

---

### Feature 04 — Embedding & Indexing

**Goal**: Semantic search works locally.

Exit Criteria:

- Embeddings created in batches
- Vector search returns relevant chunks

Checkpoint:

- ✅ Vector query tested

---

### Feature 05 — Context Isolation

**Goal**: Zero cross‑domain leakage.

Rules:

- No unscoped queries allowed

Exit Criteria:

- Cross‑collection queries blocked

Checkpoint:

- ✅ Isolation tests pass

---

### Feature 06 — Hybrid Retrieval

**Goal**: Correct retrieval path selection.

Decision:

- Numeric / table → SQL
- Text → Vector

Exit Criteria:

- Excel answers are exact
- Text answers are relevant

Checkpoint:

- ✅ Decision logic verified

---

### Feature 07 — Prompt Engine

**Goal**: Deterministic, explainable prompts.

Structure:

1. System rules
2. Retrieved context
3. User question

Exit Criteria:

- Output language respected
- No fabricated sources

Checkpoint:

- ✅ Prompt inspection approved

---

### Feature 08 — Web Crawler

**Goal**: Turn documentation sites into local knowledge.

Exit Criteria:

- Internal links crawled
- HTML cleaned
- Loop protection active

Checkpoint:

- ✅ One full site indexed

---

### Feature 09 — UI / UX

**Goal**: User understands scope and sources.

Exit Criteria:

- User selects context explicitly
- Sources shown with answers

Checkpoint:

- ✅ User can trace answers

---

## 5. Checkpoint Rules (Strict Mode)

- ❌ No feature skipping
- ❌ No checklist without testing
- ❌ No refactor mid‑feature
- ✅ One feature = one LLM session

---

## 6. Recommended Build Order

1. Feature 01–03 (foundation)
2. Feature 04–06 (intelligence)
3. Feature 07–09 (experience)

Freeze this document before implementation.
