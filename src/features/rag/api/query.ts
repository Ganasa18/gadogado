import { invoke } from "@tauri-apps/api/core";
import type { RagQueryRequest, RagQueryResponse, RagQueryResult } from "../types";

export async function ragQuery(request: RagQueryRequest): Promise<RagQueryResponse> {
  return await invoke<RagQueryResponse>("rag_query", { request });
}

// ============================================================
// CHAT WITH CONTEXT API
// ============================================================

export interface ChatMessage {
  role: string;
  content: string;
}

export interface ChatWithContextRequest {
  collection_id: number;
  query: string;
  conversation_id?: number;
  messages?: ChatMessage[];
  top_k?: number;
  enable_verification?: boolean;
  language?: string;
  provider?: string;
  model?: string;
}

export interface ContextManagedInfo {
  was_compacted: boolean;
  strategy_used: string;
  token_estimate: number;
  messages_used: number;
  messages_total: number;
}

export interface ChatWithContextResponse {
  prompt: string;
  results: RagQueryResult[];
  conversation_id?: number;
  context_summary?: string;
  verified: boolean;
  context_managed?: ContextManagedInfo;
}

export async function ragChatWithContext(
  request: ChatWithContextRequest,
): Promise<ChatWithContextResponse> {
  return await invoke<ChatWithContextResponse>("rag_chat_with_context", { request });
}

// ============================================================
// PHASE 9: Conversation Persistence
// ============================================================

export interface Conversation {
  id: number;
  collection_id: number | null;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface ConversationMessage {
  id: number;
  conversation_id: number;
  role: "user" | "assistant" | "system";
  content: string;
  sources: string | null;
  created_at: string;
}

export async function createConversation(
  collectionId: number | null,
  title?: string,
): Promise<number> {
  return await invoke<number>("rag_create_conversation", {
    collectionId,
    title,
  });
}

export async function addConversationMessage(
  conversationId: number,
  role: "user" | "assistant" | "system",
  content: string,
  sources?: number[],
): Promise<number> {
  return await invoke<number>("rag_add_conversation_message", {
    conversationId,
    role,
    content,
    sources,
  });
}

export async function getConversationMessages(
  conversationId: number,
  limit?: number,
): Promise<ConversationMessage[]> {
  return await invoke<ConversationMessage[]>("rag_get_conversation_messages", {
    conversationId,
    limit,
  });
}

export async function listConversations(
  collectionId?: number,
): Promise<Conversation[]> {
  return await invoke<Conversation[]>("rag_list_conversations", {
    collectionId,
  });
}

export async function deleteConversation(conversationId: number): Promise<void> {
  return await invoke<void>("rag_delete_conversation", { conversationId });
}
