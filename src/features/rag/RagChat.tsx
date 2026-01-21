import { useState, useEffect, useRef, useCallback } from "react";
import {
  Send,
  Sparkles,
  Copy,
  RotateCcw,
  Trash2,
  ChevronDown,
  ChevronUp,
  Loader2,
  Settings2,
  Box,
  MessageSquare,
  Plus,
} from "lucide-react";
import { useSettingsStore } from "../../store/settings";
import { useEnhanceMutation, useModelsQuery } from "../../hooks/useLlmApi";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import {
  ragQuery,
  listRagCollections,
  createConversation,
  addConversationMessage,
  getConversationMessages,
  listConversations,
  deleteConversation,
  recordRetrievalGap,
  type Conversation,
  type ConversationMessage as DbConversationMessage,
} from "./api";
import type {
  RagQueryResponse,
  ChatMessage,
  RagCollection,
  RagQueryResult,
} from "./types";
import AnimatedContainer from "../../shared/components/AnimatedContainer";
import { cn } from "../../utils/cn";

const GEMINI_MODELS = [
  "gemini-2.5-flash-lite",
  "gemini-2.0-flash-lite",
  "gemini-3-flash-preview",
];
const OPENAI_MODELS = ["gpt-4o", "gpt-4o-mini"];

export default function RagChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [collections, setCollections] = useState<RagCollection[]>([]);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<
    number | null
  >(null);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [selectedCollectionId, setSelectedCollectionId] = useState<
    number | null
  >(null);
  const [answerLanguage, setAnswerLanguage] = useState<"id" | "en">("id");
  const [showSources, setShowSources] = useState<{ [key: string]: boolean }>(
    {},
  );
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [showConversations, setShowConversations] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const { provider, model, localModels, setModel, setLocalModels } =
    useSettingsStore();
  const buildConfig = useLlmConfigBuilder();
  const { mutateAsync: enhanceAsync } = useEnhanceMutation();
  const isLocalProvider =
    provider === "local" || provider === "ollama" || provider === "llama_cpp";

  // Build config for fetching models (only for local provider)
  const localConfig = buildConfig({ maxTokens: 1024, temperature: 0.7 });
  const modelsQuery = useModelsQuery(localConfig, isLocalProvider);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Fetch and sync local models when using local provider
  useEffect(() => {
    if (!isLocalProvider) return;
    if (!modelsQuery.data) return;
    setLocalModels(modelsQuery.data);
    // If current model is not in the list, set it to the first available model
    if (modelsQuery.data.length > 0 && !modelsQuery.data.includes(model)) {
      setModel(modelsQuery.data[0]);
    }
  }, [isLocalProvider, modelsQuery.data, setLocalModels, setModel, model]);

  useEffect(() => {
    listRagCollections(50).then(setCollections).catch(console.error);
  }, []);

  // Load conversations when collection changes
  useEffect(() => {
    if (selectedCollectionId) {
      listConversations(selectedCollectionId)
        .then(setConversations)
        .catch(console.error);
    } else {
      setConversations([]);
    }
  }, [selectedCollectionId]);

  useEffect(() => {
    const savedCollection = localStorage.getItem("rag-selected-collection");
    if (savedCollection) {
      setSelectedCollectionId(parseInt(savedCollection));
    }
    const savedLanguage = localStorage.getItem("rag-answer-language");
    if (savedLanguage === "en" || savedLanguage === "id") {
      setAnswerLanguage(savedLanguage);
    }
  }, []);

  useEffect(() => {
    if (selectedCollectionId) {
      localStorage.setItem(
        "rag-selected-collection",
        selectedCollectionId.toString(),
      );
    }
  }, [selectedCollectionId]);

  useEffect(() => {
    localStorage.setItem("rag-answer-language", answerLanguage);
  }, [answerLanguage]);

  useEffect(() => {
    if (isLocalProvider) {
      if (localModels.length > 0 && !localModels.includes(model)) {
        setModel(localModels[0]);
      }
      return;
    }
    if (provider === "gemini" && !GEMINI_MODELS.includes(model)) {
      setModel(GEMINI_MODELS[0]);
      return;
    }
    if (provider === "openai" && !OPENAI_MODELS.includes(model)) {
      setModel(OPENAI_MODELS[0]);
    }
  }, [isLocalProvider, localModels, model, provider, setModel]);

  // Load conversation history when conversation changes
  const loadConversationHistory = useCallback(
    async (conversationId: number) => {
      setIsLoadingHistory(true);
      try {
        const dbMessages = await getConversationMessages(conversationId, 100);
        const chatMessages: ChatMessage[] = dbMessages.map(
          (msg: DbConversationMessage) => ({
            id: msg.id.toString(),
            type: msg.role as "user" | "assistant" | "system",
            content: msg.content,
            timestamp: new Date(msg.created_at),
            sources: msg.sources ? JSON.parse(msg.sources) : undefined,
          }),
        );
        setMessages(chatMessages);
      } catch (err) {
        console.error("Failed to load conversation history:", err);
      } finally {
        setIsLoadingHistory(false);
      }
    },
    [],
  );

  // Handle conversation selection
  const handleSelectConversation = useCallback(
    async (conversationId: number) => {
      setCurrentConversationId(conversationId);
      await loadConversationHistory(conversationId);
      setShowConversations(false);
    },
    [loadConversationHistory],
  );

  // Start new conversation
  const handleNewConversation = useCallback(() => {
    setCurrentConversationId(null);
    setMessages([]);
    setShowConversations(false);
  }, []);

  // Delete conversation
  const handleDeleteConversation = useCallback(
    async (conversationId: number, e: React.MouseEvent) => {
      e.stopPropagation();
      if (!confirm("Delete this conversation?")) return;

      try {
        await deleteConversation(conversationId);
        setConversations((prev) => prev.filter((c) => c.id !== conversationId));
        if (currentConversationId === conversationId) {
          setCurrentConversationId(null);
          setMessages([]);
        }
      } catch (err) {
        console.error("Failed to delete conversation:", err);
      }
    },
    [currentConversationId],
  );

  const handleSend = useCallback(async () => {
    const query = input.trim();
    if (!query || !selectedCollectionId) return;

    setInput("");
    setIsLoading(true);

    // Create conversation if this is the first message
    let conversationId = currentConversationId;
    if (!conversationId) {
      try {
        conversationId = await createConversation(
          selectedCollectionId,
          query.slice(0, 50) + (query.length > 50 ? "..." : ""),
        );
        setCurrentConversationId(conversationId);
        // Refresh conversation list
        const updatedConversations =
          await listConversations(selectedCollectionId);
        setConversations(updatedConversations);
      } catch (err) {
        console.error("Failed to create conversation:", err);
      }
    }

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      type: "user",
      content: query,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);

    // Persist user message to database
    if (conversationId) {
      try {
        await addConversationMessage(conversationId, "user", query);
      } catch (err) {
        console.error("Failed to persist user message:", err);
      }
    }

    try {
      const response: RagQueryResponse = await ragQuery({
        collection_id: selectedCollectionId,
        query,
        top_k: 5,
      });

      // Log retrieval gaps for analytics if results are low-confidence or missing
      const scores = response.results.map((r) => r.score || 0);
      const maxConfidence = scores.length > 0 ? Math.max(...scores) : 0;
      const avgConfidence =
        scores.length > 0
          ? scores.reduce((a, b) => a + b, 0) / scores.length
          : 0;
      // Use RAG context if: has results AND max confidence >= 0.4 (40%) AND avg confidence >= 0.25 (25%)
      // Scores >= 40% are considered reliable, scores < 30% are doubtful
      const hasStrongContext =
        response.results.length > 0 &&
        maxConfidence >= 0.4 &&
        avgConfidence >= 0.25;
      const hasSources = response.results.length > 0;

      // Record gap if: no results, low max confidence (<0.3), or low avg confidence (<0.25)
      if (
        response.results.length === 0 ||
        maxConfidence < 0.3 ||
        avgConfidence < 0.25
      ) {
        const gapType =
          response.results.length === 0
            ? "no_results"
            : maxConfidence < 0.2
              ? "low_confidence"
              : "partial_match";

        // Hash query for privacy (simple hash, not crypto)
        const queryHash = query
          .split("")
          .reduce((a, b) => {
            a = (a << 5) - a + b.charCodeAt(0);
            return a & a;
          }, 0)
          .toString(16);

        recordRetrievalGap({
          collection_id: selectedCollectionId,
          query_hash: queryHash,
          query_length: query.length,
          result_count: response.results.length,
          max_confidence: maxConfidence,
          avg_confidence: avgConfidence,
          gap_type: gapType,
        }).catch((err) =>
          console.error("Failed to record retrieval gap:", err),
        );
      }

      const languageInstruction =
        answerLanguage === "en"
          ? "Answer in English only. Always answer in English even if the question is in another language."
          : "Answer in Indonesian only. Always answer in Indonesian even if the question is in another language.";

      let promptContent: string;
      let systemPrompt: string;

      if (hasStrongContext) {
        // We have context from RAG - use the retrieved context
        const responseRules = `Response rules:\n- ${languageInstruction}\n- Do not include source citations in the response.\n- Provide only the final answer.`;
        promptContent = `${responseRules}\n\n${response.prompt}`;
        systemPrompt = `You are a helpful assistant answering questions based on the provided context. ${languageInstruction} Respond with only the final answer and do not include source citations.`;
      } else {
        // No context found - act as a general chatbot
        promptContent = query;
        systemPrompt = `You are a helpful and friendly AI assistant. ${languageInstruction} When you don't have specific information about a topic, you can still provide general helpful information, clarify the question, or ask follow-up questions to better understand what the user needs. Be conversational and natural in your responses.`;
      }

      // For local provider, ensure we use a valid model from localModels
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
        content: promptContent,
        system_prompt: systemPrompt,
      });

      const cleanedAnswer = llmResponse.result
        .replace(/\[Source:[^\]]+\]/g, "")
        .trim();

      const assistantMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: "assistant",
        content: cleanedAnswer,
        timestamp: new Date(),
        sources: hasSources ? response.results : undefined,
        query: query,
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // Persist assistant message to database with source chunk IDs
      if (conversationId) {
        try {
          const sourceIds = hasSources
            ? response.results.map((r) => r.source_id)
            : undefined;
          await addConversationMessage(
            conversationId,
            "assistant",
            cleanedAnswer,
            sourceIds,
          );
        } catch (err) {
          console.error("Failed to persist assistant message:", err);
        }
      }
    } catch (err) {
      console.error("Failed to query RAG:", err);
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: "system",
        content: `Error: ${
          err instanceof Error ? err.message : "Failed to query RAG"
        }`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  }, [
    input,
    selectedCollectionId,
    currentConversationId,
    answerLanguage,
    enhanceAsync,
    buildConfig,
  ]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCopy = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleClear = async () => {
    if (messages.length > 0 && confirm("Clear this conversation?")) {
      if (currentConversationId) {
        try {
          await deleteConversation(currentConversationId);
          setConversations((prev) =>
            prev.filter((c) => c.id !== currentConversationId),
          );
        } catch (err) {
          console.error("Failed to delete conversation:", err);
        }
      }
      setMessages([]);
      setCurrentConversationId(null);
    }
  };

  const isLowConfidenceSources = (sources?: RagQueryResult[]) => {
    if (!sources || sources.length === 0) return false;
    const scores = sources.map((source) => source.score || 0);
    const maxConfidence = Math.max(...scores);
    const avgConfidence =
      scores.reduce((sum, score) => sum + score, 0) / scores.length;
    // Consider low confidence if max < 0.4 (40%) or avg < 0.25 (25%)
    return maxConfidence < 0.4 || avgConfidence < 0.25;
  };

  const toggleSource = (messageId: string) => {
    setShowSources((prev) => ({
      ...prev,
      [messageId]: !prev[messageId],
    }));
  };

  return (
    <div className="flex h-full bg-app-bg text-app-text overflow-hidden font-sans">
      {/* Sidebar */}
      <aside className="w-[300px] border-r border-app-border/40 flex flex-col bg-app-bg">
        {/* Sidebar Header */}
        <div className="px-5 py-6 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-lg bg-app-accent/10">
              <Sparkles className="w-5 h-5 text-app-accent" />
            </div>
            <div>
              <h2 className="text-sm font-semibold tracking-tight">
                RAG Assistant
              </h2>
              <div className="text-[10px] text-app-text-muted font-medium">
                Knowledge Base
              </div>
            </div>
          </div>
        </div>

        {/* Collections List */}
        <div className="flex-1 overflow-y-auto px-3 min-h-0">
          <div className="flex items-center justify-between px-3 mb-2 mt-2">
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
              Collections
            </span>
          </div>

          <div className="space-y-1">
            {collections.length === 0 ? (
              <div className="px-3 py-4 text-center">
                <p className="text-xs text-app-text-muted">
                  No collections found
                </p>
              </div>
            ) : (
              collections.map((c) => (
                <button
                  key={c.id}
                  onClick={() => {
                    setSelectedCollectionId(c.id);
                    setCurrentConversationId(null);
                    setMessages([]);
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-3 group",
                    selectedCollectionId === c.id
                      ? "bg-app-accent text-white shadow-md shadow-app-accent/10"
                      : "text-app-text-muted hover:bg-app-card hover:text-app-text",
                  )}>
                  <Box
                    className={cn(
                      "w-4 h-4",
                      selectedCollectionId === c.id
                        ? "text-white/90"
                        : "text-app-text-muted group-hover:text-app-text",
                    )}
                  />
                  <span className="truncate">{c.name}</span>
                </button>
              ))
            )}
          </div>

          {/* Conversation History */}
          {selectedCollectionId && conversations.length > 0 && (
            <div className="mt-6">
              <button
                onClick={() => setShowConversations(!showConversations)}
                className="flex items-center justify-between w-full px-3 mb-2">
                <span className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                  Chat History ({conversations.length})
                </span>
                {showConversations ? (
                  <ChevronUp className="w-3.5 h-3.5 text-app-subtext" />
                ) : (
                  <ChevronDown className="w-3.5 h-3.5 text-app-subtext" />
                )}
              </button>

              {showConversations && (
                <div className="space-y-1">
                  {conversations.map((conv) => (
                    <button
                      key={conv.id}
                      onClick={() => handleSelectConversation(conv.id)}
                      className={cn(
                        "w-full text-left px-3 py-2 rounded-lg text-xs transition-all duration-200 flex items-center justify-between gap-2 group",
                        currentConversationId === conv.id
                          ? "bg-app-card border border-app-accent/30"
                          : "hover:bg-app-card/50",
                      )}>
                      <div className="flex items-center gap-2 min-w-0">
                        <MessageSquare className="w-3.5 h-3.5 text-app-text-muted shrink-0" />
                        <span className="truncate text-app-text-muted">
                          {conv.title || "Untitled"}
                        </span>
                      </div>
                      <button
                        onClick={(e) => handleDeleteConversation(conv.id, e)}
                        className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-500 transition-all">
                        <Trash2 className="w-3 h-3" />
                      </button>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Session Config */}
        <div className="p-4 mt-auto border-t border-app-border/40 bg-app-card/20 backdrop-blur-sm space-y-5">
          <div className="flex items-center gap-2 text-app-subtext mb-1">
            <Settings2 className="w-3.5 h-3.5" />
            <span className="text-[10px] font-bold uppercase tracking-wider">
              Session Config
            </span>
          </div>

          <div className="space-y-3">
            {/* Model Select */}
            <div className="space-y-1.5">
              <label className="text-[10px] text-app-text-muted font-medium ml-1">
                Model ({provider})
              </label>
              <div className="relative">
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="w-full appearance-none bg-app-card border border-app-border/60 rounded-lg py-2 pl-3 pr-8 text-xs outline-none focus:border-app-accent/50 focus:ring-1 focus:ring-app-accent/20 transition-all text-app-text font-medium">
                  {provider === "local" ||
                  provider === "ollama" ||
                  provider === "llama_cpp" ? (
                    localModels && localModels.length > 0 ? (
                      localModels.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))
                    ) : (
                      <option value={model} disabled>
                        No local models found
                      </option>
                    )
                  ) : provider === "gemini" ? (
                    <>
                      <option value="gemini-2.5-flash-lite">
                        gemini-2.5-flash-lite
                      </option>
                      <option value="gemini-2.0-flash-lite">
                        gemini-2.0-flash-lite
                      </option>
                      <option value="gemini-3-flash-preview">
                        gemini-3-flash-preview
                      </option>
                    </>
                  ) : provider === "openai" ? (
                    <>
                      <option value="gpt-4o">gpt-4o</option>
                      <option value="gpt-4o-mini">gpt-4o-mini</option>
                    </>
                  ) : (
                    <option value={model}>{model || "Custom Model"}</option>
                  )}
                </select>
                <ChevronDown className="absolute right-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-app-text-muted pointer-events-none" />
              </div>
            </div>

            {/* Language Select */}
            <div className="space-y-1.5">
              <label className="text-[10px] text-app-text-muted font-medium ml-1">
                Response Language
              </label>
              <div className="flex items-center gap-1 p-1 bg-app-card border border-app-border/60 rounded-lg">
                <button
                  onClick={() => setAnswerLanguage("en")}
                  className={cn(
                    "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all flex items-center justify-center gap-1.5",
                    answerLanguage === "en"
                      ? "bg-app-accent text-white shadow-sm"
                      : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                  )}>
                  English
                </button>
                <button
                  onClick={() => setAnswerLanguage("id")}
                  className={cn(
                    "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all flex items-center justify-center gap-1.5",
                    answerLanguage === "id"
                      ? "bg-app-accent text-white shadow-sm"
                      : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                  )}>
                  Indonesia
                </button>
              </div>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Chat Area */}
      <main className="flex-1 flex flex-col min-w-0 bg-gradient-to-br from-app-bg via-app-bg to-app-card/20 relative">
        <div className="absolute top-0 left-0 w-full h-[150px] bg-gradient-to-b from-app-card/10 to-transparent pointer-events-none" />

        {/* Header / Top Bar in Main Area */}
        {selectedCollectionId && (
          <div className="absolute top-4 right-6 z-10 flex items-center gap-2">
            <button
              onClick={handleNewConversation}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-app-accent text-white rounded-full text-[10px] font-medium hover:brightness-110 transition-all shadow-md shadow-app-accent/20">
              <Plus className="w-3 h-3" />
              New Chat
            </button>
            <button
              onClick={handleClear}
              disabled={messages.length === 0}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-app-card/60 backdrop-blur-md border border-app-border/40 rounded-full text-[10px] font-medium text-app-text-muted hover:text-red-400 hover:border-red-400/30 transition-all disabled:opacity-0">
              <Trash2 className="w-3 h-3" />
              Clear
            </button>
          </div>
        )}

        <div className="flex-1 overflow-y-auto px-6 py-6 scroll-smooth">
          {isLoadingHistory ? (
            <div className="h-full flex items-center justify-center">
              <div className="flex items-center gap-3">
                <Loader2 className="w-5 h-5 text-app-accent animate-spin" />
                <span className="text-sm text-app-text-muted">
                  Loading conversation...
                </span>
              </div>
            </div>
          ) : messages.length === 0 ? (
            <AnimatedContainer
              animation="fadeIn"
              className="h-full flex flex-col items-center justify-center p-8">
              <div className="max-w-md w-full text-center space-y-6">
                <div className="w-24 h-24 mx-auto rounded-[32px] bg-gradient-to-br from-app-accent/20 to-app-card border border-app-border/50 flex items-center justify-center shadow-xl shadow-app-accent/5 backdrop-blur-sm">
                  <Sparkles className="w-12 h-12 text-app-accent" />
                </div>
                <div className="space-y-2">
                  <h3 className="text-2xl font-bold text-app-text tracking-tight">
                    {selectedCollectionId
                      ? "Ready to explore"
                      : "Select a collection"}
                  </h3>
                  <p className="text-sm text-app-text-muted leading-relaxed max-w-[300px] mx-auto">
                    {selectedCollectionId
                      ? "Ask me anything about the documents in your collection. Your chat history is automatically saved."
                      : "Choose a collection from the sidebar to start your knowledge discovery session."}
                  </p>
                </div>
              </div>
            </AnimatedContainer>
          ) : (
            <div className="max-w-4xl mx-auto space-y-8 pb-4">
              {messages.map((message) => (
                <AnimatedContainer
                  key={message.id}
                  animation="slideUp"
                  className="w-full">
                  <div
                    className={cn(
                      "group relative flex flex-col gap-2",
                      message.type === "user" ? "items-end" : "items-start",
                    )}>
                    <div className="flex items-center gap-2 mb-1 px-1">
                      <span
                        className={cn(
                          "text-[10px] font-bold uppercase tracking-wider",
                          message.type === "user"
                            ? "text-app-accent"
                            : "text-app-text-muted",
                        )}>
                        {message.type === "user" ? "You" : "Assistant"}
                      </span>
                      <span className="text-[10px] text-app-border">•</span>
                      <span className="text-[10px] text-app-text-muted/60">
                        {new Date(message.timestamp).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>

                    <div
                      className={cn(
                        "rounded-2xl p-5 max-w-[85%] shadow-sm",
                        message.type === "user"
                          ? "bg-app-accent text-white rounded-tr-sm"
                          : message.type === "system"
                            ? "bg-red-500/10 border border-red-500/20 text-red-500 text-sm"
                            : "bg-app-card border border-app-border/40 text-app-text rounded-tl-sm",
                      )}>
                      <p className="text-sm leading-7 whitespace-pre-wrap">
                        {message.content}
                      </p>

                      {/* Sources Section */}
                      {message.type === "assistant" &&
                        message.sources &&
                        message.sources.length > 0 && (
                          <div className="mt-4 pt-3 border-t border-white/10 opacity-90">
                            <div className="flex items-center justify-between gap-2">
                              <div className="flex items-center gap-2">
                                <button
                                  onClick={() => toggleSource(message.id)}
                                  className="flex items-center gap-1.5 text-xs text-app-subtext hover:text-app-accent transition-colors font-medium">
                                  {showSources[message.id] ? (
                                    <ChevronUp className="w-3.5 h-3.5" />
                                  ) : (
                                    <ChevronDown className="w-3.5 h-3.5" />
                                  )}
                                  {showSources[message.id]
                                    ? "Hide References"
                                    : `${message.sources.length} References`}
                                </button>
                                {isLowConfidenceSources(message.sources) && (
                                  <span className="text-[9px] font-bold uppercase tracking-wider text-amber-400 bg-amber-400/10 border border-amber-400/20 px-1.5 py-0.5 rounded">
                                    Low confidence
                                  </span>
                                )}
                              </div>
                              {!showSources[message.id] && (
                                <div className="flex items-center gap-1.5 text-[9px] text-app-text-muted">
                                  <span>Max:</span>
                                  <span className={cn(
                                    "font-bold",
                                    (() => {
                                      const maxScore = Math.max(...message.sources.map(s => s.score || 0));
                                      return maxScore >= 0.4 ? "text-green-500" : maxScore >= 0.3 ? "text-amber-400" : "text-red-400";
                                    })()
                                  )}>
                                    {Math.round((Math.max(...message.sources.map(s => s.score || 0))) * 100)}%
                                  </span>
                                </div>
                              )}
                            </div>

                            {showSources[message.id] && (
                              <div className="mt-3 space-y-2">
                                {message.sources.map((source, idx) => (
                                  <div
                                    key={idx}
                                    className="p-3 bg-app-bg/30 rounded-lg border border-app-border/20 text-left">
                                    <div className="flex items-center justify-between gap-2 mb-1.5">
                                      <div className="flex items-center gap-2 flex-wrap min-w-0">
                                        <div className="flex items-center gap-1.5 p-1 rounded bg-app-bg/50 border border-app-border/10">
                                          <span className="text-[9px] font-bold text-app-accent uppercase tracking-wider">
                                            {source.source_type}
                                          </span>
                                        </div>
                                        {source.doc_name && (
                                          <span
                                            className="text-[10px] text-app-text-muted truncate max-w-[150px]"
                                            title={source.doc_name}>
                                            {source.doc_name}
                                          </span>
                                        )}
                                      </div>
                                      <div className="flex items-center gap-2 shrink-0">
                                        {source.score !== null && source.score !== undefined && (
                                          <div className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider border"
                                            style={{
                                              backgroundColor: source.score >= 0.4
                                                ? 'rgba(34, 197, 94, 0.1)'
                                                : source.score >= 0.3
                                                  ? 'rgba(251, 191, 36, 0.1)'
                                                  : 'rgba(239, 68, 68, 0.1)',
                                              borderColor: source.score >= 0.4
                                                ? 'rgba(34, 197, 94, 0.2)'
                                                : source.score >= 0.3
                                                  ? 'rgba(251, 191, 36, 0.2)'
                                                  : 'rgba(239, 68, 68, 0.2)',
                                              color: source.score >= 0.4
                                                ? '#22c55e'
                                                : source.score >= 0.3
                                                  ? '#fbbf24'
                                                  : '#ef4444'
                                            }}>
                                            <span>{Math.round(source.score * 100)}%</span>
                                          </div>
                                        )}
                                        <button
                                          onClick={() =>
                                            handleCopy(
                                              source.content,
                                              `${message.id}-${idx}`,
                                            )
                                          }
                                          className="text-app-subtext hover:text-app-accent transition-colors shrink-0">
                                          {copiedId === `${message.id}-${idx}` ? (
                                            <div className="text-[10px] text-green-500 font-medium animate-pulse">
                                              Copied
                                            </div>
                                          ) : (
                                            <Copy className="w-3 h-3" />
                                          )}
                                        </button>
                                      </div>
                                    </div>
                                    <p className="text-[11px] text-app-text-muted/80 leading-relaxed line-clamp-3 hover:line-clamp-none transition-all cursor-default">
                                      {source.content}
                                    </p>
                                  </div>
                                ))}
                              </div>
                            )}
                          </div>
                        )}
                    </div>

                    {/* Action Buttons */}
                    {message.type === "assistant" && (
                      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity px-2">
                        <button
                          onClick={() =>
                            handleCopy(message.content, message.id)
                          }
                          className="p-1.5 rounded-full hover:bg-app-card text-app-text-muted hover:text-app-text transition-colors"
                          title="Copy Answer">
                          {copiedId === message.id ? (
                            <span className="text-[10px] text-green-500 font-bold">
                              ✓
                            </span>
                          ) : (
                            <Copy className="w-3.5 h-3.5" />
                          )}
                        </button>
                        <button
                          onClick={() => {
                            if (message.query) {
                              setInput(message.query);
                              inputRef.current?.focus();
                            }
                          }}
                          className="p-1.5 rounded-full hover:bg-app-card text-app-text-muted hover:text-app-text transition-colors"
                          title="Regenerate/Edit Question">
                          <RotateCcw className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    )}
                  </div>
                </AnimatedContainer>
              ))}
            </div>
          )}

          {isLoading && (
            <div className="max-w-4xl mx-auto pl-1">
              <div className="flex items-center gap-3 animate-pulse">
                <div className="w-6 h-6 rounded-full bg-app-accent/20 flex items-center justify-center">
                  <Loader2 className="w-3.5 h-3.5 text-app-accent animate-spin" />
                </div>
                <span className="text-xs font-medium text-app-text-muted">
                  Analyzing documents...
                </span>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Input Area */}
        <div className="p-6 bg-gradient-to-t from-app-bg via-app-bg to-transparent">
          <div className="max-w-4xl mx-auto relative group">
            <div
              className={cn(
                "absolute -inset-0.5 rounded-2xl opacity-0 transition-opacity duration-300",
                selectedCollectionId
                  ? "group-hover:opacity-100"
                  : "group-hover:opacity-0",
              )}
            />
            <div className="relative flex gap-3 bg-app-card border border-app-border/60 rounded-xl p-2 shadow-lg shadow-black/5 transition-all focus-within:border-app-accent/50 focus-within:ring-1 focus-within:ring-app-accent/20">
              <div className="flex-1 flex items-center">
                <textarea
                  ref={inputRef}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={
                    selectedCollectionId
                      ? "Ask follow-up questions..."
                      : "Select a collection to start..."
                  }
                  disabled={!selectedCollectionId || isLoading}
                  className="w-full bg-transparent border-none text-sm outline-none px-3 py-2 text-app-text disabled:opacity-50 disabled:cursor-not-allowed resize-none placeholder:text-app-text-muted/60"
                  rows={1}
                  style={{ minHeight: "44px", maxHeight: "150px" }}
                />
              </div>
              <button
                onClick={handleSend}
                disabled={!input.trim() || !selectedCollectionId || isLoading}
                className="px-4 bg-app-accent text-white rounded-lg hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center shadow-md shadow-app-accent/20">
                {isLoading ? (
                  <Loader2 className="w-5 h-5 animate-spin" />
                ) : (
                  <Send className="w-5 h-5" />
                )}
              </button>
            </div>
            <div className="flex items-center justify-center gap-4 mt-2.5">
              <span className="text-[10px] text-app-text-muted/70 flex items-center gap-1.5">
                Enter to send
              </span>
              <span className="text-[10px] text-app-text-muted/70 flex items-center gap-1.5">
                Shift + Enter for new line
              </span>
              <span className="text-[10px] text-app-text-muted/70 flex items-center gap-1.5">
                Chat auto-saved
              </span>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
