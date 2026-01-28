import { invoke } from "@tauri-apps/api/core";
import type { QueryTemplate, QueryTemplateInput } from "../types";

// Logging utility for debugging
const add_log = (category: string, message: string, data?: unknown) => {
  const timestamp = new Date().toISOString();
  const logEntry = `[${timestamp}] [${category}] ${message}`;
  if (data) {
    console.log(logEntry, data);
  } else {
    console.log(logEntry);
  }
};

export async function dbListQueryTemplates(profileId?: number): Promise<QueryTemplate[]> {
  add_log("API", "dbListQueryTemplates: Calling", { profileId });
  try {
    const result = await invoke<QueryTemplate[]>("db_list_query_templates", { profileId });
    add_log("API", "dbListQueryTemplates: Success", { count: result.length });
    return result;
  } catch (err) {
    add_log("API", "dbListQueryTemplates: ERROR", { error: err, profileId });
    throw err;
  }
}

export async function dbCreateQueryTemplate(
  input: QueryTemplateInput,
): Promise<QueryTemplate> {
  add_log("API", "dbCreateQueryTemplate: Calling", { input });
  try {
    const result = await invoke<QueryTemplate>("db_create_query_template", { input });
    add_log("API", "dbCreateQueryTemplate: Success", { result });
    return result;
  } catch (err) {
    add_log("API", "dbCreateQueryTemplate: ERROR", { error: err, input });
    throw err;
  }
}

export async function dbUpdateQueryTemplate(
  templateId: number,
  input: Partial<QueryTemplateInput>,
): Promise<QueryTemplate> {
  add_log("API", "dbUpdateQueryTemplate: Calling", { templateId, input });
  try {
    const result = await invoke<QueryTemplate>("db_update_query_template", {
      templateId,
      input,
    });
    add_log("API", "dbUpdateQueryTemplate: Success", { result });
    return result;
  } catch (err) {
    add_log("API", "dbUpdateQueryTemplate: ERROR", { error: err, templateId, input });
    throw err;
  }
}

export async function dbDeleteQueryTemplate(templateId: number): Promise<void> {
  add_log("API", "dbDeleteQueryTemplate: Calling", { templateId });
  try {
    await invoke<void>("db_delete_query_template", { templateId });
    add_log("API", "dbDeleteQueryTemplate: Success", { templateId });
  } catch (err) {
    add_log("API", "dbDeleteQueryTemplate: ERROR", { error: err, templateId });
    throw err;
  }
}

export async function dbToggleQueryTemplate(
  templateId: number,
  isEnabled: boolean,
): Promise<QueryTemplate> {
  add_log("API", "dbToggleQueryTemplate: Calling", { templateId, isEnabled });
  try {
    const result = await invoke<QueryTemplate>("db_toggle_query_template", {
      templateId,
      isEnabled,
    });
    add_log("API", "dbToggleQueryTemplate: Success", { result });
    return result;
  } catch (err) {
    add_log("API", "dbToggleQueryTemplate: ERROR", { error: err, templateId, isEnabled });
    throw err;
  }
}
