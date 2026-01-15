import { useState, useEffect, useRef, useCallback } from "react";
import {
  Send,
  FileText,
  Database,
  Sparkles,
  Copy,
  RotateCcw,
  Trash2,
  ChevronDown,
  ChevronUp,
  Loader2,
} from "lucide-react";
import { useSettingsStore } from "../../store/settings";
import { useEnhanceMutation } from "../../hooks/useLlmApi";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import { ragQuery, listRagCollections } from "./api";
import type { RagQueryResponse, ChatMessage, RagCollection } from "./types";
import AnimatedContainer from "../../shared/components/AnimatedContainer";
import { cn } from "../../utils/cn";

export default function RagChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [collections, setCollections] = useState<RagCollection[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(null);
  const [answerLanguage, setAnswerLanguage] = useState<"id" | "en">("id");
  const [showSources, setShowSources] = useState<{ [key: string]: boolean }>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const { provider, model, apiKey, baseUrl } = useSettingsStore();
  const buildConfig = useLlmConfigBuilder();
  const { mutateAsync: enhanceAsync } = useEnhanceMutation();

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    listRagCollections(50).then(setCollections).catch(console.error);
  }, []);

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
      localStorage.setItem("rag-selected-collection", selectedCollectionId.toString());
    }
  }, [selectedCollectionId]);

  useEffect(() => {
    localStorage.setItem("rag-answer-language", answerLanguage);
  }, [answerLanguage]);

  const handleSend = useCallback(async () => {
    const query = input.trim();
    if (!query || !selectedCollectionId) return;

    setInput("");
    setIsLoading(true);

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      type: "user",
      content: query,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);

    try {
      const response: RagQueryResponse = await ragQuery({
        collection_id: selectedCollectionId,
        query,
        top_k: 5,
      });

      if (response.results.length === 0) {
        const assistantMessage: ChatMessage = {
          id: (Date.now() + 1).toString(),
          type: "assistant",
          content:
            "Tidak ditemukan konteks yang relevan di koleksi ini untuk menjawab pertanyaan tersebut.",
          timestamp: new Date(),
          sources: response.results,
          query: query,
        };
        setMessages((prev) => [...prev, assistantMessage]);
        return;
      }

      const languageInstruction =
        answerLanguage === "en"
          ? "Answer in English."
          : "Answer in Indonesian.";

      const llmResponse = await enhanceAsync({
        config: buildConfig({ maxTokens: 900, temperature: 0.2 }),
        content: response.prompt,
        system_prompt: `You are a helpful assistant. Follow the instructions in the user prompt exactly. ${languageInstruction} Respond with only the final answer and do not include source citations.`,
      });

      const cleanedAnswer = llmResponse.result
        .replace(/\[Source:[^\]]+\]/g, "")
        .trim();

      const assistantMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: "assistant",
        content: cleanedAnswer,
        timestamp: new Date(),
        sources: response.results,
        query: query,
      };

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (err) {
      console.error("Failed to query RAG:", err);
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: "system",
        content: `Error: ${err instanceof Error ? err.message : "Failed to query RAG"}`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  }, [input, selectedCollectionId, answerLanguage, enhanceAsync, buildConfig]);

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

  const handleClear = () => {
    if (messages.length > 0 && confirm("Clear all messages?")) {
      setMessages([]);
    }
  };

  const toggleSource = (messageId: string) => {
    setShowSources((prev) => ({
      ...prev,
      [messageId]: !prev[messageId],
    }));
  };

  return (
    <div className="flex h-full bg-app-bg text-app-text">
      <aside className="w-80 border-r border-app-border flex flex-col">
        <div className="p-4 border-b border-app-border">
          <div className="flex items-center gap-2 mb-3">
            <Sparkles className="w-5 h-5 text-app-accent" />
            <h2 className="text-lg font-semibold">RAG Chat</h2>
          </div>
          <div className="text-xs text-app-text-muted">
            Query your knowledge base with AI assistance
          </div>
        </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            <div>
              <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                Select Collection
              </label>
              <select
                value={selectedCollectionId ?? ""}
                onChange={(e) => setSelectedCollectionId(Number(e.target.value))}
                className="w-full bg-app-card border border-app-border rounded-lg p-2.5 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all">
                <option value="" disabled>
                  Choose a collection
                </option>
                {collections.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                Answer Language
              </label>
              <select
                value={answerLanguage}
                onChange={(e) =>
                  setAnswerLanguage(e.target.value === "en" ? "en" : "id")
                }
                className="w-full bg-app-card border border-app-border rounded-lg p-2.5 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all">
                <option value="id">Indonesian</option>
                <option value="en">English</option>
              </select>
            </div>


          <div className="p-3 bg-app-card rounded-lg border border-app-border/50">
            <div className="flex items-center gap-2 mb-2">
              <Database className="w-4 h-4 text-app-subtext" />
              <span className="text-xs font-medium">LLM Settings</span>
            </div>
            <div className="space-y-1.5 text-[10px] text-app-text-muted">
              <div className="flex justify-between">
                <span>Provider</span>
                <span className="text-app-text font-medium">{provider}</span>
              </div>
              <div className="flex justify-between">
                <span>Model</span>
                <span className="text-app-text font-medium">{model}</span>
              </div>
              {baseUrl && (
                <div className="flex justify-between">
                  <span>Base URL</span>
                  <span className="text-app-text font-medium truncate max-w-[120px]">
                    {baseUrl}
                  </span>
                </div>
              )}
              {apiKey && (
                <div className="flex justify-between">
                  <span>API Key</span>
                  <span className="text-app-text font-medium">
                    •••••••••••••
                  </span>
                </div>
              )}
            </div>
          </div>

          <div className="p-3 bg-app-accent/5 rounded-lg border border-app-accent/10">
            <div className="flex items-center gap-2 mb-2">
              <FileText className="w-4 h-4 text-app-accent" />
              <span className="text-xs font-medium text-app-accent">
                Chat History
              </span>
            </div>
            <div className="text-[10px] text-app-text-muted">
              {messages.length} {messages.length === 1 ? "message" : "messages"}
            </div>
          </div>
        </div>

        <div className="p-4 border-t border-app-border">
          <button
            onClick={handleClear}
            disabled={messages.length === 0}
            className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-app-card border border-app-border rounded-lg text-sm hover:bg-red-500/5 hover:border-red-500/30 hover:text-red-500 disabled:opacity-50 disabled:cursor-not-allowed transition-all">
            <Trash2 className="w-4 h-4" />
            Clear Chat
          </button>
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0">
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {messages.length === 0 ? (
            <AnimatedContainer animation="fadeIn" className="h-full flex items-center justify-center">
              <div className="text-center max-w-md">
                <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-app-card border border-app-border flex items-center justify-center">
                  <Sparkles className="w-10 h-10 text-app-text-muted/70" />
                </div>
                <h3 className="text-xl font-semibold text-app-text mb-2">
                  Start a conversation
                </h3>
                <p className="text-app-text-muted text-sm">
                  Select a collection and ask questions about your knowledge base
                </p>
              </div>
            </AnimatedContainer>
          ) : (
            messages.map((message) => (
              <AnimatedContainer key={message.id} animation="slideUp" className="w-full">
                <div
                  className={cn(
                    "rounded-2xl p-5 max-w-[85%]",
                    message.type === "user"
                      ? "ml-auto bg-app-accent text-white"
                      : message.type === "system"
                      ? "mx-auto bg-red-500/10 border border-red-500/20 text-red-500 text-sm"
                      : "mr-auto bg-app-card border border-app-border/50"
                  )}>
                  <div className="flex items-start justify-between gap-3 mb-2">
                    <span className="text-[10px] uppercase tracking-wider font-medium opacity-60">
                      {message.type === "user"
                        ? "You"
                        : message.type === "system"
                        ? "System"
                        : "Assistant"}
                    </span>
                    <span className="text-[10px] opacity-50">
                      {new Date(message.timestamp).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>

                  <p className="text-sm leading-relaxed whitespace-pre-wrap">
                    {message.content}
                  </p>

                  {message.type === "assistant" && message.sources && message.sources.length > 0 && (
                    <div className="mt-4 pt-4 border-t border-app-border/50">
                      <button
                        onClick={() => toggleSource(message.id)}
                        className="flex items-center gap-2 text-xs text-app-subtext hover:text-app-accent transition-colors mb-3">
                        {showSources[message.id] ? (
                          <ChevronUp className="w-3.5 h-3.5" />
                        ) : (
                          <ChevronDown className="w-3.5 h-3.5" />
                        )}
                        {showSources[message.id]
                          ? "Hide Sources"
                          : `Show ${message.sources.length} Source${message.sources.length > 1 ? "s" : ""}`}
                      </button>

                      {showSources[message.id] && (
                        <div className="space-y-2">
                          {message.sources.map((source, idx) => (
                            <div
                              key={idx}
                              className="p-3 bg-app-bg/50 rounded-lg border border-app-border/30">
                              <div className="flex items-start justify-between gap-2 mb-2">
                                <span className="text-[10px] font-medium text-app-accent uppercase tracking-wider">
                                  {source.source_type}
                                </span>
                                <button
                                  onClick={() =>
                                    handleCopy(source.content, `${message.id}-${idx}`)
                                  }
                                  className="text-app-subtext hover:text-app-accent transition-colors">
                                  {copiedId === `${message.id}-${idx}` ? (
                                    <span className="text-[10px] text-app-success">
                                      Copied!
                                    </span>
                                  ) : (
                                    <Copy className="w-3 h-3" />
                                  )}
                                </button>
                              </div>
                              <p className="text-xs text-app-text leading-relaxed">
                                {source.content}
                              </p>
                              {source.score && (
                                <div className="mt-2 text-[10px] text-app-text-muted">
                                  Relevance: {(source.score * 100).toFixed(1)}%
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {message.type === "assistant" && (
                    <div className="mt-4 flex gap-2">
                      <button
                        onClick={() => handleCopy(message.content, message.id)}
                        className="p-1.5 text-app-subtext hover:text-app-accent transition-colors">
                        {copiedId === message.id ? (
                          <span className="text-[10px] text-app-success">
                            Copied!
                          </span>
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                      <button
                        onClick={() => {
                          if (message.query) {
                            setInput(message.query);
                            inputRef.current?.focus();
                          }
                        }}
                        className="p-1.5 text-app-subtext hover:text-app-accent transition-colors"
                        title="Requery">
                        <RotateCcw className="w-4 h-4" />
                      </button>
                    </div>
                  )}
                </div>
              </AnimatedContainer>
            ))
          )}

          {isLoading && (
            <AnimatedContainer animation="fadeIn">
              <div className="w-full">
                <div className="mr-auto bg-app-card border border-app-border/50 rounded-2xl p-5 max-w-[85%]">
                  <div className="flex items-center gap-3">
                    <Loader2 className="w-5 h-5 text-app-accent animate-spin" />
                    <span className="text-sm text-app-text-muted">
                      Thinking...
                    </span>
                  </div>
                </div>
              </div>
            </AnimatedContainer>
          )}

          <div ref={messagesEndRef} />
        </div>

        <div className="p-4 border-t border-app-border bg-app-card/30">
          <div className="flex gap-3 max-w-4xl mx-auto">
            <textarea
              ref={inputRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={
                selectedCollectionId
                  ? "Ask a question about your knowledge base..."
                  : "Select a collection to start chatting..."
              }
              disabled={!selectedCollectionId || isLoading}
              className="flex-1 bg-app-bg border border-app-border rounded-xl px-4 py-3 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 disabled:opacity-50 disabled:cursor-not-allowed resize-none transition-all"
              rows={1}
              style={{ minHeight: "44px", maxHeight: "150px" }}
            />
            <button
              onClick={handleSend}
              disabled={!input.trim() || !selectedCollectionId || isLoading}
              className="px-5 bg-app-accent text-white rounded-xl hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center">
              {isLoading ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <Send className="w-5 h-5" />
              )}
            </button>
          </div>
          <div className="text-center mt-2 text-[10px] text-app-text-muted">
            Press Enter to send, Shift + Enter for new line
          </div>
        </div>
      </main>
    </div>
  );
}
