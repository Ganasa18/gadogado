import { useCallback } from "react";
import type { KeyboardEvent, RefObject } from "react";
import { Loader2, Send } from "lucide-react";
import { cn } from "../../../utils/cn";

export function RagChatComposer(props: {
  input: string;
  setInput: (next: string) => void;
  inputRef: RefObject<HTMLTextAreaElement | null>;
  selectedCollectionId: number | null;
  isLoading: boolean;
  onSend: () => void;
}) {
  const { input, setInput, inputRef, selectedCollectionId, isLoading, onSend } =
    props;

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        onSend();
      }
    },
    [onSend],
  );

  return (
    <div className="p-6 bg-gradient-to-t from-app-bg via-app-bg to-transparent">
      <div className="max-w-4xl mx-auto relative group">
        <div
          className={cn(
            "absolute -inset-0.5 rounded-2xl opacity-0 transition-opacity duration-300",
            selectedCollectionId ? "group-hover:opacity-100" : "group-hover:opacity-0",
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
            onClick={onSend}
            disabled={!input.trim() || !selectedCollectionId || isLoading}
            className="px-4 bg-app-accent text-white rounded-lg hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center shadow-md shadow-app-accent/20">
            {isLoading ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
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
  );
}
