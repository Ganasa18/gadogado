import { invoke } from "@tauri-apps/api/core";
import type { RagDocument } from "../types";

export async function getRagDocument(id: number): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_get_document", { id });
}

export async function listRagDocuments(
  collectionId?: number,
  limit?: number,
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
  collectionId?: number,
): Promise<RagDocument> {
  return await invoke<RagDocument>("rag_import_file", {
    filePath,
    collectionId,
  });
}

export type WebCrawlMode = "html" | "ocr";

export async function importRagWeb(
  url: string,
  collectionId: number | undefined,
  maxPages?: number,
  maxDepth?: number,
  mode: WebCrawlMode = "html",
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
