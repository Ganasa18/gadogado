import { invoke } from "@tauri-apps/api/core";
import type {
  CacheConfig,
  ChatConfig,
  ChunkingConfig,
  ConfigValidation,
  EmbeddingConfig,
  LogEntry,
  OcrConfig,
  RagConfig,
  RetrievalConfig,
  SystemStats,
} from "../types";

export async function getLogs(): Promise<LogEntry[]> {
  return await invoke<LogEntry[]>("get_logs");
}

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

export async function getSystemStats(): Promise<SystemStats> {
  return await invoke<SystemStats>("rag_get_system_stats");
}
