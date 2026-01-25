import { useState } from "react";
import {
  Box,
  ChevronDown,
  ChevronUp,
  MessageSquare,
  Sparkles,
  Trash2,
  Database,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import type { Conversation } from "../api";
import type { RagCollection, ChatMode } from "../types";

export function RagChatSidebar(props: {
  collections: RagCollection[];
  selectedCollectionId: number | null;
  onSelectCollection: (collectionId: number) => void;
  conversations: Conversation[];
  freeChatConversations: Conversation[];
  currentConversationId: number | null;
  currentFreeChatConversationId: number | null;
  onSelectConversation: (conversationId: number) => Promise<void>;
  onSelectFreeChatConversation: (conversationId: number) => Promise<void>;
  onDeleteConversation: (conversationId: number) => void;
  onOpenSessionConfig: () => void;
  retrievalSummary: string;
  chatMode: ChatMode;
  onChangeChatMode: (mode: ChatMode) => void;
  onNewFreeChat: () => void;
}) {
  const {
    collections,
    selectedCollectionId,
    onSelectCollection,
    conversations,
    freeChatConversations,
    currentConversationId,
    currentFreeChatConversationId,
    onSelectConversation,
    onSelectFreeChatConversation,
    onDeleteConversation,
    onOpenSessionConfig,
    retrievalSummary,
    chatMode,
    onChangeChatMode,
    onNewFreeChat,
  } = props;

  const [showConversations, setShowConversations] = useState(false);

  return (
    <aside className="w-[300px] border-r border-app-border/40 flex flex-col bg-app-bg">
      <div className="px-5 py-6 flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="p-2 rounded-lg bg-app-accent/10">
            <Sparkles className="w-5 h-5 text-app-accent" />
          </div>
          <div>
            <h2 className="text-sm font-semibold tracking-tight">RAG Assistant</h2>
            <div className="text-[10px] text-app-text-muted font-medium">Knowledge Base</div>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-3 min-h-0">
        {/* Mode Toggle Section */}
        <div className="mb-4">
          <div className="flex items-center justify-between px-3 mb-2">
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
              Mode
            </span>
          </div>

          <button
            onClick={() => onChangeChatMode("free")}
            className={cn(
              "w-full text-left px-3 py-2.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-3 group mb-1",
              chatMode === "free"
                ? "bg-purple-500 text-white shadow-md"
                : "text-app-text-muted hover:bg-app-card hover:text-app-text",
            )}>
            <MessageSquare className="w-4 h-4" />
            <span>Free Chat</span>
          </button>

          <button
            onClick={() => onChangeChatMode("rag")}
            className={cn(
              "w-full text-left px-3 py-2.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-3 group",
              chatMode === "rag"
                ? "bg-app-accent text-white shadow-md"
                : "text-app-text-muted hover:bg-app-card hover:text-app-text",
            )}>
            <Database className="w-4 h-4" />
            <span>RAG Mode</span>
          </button>
        </div>

        {/* Collections Section - Only show in RAG mode */}
        {chatMode === "rag" && (
          <>
            <div className="flex items-center justify-between px-3 mb-2 mt-2">
              <span className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                Collections
              </span>
            </div>

            <div className="space-y-1">
              {collections.length === 0 ? (
                <div className="px-3 py-4 text-center">
                  <p className="text-xs text-app-text-muted">No collections found</p>
                </div>
              ) : (
                collections.map((c) => (
                  <button
                    key={c.id}
                    onClick={() => onSelectCollection(c.id)}
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
          </>
        )}

        {/* Free Chat Conversations */}
        {chatMode === "free" && (
          <>
            <div className="flex items-center justify-between px-3 mb-2 mt-2">
              <span className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                Free Chat History
              </span>
              <button
                onClick={onNewFreeChat}
                className="text-[10px] text-purple-400 hover:text-purple-300 font-medium">
                + New
              </button>
            </div>

            <div className="space-y-1">
              {freeChatConversations.length === 0 ? (
                <div className="px-3 py-4 text-center">
                  <p className="text-xs text-app-text-muted">No chat history</p>
                </div>
              ) : (
                freeChatConversations.map((conv) => (
                  <button
                    key={conv.id}
                    onClick={() => void onSelectFreeChatConversation(conv.id)}
                    className={cn(
                      "w-full text-left px-3 py-2 rounded-lg text-xs transition-all duration-200 flex items-center justify-between gap-2 group",
                      currentFreeChatConversationId === conv.id
                        ? "bg-app-card border border-purple-500/30"
                        : "hover:bg-app-card/50",
                    )}>
                    <div className="flex items-center gap-2 min-w-0">
                      <MessageSquare className="w-3.5 h-3.5 text-app-text-muted shrink-0" />
                      <span className="truncate text-app-text-muted">{conv.title || "Untitled"}</span>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteConversation(conv.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-500 transition-all"
                      aria-label="Delete conversation">
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </button>
                ))
              )}
            </div>
          </>
        )}

        {/* RAG Chat History - Only show in RAG mode */}
        {chatMode === "rag" && selectedCollectionId && conversations.length > 0 && (
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
                    onClick={() => void onSelectConversation(conv.id)}
                    className={cn(
                      "w-full text-left px-3 py-2 rounded-lg text-xs transition-all duration-200 flex items-center justify-between gap-2 group",
                      currentConversationId === conv.id
                        ? "bg-app-card border border-app-accent/30"
                        : "hover:bg-app-card/50",
                    )}>
                    <div className="flex items-center gap-2 min-w-0">
                      <MessageSquare className="w-3.5 h-3.5 text-app-text-muted shrink-0" />
                      <span className="truncate text-app-text-muted">{conv.title || "Untitled"}</span>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteConversation(conv.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-500 transition-all"
                      aria-label="Delete conversation">
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      <div className="p-4 mt-auto border-t border-app-border/40 bg-app-card/20 backdrop-blur-sm space-y-5">
        <button
          type="button"
          onClick={onOpenSessionConfig}
          className="w-full flex items-center justify-between px-1 py-1 rounded-lg text-left transition-colors hover:bg-app-bg/40">
          <div className="min-w-0">
            <div className="text-xs font-medium text-app-subtext">Session config</div>
            <div className="text-[10px] text-app-text-muted mt-1 truncate">{retrievalSummary}</div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <span className="text-[10px] font-medium text-app-text-muted">Settings</span>
          </div>
        </button>
      </div>
    </aside>
  );
}
