import { Trash2, Plus } from "lucide-react";

export function RagChatHeader(props: {
  onClear: () => void;
  onNewSession: () => void;
  hasMessages: boolean;
}) {
  const { onClear, onNewSession, hasMessages } = props;

  return (
    <header className="px-4 py-4 border-b border-app-border/30 bg-app-bg/50 sticky top-0 z-20">
      <div className="max-w-5xl mx-auto w-full flex items-center justify-between">
        <div className="flex items-center gap-3">
          {/* <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20">
            <div className="w-2 h-2 rounded-full bg-emerald-500" />
            <span className="text-[11px] font-bold text-emerald-500 uppercase tracking-widest">
              System Operational
            </span>
          </div> */}
        </div>

        <div className="flex items-center gap-3">
          {hasMessages && (
            <button
              onClick={onClear}
              className="flex items-center gap-2 px-4 py-2 text-xs font-semibold text-app-subtext hover:text-red-400 hover:bg-app-card/50 rounded-xl transition-all border border-transparent hover:border-app-border/40">
              <Trash2 className="w-4 h-4" />
              Clear Chat
            </button>
          )}
          <button
            onClick={onNewSession}
            className="flex items-center gap-2 px-4 py-2 text-xs font-bold text-white bg-app-accent hover:opacity-90 rounded-xl transition-all active:scale-95">
            <Plus className="w-4 h-4" />
            New Session
          </button>
        </div>
      </div>
    </header>
  );
}
