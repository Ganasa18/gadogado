/**
 * API functions for RAG context settings
 * Handles communication with Tauri backend for context management
 */

import { invoke } from "@tauri-apps/api/core";
import type { RagContextSettings, ModelContextLimit } from "../../../store/settings";

/**
 * Get global RAG context settings
 */
export async function getRagGlobalSettings(): Promise<RagContextSettings> {
  return invoke("get_rag_global_settings");
}

/**
 * Update global RAG context settings
 */
export async function updateRagGlobalSettings(
  settings: RagContextSettings
): Promise<void> {
  return invoke("update_rag_global_settings", { settings });
}

/**
 * Get model context limit for specific provider/model
 */
export async function getModelContextLimit(
  provider: string,
  modelName: string
): Promise<ModelContextLimit> {
  return invoke("get_model_context_limit", {
    provider,
    modelName,
  });
}

/**
 * Get all available model limits from database
 */
export async function getAllModelLimits(): Promise<ModelContextLimit[]> {
  return invoke("get_all_model_limits");
}

/**
 * Insert or update a model context limit
 */
export async function upsertModelLimit(
  provider: string,
  modelName: string,
  contextWindow: number,
  maxOutputTokens: number
): Promise<number> {
  return invoke("upsert_model_limit", {
    provider,
    modelName,
    contextWindow,
    maxOutputTokens,
  });
}
