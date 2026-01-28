import {
  MessageSquare,
  Trash2,
  Database,
  Settings,
  Plus,
  MessageCircleMore,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import { Select } from "../../../shared/components/Select";
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
    chatMode,
    onChangeChatMode,
    onNewFreeChat,
  } = props;

  return (
    <aside className="w-[280px] border-r border-app-border/30 flex flex-col bg-app-bg select-none">
      {/* Brand Header */}
      <div className="p-6">
        <div className="flex items-center gap-3 bg-app-card/30 p-4 rounded-2xl border border-app-border/20">
          <div className="w-10 h-10 rounded-xl bg-app-accent flex items-center justify-center">
            <MessageCircleMore className="w-6 h-6 text-white" />
          </div>
          <div className="flex flex-col">
            <h2 className="text-sm font-bold text-app-text leading-tight">
              RAG Assistant
            </h2>
            <span className="text-[11px] text-app-subtext font-medium">Enterprise Knowledge</span>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-2 space-y-6">
        {/* Mode Selector */}
        <div className="space-y-2">
          <button
            onClick={() => onChangeChatMode("free")}
            className={cn(
              "w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all font-semibold text-sm border",
              chatMode === "free"
                ? "bg-app-accent text-white border-transparent"
                : "bg-transparent text-app-subtext border-app-border/40 hover:bg-app-card/50",
            )}
          >
            <MessageSquare className="w-5 h-5" />
            <span>Free Chat</span>
          </button>

          <button
            onClick={() => onChangeChatMode("rag")}
            className={cn(
              "w-full flex items-center justify-between gap-3 px-4 py-3 rounded-xl transition-all font-semibold text-sm border",
              chatMode === "rag"
                ? "bg-app-accent text-white border-transparent"
                : "bg-transparent text-app-subtext border-app-border/40 hover:bg-app-card/50",
            )}
          >
            <div className="flex items-center gap-3">
              <Database className="w-5 h-5" />
              <span>RAG Mode</span>
            </div>
          </button>
        </div>

        {/* Collections in RAG mode */}
        {chatMode === "rag" && collections.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between px-2">
              <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
                Collections
              </span>
            </div>
            <div className="px-1">
              <Select
                options={collections.map((c) => ({
                  value: String(c.id),
                  label: c.name,
                }))}
                value={String(selectedCollectionId || "")}
                onChange={(val) => onSelectCollection(Number(val))}
                placeholder="Select Collection..."
                searchable={true}
              />
            </div>
          </div>
        )}

        {/* Recent Chats Section */}
        <div className="space-y-4">
          <div className="flex items-center justify-between px-2">
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
              Recent Chats
            </span>
            <button
              onClick={chatMode === "free" ? onNewFreeChat : () => {}}
              className="p-1 hover:bg-app-card rounded transition-colors"
            >
              <Plus className="w-3.5 h-3.5 text-app-accent" />
            </button>
          </div>

          <div className="space-y-1">
            {chatMode === "free" ? (
              freeChatConversations.length === 0 ? (
                <div className="px-3 py-4 text-center">
                  <p className="text-[11px] text-app-subtext">No recent free chats</p>
                </div>
              ) : (
                freeChatConversations.map((conv) => (
                  <HistoryItem
                    key={conv.id}
                    conv={conv}
                    isActive={currentFreeChatConversationId === conv.id}
                    onSelect={() => onSelectFreeChatConversation(conv.id)}
                    onDelete={() => onDeleteConversation(conv.id)}
                  />
                ))
              )
            ) : (
              conversations.length === 0 ? (
                <div className="px-3 py-4 text-center">
                  <p className="text-[11px] text-app-subtext">No recent RAG chats</p>
                </div>
              ) : (
                conversations.map((conv) => (
                  <HistoryItem
                    key={conv.id}
                    conv={conv}
                    isActive={currentConversationId === conv.id}
                    onSelect={() => onSelectConversation(conv.id)}
                    onDelete={() => onDeleteConversation(conv.id)}
                  />
                ))
              )
            )}
          </div>
        </div>
      </div>

      {/* Sidebar Footer */}
      <div className="p-4 border-t border-app-border/30 bg-app-card/5">
        <div className="flex items-center justify-between mb-2 px-2">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full bg-app-card border border-app-border/40 flex items-center justify-center">
              <div className="w-4 h-4 rounded-full bg-app-accent/20 flex items-center justify-center">
                <div className="w-2 h-2 rounded-full bg-app-accent" />
              </div>
            </div>
            <span className="text-xs font-semibold text-app-text">
              Developer Mode
            </span>
          </div>
          <button
            onClick={onOpenSessionConfig}
            className="p-2 hover:bg-app-card rounded-lg transition-all text-app-subtext hover:text-app-accent border border-transparent hover:border-app-border/40"
          >
            <Settings className="w-4 h-4" />
          </button>
        </div>
      </div>
    </aside>
  );
}

function HistoryItem({
  conv,
  isActive,
  onSelect,
  onDelete,
}: {
  conv: Conversation;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      onClick={onSelect}
      className={cn(
        "group relative w-full flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer transition-all border border-transparent",
        isActive
          ? "bg-app-card text-app-text border-app-border/60 shadow-sm"
          : "text-app-subtext hover:bg-app-card/40 hover:text-app-text",
      )}
    >
      <MessageSquare className={cn("w-3.5 h-3.5 shrink-0", isActive ? "text-app-accent" : "text-app-subtext")} />
      <span className="truncate text-[12px] font-medium grow">
        {conv.title || "Untitled Chat"}
      </span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 transition-all rounded"
      >
        <Trash2 className="w-3 h-3" />
      </button>
    </div>
  );
}
