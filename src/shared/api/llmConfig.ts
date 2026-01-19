import type { LLMProvider } from "../../store/settings";
import type { LlmConfig } from "./apiClient";
import type { EmbeddingProvider } from "../../store/settings";

export const LOCAL_LLM_BASE_URL = "http://localhost:1234/v1";

export interface LlmConfigOverrides {
  maxTokens?: number;
  temperature?: number;
  baseUrl?: string;
  model?: string;
}

const providerLabels: Record<LLMProvider, LlmConfig["provider"]> = {
  local: "local",
  openai: "openai",
  gemini: "gemini",
  ollama: "ollama",
  llama_cpp: "llama_cpp",
  dll: "dll",
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
  const isEmbeddedLocal = settings.provider === "local";
  const isKeyless =
    settings.provider === "local" ||
    settings.provider === "ollama" ||
    settings.provider === "llama_cpp";
  return {
    provider: providerLabels[settings.provider],
    base_url: isEmbeddedLocal
      ? LOCAL_LLM_BASE_URL
      : overrides.baseUrl ?? settings.baseUrl,
    model: overrides.model ?? settings.model,
    api_key: isKeyless ? null : settings.apiKey || null,
    max_tokens: overrides.maxTokens ?? 1024,
    temperature: overrides.temperature ?? 0.7,
  };
}

export function createEmbeddingConfig(settings: {
  provider: EmbeddingProvider;
  model: string;
}): LlmConfig {
  return {
    provider: providerLabels[settings.provider as LLMProvider],
    base_url: "",
    model: settings.model,
    api_key: null,
    max_tokens: 1024,
    temperature: 0.7,
  };
}
