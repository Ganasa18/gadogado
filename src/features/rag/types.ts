export type ChatMode = 'rag' | 'free';

export interface RagCollection {
  id: number;
  name: string;
  description: string | null;
  kind: CollectionKind;
  config_json: string;
  created_at: string;
}

export interface RagDocument {
  id: number;
  collection_id: number | null;
  file_name: string;
  file_path: string | null;
  file_type: string;
  language: string;
  total_pages: number;
  quality_score: number | null;      // Overall document quality (0.0-1.0)
  ocr_confidence: number | null;     // Average OCR confidence (0.0-1.0)
  chunk_count: number;               // Total number of chunks
  warning_count: number;             // Number of quality warnings
  created_at: string;
}

export interface RagDocumentChunk {
  id: number;
  doc_id: number;
  content: string;
  page_number: number | null;
  page_offset: number | null;
  chunk_index: number;
  token_count: number | null;
  chunk_quality: number | null;      // Chunk quality score (0.0-1.0)
  content_type: string | null;       // Detected content type
}

export interface RagExcelData {
  id: number;
  doc_id: number;
  row_index: number;
  data_json: string | null;
  val_a: string | null;
  val_b: string | null;
  val_c: number | null;
}

export interface RagCollectionInput {
  name: string;
  description?: string;
}

export interface RagDocumentInput {
  collection_id?: number;
  file_name: string;
  file_path?: string;
  file_type: string;
  language?: string;
  total_pages?: number;
}

export interface RagQueryRequest {
  collection_id: number;
  query: string;
  top_k?: number;
  candidate_k?: number;
  rerank_k?: number;
  /** Enable few-shot conversational examples */
  enable_few_shot?: boolean;
  /** Language for conversational responses ('id', 'en', 'indonesia', 'english', etc.) */
  language?: string;
}

export interface RagQueryResult {
  content: string;
  source_type: string;
  source_id: number;
  score: number | null;
  page_number: number | null;
  page_offset: number | null;
  doc_name: string | null;
}

export interface RagQueryResponse {
  prompt: string;
  results: RagQueryResult[];
}

// Phase 6: Enhanced OCR and chunking
export interface EnhancedOcrRequest {
  file_path: string;
  config?: OcrConfig;
}

export interface OcrPage {
  page_number: number;
  text: string;
}

export interface OcrResult {
  text: string;
  pages?: OcrPage[];
  total_pages: number;
  engine: string;
  preprocessing_mode: string;
  preprocessing_enabled: boolean;
}

export interface SmartChunkingRequest {
  text: string;
  config?: ChunkingConfig;
}

export interface SmartChunk {
  index: number;
  content: string;
  token_count: number;
  quality_score: number | null;
  content_type: string | null;
}

export interface HybridRetrievalOptions {
  top_k?: number;
  use_cache?: boolean;
  optimized?: boolean;
}

export interface HybridRetrievalResponse {
  results: RagQueryResult[];
  cache_hit: boolean;
}

// Phase 7: Validation suite
export interface ValidationCase {
  collection_id: number;
  query: string;
  expected_keywords: string[];
  document_id?: number;
  top_k?: number;
}

export interface ValidationOptions {
  top_k: number;
  use_cache: boolean;
  optimized: boolean;
}

export interface ValidationReport {
  total_cases: number;
  avg_retrieval_precision: number;
  avg_answer_relevance: number;
  avg_chunking_quality: number;
  avg_extraction_accuracy: number;
  avg_latency_ms: number;
  results: ValidationResult[];
}

export interface ValidationResult {
  query: string;
  result_count: number;
  retrieval_precision: number;
  answer_relevance: number;
  chunking_quality: number;
  extraction_accuracy: number;
  latency_ms: number;
  cache_hit: boolean;
  issues: string[];
}

// Phase 8: Analytics
export type AnalyticsEventType = "extraction" | "retrieval" | "chat";

export interface AnalyticsMetadata {
  doc_type?: string;
  collection_id?: number;
  query_hash?: string;
  query_length?: number;
  sources?: number;
  confidence?: number;
  answer_length?: number;
  feedback?: string;
}

export interface AnalyticsEvent {
  event_type: AnalyticsEventType;
  timestamp_ms: number;
  success: boolean;
  duration_ms: number;
  metadata: AnalyticsMetadata;
}

export interface AnalyticsSummary {
  total_events: number;
  extraction_count: number;
  retrieval_count: number;
  chat_count: number;
  avg_extraction_ms: number;
  avg_retrieval_ms: number;
  avg_chat_ms: number;
  success_rate: number;
}

export type ChatMessageType = "user" | "assistant" | "system";

export interface ChatMessage {
  id: string;
  type: ChatMessageType;
  content: string;
  timestamp: Date;
  sources?: RagQueryResult[];
  query?: string;
  /** Telemetry for DB collection queries */
  telemetry?: DbQueryTelemetry;
}

export interface RagWebImportRequest {
  collection_id: number | undefined;
  url: string;
  max_pages?: number;
  max_depth?: number;
}

export interface LogEntry {
  time: string;
  level: string;
  source: string;
  message: string;
}

// Phase 5: Configuration types
export interface ChunkingConfig {
  strategy: string;
  chunk_size: number;
  overlap: number;
  min_quality_score: number;
  respect_boundaries: boolean;
}

export interface RetrievalConfig {
  mode: string;
  top_k: number;
  vector_weight: number;
  keyword_weight: number;
  reranking_enabled: boolean;
  min_relevance_score: number;
  query_expansion_enabled: boolean;
}

export interface EmbeddingConfig {
  model: string;
  dimension: number;
  api_endpoint: string;
  batch_size: number;
  timeout_ms: number;
}

export interface OcrConfig {
  engine: string;
  languages: string;
  preprocessing_enabled: boolean;
  preprocessing_mode: string;
  min_confidence: number;
}

export interface CacheConfig {
  embedding_cache_size: number;
  embedding_cache_ttl_secs: number;
  retrieval_cache_size: number;
  retrieval_cache_ttl_secs: number;
  enabled: boolean;
}

export interface ChatConfig {
  max_history_length: number;
  max_summary_tokens: number;
  self_correction_enabled: boolean;
  show_confidence: boolean;
  show_citations: boolean;
  feedback_enabled: boolean;
}

export interface RagConfig {
  chunking: ChunkingConfig;
  retrieval: RetrievalConfig;
  embedding: EmbeddingConfig;
  ocr: OcrConfig;
  cache: CacheConfig;
  chat: ChatConfig;
}

export interface ConfigValidation {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// Phase 5: Chunk with quality
export interface ChunkWithQuality {
  chunk: RagDocumentChunk;
  quality_score: number;
  has_embedding: boolean;
  token_estimate: number;
}

// Phase 5: Feedback types
export type FeedbackRating = "ThumbsUp" | "ThumbsDown" | "Neutral";

export interface UserFeedback {
  query_id: string;
  query_text: string;
  response_text: string;
  rating: FeedbackRating;
  comment?: string;
  timestamp: number;
  collection_id?: number;
  retrieval_mode?: string;
  chunks_used?: string[];
}

export interface FeedbackStats {
  total_count: number;
  positive_count: number;
  negative_count: number;
  neutral_count: number;
  positive_rate: number;
}

// Phase 5: System stats
export interface SystemStats {
  uptime_secs: number;
  total_operations: number;
  avg_latency_ms: number;
  cache_hit_rate: number;
  embedding_cache_entries: number;
  retrieval_cache_entries: number;
  retrieval_cache_hit_rate: number;
}

// Phase 5: Document quality analysis
export interface DocumentQualityAnalysis {
  document_id: number;
  document_name: string;
  total_chunks: number;
  avg_chunk_quality: number;
  min_chunk_quality: number;
  max_chunk_quality: number;
  low_quality_chunk_count: number;
  avg_chunk_length: number;
  total_tokens: number;
  extraction_quality: "Excellent" | "Good" | "Fair" | "Poor" | "Unknown";
  issues: string[];
}

// ============================================================
// DB CONNECTOR TYPES
// ============================================================

export type CollectionKind = 'Files' | 'Db';

export interface RagCollectionWithKind {
  id: number;
  name: string;
  description: string | null;
  kind: CollectionKind;
  config_json: string;
  created_at: string;
}

export interface DbConnection {
  id: number;
  name: string;
  db_type: 'postgres' | 'sqlite';
  host: string | null;
  port: number | null;
  database_name: string | null;
  username: string | null;
  password_ref: string | null;
  ssl_mode: string;
  is_enabled: boolean;
  config_json: string | null;
  created_at: string;
}

export interface DbConnectionInput {
  name: string;
  db_type: 'postgres' | 'sqlite';
  host?: string;
  port?: number;
  database_name?: string;
  username?: string;
  password: string;
  ssl_mode?: string;
}

export interface DbAllowlistProfile {
  id: number;
  name: string;
  description: string | null;
  rules_json: string;
  created_at: string;
}

export interface TableInfo {
  table_name: string;
  table_schema: string | null;
  row_count: number | null;
}

export interface ColumnInfo {
  column_name: string;
  data_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
  position: number;
}

export interface DbCollectionConfig {
  db_conn_id: number;
  allowlist_profile_id: number;
  selected_tables: string[];
  max_limit: number;
  external_llm_policy: 'allow' | 'block' | 'local_only';
}

export interface DbConnectionConfig {
  profile_id?: number;
  selected_tables: string[];
  selected_columns: { [tableName: string]: string[] };
  default_limit?: number;
  updated_at?: string;
}

export interface DbTestConnectionResult {
  success: boolean;
  message: string;
}

export interface DbCitation {
  table_name: string;
  row_id: string;
  columns: Record<string, unknown>;
}

export interface DbQueryTelemetry {
  rowCount: number;
  latencyMs: number;
  llmRoute: 'local' | 'external' | 'blocked';
  queryPlan?: string;
  /** Actual executed SQL query (for debugging/transparency) */
  executedSql?: string;
  // Few-shot template info (Feature 31)
  templateId?: number;
  templateName?: string;
  templateMatchCount?: number;
  matchedTemplates?: TemplateMatch[];
  /** @deprecated Use modifiedWhereClause instead */
  columnMappings?: Record<string, string>;
  /** Modified WHERE clause generated by LLM */
  modifiedWhereClause?: string;
  /** Enriched/rewritten query from intent enricher */
  enrichedQuery?: string;
  /** Detected intent from enricher */
  detectedIntent?: string;
}

export interface TemplateMatch {
  template_id: number;
  template_name: string;
  score: number;
  reason: string;
  /** Example question for this template (for UI display) */
  example_question?: string;
  /** SQL query pattern (for UI display) */
  query_pattern?: string;
}

/** Simplified conversation history item for API requests */
export interface ConversationHistoryItem {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface DbQueryRequest {
  collection_id: number;
  query: string;
  limit?: number;
  final_k?: number;
  /** Flag to distinguish new query from regeneration */
  is_new_query?: boolean;
  /** Optional conversation history for NL response generation */
  conversation_history?: ConversationHistoryItem[];
}

/** Request to query with a specific template (for regeneration) */
export interface DbQueryWithTemplateRequest {
  collection_id: number;
  query: string;
  template_id: number;
  limit?: number;
  final_k?: number;
  /** Optional conversation history for NL response generation */
  conversation_history?: ConversationHistoryItem[];
}

/** Request to submit template feedback (learning) */
export interface TemplateFeedbackRequest {
  collection_id: number;
  query: string;
  auto_selected_template_id?: number;
  user_selected_template_id: number;
}

/** Response for template feedback submission */
export interface TemplateFeedbackResponse {
  success: boolean;
  message: string;
}

export interface DbQueryResponse {
  answer: string;
  citations: DbCitation[];
  telemetry: DbQueryTelemetry;
  plan?: Record<string, unknown>;
}

export interface RateLimitStatus {
  collection_id: number;
  queries_count: number;
  max_queries_per_hour: number;
  blocked_count: number;
  is_rate_limited: boolean;
  is_cooldown_active: boolean;
  retry_after_seconds: number | null;
  session_started_at: string;
  last_used_at: string;
}

// ============================================================
// QUERY TEMPLATES (Feature 31)
// ============================================================

export type QueryPatternType =
  | 'select_where_in'
  | 'select_where_eq'
  | 'select_with_join'
  | 'aggregate'
  | 'select_where_between'
  | 'select_where_like'
  | 'custom';

export interface QueryTemplate {
  id: number;
  allowlist_profile_id: number;
  name: string;
  description: string | null;
  intent_keywords: string[];  // Parsed from JSON
  example_question: string;
  query_pattern: string;
  pattern_type: QueryPatternType;
  tables_used: string[];      // Parsed from JSON
  priority: number;
  is_enabled: boolean;
  is_pattern_agnostic: boolean;  // Pattern-agnostic mode: ignore table overlap
  created_at: string;
  updated_at: string;
}

export interface QueryTemplateInput {
  allowlist_profile_id: number;
  name: string;
  description?: string;
  intent_keywords: string[];
  example_question: string;
  query_pattern: string;
  pattern_type: QueryPatternType;
  tables_used: string[];
  priority?: number;
  is_enabled?: boolean;
  is_pattern_agnostic?: boolean;  // Pattern-agnostic mode: ignore table overlap
}

export interface QueryTemplateDuplicateInfo {
  kind: "exact" | "name" | "pattern";
  existing_template_id: number;
  existing_template_name: string;
}

export interface QueryTemplateImportPreviewItem {
  key: string;
  original_allowlist_profile_id: number;
  template: QueryTemplateInput;
  issues: string[];
  duplicate: QueryTemplateDuplicateInfo | null;
}

export interface QueryTemplateImportPreview {
  file_path: string;
  target_profile_id: number;
  statement_count: number;
  parsed_count: number;
  ok_count: number;
  warning_count: number;
  error_count: number;
  duplicate_count: number;
  statement_errors: string[];
  items: QueryTemplateImportPreviewItem[];
}

export interface QueryTemplateImportResult {
  requested: number;
  imported: number;
  skipped_duplicates: number;
}
