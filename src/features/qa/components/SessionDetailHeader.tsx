import { ArrowLeft, RefreshCcw } from "lucide-react";
import type { QaSession } from "../../../types/qa/types";

type SessionDetailHeaderProps = {
  session: QaSession | null;
  sessionLoading: boolean;
  sessionError: string | null;
  isRecording: boolean;
  onBack: () => void;
  onRetry: () => void;
  backLabel?: string;
  onViewAiOutputs?: () => void;
};

export default function SessionDetailHeader({
  session,
  sessionLoading,
  sessionError,
  isRecording,
  onBack,
  onRetry,
  backLabel,
  onViewAiOutputs,
}: SessionDetailHeaderProps) {
  return (
    <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={onBack}
            className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-3 py-2 text-[11px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition">
            <ArrowLeft className="w-3.5 h-3.5" />
            {backLabel ?? "Back to History"}
          </button>
          {onViewAiOutputs && (
            <button
              type="button"
              onClick={onViewAiOutputs}
              className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-3 py-2 text-[11px] text-app-subtext hover:text-app-text hover:border-sky-600/60 transition">
              AI Outputs
            </button>
          )}
          <div>
            <div className="text-xs text-app-subtext uppercase tracking-wide">
              QA Session
            </div>
            <div className="text-sm font-semibold text-app-text">
              {sessionLoading
                ? "Loading session..."
                : session?.title || "Untitled Session"}
            </div>
            <div className="text-[11px] text-app-subtext">
              {sessionLoading ? "Loading metadata..." : session?.goal}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2 text-[11px] text-app-subtext">
          <div className="rounded-full border border-app-border px-2 py-1">
            {session?.session_type ? session.session_type.toUpperCase() : "TYPE"}
          </div>
          <div className="rounded-full border border-app-border px-2 py-1">
            {session?.ended_at ? "Ended" : "Open"}
          </div>
          <div className="rounded-full border border-app-border px-2 py-1">
            {isRecording ? "Recording" : "Not recording"}
          </div>
        </div>
      </div>
      {sessionError && (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-red-900/50 bg-red-900/10 px-3 py-2 text-[11px] text-red-200">
          <span>{sessionError}</span>
          <button
            type="button"
            onClick={onRetry}
            className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
            <RefreshCcw className="w-3 h-3" />
            Retry
          </button>
        </div>
      )}
    </div>
  );
}
