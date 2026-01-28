import { invoke } from "@tauri-apps/api/core";
import type { RagCollection, RagCollectionInput } from "../types";

export async function createRagCollection(
  input: RagCollectionInput,
): Promise<RagCollection> {
  return await invoke<RagCollection>("rag_create_collection", { input });
}

export async function getRagCollection(id: number): Promise<RagCollection> {
  return await invoke<RagCollection>("rag_get_collection", { id });
}

export async function listRagCollections(
  limit?: number,
): Promise<RagCollection[]> {
  return await invoke<RagCollection[]>("rag_list_collections", { limit });
}

export async function deleteRagCollection(id: number): Promise<number> {
  return await invoke<number>("rag_delete_collection", { id });
}
