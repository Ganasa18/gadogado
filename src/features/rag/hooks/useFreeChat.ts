import { useCallback, useState } from "react";
import { createConversation, addConversationMessage, listConversations, getConversationMessages, deleteConversation as deleteConversationApi } from "../api";
import type { ChatMessage } from "../types";
import type { Conversation, ConversationMessage } from "../api";

import type { LlmConfig, LlmResponse } from "../../../shared/api/apiClient";
import type { LlmConfigOverrides } from "../../../shared/api/llmConfig";

type EnhanceAsync = (payload: {
  config: LlmConfig;
  content: string;
  system_prompt: string;
}) => Promise<LlmResponse>;

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_MAX_TOKENS = 2000;
const DEFAULT_TEMPERATURE = 0.7;
const DEFAULT_SYSTEM_PROMPT = "You are a helpful assistant. Answer the user's question clearly and concisely.";

// ============================================================================
// Helper Functions
// ============================================================================

function createUserMessage(query: string): ChatMessage {
  return {
    id: Date.now().toString(),
    type: "user",
    content: query,
    timestamp: new Date(),
  };
}

function createAssistantMessage(content: string, query: string): ChatMessage {
  return {
    id: (Date.now() + 1).toString(),
    type: "assistant",
    content,
    timestamp: new Date(),
    query,
  };
}

function createErrorMessage(error: unknown): ChatMessage {
  return {
    id: (Date.now() + 1).toString(),
    type: "system",
    content: `Error: ${error instanceof Error ? error.message : "Failed to get response"}`,
    timestamp: new Date(),
  };
}

function convertConversationMessageToChatMessage(msg: ConversationMessage): ChatMessage {
  return {
    id: msg.id.toString(),
    type: msg.role,
    content: msg.content,
    timestamp: new Date(msg.created_at),
  };
}

// ============================================================================
// Hook
// ============================================================================

export function useFreeChat(input: {
  chatMode: "rag" | "free";
  enhanceAsync: EnhanceAsync;
  buildConfig: (overrides?: LlmConfigOverrides) => LlmConfig;
}) {
  const { chatMode, enhanceAsync, buildConfig } = input;

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);

  // Load free chat conversations (collection_id = null)
  const refreshConversations = useCallback(async () => {
    if (chatMode !== "free") return;
    try {
      setIsLoadingHistory(true);
      const allConversations = await listConversations();
      // Filter for null collection_id (free chat conversations)
      const freeChatConversations = allConversations.filter((c) => c.collection_id === null);
      setConversations(freeChatConversations);
    } catch (err) {
      console.error("Failed to load free chat conversations:", err);
    } finally {
      setIsLoadingHistory(false);
    }
  }, [chatMode]);

  // Load conversation messages
  const loadConversation = useCallback(async (conversationId: number) => {
    if (chatMode !== "free") return;
    try {
      setIsLoadingHistory(true);
      setCurrentConversationId(conversationId);
      const conversationMessages = await getConversationMessages(conversationId);
      const chatMessages = conversationMessages.map(convertConversationMessageToChatMessage);
      setMessages(chatMessages);
    } catch (err) {
      console.error("Failed to load conversation:", err);
    } finally {
      setIsLoadingHistory(false);
    }
  }, [chatMode]);

  // Create new conversation
  const newConversation = useCallback(async () => {
    if (chatMode !== "free") return null;
    try {
      const conversationId = await createConversation(null, "New Chat");
      await refreshConversations();
      setCurrentConversationId(conversationId);
      setMessages([]);
      return conversationId;
    } catch (err) {
      console.error("Failed to create conversation:", err);
      return null;
    }
  }, [chatMode, refreshConversations]);

  // Delete conversation
  const deleteConversation = useCallback(async (conversationId: number) => {
    try {
      await deleteConversationApi(conversationId);
      await refreshConversations();
      if (currentConversationId === conversationId) {
        setCurrentConversationId(null);
        setMessages([]);
      }
    } catch (err) {
      console.error("Failed to delete conversation:", err);
    }
  }, [currentConversationId, refreshConversations]);

  // Send message
  const sendMessage = useCallback(
    async (rawQuery: string) => {
      const query = rawQuery.trim();
      if (!query || chatMode !== "free") return;

      setIsLoading(true);

      // Ensure conversation exists
      let convId = currentConversationId;
      if (!convId) {
        convId = await createConversation(null, query.slice(0, 50));
        await refreshConversations();
        setCurrentConversationId(convId);
      }

      // Create and append user message
      const userMessage = createUserMessage(query);
      setMessages((prev) => [...prev, userMessage]);
      await addConversationMessage(convId, "user", query);

      try {
        // Direct LLM call without RAG context
        const llmResponse = await enhanceAsync({
          config: buildConfig({ maxTokens: DEFAULT_MAX_TOKENS, temperature: DEFAULT_TEMPERATURE }),
          content: query,
          system_prompt: DEFAULT_SYSTEM_PROMPT,
        });

        const assistantMessage = createAssistantMessage(llmResponse.result, query);
        setMessages((prev) => [...prev, assistantMessage]);
        await addConversationMessage(convId, "assistant", llmResponse.result);
      } catch (err) {
        console.error("Failed to get response:", err);
        const errorMessage = createErrorMessage(err);
        setMessages((prev) => [...prev, errorMessage]);
        await addConversationMessage(convId, "system", errorMessage.content);
      } finally {
        setIsLoading(false);
      }
    },
    [currentConversationId, chatMode, enhanceAsync, buildConfig, refreshConversations],
  );

  return {
    messages,
    conversations,
    currentConversationId,
    isLoading,
    isLoadingHistory,
    sendMessage,
    refreshConversations,
    loadConversation,
    newConversation,
    deleteConversation,
  };
}
