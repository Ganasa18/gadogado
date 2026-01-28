import { useCallback, useState } from "react";
import {
  dbQueryRag,
  dbQueryRagWithTemplate,
  ragQuery,
  ragChatWithContext,
  getConversationMessages,
  recordRetrievalGap,
  submitTemplateFeedback,
} from "../api";
import type { ChatMessage, RagQueryResult } from "../types";
import type { ConversationMessage } from "../api";
import { hashQueryToHex, planRagPrompt } from "../ragChatUtils";

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

const DEFAULT_MAX_TOKENS = 900;
const DEFAULT_TEMPERATURE = 0.2;

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Processes a DB collection query and returns the assistant message with sources
 */
async function processDbQuery(
  collectionId: number,
  query: string,
  dbFinalK: number,
  conversationId?: number | null,
): Promise<{ message: ChatMessage; sourceIds: number[] }> {
  // Load conversation history if available (for NL response context)
  let conversationHistory: ConversationMessage[] | undefined;
  if (conversationId) {
    try {
      conversationHistory = await getConversationMessages(conversationId, 50);
    } catch (err) {
      console.error("Failed to load conversation messages:", err);
    }
  }

  // Don't pass limit - backend uses conn_config.default_limit from config_json
  // Pass final_k to control how many results after reranking
  // Mark as new query (don't use template feedback)
  const dbResponse = await dbQueryRag({
    collection_id: collectionId,
    query,
    final_k: dbFinalK,
    is_new_query: true,
    // Pass conversation history for NL response generation (SQL generation remains standalone)
    conversation_history: conversationHistory?.map((m) => ({
      role: m.role,
      content: m.content,
    })),
  });

  const sources: RagQueryResult[] = dbResponse.citations.map((citation) => ({
    content: JSON.stringify(citation.columns),
    source_type: "db_row",
    source_id: parseInt(citation.row_id, 10) || 0,
    score: null,
    page_number: null,
    page_offset: null,
    doc_name: citation.table_name,
  }));

  const sourceIds = sources.map((s) => s.source_id);
  const message: ChatMessage = {
    id: (Date.now() + 1).toString(),
    type: "assistant",
    content: dbResponse.answer,
    timestamp: new Date(),
    sources: sources.length > 0 ? sources : undefined,
    query,
    telemetry: dbResponse.telemetry,
  };

  return { message, sourceIds };
}

/**
 * Processes a DB collection query with a specific template
 * Used when user selects a different template from the matched templates
 */
async function processDbQueryWithTemplate(
  collectionId: number,
  query: string,
  templateId: number,
  dbFinalK: number,
  conversationId?: number | null,
): Promise<{ message: ChatMessage; sourceIds: number[] }> {
  // Load conversation history if available (for NL response context)
  let conversationHistory: ConversationMessage[] | undefined;
  if (conversationId) {
    try {
      conversationHistory = await getConversationMessages(conversationId, 50);
    } catch (err) {
      console.error("Failed to load conversation messages:", err);
    }
  }

  const dbResponse = await dbQueryRagWithTemplate({
    collection_id: collectionId,
    query,
    template_id: templateId,
    final_k: dbFinalK,
    // Pass conversation history for NL response generation (SQL generation remains standalone)
    conversation_history: conversationHistory?.map((m) => ({
      role: m.role,
      content: m.content,
    })),
  });

  const sources: RagQueryResult[] = dbResponse.citations.map((citation) => ({
    content: JSON.stringify(citation.columns),
    source_type: "db_row",
    source_id: parseInt(citation.row_id, 10) || 0,
    score: null,
    page_number: null,
    page_offset: null,
    doc_name: citation.table_name,
  }));

  const sourceIds = sources.map((s) => s.source_id);
  const message: ChatMessage = {
    id: (Date.now() + 1).toString(),
    type: "assistant",
    content: dbResponse.answer,
    timestamp: new Date(),
    sources: sources.length > 0 ? sources : undefined,
    query,
    telemetry: dbResponse.telemetry,
  };

  return { message, sourceIds };
}

/**
 * Processes a file-based RAG query with conversation history and returns the assistant message with sources
 */
async function processFileRagQuery(
  collectionId: number,
  query: string,
  topK: number,
  candidateK: number,
  rerankK: number,
  answerLanguage: "id" | "en",
  strictRagMode: boolean,
  isLocalProvider: boolean,
  localModels: string[],
  model: string,
  provider: string,
  enhanceAsync: EnhanceAsync,
  buildConfig: (overrides?: LlmConfigOverrides) => LlmConfig,
  conversationId?: number | null,
): Promise<{ message: ChatMessage; sourceIds?: number[] }> {
  // Load conversation history if available
  let conversationMessages: ConversationMessage[] | undefined;
  if (conversationId) {
    try {
      conversationMessages = await getConversationMessages(conversationId, 50);
    } catch (err) {
      console.error("Failed to load conversation messages:", err);
    }
  }

  // Use chat with context API if we have conversation history
  const response = conversationMessages && conversationMessages.length > 0
    ? await ragChatWithContext({
        collection_id: collectionId,
        query,
        conversation_id: conversationId || undefined,
        messages: conversationMessages.map((m) => ({
          role: m.role,
          content: m.content,
        })),
        top_k: Math.max(1, topK),
        language: answerLanguage === "id" ? "indonesia" : "english",
        provider,
        model,
      })
    : await ragQuery({
        collection_id: collectionId,
        query,
        top_k: Math.max(1, topK),
        candidate_k: Math.max(1, candidateK),
        rerank_k: Math.max(1, rerankK),
        language: answerLanguage === "id" ? "indonesia" : "english",
      });

  // Log context management info if available
  if ("context_managed" in response && response.context_managed) {
    console.log("[RAG Context Management]", response.context_managed);
  }

  const plan = planRagPrompt({
    query,
    answerLanguage,
    strictRagMode,
    ragPrompt: response.prompt,
    results: response.results,
  });

  // Record retrieval gap if detected (non-blocking)
  if (plan.shouldRecordGap && plan.gapType) {
    recordRetrievalGap({
      collection_id: collectionId,
      query_hash: hashQueryToHex(query),
      query_length: query.length,
      result_count: response.results.length,
      max_confidence: plan.maxConfidence,
      avg_confidence: plan.avgConfidence,
      gap_type: plan.gapType,
    }).catch((err) => console.error("Failed to record retrieval gap:", err));
  }

  // Determine effective model
  const effectiveModel =
    isLocalProvider && localModels.length > 0
      ? localModels.includes(model)
        ? model
        : localModels[0]
      : model;

  const configOverrides: LlmConfigOverrides = {
    maxTokens: DEFAULT_MAX_TOKENS,
    temperature: DEFAULT_TEMPERATURE,
    ...(effectiveModel !== model && { model: effectiveModel }),
  };

  const llmResponse = await enhanceAsync({
    config: buildConfig(configOverrides),
    content: plan.promptContent,
    system_prompt: plan.systemPrompt,
  });

  const cleanedAnswer = llmResponse.result
    .replace(/\[Source:[^\]]+\]/g, "")
    .trim();

  const results = response.results;
  const hasSources = results.length > 0;
  const message: ChatMessage = {
    id: (Date.now() + 1).toString(),
    type: "assistant",
    content: cleanedAnswer,
    timestamp: new Date(),
    sources: hasSources ? results : undefined,
    query,
  };

  const sourceIds = hasSources ? results.map((r: RagQueryResult) => r.source_id) : undefined;
  return { message, sourceIds };
}

/**
 * Creates a user message for the chat
 */
function createUserMessage(query: string): ChatMessage {
  return {
    id: Date.now().toString(),
    type: "user",
    content: query,
    timestamp: new Date(),
  };
}

/**
 * Creates an error message for failed queries
 */
function createErrorMessage(error: unknown): ChatMessage {
  return {
    id: (Date.now() + 1).toString(),
    type: "system",
    content: `Error: ${error instanceof Error ? error.message : "Failed to query RAG"}`,
    timestamp: new Date(),
  };
}

// ============================================================================
// Hook
// ============================================================================

export function useRagSend(input: {
  selectedCollectionId: number | null;
  isDbCollection: boolean;
  answerLanguage: "id" | "en";
  strictRagMode: boolean;
  topK: number;
  candidateK: number;
  rerankK: number;
  dbFinalK: number;
  isLocalProvider: boolean;
  localModels: string[];
  model: string;
  provider: string;
  enhanceAsync: EnhanceAsync;
  buildConfig: (overrides?: LlmConfigOverrides) => LlmConfig;
  ensureConversation: (collectionId: number, query: string) => Promise<number | null>;
  appendMessage: (msg: ChatMessage) => void;
  persistMessage: (
    conversationId: number | null,
    role: "user" | "assistant" | "system",
    content: string,
    sourceIds?: number[],
  ) => Promise<void>;
}) {
  const {
    selectedCollectionId,
    isDbCollection,
    answerLanguage,
    strictRagMode,
    topK,
    candidateK,
    rerankK,
    dbFinalK,
    isLocalProvider,
    localModels,
    model,
    provider,
    enhanceAsync,
    buildConfig,
    ensureConversation,
    appendMessage,
    persistMessage,
  } = input;

  const [isLoading, setIsLoading] = useState(false);

  const sendMessage = useCallback(
    async (rawQuery: string) => {
      const query = rawQuery.trim();
      if (!query || !selectedCollectionId) return;

      setIsLoading(true);

      const conversationId = await ensureConversation(selectedCollectionId, query);

      // Create and append user message
      const userMessage = createUserMessage(query);
      appendMessage(userMessage);
      await persistMessage(conversationId, "user", query);

      try {
        let assistantMessage: ChatMessage;
        let sourceIds: number[] | undefined;

        // Branch based on collection type
        if (isDbCollection) {
          const result = await processDbQuery(selectedCollectionId, query, dbFinalK, conversationId);
          assistantMessage = result.message;
          sourceIds = result.sourceIds;
        } else {
          const result = await processFileRagQuery(
            selectedCollectionId,
            query,
            topK,
            candidateK,
            rerankK,
            answerLanguage,
            strictRagMode,
            isLocalProvider,
            localModels,
            model,
            provider,
            enhanceAsync,
            buildConfig,
            conversationId,
          );
          assistantMessage = result.message;
          sourceIds = result.sourceIds;
        }

        // Append assistant message and persist
        appendMessage(assistantMessage);
        await persistMessage(conversationId, "assistant", assistantMessage.content, sourceIds);
      } catch (err) {
        console.error("Failed to query RAG:", err);
        const errorMessage = createErrorMessage(err);
        appendMessage(errorMessage);
        await persistMessage(conversationId, "system", errorMessage.content);
      } finally {
        setIsLoading(false);
      }
    },
    [
      selectedCollectionId,
      isDbCollection,
      answerLanguage,
      strictRagMode,
      topK,
      candidateK,
      rerankK,
      dbFinalK,
      isLocalProvider,
      localModels,
      model,
      provider,
      enhanceAsync,
      buildConfig,
      ensureConversation,
      appendMessage,
      persistMessage,
    ],
  );

  /**
   * Regenerate a query using a specific template (for DB collections)
   * This is called when user selects a different template from the matched templates
   */
  const regenerateWithTemplate = useCallback(
    async (query: string, templateId: number, autoSelectedTemplateId?: number) => {
      if (!query || !selectedCollectionId || !isDbCollection) return;

      setIsLoading(true);

      // Submit template feedback in background (non-blocking)
      // This helps the system learn user preferences
      submitTemplateFeedback({
        collection_id: selectedCollectionId,
        query,
        auto_selected_template_id: autoSelectedTemplateId,
        user_selected_template_id: templateId,
      }).catch((err) => console.error("Failed to submit template feedback:", err));

      const conversationId = await ensureConversation(selectedCollectionId, query);

      // Create and append user message indicating template regeneration
      const userMessage = createUserMessage(`[Regenerate with template #${templateId}] ${query}`);
      appendMessage(userMessage);
      await persistMessage(conversationId, "user", userMessage.content);

      try {
        const result = await processDbQueryWithTemplate(
          selectedCollectionId,
          query,
          templateId,
          dbFinalK,
          conversationId,
        );
        const assistantMessage = result.message;
        const sourceIds = result.sourceIds;

        // Append assistant message and persist
        appendMessage(assistantMessage);
        await persistMessage(conversationId, "assistant", assistantMessage.content, sourceIds);
      } catch (err) {
        console.error("Failed to regenerate with template:", err);
        const errorMessage = createErrorMessage(err);
        appendMessage(errorMessage);
        await persistMessage(conversationId, "system", errorMessage.content);
      } finally {
        setIsLoading(false);
      }
    },
    [
      selectedCollectionId,
      isDbCollection,
      dbFinalK,
      ensureConversation,
      appendMessage,
      persistMessage,
    ],
  );

  return { isLoading, sendMessage, regenerateWithTemplate };
}
