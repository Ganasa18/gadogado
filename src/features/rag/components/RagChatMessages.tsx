import { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp, Copy, Loader2, RotateCcw, Sparkles } from "lucide-react";
import AnimatedContainer from "../../../shared/components/AnimatedContainer";
import { cn } from "../../../utils/cn";
import type { ChatMessage } from "../types";
import { isLowConfidenceSources } from "../ragChatUtils";

export function RagChatMessages(props: {
  messages: ChatMessage[];
  isLoadingHistory: boolean;
  isLoading: boolean;
  selectedCollectionId: number | null;
  onRegenerate: (query: string) => void;
}) {
  const { messages, isLoadingHistory, isLoading, selectedCollectionId, onRegenerate } =
    props;

  const [showSources, setShowSources] = useState<{ [key: string]: boolean }>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, isLoading]);

  const toggleSource = (messageId: string) => {
    setShowSources((prev) => ({ ...prev, [messageId]: !prev[messageId] }));
  };

  const handleCopy = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    window.setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <div className="flex-1 overflow-y-auto px-6 py-6 scroll-smooth">
      {isLoadingHistory ? (
        <div className="h-full flex items-center justify-center">
          <div className="flex items-center gap-3">
            <Loader2 className="w-5 h-5 text-app-accent animate-spin" />
            <span className="text-sm text-app-text-muted">Loading conversation...</span>
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
                {selectedCollectionId ? "Ready to explore" : "Select a collection"}
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
            <AnimatedContainer key={message.id} animation="slideUp" className="w-full">
              <div
                className={cn(
                  "group relative flex flex-col gap-2",
                  message.type === "user" ? "items-end" : "items-start",
                )}>
                <div className="flex items-center gap-2 mb-1 px-1">
                  <span
                    className={cn(
                      "text-[10px] font-bold uppercase tracking-wider",
                      message.type === "user" ? "text-app-accent" : "text-app-text-muted",
                    )}>
                    {message.type === "user" ? "You" : "Assistant"}
                  </span>
                  <span className="text-[10px] text-app-border">-</span>
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
                  <p className="text-sm leading-7 whitespace-pre-wrap">{message.content}</p>

                  {message.type === "assistant" && message.sources && message.sources.length > 0 && (
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
                      </div>

                      {showSources[message.id] && (
                        <div className="mt-3 space-y-2">
                          {message.sources.map((source, idx) => {
                            // DB Citation Card - structured display for database results
                            if (source.source_type === "db_row") {
                              let columns: Record<string, unknown> = {};
                              try {
                                columns = JSON.parse(source.content);
                              } catch {
                                columns = {};
                              }

                              return (
                                <div
                                  key={idx}
                                  className="p-3 bg-gradient-to-br from-blue-500/5 to-app-bg/30 rounded-lg border border-blue-500/20 text-left">
                                  <div className="flex items-center justify-between gap-2 mb-2">
                                    <div className="flex items-center gap-2 flex-wrap">
                                      <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-blue-500/10 border border-blue-500/20">
                                        <span className="text-[9px] font-bold text-blue-400 uppercase tracking-wider">
                                          Database Row
                                        </span>
                                      </div>
                                      <span className="text-[10px] text-app-text font-medium">
                                        {source.doc_name}
                                      </span>
                                      {source.source_id > 0 && (
                                        <span className="text-[9px] text-app-text-muted/60">
                                          (ID: {source.source_id})
                                        </span>
                                      )}
                                    </div>
                                    <button
                                      onClick={() =>
                                        handleCopy(JSON.stringify(columns, null, 2), `${message.id}-${idx}`)
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

                                  {Object.keys(columns).length > 0 ? (
                                    <div className="space-y-1.5 bg-app-bg/20 rounded p-2">
                                      {Object.entries(columns).map(([key, value]) => (
                                        <div key={key} className="flex items-start gap-2 text-[11px]">
                                          <span className="text-blue-400 font-medium min-w-[80px] shrink-0">
                                            {key}:
                                          </span>
                                          <span className="text-app-text-muted break-all">
                                            {value !== null && value !== undefined
                                              ? String(value)
                                              : "(null)"}
                                          </span>
                                        </div>
                                      ))}
                                    </div>
                                  ) : (
                                    <p className="text-[11px] text-app-text-muted/60 italic">No data</p>
                                  )}
                                </div>
                              );
                            }

                            // Standard File Citation Card
                            return (
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
                                      <div
                                        className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider border"
                                        style={{
                                          backgroundColor:
                                            source.score >= 0.4
                                              ? "rgba(34, 197, 94, 0.1)"
                                              : source.score >= 0.3
                                                ? "rgba(251, 191, 36, 0.1)"
                                                : "rgba(239, 68, 68, 0.1)",
                                          borderColor:
                                            source.score >= 0.4
                                              ? "rgba(34, 197, 94, 0.2)"
                                              : source.score >= 0.3
                                                ? "rgba(251, 191, 36, 0.2)"
                                                : "rgba(239, 68, 68, 0.2)",
                                          color:
                                            source.score >= 0.4
                                              ? "#22c55e"
                                              : source.score >= 0.3
                                                ? "#fbbf24"
                                                : "#ef4444",
                                        }}>
                                        <span>{Math.round(source.score * 100)}%</span>
                                      </div>
                                    )}

                                    <button
                                      onClick={() => handleCopy(source.content, `${message.id}-${idx}`)}
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
                            );
                          })}
                        </div>
                      )}
                    </div>
                  )}
                </div>

                {message.type === "assistant" && (
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity px-2">
                    <button
                      onClick={() => handleCopy(message.content, message.id)}
                      className="p-1.5 rounded-full hover:bg-app-card text-app-text-muted hover:text-app-text transition-colors"
                      title="Copy Answer">
                      {copiedId === message.id ? (
                        <span className="text-[10px] text-green-500 font-bold">Copied</span>
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </button>
                    <button
                      onClick={() => {
                        if (message.query) onRegenerate(message.query);
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
            <span className="text-xs font-medium text-app-text-muted">Analyzing documents...</span>
          </div>
        </div>
      )}

      <div ref={messagesEndRef} />
    </div>
  );
}
