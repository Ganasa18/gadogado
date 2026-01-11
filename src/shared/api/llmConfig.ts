import type { LLMProvider } from "../../store/settings";
import type { LlmConfig } from "./apiClient";

export const LOCAL_LLM_BASE_URL = "http://localhost:1234/v1";

export interface LlmConfigOverrides {
  maxTokens?: number;
  temperature?: number;
  baseUrl?: string;
  model?: string;
}

const providerLabels: Record<LLMProvider, LlmConfig["provider"]> = {
  local: "Local",
  openai: "OpenAI",
  google: "Google",
  dll: "DLL",
};

export function createLlmConfig(
  settings: {
    provider: LLMProvider;
    model: string;
    apiKey: string;
    baseUrl: string;
  },
  overrides: LlmConfigOverrides = {}
): LlmConfig {
  const isLocal = settings.provider === "local";
  return {
    provider: providerLabels[settings.provider],
    base_url: isLocal
      ? LOCAL_LLM_BASE_URL
      : overrides.baseUrl ?? settings.baseUrl,
    model: overrides.model ?? settings.model,
    api_key: isLocal ? null : settings.apiKey || null,
    max_tokens: overrides.maxTokens ?? 1024,
    temperature: overrides.temperature ?? 0.7,
  };
}
