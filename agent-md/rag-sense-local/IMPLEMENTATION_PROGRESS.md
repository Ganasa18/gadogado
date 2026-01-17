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
- [x] Feature 09 - UI / UX
- [x] Feature 10 - Phase 1 Improvements (OCR, Query Expansion, Indexes)
- [x] Feature 11 - Phase 2 Improvements (BM25, Content-Aware Chunking, Reranking)
- [x] Feature 12 - Phase 3 Improvements (Caching, Self-Correction, Health Check)
- [x] Feature 13 - Phase 4 Improvements (Metrics, A/B Testing, Quality Analysis)
- [x] Feature 14 - Phase 5 Improvements (Configuration, Chunk Viewer, Feedback)

## Feature 01 Tasks — Project Bootstrap

- [x] Add ANN vector index backend (pure Rust, no sqlite-vss) to Cargo.toml
- [x] Create RAG database connection module (connection.rs)
- [x] Create RAG domain models (Collection, Document, DocumentChunk, ExcelData)
- [x] Create RAG schema initialization file
- [x] Create RAG repositories for collections, documents, chunks
- [x] Initialize RAG database in app startup with add_log() calls
- [x] Verify app boot log clean

## Feature 02 Tasks — Multi‑Format Ingestion

- [x] Add file parsing dependencies (pdf, docx, xlsx)
- [x] Create file ingestion use case for RAG
- [x] Create Tauri commands for RAG operations
- [x] Create frontend RAG feature structure
- [x] Create RAG import UI component
- [x] Connect frontend to backend for file import
- [x] Implement DOCX text extraction (using docx-rs)
- [x] Verify documents visible in DB
- [x] Store PDF/DOCX page source metadata per chunk (page_number + page_offset)
- [x] Expose page location in chunk citations for PDF sources
- [x] Test file import for each format (PDF, DOCX, TXT, XLSX, CSV, .MD) [[PDF ok but must improved grey to make ocr better],[DOCX not tested if ask page not rag not answer where is page location], [XLSX ok for now], [CSV not implemented need improved], [TXT good for now], [.MD need improved for now]]

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

## Feature 08 Tasks - Web Ingestion

Goal: Turn documentation sites into local knowledge using a screenshot-first, OCR-backed pipeline.
Exit Criteria:

Internal links crawled
HTML or OCR text cleaned and converted to Markdown
Loop protection active
Uses Playwright + Tesseract + grayscale preprocessing
Output stored as web-type document with chunks and Fassembed embeddings

- [x] Checkpoint: One full site indexed (e.g., docs page with 5+ internal links)
- [x] Create web_crawler.rs module
- [x] Implement Playwright-based capture (headless, viewport sweep)
- [x] Add grayscale preprocessing before OCR (image crate)
- [x] Integrate Tesseract OCR with language support (eng+ind)
- [x] Implement loop detection (visited URL set)
- [x] Enforce max depth (3) and max pages (20)
- [x] Extract only same-origin internal links
- [x] Generate artifacts: out.md, links.json, manifest.json, tiles/
- [x] Add "web" as valid file_type in ingestion pipeline
- [x] Implement parse_web in rag_ingestion.rs
- [x] Trigger Fassembed embedding and store in document_chunks.embedding_api
- [x] Log progress via add_log() (no sensitive data)
- [x] Run cargo check to verify no errors

## Feature 09 Tasks - UI / UX

Goal: User understands scope and sources.
Exit Criteria:

User selects context explicitly
Sources shown with answers
Web crawl UI functional and error-free
Checkpoint:
Web crawl UI functional and error-free

- [x] Checkpoint: User can trace answers back to original chunks and URLs
- [x] Create reusable framer motion components (AnimatedContainer, AnimatedList)
- [x] Create RagChat.tsx component with full chat UI
- [x] Add chat message types and query result types to types.ts
- [x] Add ragQuery and importRagWeb API functions
- [x] Add rag_query Tauri command (backend, enforces collection scoping)
- [x] Add rag_import_web Tauri command (backend)
- [x] Update navigation.ts with RAG Chat item
- [x] Update router.tsx with /rag-chat route
- [x] Add web crawl UI to RagTab.tsx (URL input, collection selector, live logs)
- [x] Test full RAG chat flow (ingest + embed + query + cite)
- [x] Test web crawl functionality (URL + Playwright + OCR + DB + searchable)
- [x] Keep format clarity so screenshot + OCR + Fassembed workflow shines (not basic HTML scraping)

## Feature 10 Tasks — Phase 1 Improvements (NEW)

Goal: Improve RAG quality with immediate fixes for OCR, retrieval, and file support.
Exit Criteria:

- Database queries optimized with indexes
- OCR quality improved for low-contrast images
- CSV and Markdown file support added
- Query expansion improves retrieval recall

- [x] Add database indexes for performance (collection_id, doc_id, page_number)
- [x] Add conversation memory tables (conversations, conversation_messages)
- [x] Implement advanced image preprocessing (Otsu thresholding, contrast enhancement)
- [x] Add automatic preprocessing detection based on image contrast analysis
- [x] Implement CSV parsing with quoted field support
- [x] Add Markdown (.md) file support
- [x] Implement query expansion with synonym mapping
- [x] Add Reciprocal Rank Fusion (RRF) for multi-query result combination
- [x] Enhance keyword fallback with word boundary matching
- [x] Run cargo check to verify no errors

## Notes

### Phase 1 Improvements (2026-01-16)

- **Database Indexes**: Added performance indexes on collection_id, doc_id, val_a, val_b columns for faster queries.
- **Conversation Memory**: Added tables for multi-turn chat support (conversations, conversation_messages).
- **Image Preprocessing**: Implemented Otsu's thresholding and histogram stretching for OCR improvement on low-contrast images.
- **CSV Support**: Added full CSV parsing with RFC 4180 compliant quoted field handling.
- **Markdown Support**: Added .md file extension support (uses txt parser).
- **Query Expansion**: Added synonym mapping for common technical terms (function/method, error/bug, etc.).
- **RRF Fusion**: Implemented Reciprocal Rank Fusion to combine results from multiple expanded queries.
- **Enhanced Keyword Search**: Improved fallback search with word boundary matching and expanded term support.

## Feature 11 Tasks — Phase 2 Improvements (NEW)

Goal: Enhance retrieval quality with hybrid search, content-aware chunking, and conversation support.
Exit Criteria:

- BM25 keyword search integrated with vector search
- Content-aware chunking preserves document structure
- LLM-based reranking improves result ordering
- Conversation memory enables multi-turn chat

- [x] Implement BM25 scoring algorithm with IDF and TF normalization
- [x] Add hybrid search combining BM25 + vector with weighted fusion
- [x] Add content-aware chunking (detect headers, lists, code blocks, tables, paragraphs)
- [x] Implement ChunkStrategy enum (FixedSize, ContentAware, Semantic)
- [x] Add chunk quality scoring based on length, punctuation, capitalization
- [x] Create conversation_service.rs for multi-turn chat
- [x] Implement conversation CRUD operations (create, add_message, list, delete)
- [x] Add conversation context building with summarization
- [x] Implement LLM-based reranking prompt builder
- [x] Add reranking score parser and result reordering
- [x] Add conversational prompt builder with history context
- [x] Add truncate_for_rerank helper for token-limited prompts
- [x] Run cargo check to verify no errors

### Phase 2 Improvements (2026-01-16)

- **BM25 Scoring**: Implemented BM25 algorithm with k1=1.2, b=0.75 parameters for keyword relevance scoring.
- **Hybrid Search**: Combined vector similarity + BM25 keyword scores with weighted averaging (0.7 vector, 0.3 BM25).
- **Content-Aware Chunking**: Added detection for headers (#), lists (- or *), code blocks (```), tables (|), and paragraphs.
- **Chunk Quality Scoring**: Scores chunks based on content length, proper punctuation, sentence structure, and capitalization.
- **Conversation Service**: Full conversation memory with message history, context summarization, and entity extraction.
- **LLM Reranking**: Prompt-based reranking that asks LLM to score document relevance 0-10, then blends with original scores.
- **Conversational Prompts**: Builds prompts that include conversation summary, recent messages, and extracted topics.

## Feature 12 Tasks — Phase 3 Improvements (NEW)

Goal: Add caching, context optimization, self-correcting RAG, and health monitoring.
Exit Criteria:

- Embedding cache reduces API calls for repeated queries
- Context optimization removes duplicates and enriches metadata
- Self-correcting RAG verifies and corrects answers
- Health check endpoint monitors system status

- [x] Implement LRU embedding cache with TTL support
- [x] Add cache statistics and management methods
- [x] Implement context optimization (duplicate removal)
- [x] Add adjacent chunk merging for better context
- [x] Add metadata enrichment based on document type
- [x] Implement query_optimized method combining all optimizations
- [x] Add self-correcting RAG verification prompt builder
- [x] Add verification result parser
- [x] Implement correction prompt builder for failed verifications
- [x] Add reflective prompt builder for higher quality answers
- [x] Create rag_chat_with_context Tauri command
- [x] Create rag_build_verification_prompt command
- [x] Create rag_build_correction_prompt command
- [x] Add rag_health_check endpoint
- [x] Add rag_clear_cache endpoint
- [x] Run cargo check to verify no errors

### Phase 3 Improvements (2026-01-16)

- **Embedding Cache**: LRU cache with 1000 entry limit and 1-hour TTL, reduces redundant API calls for repeated queries.
- **Cache Management**: Added cache_stats(), clear_cache(), and cleanup_cache() methods for monitoring and maintenance.
- **Context Optimization**: Three-stage pipeline: duplicate removal (Jaccard similarity), adjacent chunk merging, metadata enrichment.
- **Duplicate Detection**: Uses 0.85 Jaccard similarity threshold to identify near-duplicate content.
- **Chunk Merging**: Automatically merges adjacent chunks from same document/page for more coherent context.
- **Metadata Enrichment**: Updates source_type to reflect document format (pdf_chunk, docx_chunk, markdown_chunk).
- **Self-Correcting RAG**: Verification prompts ask LLM to validate answer against context, returns structured JSON with issues.
- **Correction Flow**: If verification fails, correction prompt includes feedback about unsupported claims and missing citations.
- **Reflective Prompts**: Encourages model to consider sources before answering for higher quality responses.
- **Chat with Context**: New command supports conversation history, message summaries, and optimized retrieval.
- **Health Check**: Monitors database, embedding service, and cache status with timestamps.

## Feature 13 Tasks — Phase 4 Improvements (NEW)

Goal: Add metrics tracking, A/B experiments, retrieval caching, and quality analysis.
Exit Criteria:

- RAG metrics track latency, precision, and quality for all operations
- A/B experiment system allows testing different retrieval configurations
- Retrieval cache reduces latency for repeated queries
- Document quality analysis identifies extraction issues

- [x] Create rag_metrics.rs module with RagOperationMetrics and AggregatedMetrics
- [x] Implement LRU metrics history with percentile calculations (P50, P95, P99)
- [x] Add MetricsTimer helper for timing operations
- [x] Create ExperimentConfig and ExperimentVariant structures
- [x] Implement ExperimentManager with weighted variant assignment
- [x] Add SharedMetricsCollector and SharedExperimentManager thread-safe wrappers
- [x] Add RetrievalCache with LRU eviction and TTL support
- [x] Implement cache statistics (hits, misses, hit rate)
- [x] Add cache invalidation by collection
- [x] Create query_cached method combining cache with retrieval
- [x] Implement DocumentQualityAnalysis with quality scoring
- [x] Add ExtractionQuality enum (Excellent, Good, Fair, Poor)
- [x] Add document quality estimation based on content characteristics
- [x] Create Tauri commands for metrics (get, record, clear)
- [x] Create Tauri commands for experiments (register, list, assign, deactivate)
- [x] Create Tauri commands for cache (stats, clear, invalidate)
- [x] Add rag_analyze_document_quality command
- [x] Add rag_get_system_stats command for overview metrics
- [x] Register metrics_collector and experiment_manager in AppState
- [x] Run cargo check to verify no errors

### Phase 4 Improvements (2026-01-16)

- **RAG Metrics**: Full operation metrics tracking with latency, result counts, relevance scores, and cache hit tracking.
- **Aggregated Metrics**: Calculates P50/P95/P99 latencies, cache hit rates, and per-operation type breakdowns.
- **A/B Experiments**: Register experiments with weighted variants, assign sessions consistently, track experiment results.
- **Experiment Variants**: Each variant can override retrieval mode, top-k, weights, reranking, and custom parameters.
- **Retrieval Cache**: LRU cache with 5-minute TTL for query results, reduces latency for repeated queries.
- **Cache Statistics**: Tracks hits/misses, calculates hit rate, monitors cache size and validity.
- **Document Quality Analysis**: Analyzes chunk quality based on content length, character ratios, and sentence structure.
- **Quality Indicators**: ExtractionQuality enum categorizes documents as Excellent/Good/Fair/Poor based on average chunk quality.
- **System Stats**: Single endpoint for uptime, operation counts, latency, and cache metrics overview.

## Feature 14 Tasks — Phase 5 Improvements (NEW)

Goal: Add configuration management, frontend improvements for document upload, chunk visualization, and enhanced chat UI.
Exit Criteria:

- Configuration system allows runtime adjustment of RAG parameters
- Document upload shows progress and quality indicators
- Chunk visualization enables manual editing and quality review
- Chat UI shows confidence scores, citations, and feedback options

- [x] Create rag_config.rs module with RagConfig and ConfigManager
- [x] Implement config validation and default values
- [x] Add config persistence (save/load from JSON file)
- [x] Create Tauri commands for config management (get, update, reset)
- [x] Create ChunkViewer React component for chunk visualization
- [x] Add chunk quality indicators and filtering UI
- [x] Create DocumentUploader component with progress stages
- [x] Add OCR engine selection and preprocessing toggle (in config)
- [x] Add source highlighting in chat responses (via ChunkViewer)
- [x] Add thumbs up/down feedback collection
- [x] Create rag_submit_feedback Tauri command
- [x] Create FeedbackButtons React component
- [x] Add chunk management commands (delete, update, re-embed)
- [x] Run cargo check to verify no errors

### Phase 5 Improvements (2026-01-16)

- **Configuration System**: Full RagConfig with ChunkingConfig, RetrievalConfig, EmbeddingConfig, OcrConfig, CacheConfig, ChatConfig.
- **Config Validation**: Validates all parameters with errors and warnings, prevents invalid configurations.
- **Config Persistence**: Saves/loads config from JSON file in app data directory.
- **ChunkViewer Component**: Displays chunks with quality scores, allows editing, deletion, and re-embedding.
- **Quality Indicators**: Shows chunk quality as Excellent/Good/Fair/Poor with color-coded badges.
- **DocumentUploader Component**: Drag-and-drop file upload with progress stages tracking.
- **FeedbackButtons Component**: Thumbs up/down with optional comment for negative feedback.
- **Chunk Management API**: Commands for getting chunks with quality, deleting, updating, and re-embedding.
- **Feedback Collection**: Stores user feedback with rating, comment, query context, and timestamps.
- **System Stats**: Enhanced stats endpoint with cache hit rates and operation counts.

## Feature 15 Tasks – Phase 6 Improvements (NEW)

Goal: Extend backend API for OCR preview, smart chunking, and hybrid retrieval options.
Exit Criteria:

- Enhanced OCR endpoint returns text with config-aware preprocessing and language settings
- Smart chunking endpoint exposes strategy-based chunking for arbitrary text
- Hybrid retrieval endpoint supports cache + optimization options

- [x] Add enhanced OCR flow with configurable preprocessing and language support
- [x] Create rag_enhanced_ocr Tauri command
- [x] Add smart chunking Tauri command with strategy mapping
- [x] Add hybrid retrieval Tauri command with cache/optimized options
- [x] Register Phase 6 commands in invoke_handler
- [ ] Run cargo check to verify no errors

### Phase 6 Improvements (2026-01-16)

- **Enhanced OCR API**: Runs OCR on PDFs or images with config-controlled preprocessing and languages.
- **Smart Chunking API**: Exposes content-aware, semantic, and fixed chunking on demand.
- **Hybrid Retrieval API**: Adds cache-aware retrieval with optional context optimization.

## Feature 16 Tasks – Phase 7 Improvements (NEW)

Goal: Add testing and validation utilities for RAG quality checks.
Exit Criteria:

- Validation suite runs queries with expected keywords
- Metrics summarize retrieval precision, relevance, chunking quality, and latency
- Validation exposed via Tauri command

- [x] Create rag_validation module for validation suite and scoring
- [x] Add rag_run_validation_suite Tauri command
- [x] Register validation command in invoke_handler
- [ ] Run cargo check to verify no errors

### Phase 7 Improvements (2026-01-16)

- **Validation Suite**: Runs lightweight quality checks over queries and expected keywords.
- **Validation Metrics**: Aggregates retrieval precision, answer relevance, chunking quality, and latency.

## Feature 17 Tasks – Phase 8 Improvements (NEW)

Goal: Add logging and analytics for extraction, retrieval, and chat operations.
Exit Criteria:

- Analytics logger records extraction, retrieval, and chat events
- Summary and recent analytics can be fetched via Tauri commands
- Analytics includes safe query hashing and duration tracking

- [x] Create rag_analytics module with event logging and summaries
- [x] Add analytics logger to AppState
- [x] Add analytics Tauri commands (summary, recent, clear)
- [x] Log extraction, retrieval, and chat events in RAG commands
- [ ] Run cargo check to verify no errors

### Phase 8 Improvements (2026-01-16)

- **Analytics Logger**: Tracks extraction, retrieval, and chat activity with durations.
- **Analytics Commands**: Exposes summary and recent events for monitoring.
