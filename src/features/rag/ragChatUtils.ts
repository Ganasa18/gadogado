import type { RagQueryResult } from "./types";

export type AnswerLanguage = "id" | "en";

export function isGreeting(text: string): boolean {
  const q = text.trim().toLowerCase();
  // Keep this minimal and language-agnostic.
  return /^(hi|hello|hey|halo|hai|pagi|siang|sore|malam)(\b|!|\.|,)/.test(q);
}

export function isLowConfidenceSources(sources?: RagQueryResult[]): boolean {
  if (!sources || sources.length === 0) return false;
  const scores = sources.map((source) => source.score || 0);
  const maxConfidence = Math.max(...scores);
  const avgConfidence = scores.reduce((sum, score) => sum + score, 0) / scores.length;
  // Consider low confidence if max < 0.4 (40%) or avg < 0.25 (25%)
  return maxConfidence < 0.4 || avgConfidence < 0.25;
}

export function hashQueryToHex(query: string): string {
  return query
    .split("")
    .reduce((a, b) => {
      a = (a << 5) - a + b.charCodeAt(0);
      return a & a;
    }, 0)
    .toString(16);
}

export function computeConfidence(results: RagQueryResult[]) {
  const scores = results.map((r) => r.score || 0);
  const maxConfidence = scores.length > 0 ? Math.max(...scores) : 0;
  const avgConfidence =
    scores.length > 0 ? scores.reduce((a, b) => a + b, 0) / scores.length : 0;
  const hasStrongContext = results.length > 0 && maxConfidence >= 0.4 && avgConfidence >= 0.25;
  const hasSources = results.length > 0;

  return { maxConfidence, avgConfidence, hasStrongContext, hasSources };
}

export function planRagPrompt(input: {
  query: string;
  answerLanguage: AnswerLanguage;
  strictRagMode: boolean;
  ragPrompt: string;
  results: RagQueryResult[];
}): {
  promptContent: string;
  systemPrompt: string;
  shouldRecordGap: boolean;
  gapType: "no_results" | "low_confidence" | "partial_match" | null;
  maxConfidence: number;
  avgConfidence: number;
} {
  const { query, answerLanguage, strictRagMode, ragPrompt, results } = input;
  const { maxConfidence, avgConfidence, hasStrongContext, hasSources } =
    computeConfidence(results);

  const shouldRecordGap =
    results.length === 0 || maxConfidence < 0.3 || avgConfidence < 0.25;
  const gapType: "no_results" | "low_confidence" | "partial_match" | null =
    !shouldRecordGap
      ? null
      : results.length === 0
        ? "no_results"
        : maxConfidence < 0.2
          ? "low_confidence"
          : "partial_match";

  const languageInstruction =
    answerLanguage === "en"
      ? "Answer in English only. Always answer in English even if the question is in another language."
      : "Answer in Indonesian only. Always answer in Indonesian even if the question is in another language.";

  const allowChatbot = !strictRagMode || isGreeting(query);
  const shouldUseRagContext = strictRagMode ? hasSources : hasStrongContext;
  const isStructuredQuery = results.some(
    (r) =>
      r.source_type === "structured_row" ||
      r.source_type === "structured_count" ||
      r.source_type === "search_context",
  );

  if (shouldUseRagContext) {
    const responseRules = `Response rules:\n- ${languageInstruction}\n- Do not include source citations in the response.\n- Provide only the final answer.`;
    const promptContent = `${responseRules}\n\n${ragPrompt}`;

    const systemPrompt = isStructuredQuery
      ? strictRagMode
        ? `You are a data retrieval assistant. The context below contains database records that match the user's search criteria. Present this data clearly and helpfully. If the data doesn't directly answer what the user asked, explain what data was found and how it relates to their query. ${languageInstruction} Respond with only the final answer and do not include source citations.`
        : `You are a helpful assistant presenting database search results. The context contains records matching the user's query. Summarize or present this data in a clear, helpful way. ${languageInstruction} Respond with only the final answer and do not include source citations.`
      : strictRagMode
        ? `You are a strict retrieval-augmented assistant. Use ONLY the provided context. If the context is insufficient, say you don't have enough information from the local data and suggest how to refine the query or import more data. ${languageInstruction} Respond with only the final answer and do not include source citations.`
        : `You are a helpful assistant answering questions based on the provided context. ${languageInstruction} Respond with only the final answer and do not include source citations.`;

    return {
      promptContent,
      systemPrompt,
      shouldRecordGap,
      gapType,
      maxConfidence,
      avgConfidence,
    };
  }

  if (allowChatbot) {
    return {
      promptContent: query,
      systemPrompt: `You are a helpful and friendly AI assistant. ${languageInstruction} When you don't have specific information about a topic, you can still provide general helpful information, clarify the question, or ask follow-up questions to better understand what the user needs. Be conversational and natural in your responses.`,
      shouldRecordGap,
      gapType,
      maxConfidence,
      avgConfidence,
    };
  }

  return {
    promptContent: `User question: ${query}\n\nNo reliable context was retrieved from the local collection.`,
    systemPrompt: `You are a local-data assistant. ${languageInstruction} If you don't have enough information from the retrieved local context, do NOT guess. Say you don't have enough information from the local data and ask the user to be more specific (e.g. add filters like category:/source:/kata kunci) or to import the relevant files into the collection.`,
    shouldRecordGap,
    gapType,
    maxConfidence,
    avgConfidence,
  };
}
