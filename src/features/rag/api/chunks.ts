import { invoke } from "@tauri-apps/api/core";
import type {
  ChunkWithQuality,
  DocumentQualityAnalysis,
  RagDocumentChunk,
  RagExcelData,
} from "../types";

export async function listRagChunks(
  docId: number,
  limit?: number,
): Promise<RagDocumentChunk[]> {
  return await invoke<RagDocumentChunk[]>("rag_list_chunks", {
    doc_id: docId,
    limit,
  });
}

export async function listRagExcelData(
  docId: number,
  limit?: number,
): Promise<RagExcelData[]> {
  return await invoke<RagExcelData[]>("rag_list_excel_data", {
    doc_id: docId,
    limit,
  });
}

export async function getChunksWithQuality(
  documentId: number,
  limit?: number,
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
  newContent: string,
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
  qualityThreshold: number,
): Promise<ChunkWithQuality[]> {
  return await invoke<ChunkWithQuality[]>("rag_filter_low_quality_chunks", {
    document_id: documentId,
    quality_threshold: qualityThreshold,
  });
}

export async function analyzeDocumentQuality(
  documentId: number,
): Promise<DocumentQualityAnalysis> {
  return await invoke<DocumentQualityAnalysis>("rag_analyze_document_quality", {
    document_id: documentId,
  });
}
