import { useCallback, useState } from "react";
import { dbQueryRag, ragQuery, recordRetrievalGap } from "../api";
import type { ChatMessage, RagQueryResult } from "../types";
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
  topK: number,
): Promise<{ message: ChatMessage; sourceIds: number[] }> {
  const dbResponse = await dbQueryRag({
    collection_id: collectionId,
    query,
    limit: Math.max(1, topK),
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
 * Processes a file-based RAG query and returns the assistant message with sources
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
  enhanceAsync: EnhanceAsync,
  buildConfig: (overrides?: LlmConfigOverrides) => LlmConfig,
): Promise<{ message: ChatMessage; sourceIds?: number[] }> {
  const response = await ragQuery({
    collection_id: collectionId,
    query,
    top_k: Math.max(1, topK),
    candidate_k: Math.max(1, candidateK),
    rerank_k: Math.max(1, rerankK),
  });

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

  const hasSources = response.results.length > 0;
  const message: ChatMessage = {
    id: (Date.now() + 1).toString(),
    type: "assistant",
    content: cleanedAnswer,
    timestamp: new Date(),
    sources: hasSources ? response.results : undefined,
    query,
  };

  const sourceIds = hasSources ? response.results.map((r) => r.source_id) : undefined;
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
  isLocalProvider: boolean;
  localModels: string[];
  model: string;
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
    isLocalProvider,
    localModels,
    model,
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
          const result = await processDbQuery(selectedCollectionId, query, topK);
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
            enhanceAsync,
            buildConfig,
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
      isLocalProvider,
      localModels,
      model,
      enhanceAsync,
      buildConfig,
      ensureConversation,
      appendMessage,
      persistMessage,
    ],
  );

  return { isLoading, sendMessage };
}
