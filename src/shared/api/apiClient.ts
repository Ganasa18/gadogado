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

export type LlmProviderLabel = 'Local' | 'OpenAI' | 'Google' | 'DLL';

export interface LlmConfig {
  provider: LlmProviderLabel;
  base_url: string;
  model: string;
  api_key: string | null;
  max_tokens: number;
  temperature: number;
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
  getLogs: async (): Promise<LogEntry[]> => {
    const response = await apiClient.get<LogEntry[]>('/logs');
    return response.data;
  },
};

export default apiClient;
