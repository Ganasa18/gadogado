import { invoke } from "@tauri-apps/api/core";

export async function getAnalyticsSummary(collectionId?: number): Promise<any> {
  return await invoke<any>("rag_get_analytics_summary", {
    collection_id: collectionId,
  });
}

export async function getRecentAnalytics(
  limit?: number,
  collectionId?: number,
): Promise<any[]> {
  return await invoke<any[]>("rag_get_recent_analytics", {
    limit,
    collection_id: collectionId,
  });
}

export async function clearAnalytics(): Promise<string> {
  return await invoke<string>("rag_clear_analytics");
}
