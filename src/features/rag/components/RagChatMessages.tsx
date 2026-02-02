import { useEffect, useRef, useState } from "react";
import { Loader2, MessageCircleMore } from "lucide-react";
import AnimatedContainer from "../../../shared/components/AnimatedContainer";
import type { ChatMessage } from "../types";
import { MessageItem } from "./MessageItem";

interface RagChatMessagesProps {
  messages: ChatMessage[];
  isLoadingHistory: boolean;
  isLoading: boolean;
  selectedCollectionId: number | null;
  onRegenerate: (query: string) => void;
  onRegenerateWithTemplate?: (query: string, templateId: number) => void;
}

export function RagChatMessages({
  messages = [],
  isLoadingHistory,
  isLoading,
  selectedCollectionId,
  onRegenerate,
  onRegenerateWithTemplate,
}: RagChatMessagesProps) {
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [typedIds, setTypedIds] = useState<Set<string>>(new Set());
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Initialize typedIds with current messages when history is loaded to prevent re-animation
  useEffect(() => {
    if (!isLoadingHistory && messages.length > 0 && typedIds.size === 0) {
      setTypedIds(new Set(messages.map((m) => m.id)));
    }
  }, [isLoadingHistory, messages, typedIds.size]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, isLoading]);

  const handleCopy = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    window.setTimeout(() => setCopiedId(null), 2000);
  };

  const handleTypewriterComplete = (id: string) => {
    setTypedIds((prev) => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  };

  if (isLoadingHistory) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8 bg-app-bg">
        <div className="relative">
          <Loader2 className="w-12 h-12 text-app-accent animate-spin" />
        </div>
        <div className="mt-4 flex flex-col items-center">
          <span className="text-sm font-bold text-app-text tracking-widest uppercase">Initializing Session</span>
          <span className="text-[11px] text-app-subtext mt-1">Retrieving conversation history...</span>
        </div>
      </div>
    );
  }

  if (messages.length === 0) {
    return (
      <div className="flex-1 overflow-y-auto px-6 py-6 scroll-smooth">
        <AnimatedContainer
          animation="fadeIn"
          className="h-full flex flex-col items-center justify-center p-8">
          <div className="max-w-md w-full text-center space-y-8">
            <div className="relative w-32 h-32 mx-auto">
              <div className="relative w-full h-full rounded-[40px] bg-app-card border border-app-border/40 flex items-center justify-center backdrop-blur-md">
                <MessageCircleMore className="w-16 h-16 text-app-accent" />
              </div>
            </div>
            <div className="space-y-3">
              <h3 className="text-2xl font-black text-app-text tracking-tighter uppercase italic">
                {selectedCollectionId ? "Ready to analyze" : "Knowledge Base"}
              </h3>
              <p className="text-[13px] text-app-subtext leading-relaxed max-w-[320px] mx-auto font-medium">
                {selectedCollectionId
                  ? "I've indexed your collection. Ask me anything and I'll find the most relevant information for you."
                  : "Select a collection from the sidebar to begin your knowledge-powered conversation."}
              </p>
            </div>
          </div>
        </AnimatedContainer>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-3 sm:px-6 py-4 scroll-smooth space-y-6">
      <div className="max-w-5xl mx-auto space-y-6">
        {messages.map((message, index) => (
          <AnimatedContainer key={message.id} animation="slideUp" className="w-full">
            <MessageItem
              message={message}
              isLatest={index === messages.length - 1 && !typedIds.has(message.id)}
              onRegenerate={onRegenerate}
              onRegenerateWithTemplate={onRegenerateWithTemplate}
              onCopy={handleCopy}
              copiedId={copiedId}
              onTypewriterComplete={() => handleTypewriterComplete(message.id)}
            />
          </AnimatedContainer>
        ))}
      </div>

      {isLoading && (
        <div className="max-w-5xl mx-auto pl-4">
          <div className="flex items-center gap-4">
            <div className="relative">
              <div className="w-8 h-8 rounded-xl bg-app-accent/5 flex items-center justify-center border border-app-accent/20">
                <Loader2 className="w-4 h-4 text-app-accent animate-spin" />
              </div>
            </div>
            <div className="flex flex-col">
              <span className="text-[11px] font-bold text-app-accent uppercase tracking-widest">Assistant is thinking</span>
              <span className="text-[10px] text-app-subtext font-medium">Scanning knowledge base...</span>
            </div>
          </div>
        </div>
      )}

      <div ref={messagesEndRef} className="h-4" />
    </div>
  );
}
