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
    docId,
    limit,
  });
}

export async function listRagExcelData(
  docId: number,
  limit?: number
): Promise<RagExcelData[]> {
  return await invoke<RagExcelData[]>("rag_list_excel_data", {
    docId,
    limit,
  });
}

export async function ragQuery(
  request: RagQueryRequest
): Promise<RagQueryResponse> {
  return await invoke<RagQueryResponse>("rag_query", { request });
}

export async function importRagWeb(
  url: string,
  collectionId: number | undefined,
  maxPages?: number,
  maxDepth?: number
): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_import_web", {
    url,
    collectionId,
    maxPages,
    maxDepth,
  });
}

export async function getLogs(): Promise<LogEntry[]> {
  return await invoke<LogEntry[]>("get_logs");
}
