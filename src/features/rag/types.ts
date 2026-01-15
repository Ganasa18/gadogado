export interface RagCollection {
  id: number;
  name: string;
  description: string | null;
  created_at: string;
}

export interface RagDocument {
  id: number;
  collection_id: number | null;
  file_name: string;
  file_path: string | null;
  file_type: string;
  language: string;
  total_pages: number;
  created_at: string;
}

export interface RagDocumentChunk {
  id: number;
  doc_id: number;
  content: string;
  page_number: number | null;
  chunk_index: number;
  token_count: number | null;
}

export interface RagExcelData {
  id: number;
  doc_id: number;
  row_index: number;
  data_json: string | null;
  val_a: string | null;
  val_b: string | null;
  val_c: number | null;
}

export interface RagCollectionInput {
  name: string;
  description?: string;
}

export interface RagDocumentInput {
  collection_id?: number;
  file_name: string;
  file_path?: string;
  file_type: string;
  language?: string;
  total_pages?: number;
}

export interface RagQueryRequest {
  collection_id: number;
  query: string;
  top_k?: number;
}

export interface RagQueryResult {
  content: string;
  source_type: string;
  source_id: number;
  score: number | null;
}

export interface RagQueryResponse {
  prompt: string;
  results: RagQueryResult[];
}

export type ChatMessageType = "user" | "assistant" | "system";

export interface ChatMessage {
  id: string;
  type: ChatMessageType;
  content: string;
  timestamp: Date;
  sources?: RagQueryResult[];
  query?: string;
}

export interface RagWebImportRequest {
  collection_id: number | undefined;
  url: string;
  max_pages?: number;
  max_depth?: number;
}

export interface LogEntry {
  time: string;
  level: string;
  source: string;
  message: string;
}
