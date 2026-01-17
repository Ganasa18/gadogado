import { invoke } from "@tauri-apps/api/core";
import type {
  RagCollection,
  RagCollectionInput,
  RagDocument,
  RagDocumentChunk,
  RagExcelData,
  RagQueryRequest,
  RagQueryResponse,
  LogEntry,
  RagConfig,
  ConfigValidation,
  ChunkWithQuality,
  UserFeedback,
  FeedbackStats,
  SystemStats,
  DocumentQualityAnalysis,
  ChunkingConfig,
  RetrievalConfig,
  EmbeddingConfig,
  OcrConfig,
  CacheConfig,
  ChatConfig,
} from "./types";

export async function createRagCollection(
  input: RagCollectionInput
): Promise<RagCollection> {
  return await invoke<RagCollection>("rag_create_collection", { input });
}

export async function getRagCollection(id: number): Promise<RagCollection> {
  return await invoke<RagCollection>("rag_get_collection", { id });
}

export async function listRagCollections(limit?: number): Promise<RagCollection[]> {
  return await invoke<RagCollection[]>("rag_list_collections", { limit });
}

export async function deleteRagCollection(id: number): Promise<number> {
  return await invoke<number>("rag_delete_collection", { id });
}

export async function getRagDocument(id: number): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_get_document", { id });
}

export async function listRagDocuments(
  collectionId?: number,
  limit?: number
): Promise<RagDocument[]> {
  return await invoke<RagDocument[]>("rag_list_documents", {
    collectionId,
    limit,
  });
}

export async function deleteRagDocument(id: number): Promise<number> {
  return await invoke<number>("rag_delete_document", { id });
}

export async function importRagFile(
  filePath: string,
  collectionId?: number
): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_import_file", {
    filePath,
    collectionId,
  });
}

export async function listRagChunks(
  docId: number,
  limit?: number
): Promise<RagDocumentChunk[]> {
  return await invoke<RagDocumentChunk[]>("rag_list_chunks", {
    doc_id: docId,
    limit,
  });
}

export async function listRagExcelData(
  docId: number,
  limit?: number
): Promise<RagExcelData[]> {
  return await invoke<RagExcelData[]>("rag_list_excel_data", {
    doc_id: docId,
    limit,
  });
}

export async function ragQuery(
  request: RagQueryRequest
): Promise<RagQueryResponse> {
  return await invoke<RagQueryResponse>("rag_query", { request });
}

export type WebCrawlMode = "html" | "ocr";

export async function importRagWeb(
  url: string,
  collectionId: number | undefined,
  maxPages?: number,
  maxDepth?: number,
  mode: WebCrawlMode = "html"
): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_import_web", {
    request: {
      url,
      collection_id: collectionId,
      max_pages: maxPages,
      max_depth: maxDepth,
      mode,
    },
  });
}

export async function getLogs(): Promise<LogEntry[]> {
  return await invoke<LogEntry[]>("get_logs");
}

// ============================================================
// PHASE 5: Configuration Management
// ============================================================

export async function getRagConfig(): Promise<RagConfig> {
  return await invoke<RagConfig>("rag_get_config");
}

export async function updateRagConfig(config: RagConfig): Promise<ConfigValidation> {
  return await invoke<ConfigValidation>("rag_update_config", { config });
}

export async function updateChunkingConfig(config: ChunkingConfig): Promise<string> {
  return await invoke<string>("rag_update_chunking_config", { config });
}

export async function updateRetrievalConfig(config: RetrievalConfig): Promise<string> {
  return await invoke<string>("rag_update_retrieval_config", { config });
}

export async function updateEmbeddingConfig(config: EmbeddingConfig): Promise<string> {
  return await invoke<string>("rag_update_embedding_config", { config });
}

export async function updateOcrConfig(config: OcrConfig): Promise<string> {
  return await invoke<string>("rag_update_ocr_config", { config });
}

export async function updateCacheConfig(config: CacheConfig): Promise<string> {
  return await invoke<string>("rag_update_cache_config", { config });
}

export async function updateChatConfig(config: ChatConfig): Promise<string> {
  return await invoke<string>("rag_update_chat_config", { config });
}

export async function resetRagConfig(): Promise<RagConfig> {
  return await invoke<RagConfig>("rag_reset_config");
}

export async function validateRagConfig(): Promise<ConfigValidation> {
  return await invoke<ConfigValidation>("rag_validate_config");
}

// ============================================================
// PHASE 5: User Feedback
// ============================================================

export async function submitFeedback(feedback: UserFeedback): Promise<string> {
  return await invoke<string>("rag_submit_feedback", { feedback });
}

export async function getFeedbackStats(): Promise<FeedbackStats> {
  return await invoke<FeedbackStats>("rag_get_feedback_stats");
}

export async function getRecentFeedback(limit?: number): Promise<UserFeedback[]> {
  return await invoke<UserFeedback[]>("rag_get_recent_feedback", { limit });
}

export async function clearFeedback(): Promise<string> {
  return await invoke<string>("rag_clear_feedback");
}

// ============================================================
// PHASE 5: Chunk Management
// ============================================================

export async function getChunksWithQuality(
  documentId: number,
  limit?: number
): Promise<ChunkWithQuality[]> {
  return await invoke<ChunkWithQuality[]>("rag_get_chunks_with_quality", {
    document_id: documentId,
    limit,
  });
}

export async function deleteChunk(chunkId: number): Promise<number> {
  return await invoke<number>("rag_delete_chunk", { chunk_id: chunkId });
}

export async function updateChunkContent(
  chunkId: number,
  newContent: string
): Promise<RagDocumentChunk> {
  return await invoke<RagDocumentChunk>("rag_update_chunk_content", {
    chunk_id: chunkId,
    new_content: newContent,
  });
}

export async function reembedChunk(chunkId: number): Promise<string> {
  return await invoke<string>("rag_reembed_chunk", { chunk_id: chunkId });
}

export async function filterLowQualityChunks(
  documentId: number,
  qualityThreshold: number
): Promise<ChunkWithQuality[]> {
  return await invoke<ChunkWithQuality[]>("rag_filter_low_quality_chunks", {
    document_id: documentId,
    quality_threshold: qualityThreshold,
  });
}

// ============================================================
// PHASE 5: Document Quality & System Stats
// ============================================================

export async function analyzeDocumentQuality(
  documentId: number
): Promise<DocumentQualityAnalysis> {
  return await invoke<DocumentQualityAnalysis>("rag_analyze_document_quality", {
    document_id: documentId,
  });
}

export async function getSystemStats(): Promise<SystemStats> {
  return await invoke<SystemStats>("rag_get_system_stats");
}

// ============================================================
// PHASE 9: Conversation Persistence
// ============================================================

export interface Conversation {
  id: number;
  collection_id: number | null;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface ConversationMessage {
  id: number;
  conversation_id: number;
  role: "user" | "assistant" | "system";
  content: string;
  sources: string | null;
  created_at: string;
}

export async function createConversation(
  collectionId: number | null,
  title?: string
): Promise<number> {
  return await invoke<number>("rag_create_conversation", {
    collectionId,
    title,
  });
}

export async function addConversationMessage(
  conversationId: number,
  role: "user" | "assistant" | "system",
  content: string,
  sources?: number[]
): Promise<number> {
  return await invoke<number>("rag_add_conversation_message", {
    conversationId,
    role,
    content,
    sources,
  });
}

export async function getConversationMessages(
  conversationId: number,
  limit?: number
): Promise<ConversationMessage[]> {
  return await invoke<ConversationMessage[]>("rag_get_conversation_messages", {
    conversationId,
    limit,
  });
}

export async function listConversations(
  collectionId?: number
): Promise<Conversation[]> {
  return await invoke<Conversation[]>("rag_list_conversations", {
    collectionId,
  });
}

export async function deleteConversation(conversationId: number): Promise<void> {
  return await invoke<void>("rag_delete_conversation", { conversationId });
}

// ============================================================
// ANALYTICS
// ============================================================

export async function getAnalyticsSummary(collectionId?: number): Promise<any> {
  return await invoke<any>("rag_get_analytics_summary", { collection_id: collectionId });
}

export async function getRecentAnalytics(limit?: number, collectionId?: number): Promise<any[]> {
  return await invoke<any[]>("rag_get_recent_analytics", { limit, collection_id: collectionId });
}

export async function clearAnalytics(): Promise<string> {
  return await invoke<string>("rag_clear_analytics");
}

// ============================================================
// PHASE 10: Quality Analytics
// ============================================================

export interface CollectionQualityMetrics {
  id: number;
  collection_id: number;
  computed_at: string;
  avg_quality_score: number | null;
  avg_ocr_confidence: number | null;
  total_documents: number;
  documents_with_warnings: number;
  total_chunks: number;
  avg_chunk_quality: number | null;
  best_reranker: string | null;
  reranker_score: number | null;
}

export interface DocumentWarning {
  id: number;
  doc_id: number;
  warning_type: string;
  page_number: number | null;
  chunk_index: number | null;
  severity: string;
  message: string;
  suggestion: string | null;
  created_at: string;
}

export interface DocumentWarningInput {
  doc_id: number;
  warning_type: string;
  page_number?: number;
  chunk_index?: number;
  severity: string;
  message: string;
  suggestion?: string;
}

export interface RetrievalGap {
  id: number;
  collection_id: number;
  query_hash: string;
  query_length: number | null;
  result_count: number | null;
  max_confidence: number | null;
  avg_confidence: number | null;
  gap_type: string | null;
  created_at: string;
}

export interface RetrievalGapInput {
  collection_id: number;
  query_hash: string;
  query_length?: number;
  result_count?: number;
  max_confidence?: number;
  avg_confidence?: number;
  gap_type?: string;
}

export async function getCollectionQuality(
  collectionId: number
): Promise<CollectionQualityMetrics | null> {
  return await invoke<CollectionQualityMetrics | null>("rag_get_collection_quality", {
    collectionId,
  });
}

export async function computeCollectionQuality(
  collectionId: number
): Promise<CollectionQualityMetrics> {
  return await invoke<CollectionQualityMetrics>("rag_compute_collection_quality", {
    collectionId,
  });
}

export async function getDocumentWarnings(docId: number): Promise<DocumentWarning[]> {
  return await invoke<DocumentWarning[]>("rag_get_document_warnings", { doc_id: docId });
}

export async function createDocumentWarning(
  input: DocumentWarningInput
): Promise<DocumentWarning> {
  return await invoke<DocumentWarning>("rag_create_document_warning", { input });
}

export async function getLowQualityDocuments(
  collectionId: number,
  threshold?: number,
  limit?: number
): Promise<RagDocument[]> {
  return await invoke<RagDocument[]>("rag_get_low_quality_documents", {
    collectionId,
    threshold,
    limit,
  });
}

export async function recordRetrievalGap(input: RetrievalGapInput): Promise<RetrievalGap> {
  return await invoke<RetrievalGap>("rag_record_retrieval_gap", { input });
}

export async function getRetrievalGaps(
  collectionId: number,
  limit?: number
): Promise<RetrievalGap[]> {
  return await invoke<RetrievalGap[]>("rag_get_retrieval_gaps", { collectionId, limit });
}
