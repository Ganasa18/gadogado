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

      const userMessage: ChatMessage = {
        id: Date.now().toString(),
        type: "user",
        content: query,
        timestamp: new Date(),
      };
      appendMessage(userMessage);
      await persistMessage(conversationId, "user", query);

      try {
        let assistantMessage: ChatMessage;
        let sourceIds: number[] | undefined;

        if (isDbCollection) {
          const dbResponse = await dbQueryRag({
            collection_id: selectedCollectionId,
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

          sourceIds = sources.map((s) => s.source_id);
          assistantMessage = {
            id: (Date.now() + 1).toString(),
            type: "assistant",
            content: dbResponse.answer,
            timestamp: new Date(),
            sources: sources.length > 0 ? sources : undefined,
            query,
          };
        } else {
          const response = await ragQuery({
            collection_id: selectedCollectionId,
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

          if (plan.shouldRecordGap && plan.gapType) {
            recordRetrievalGap({
              collection_id: selectedCollectionId,
              query_hash: hashQueryToHex(query),
              query_length: query.length,
              result_count: response.results.length,
              max_confidence: plan.maxConfidence,
              avg_confidence: plan.avgConfidence,
              gap_type: plan.gapType,
            }).catch((err) => console.error("Failed to record retrieval gap:", err));
          }

          const effectiveModel =
            isLocalProvider && localModels.length > 0
              ? localModels.includes(model)
                ? model
                : localModels[0]
              : model;

          const configOverrides = {
            maxTokens: 900,
            temperature: 0.2,
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
          assistantMessage = {
            id: (Date.now() + 1).toString(),
            type: "assistant",
            content: cleanedAnswer,
            timestamp: new Date(),
            sources: hasSources ? response.results : undefined,
            query,
          };
          sourceIds = hasSources ? response.results.map((r) => r.source_id) : undefined;
        }

        appendMessage(assistantMessage);
        await persistMessage(conversationId, "assistant", assistantMessage.content, sourceIds);
      } catch (err) {
        console.error("Failed to query RAG:", err);
        const errorMessage: ChatMessage = {
          id: (Date.now() + 1).toString(),
          type: "system",
          content: `Error: ${err instanceof Error ? err.message : "Failed to query RAG"}`,
          timestamp: new Date(),
        };
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
