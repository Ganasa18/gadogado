import axios from 'axios';

const apiClient = axios.create({
  baseURL: 'http://localhost:3001/api',
  headers: {
    'Content-Type': 'application/json',
  },
});

export interface LogEntry {
  time: string;
  level: string;
  source: string;
  message: string;
}

export type LlmProviderLabel =
  | 'local'
  | 'openai'
  | 'gemini'
  | 'ollama'
  | 'llama_cpp'
  | 'openrouter'
  | 'dll';

export interface LlmConfig {
  provider: LlmProviderLabel;
  base_url: string;
  model: string;
  api_key: string | null;
  max_tokens: number;
  temperature: number;
}

export interface OpenRouterPricing {
  prompt?: string;
  completion?: string;
  request?: string;
  image?: string;
}

export interface OpenRouterArchitecture {
  modality?: string;
  input_modalities?: string[];
  output_modalities?: string[];
  tokenizer?: string;
  instruct_type?: string;
}

export interface OpenRouterTopProvider {
  is_moderated?: boolean;
  context_length?: number;
  max_completion_tokens?: number;
}

export interface OpenRouterModel {
  id: string;
  canonical_slug?: string;
  name?: string;
  created?: number;
  pricing?: OpenRouterPricing;
  context_length?: number;
  architecture?: OpenRouterArchitecture;
  top_provider?: OpenRouterTopProvider;
  per_request_limits?: unknown;
  supported_parameters?: string[];
  default_parameters?: unknown;
  description?: string;
  expiration_date?: string | null;
  [key: string]: unknown;
}

export interface OpenRouterProvider {
  id?: string;
  name?: string;
  slug?: string;
  description?: string;
  privacy_policy_url?: string;
  terms_of_service_url?: string;
  status_page_url?: string | null;
  [key: string]: unknown;
}

export interface TranslatePayload {
  config: LlmConfig;
  content: string;
  source: string;
  target: string;
}

export interface EnhancePayload {
  config: LlmConfig;
  content: string;
  system_prompt?: string;
}

export interface TypeGenPayload {
  config: LlmConfig;
  json: string;
  language: string;
  root_name: string;
  mode: 'auto' | 'offline' | 'llm';
}

export interface LlmResponse {
  result: string;
}

export const llmApi = {
  translate: async (payload: TranslatePayload): Promise<LlmResponse> => {
    const response = await apiClient.post<LlmResponse>('/translate', payload);
    return response.data;
  },
  enhance: async (payload: EnhancePayload): Promise<LlmResponse> => {
    const response = await apiClient.post<LlmResponse>('/enhance', payload);
    return response.data;
  },
  typegen: async (payload: TypeGenPayload): Promise<LlmResponse> => {
    const response = await apiClient.post<LlmResponse>('/typegen', payload);
    return response.data;
  },
  getModels: async (config: LlmConfig): Promise<string[]> => {
    const response = await apiClient.post<string[]>('/models', config);
    return response.data;
  },
  getOpenRouterModels: async (config: LlmConfig): Promise<OpenRouterModel[]> => {
    const response = await apiClient.post<OpenRouterModel[]>(
      '/openrouter/models',
      config
    );
    return response.data;
  },
  getOpenRouterProviders: async (
    config: LlmConfig
  ): Promise<OpenRouterProvider[]> => {
    const response = await apiClient.post<OpenRouterProvider[]>(
      '/openrouter/providers',
      config
    );
    return response.data;
  },
  getLogs: async (): Promise<LogEntry[]> => {
    const response = await apiClient.get<LogEntry[]>('/logs');
    return response.data;
  },
};

export default apiClient;
