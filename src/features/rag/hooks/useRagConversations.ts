import { useCallback, useEffect, useState } from "react";
import {
  addConversationMessage,
  createConversation,
  deleteConversation,
  getConversationMessages,
  listConversations,
  type Conversation,
  type ConversationMessage as DbConversationMessage,
} from "../api";
import type { ChatMessage } from "../types";

function mapDbMessages(dbMessages: DbConversationMessage[]): ChatMessage[] {
  return dbMessages.map((msg) => ({
    id: msg.id.toString(),
    type: msg.role as "user" | "assistant" | "system",
    content: msg.content,
    timestamp: new Date(msg.created_at),
    sources: msg.sources ? JSON.parse(msg.sources) : undefined,
  }));
}

export function useRagConversations(selectedCollectionId: number | null) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<number | null>(
    null,
  );
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);

  const refreshConversations = useCallback(async () => {
    if (!selectedCollectionId) {
      setConversations([]);
      return;
    }

    try {
      const list = await listConversations(selectedCollectionId);
      setConversations(list);
    } catch (err) {
      console.error(err);
    }
  }, [selectedCollectionId]);

  useEffect(() => {
    void refreshConversations();
  }, [refreshConversations]);

  const loadConversationHistory = useCallback(async (conversationId: number) => {
    setIsLoadingHistory(true);
    try {
      const dbMessages = await getConversationMessages(conversationId, 100);
      setMessages(mapDbMessages(dbMessages));
    } catch (err) {
      console.error("Failed to load conversation history:", err);
    } finally {
      setIsLoadingHistory(false);
    }
  }, []);

  const selectConversation = useCallback(
    async (conversationId: number) => {
      setCurrentConversationId(conversationId);
      await loadConversationHistory(conversationId);
    },
    [loadConversationHistory],
  );

  const startNewConversation = useCallback(() => {
    setCurrentConversationId(null);
    setMessages([]);
  }, []);

  const deleteConversationById = useCallback(
    async (conversationId: number) => {
      await deleteConversation(conversationId);
      setConversations((prev) => prev.filter((c) => c.id !== conversationId));
      if (currentConversationId === conversationId) {
        startNewConversation();
      }
    },
    [currentConversationId, startNewConversation],
  );

  const clearCurrentConversation = useCallback(async () => {
    if (!currentConversationId) {
      startNewConversation();
      return;
    }
    await deleteConversation(currentConversationId);
    setConversations((prev) => prev.filter((c) => c.id !== currentConversationId));
    startNewConversation();
  }, [currentConversationId, startNewConversation]);

  const ensureConversation = useCallback(
    async (collectionId: number, query: string) => {
      if (currentConversationId) return currentConversationId;

      try {
        const conversationId = await createConversation(
          collectionId,
          query.slice(0, 50) + (query.length > 50 ? "..." : ""),
        );
        setCurrentConversationId(conversationId);
        await refreshConversations();
        return conversationId;
      } catch (err) {
        console.error("Failed to create conversation:", err);
        return null;
      }
    },
    [currentConversationId, refreshConversations],
  );

  const persistMessage = useCallback(
    async (
      conversationId: number | null,
      role: "user" | "assistant" | "system",
      content: string,
      sourceIds?: number[],
    ) => {
      if (!conversationId) return;
      try {
        await addConversationMessage(conversationId, role, content, sourceIds);
      } catch (err) {
        console.error("Failed to persist message:", err);
      }
    },
    [],
  );

  const appendMessage = useCallback((msg: ChatMessage) => {
    setMessages((prev) => [...prev, msg]);
  }, []);

  return {
    messages,
    conversations,
    currentConversationId,
    isLoadingHistory,
    selectConversation,
    startNewConversation,
    deleteConversationById,
    clearCurrentConversation,
    ensureConversation,
    appendMessage,
    persistMessage,
  };
}
