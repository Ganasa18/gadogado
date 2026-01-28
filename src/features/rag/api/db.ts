import { invoke } from "@tauri-apps/api/core";
import type {
  DbAllowlistProfile,
  DbCollectionConfig,
  DbConnection,
  DbConnectionInput,
  DbQueryRequest,
  DbQueryResponse,
  DbQueryWithTemplateRequest,
  DbTestConnectionResult,
  RateLimitStatus,
  RagCollection,
  TableInfo,
  TemplateFeedbackRequest,
  TemplateFeedbackResponse,
} from "../types";

export async function dbListConnections(): Promise<DbConnection[]> {
  return await invoke<DbConnection[]>("db_list_connections");
}

export async function dbAddConnection(
  input: DbConnectionInput,
): Promise<DbConnection> {
  return await invoke<DbConnection>("db_add_connection", { input });
}

export async function dbTestConnection(
  connId: number,
): Promise<DbTestConnectionResult> {
  return await invoke<DbTestConnectionResult>("db_test_connection", { connId });
}

export async function dbTestConnectionInput(
  input: DbConnectionInput,
): Promise<DbTestConnectionResult> {
  return await invoke<DbTestConnectionResult>("db_test_connection_input", { input });
}

export async function dbDeleteConnection(connId: number): Promise<void> {
  return await invoke<void>("db_delete_connection", { connId });
}

export async function dbListTables(connId: number): Promise<TableInfo[]> {
  return await invoke<TableInfo[]>("db_list_tables", { connId });
}

export async function dbListAllowlistProfiles(): Promise<DbAllowlistProfile[]> {
  return await invoke<DbAllowlistProfile[]>("db_list_allowlist_profiles");
}

export async function dbListAllowlistedTables(profileId: number): Promise<string[]> {
  return await invoke<string[]>("db_list_allowlisted_tables", { profileId });
}

export async function dbSaveConnectionConfig(
  connId: number,
  profileId: number,
  selectedTables: string[],
  selectedColumns: Record<string, string[]>,
): Promise<DbConnection> {
  return await invoke<DbConnection>("db_save_connection_config", {
    connId,
    profileId,
    selectedTables,
    selectedColumns,
  });
}

export async function dbGetConnectionConfig(connId: number): Promise<{
  profile_id: number | null;
  selected_tables: string[];
  selected_columns: Record<string, string[]>;
}> {
  return await invoke<{
    profile_id: number | null;
    selected_tables: string[];
    selected_columns: Record<string, string[]>;
  }>("db_get_connection_config", { connId });
}

export async function dbSyncProfileTables(
  profileId: number,
  connId: number,
  tables: string[],
): Promise<DbAllowlistProfile> {
  return await invoke<DbAllowlistProfile>("db_sync_profile_tables", {
    profileId,
    connId,
    tables,
  });
}

export async function ragCreateDbCollection(
  name: string,
  description: string | undefined,
  config: DbCollectionConfig,
): Promise<RagCollection> {
  return await invoke<RagCollection>("rag_create_db_collection", {
    name,
    description,
    configJson: JSON.stringify(config),
  });
}

export async function dbGetSelectedTables(collectionId: number): Promise<string[]> {
  return await invoke<string[]>("db_get_selected_tables", { collectionId });
}

export async function dbUpdateSelectedTables(
  collectionId: number,
  selectedTables: string[],
): Promise<void> {
  return await invoke<void>("db_update_selected_tables", {
    collectionId,
    selectedTables,
  });
}

export async function dbQueryRag(request: DbQueryRequest): Promise<DbQueryResponse> {
  return await invoke<DbQueryResponse>("db_query_rag", { request });
}

/**
 * Query with a specific template (for user-selected regeneration)
 */
export async function dbQueryRagWithTemplate(
  request: DbQueryWithTemplateRequest,
): Promise<DbQueryResponse> {
  return await invoke<DbQueryResponse>("db_query_rag_with_template", { request });
}

/**
 * Submit template feedback when user selects a different template
 */
export async function submitTemplateFeedback(
  request: TemplateFeedbackRequest,
): Promise<TemplateFeedbackResponse> {
  return await invoke<TemplateFeedbackResponse>("submit_template_feedback", { request });
}

// ============================================================
// RATE LIMIT & AUDIT API
// ============================================================

export async function dbGetRateLimitStatus(
  collectionId: number,
): Promise<RateLimitStatus> {
  return await invoke<RateLimitStatus>("db_get_rate_limit_status", {
    collection_id: collectionId,
  });
}
