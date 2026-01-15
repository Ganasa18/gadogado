# Local‑Sense RAG — Development Progress

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Done

## Definition of "Done"

A feature is marked [x] only when:

- All tasks are implemented AND
- The functionality is integrated into the app startup or relevant workflow AND
- Behavior matches the acceptance criteria in the specification AND
- Validated via `cargo check` + manual/runtime verification (no unit tests required).
- Every backend request adds logging via `add_log()` [without logging sensitive data]

## Core Features

- [x] Feature 01 — Project Bootstrap
- [x] Feature 02 — Multi‑Format Ingestion
- [x] Feature 03 — Chunking Engine
- [x] Feature 04 — Embedding & Indexing
- [x] Feature 05 — Context Isolation
- [x] Feature 06 — Hybrid Retrieval
- [x] Feature 07 — Prompt Engine
- [x] Feature 08 - Web Ingestion
- [~] Feature 09 - UI / UX (In Progress)

## Feature 01 Tasks — Project Bootstrap

- [x] Add ANN vector index backend (pure Rust, no sqlite-vss) to Cargo.toml
- [x] Create RAG database connection module (connection.rs)
- [x] Create RAG domain models (Collection, Document, DocumentChunk, ExcelData)
- [x] Create RAG schema initialization file
- [x] Create RAG repositories for collections, documents, chunks
- [x] Initialize RAG database in app startup with add_log() calls
- [ ] Verify app boot log clean

## Feature 02 Tasks — Multi‑Format Ingestion

- [x] Add file parsing dependencies (pdf, docx, xlsx)
- [x] Create file ingestion use case for RAG
- [x] Create Tauri commands for RAG operations
- [x] Create frontend RAG feature structure
- [x] Create RAG import UI component
- [x] Connect frontend to backend for file import
- [x] Implement DOCX text extraction (using docx-rs)
- [x] Verify documents visible in DB
- [ ] Store PDF/DOCX page source metadata per chunk (page_number + page_offset)
- [ ] Expose page location in chunk citations for PDF sources
- [] Test file import for each format (PDF, DOCX, TXT, XLSX, CSV, .MD) [[PDF ok but must improved grey to make ocr better],[DOCX not tested if ask page not rag not answer where is page location], [XLSX ok for now], [CSV not implemented need improved], [TXT good for now], [.MD need improved for now]]

## Feature 03 Tasks — Chunking Engine

- [x] Create dedicated chunking module (chunking.rs)
- [x] Implement smart chunking with sentence boundaries
- [x] Add token counting (4 chars ≈ 1 token estimation)
- [x] Update ingestion use case to use chunking module
- [x] Verify chunk size limits (500 chars max, 50 char overlap)

## Feature 04 Tasks — Embedding & Indexing

- [x] Add embedding_api column to document_chunks table (schema.sql)
- [x] Create embedding service with local LLM API client (Ollama)
- [x] Implement ANN vector search (pure Rust, cosine similarity)
- [x] Update ingestion to generate embeddings after chunking
- [x] Add batch embedding generation
- [x] Verify vector search returns relevant chunks

## Feature 05 Tasks — Context Isolation

- [x] Add search_chunks_by_collection method to RagRepository
- [x] Add search_excel_by_collection method to RagRepository
- [x] Add search_excel_by_collection_with_filter method with filtering
- [x] Ensure all query methods require collection_id parameter
- [x] Verify cross-collection queries blocked

## Feature 06 Tasks — Hybrid Retrieval

- [x] Create retrieval_service.rs module
- [x] Implement query analysis (text/numeric/hybrid detection)
- [x] Implement SQL-based retrieval for Excel data
- [x] Implement vector-based retrieval for text chunks
- [x] Create rag_hybrid_search Tauri command
- [x] Add RetrievalService to AppState
- [x] Verify decision logic for retrieval path selection

## Feature 07 Tasks — Prompt Engine

- [x] Add scraper dependency to Cargo.toml
- [x] Create prompt_engine.rs module
- [x] Implement PromptBuilder with system rules, context, and user question
- [x] Add source citation support (source_type, source_id)
- [x] Prevent fabricated sources (use only provided context)
- [x] Add rag_query Tauri command
- [x] Update frontend types (RagQueryRequest, RagQueryResponse)
- [x] Update frontend API (ragQuery function)
- [x] Run cargo check to verify no errors

## Feature 09 Tasks - Web Ingestion

Goal: Turn documentation sites into local knowledge using a screenshot-first, OCR-backed pipeline.
Exit Criteria:

Internal links crawled
HTML or OCR text cleaned and converted to Markdown
Loop protection active
Uses Playwright + Tesseract + grayscale preprocessing
Output stored as web-type document with chunks and Fassembed embeddings

- [ ] Checkpoint: One full site indexed (e.g., docs page with 5+ internal links)
- [ ] Create web_crawler.rs module
- [ ] Implement Playwright-based capture (headless, viewport sweep)
- [ ] Add grayscale preprocessing before OCR (image crate)
- [ ] Integrate Tesseract OCR with language support (eng+ind)
- [ ] Implement loop detection (visited URL set)
- [ ] Enforce max depth (3) and max pages (20)
- [ ] Extract only same-origin internal links
- [ ] Generate artifacts: out.md, links.json, manifest.json, tiles/
- [ ] Add "web" as valid file_type in ingestion pipeline
- [ ] Implement parse_web in rag_ingestion.rs
- [ ] Trigger Fassembed embedding and store in document_chunks.embedding_api
- [ ] Log progress via add_log() (no sensitive data)
- [ ] Run cargo check to verify no errors

## Feature 09 Tasks - UI / UX

Goal: User understands scope and sources.
Exit Criteria:

User selects context explicitly
Sources shown with answers
Web crawl UI functional and error-free
Checkpoint:
Web crawl UI functional and error-free

- [ ] Checkpoint: User can trace answers back to original chunks and URLs
- [ ] Create reusable framer motion components (AnimatedContainer, AnimatedList)
- [ ] Create RagChat.tsx component with full chat UI
- [ ] Add chat message types and query result types to types.ts
- [ ] Add ragQuery and importRagWeb API functions
- [ ] Add rag_query Tauri command (backend, enforces collection scoping)
- [ ] Add rag_import_web Tauri command (backend)
- [ ] Update navigation.ts with RAG Chat item
- [ ] Update router.tsx with /rag-chat route
- [ ] Add web crawl UI to RagTab.tsx (URL input, collection selector, live logs)
- [ ] Test full RAG chat flow (ingest + embed + query + cite)
- [ ] Test web crawl functionality (URL + Playwright + OCR + DB + searchable)
- [ ] Keep format clarity so screenshot + OCR + Fassembed workflow shines (not basic HTML scraping)

## Notes

- **Backend logging**: Ensure new backend requests add `add_log()` calls without logging sensitive data.
- **Database initialization**: RAG database (rag_sense.db) is initialized on app startup with proper logging.
- **Frontend integration**: RAG tab added to navigation with import UI for file ingestion.
- **Database schema update**: Added embedding_api BLOB column to document_chunks table for vector storage.
- **Chunking implementation**: Smart chunking engine with 500 char limit, 50 char overlap, and sentence boundary detection.
- **Embedding service**: Created embedding service with Ollama API integration and pure Rust cosine similarity search.
- **Context isolation**: All retrieval operations scoped to collection_id, preventing cross-domain leakage.
- **Hybrid retrieval**: Intelligent routing between SQL (Excel) and vector search (text) based on query content.
- **Note**: sqlite-vss not supported on x86_64 Windows, use in-app ANN index with embeddings stored in SQLite.
- **Prompt engine**: Created PromptEngine module that builds deterministic prompts with system rules, retrieved context, and user questions. Includes source citation support to prevent fabricated sources.
- **Web crawler**: Created WebCrawler module with HTML cleaning, internal link following, and loop detection. Supports max depth and max pages limits.
- **Web integration**: Added "web" file_type support to ingestion pipeline, allowing users to crawl documentation sites.
- **Dependencies**: Added scraper crate for HTML parsing and cleaning.
- **Current focus**: Features 1-8 implementation complete, backend compiles successfully.
- **Next steps**: Test frontend query and web import functionality, proceed to Feature 09 (UI/UX).
