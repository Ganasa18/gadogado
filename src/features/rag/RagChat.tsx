import { useCallback, useRef, useState } from "react";
import { ChevronRight } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useEnhanceMutation } from "../../hooks/useLlmApi";
import { RagChatComposer } from "./components/RagChatComposer";
import { RagChatDbBanner } from "./components/RagChatDbBanner";
import { RagChatMessages } from "./components/RagChatMessages";
import { RagChatSidebar } from "./components/RagChatSidebar";
import { RagChatHeader } from "./components/RagChatHeader";
import { RagSessionConfigModal } from "./components/RagSessionConfigModal";
import { useRagCollections } from "./hooks/useRagCollections";
import { useRagConversations } from "./hooks/useRagConversations";
import { useRagDbCollection } from "./hooks/useRagDbCollection";
import { useRagModelSelection } from "./hooks/useRagModelSelection";
import { useRagPersistedSettings } from "./hooks/useRagPersistedSettings";
import { useRagSend } from "./hooks/useRagSend";
import { useFreeChat } from "./hooks/useFreeChat";
import type { ChatMode } from "./types";

export default function RagChat() {
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const [sessionConfigOpen, setSessionConfigOpen] = useState(false);
  const [chatMode, setChatMode] = useState<ChatMode>("rag");
  const [isBannerDismissed, setIsBannerDismissed] = useState(false);
  const [isSidebarOpen, setIsSidebarOpen] = useState(window.innerWidth > 1024);

  const {
    selectedCollectionId,
    setSelectedCollectionId,
    answerLanguage,
    setAnswerLanguage,
    strictRagMode,
    setStrictRagMode,
    topK,
    setTopK,
    candidateK,
    setCandidateK,
    rerankK,
    setRerankK,
    dbFinalK,
    setDbFinalK,
  } = useRagPersistedSettings();

  const { collections } = useRagCollections();
  const { isDbCollection, selectedTables } = useRagDbCollection({
    selectedCollectionId,
    collections,
  });

  const {
    provider,
    model,
    localModels,
    setModel,
    buildConfig,
    isLocalProvider,
    modelsQuery,
  } = useRagModelSelection();

  const {
    messages,
    conversations,
    currentConversationId,
    isLoadingHistory,
    selectConversation,
    startNewConversation,
    deleteConversationById,
    clearCurrentConversation,
    ensureConversation,
    appendMessage,
    persistMessage,
  } = useRagConversations(selectedCollectionId);

  const { mutateAsync: enhanceAsync } = useEnhanceMutation();
  const { isLoading, sendMessage, regenerateWithTemplate } = useRagSend({
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
  });

  // Free chat hook
  const freeChat = useFreeChat({
    chatMode,
    enhanceAsync,
    buildConfig,
  });

  // Computed values based on chat mode
  const currentMessages = chatMode === "free" ? freeChat.messages : messages;
  const currentIsLoading = chatMode === "free" ? freeChat.isLoading : isLoading;
  const currentIsLoadingHistory = chatMode === "free" ? freeChat.isLoadingHistory : isLoadingHistory;

  const onSend = useCallback(() => {
    const q = input.trim();
    if (!q) return;

    setInput("");
    if (chatMode === "free") {
      void freeChat.sendMessage(q).finally(() => inputRef.current?.focus());
    } else {
      void sendMessage(q).finally(() => inputRef.current?.focus());
    }
  }, [input, chatMode, freeChat.sendMessage, sendMessage]);

  const handleClear = useCallback(async () => {
    if (currentMessages.length === 0) return;
    if (!confirm("Clear this conversation?")) return;

    if (chatMode === "free") {
      void freeChat.newConversation();
    } else {
      try {
        await clearCurrentConversation();
      } catch (err) {
        console.error("Failed to delete conversation:", err);
      }
    }
  }, [currentMessages.length, chatMode, freeChat, clearCurrentConversation]);

  const handleNewSession = useCallback(() => {
    if (chatMode === "free") {
      void freeChat.newConversation();
    } else {
      startNewConversation();
      setIsBannerDismissed(false); // Reset banner on new session
    }
  }, [chatMode, freeChat, startNewConversation]);

  const handleDeleteConversation = useCallback(
    async (conversationId: number) => {
      if (!confirm("Delete this conversation?")) return;
      try {
        // Route to the appropriate delete method based on chat mode
        if (chatMode === "free") {
          await freeChat.deleteConversation(conversationId);
        } else {
          await deleteConversationById(conversationId);
        }
      } catch (err) {
        console.error("Failed to delete conversation:", err);
      }
    },
    [chatMode, freeChat, deleteConversationById],
  );

  const handleSelectCollection = useCallback(
    (collectionId: number) => {
      setSelectedCollectionId(collectionId);
      startNewConversation();
      setIsBannerDismissed(false); // Reset banner on collection change
    },
    [setSelectedCollectionId, startNewConversation],
  );

  const handleRegenerate = useCallback((query: string) => {
    setInput(query);
    inputRef.current?.focus();
  }, []);

  const handleRegenerateWithTemplate = useCallback(
    (query: string, templateId: number) => {
      void regenerateWithTemplate(query, templateId).finally(() => inputRef.current?.focus());
    },
    [regenerateWithTemplate],
  );

  // Free chat handlers
  const handleChangeChatMode = useCallback((mode: ChatMode) => {
    setChatMode(mode);
    if (mode === "free") {
      void freeChat.refreshConversations();
    }
  }, [freeChat]);

  const handleSelectFreeChatConversation = useCallback(
    async (conversationId: number) => {
      await freeChat.loadConversation(conversationId);
    },
    [freeChat],
  );

  const handleNewFreeChat = useCallback(() => {
    void freeChat.newConversation();
  }, [freeChat]);

  const toggleSidebar = useCallback(() => {
    setIsSidebarOpen((prev) => !prev);
  }, []);

  return (
    <div className="flex h-screen bg-app-bg text-app-text overflow-hidden font-sans select-none">
      <RagSessionConfigModal
        open={sessionConfigOpen}
        onClose={() => setSessionConfigOpen(false)}
        provider={provider}
        model={model}
        setModel={setModel}
        localModels={localModels}
        openRouterModels={modelsQuery.data}
        answerLanguage={answerLanguage}
        setAnswerLanguage={setAnswerLanguage}
        strictRagMode={strictRagMode}
        setStrictRagMode={setStrictRagMode}
        topK={topK}
        setTopK={setTopK}
        candidateK={candidateK}
        setCandidateK={setCandidateK}
        rerankK={rerankK}
        setRerankK={setRerankK}
        dbFinalK={dbFinalK}
        setDbFinalK={setDbFinalK}
      />

      <RagChatSidebar
        isOpen={isSidebarOpen}
        onClose={() => setIsSidebarOpen(false)}
        collections={collections}
        selectedCollectionId={selectedCollectionId}
        onSelectCollection={handleSelectCollection}
        conversations={conversations}
        freeChatConversations={freeChat.conversations}
        currentConversationId={currentConversationId}
        currentFreeChatConversationId={freeChat.currentConversationId}
        onSelectConversation={selectConversation}
        onSelectFreeChatConversation={handleSelectFreeChatConversation}
        onDeleteConversation={(id) => void handleDeleteConversation(id)}
        onOpenSessionConfig={() => setSessionConfigOpen(true)}
        retrievalSummary={`k=${topK} | cand=${candidateK} | rerank=${rerankK}`}
        chatMode={chatMode}
        onChangeChatMode={handleChangeChatMode}
        onNewFreeChat={handleNewFreeChat}
      />

      <main className="flex-1 flex flex-col min-w-0 bg-app-bg relative">
        <RagChatHeader
          onClear={() => void handleClear()}
          onNewSession={handleNewSession}
          hasMessages={currentMessages.length > 0}
        />

        {/* Floating Toggle Button */}
        <AnimatePresence>
          {!isSidebarOpen && (
            <motion.button
              initial={{ x: -20, opacity: 0 }}
              animate={{ x: 0, opacity: 0.8 }}
              exit={{ x: -20, opacity: 0 }}
              whileHover={{ x: 2, opacity: 1, backgroundColor: "var(--color-primary)" }}
              transition={{ type: "spring", stiffness: 300, damping: 25 }}
              onClick={toggleSidebar}
              className="absolute left-0 top-1/2 -translate-y-1/2 z-30 bg-app-accent text-white p-1 rounded-r-md shadow-md border border-app-accent/50 transition-colors group"
              title="Open Sidebar"
            >
              <ChevronRight className="w-3.5 h-3.5 group-hover:scale-110 transition-transform" />
            </motion.button>
          )}
        </AnimatePresence>

        {chatMode !== "free" && !isBannerDismissed && (
          <RagChatDbBanner
            isDbCollection={isDbCollection}
            selectedCollectionId={selectedCollectionId}
            selectedTables={selectedTables}
            onDismiss={() => setIsBannerDismissed(true)}
          />
        )}

        <div className="flex-1 flex flex-col min-h-0 relative">
          <RagChatMessages
            messages={currentMessages}
            isLoadingHistory={currentIsLoadingHistory}
            isLoading={currentIsLoading}
            selectedCollectionId={selectedCollectionId}
            onRegenerate={handleRegenerate}
            onRegenerateWithTemplate={isDbCollection ? handleRegenerateWithTemplate : undefined}
          />

          <RagChatComposer
            input={input}
            setInput={setInput}
            inputRef={inputRef}
            selectedCollectionId={selectedCollectionId}
            isLoading={currentIsLoading}
            onSend={onSend}
            chatMode={chatMode}
          />
        </div>
      </main>
    </div>
  );
}
