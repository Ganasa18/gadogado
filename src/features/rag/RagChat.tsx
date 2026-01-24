import { useCallback, useRef, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { useEnhanceMutation } from "../../hooks/useLlmApi";
import { RagChatComposer } from "./components/RagChatComposer";
import { RagChatDbBanner } from "./components/RagChatDbBanner";
import { RagChatMessages } from "./components/RagChatMessages";
import { RagChatSidebar } from "./components/RagChatSidebar";
import { RagSessionConfigModal } from "./components/RagSessionConfigModal";
import { useRagCollections } from "./hooks/useRagCollections";
import { useRagConversations } from "./hooks/useRagConversations";
import { useRagDbCollection } from "./hooks/useRagDbCollection";
import { useRagModelSelection } from "./hooks/useRagModelSelection";
import { useRagPersistedSettings } from "./hooks/useRagPersistedSettings";
import { useRagSend } from "./hooks/useRagSend";

export default function RagChat() {
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const [sessionConfigOpen, setSessionConfigOpen] = useState(false);

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
  const { isLoading, sendMessage } = useRagSend({
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
  });

  const onSend = useCallback(() => {
    const q = input.trim();
    if (!q) return;

    setInput("");
    void sendMessage(q).finally(() => inputRef.current?.focus());
  }, [input, sendMessage]);

  const handleClear = useCallback(async () => {
    if (messages.length === 0) return;
    if (!confirm("Clear this conversation?")) return;

    try {
      await clearCurrentConversation();
    } catch (err) {
      console.error("Failed to delete conversation:", err);
    }
  }, [messages.length, clearCurrentConversation]);

  const handleNewConversation = useCallback(() => {
    startNewConversation();
  }, [startNewConversation]);

  const handleDeleteConversation = useCallback(
    async (conversationId: number) => {
      if (!confirm("Delete this conversation?")) return;
      try {
        await deleteConversationById(conversationId);
      } catch (err) {
        console.error("Failed to delete conversation:", err);
      }
    },
    [deleteConversationById],
  );

  const handleSelectCollection = useCallback(
    (collectionId: number) => {
      setSelectedCollectionId(collectionId);
      startNewConversation();
    },
    [setSelectedCollectionId, startNewConversation],
  );

  const handleRegenerate = useCallback((query: string) => {
    setInput(query);
    inputRef.current?.focus();
  }, []);

  return (
    <div className="flex h-full bg-app-bg text-app-text overflow-hidden font-sans">
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
      />

      <RagChatSidebar
        collections={collections}
        selectedCollectionId={selectedCollectionId}
        onSelectCollection={handleSelectCollection}
        conversations={conversations}
        currentConversationId={currentConversationId}
        onSelectConversation={selectConversation}
        onDeleteConversation={(id) => void handleDeleteConversation(id)}
        onOpenSessionConfig={() => setSessionConfigOpen(true)}
        retrievalSummary={`k=${topK} | cand=${candidateK} | rerank=${rerankK}`}
      />

      <main className="flex-1 flex flex-col min-w-0 bg-gradient-to-br from-app-bg via-app-bg to-app-card/20 relative">
        <div className="absolute top-0 left-0 w-full h-[150px] bg-gradient-to-b from-app-card/10 to-transparent pointer-events-none" />

        {selectedCollectionId && (
          <div className="absolute top-4 right-6 z-10 flex items-center gap-2">
            <button
              onClick={handleNewConversation}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-app-accent text-white rounded-full text-[10px] font-medium hover:brightness-110 transition-all shadow-md shadow-app-accent/20">
              <Plus className="w-3 h-3" />
              New Chat
            </button>
            <button
              onClick={() => void handleClear()}
              disabled={messages.length === 0}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-app-card/60 backdrop-blur-md border border-app-border/40 rounded-full text-[10px] font-medium text-app-text-muted hover:text-red-400 hover:border-red-400/30 transition-all disabled:opacity-0">
              <Trash2 className="w-3 h-3" />
              Clear
            </button>
          </div>
        )}

        <RagChatDbBanner
          isDbCollection={isDbCollection}
          selectedCollectionId={selectedCollectionId}
          selectedTables={selectedTables}
        />

        <RagChatMessages
          messages={messages}
          isLoadingHistory={isLoadingHistory}
          isLoading={isLoading}
          selectedCollectionId={selectedCollectionId}
          onRegenerate={handleRegenerate}
        />

        <RagChatComposer
          input={input}
          setInput={setInput}
          inputRef={inputRef}
          selectedCollectionId={selectedCollectionId}
          isLoading={isLoading}
          onSend={onSend}
        />
      </main>
    </div>
  );
}
