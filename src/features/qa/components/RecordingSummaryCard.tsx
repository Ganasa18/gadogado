import type { QaSession } from "../../../types/qa/types";
import { formatTimestamp } from "../utils/eventFormatting";

type RecordingSummaryCardProps = {
  session: QaSession | null;
  isRecording: boolean;
};

export default function RecordingSummaryCard({
  session,
  isRecording,
}: RecordingSummaryCardProps) {
  return (
    <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[11px] text-app-subtext uppercase tracking-wide">
          Recording
        </div>
        <div className="text-[10px] text-app-subtext">
          {isRecording ? "Recording" : "Idle"}
        </div>
      </div>
      <div className="grid grid-cols-2 gap-3 text-[11px]">
        <div className="rounded-md border border-app-border bg-black/20 p-2">
          <div className="text-[10px] text-gray-500">Type</div>
          <div className="text-gray-300">
            {session?.session_type ? session.session_type.toUpperCase() : "n/a"}
          </div>
        </div>
        <div className="rounded-md border border-app-border bg-black/20 p-2">
          <div className="text-[10px] text-gray-500">Started</div>
          <div className="text-gray-300">
            {session?.started_at ? formatTimestamp(session.started_at) : "n/a"}
          </div>
        </div>
        <div className="rounded-md border border-app-border bg-black/20 p-2">
          <div className="text-[10px] text-gray-500">Ended</div>
          <div className="text-gray-300">
            {session?.ended_at ? formatTimestamp(session.ended_at) : "Still running"}
          </div>
        </div>
        <div className="rounded-md border border-app-border bg-black/20 p-2">
          <div className="text-[10px] text-gray-500">
            {session?.session_type === "api" ? "API Base" : "Target URL"}
          </div>
          <div className="text-gray-300 truncate">
            {session?.session_type === "api"
              ? session?.api_base_url || "n/a"
              : session?.target_url || "n/a"}
          </div>
        </div>
      </div>
    </div>
  );
}
