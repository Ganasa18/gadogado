import { useCallback } from "react";
import type { KeyboardEvent, RefObject } from "react";
import { Loader2, Send, Paperclip } from "lucide-react";
import { cn } from "../../../utils/cn";
import type { ChatMode } from "../types";

export function RagChatComposer(props: {
  input: string;
  setInput: (next: string) => void;
  inputRef: RefObject<HTMLTextAreaElement | null>;
  selectedCollectionId: number | null;
  isLoading: boolean;
  onSend: () => void;
  chatMode: ChatMode;
}) {
  const { input, setInput, inputRef, selectedCollectionId, isLoading, onSend, chatMode } =
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

  const isDisabled = (chatMode === "rag" && !selectedCollectionId) || isLoading;

  return (
    <div className="p-6 pt-2">
      <div className="max-w-5xl mx-auto">
        <div className={cn(
          "relative flex flex-col bg-app-card border border-app-border/40 rounded-3xl transition-all focus-within:border-primary/40 focus-within:ring-4 focus-within:ring-primary/5 overflow-hidden shadow-sm",
          isDisabled && "opacity-70 grayscale-[0.2]"
        )}>
          <div className="flex items-end gap-2 p-4 pb-3">
            <button 
              disabled={isDisabled}
              className="p-2.5 text-app-subtext hover:text-primary hover:bg-app-bg/80 rounded-xl transition-all disabled:opacity-30 disabled:hover:bg-transparent"
            >
              <Paperclip className="w-5 h-5" />
            </button>
            <div className="flex-1">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={
                  chatMode === "free"
                    ? "Ask me anything..."
                    : selectedCollectionId
                      ? "Ask follow-up questions..."
                      : "Select a collection to start..."
                }
                disabled={isDisabled}
                className="w-full bg-transparent border-none text-[15px] outline-none px-2 py-2 text-app-text disabled:cursor-not-allowed resize-none placeholder:text-app-subtext/50 leading-relaxed font-medium"
                rows={1}
                style={{ minHeight: "24px", maxHeight: "200px" }}
              />
            </div>

            <button
              onClick={onSend}
              disabled={!input.trim() || isDisabled}
              className="w-10 h-10 bg-primary/20 text-primary border border-primary/30 rounded-2xl hover:bg-primary/30 disabled:bg-app-border/20 disabled:text-app-subtext transition-all flex items-center justify-center active:scale-95 shadow-sm"
            >
              {isLoading ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
            </button>
          </div>

          <div className="flex items-center gap-6 px-6 py-2.5 bg-app-bg/30 border-t border-app-border/20">
            <div className="flex items-center gap-1.5 opacity-60">
              <span className="text-[10px] font-bold text-app-subtext uppercase">Enter</span>
              <span className="text-[10px] text-app-subtext">to send</span>
            </div>
            <div className="flex items-center gap-1.5 opacity-60">
              <span className="text-[10px] font-bold text-app-subtext uppercase">Shift + Enter</span>
              <span className="text-[10px] text-app-subtext">for new line</span>
            </div>
            <div className="ml-auto flex items-center gap-1.5 opacity-60">
              <span className="text-[10px] text-app-subtext italic">Chat auto-saved</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
