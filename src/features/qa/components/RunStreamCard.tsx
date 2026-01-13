import type { QaRunStreamEvent } from "../../../types/qa/types";
import { formatTimestamp } from "../utils/eventFormatting";

export default function RunStreamCard({
  events,
  loading,
  error,
  runId,
  onReload,
}: {
  events: QaRunStreamEvent[];
  loading: boolean;
  error: string | null;
  runId: string | null;
  onReload: () => void;
}) {
  return (
    <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[11px] text-app-subtext uppercase tracking-wide">
          Run Stream
        </div>
        <button
          type="button"
          onClick={onReload}
          className="text-[10px] px-2 py-1 rounded border border-app-border text-app-subtext hover:text-app-text hover:border-emerald-500/60 transition">
          Refresh
        </button>
      </div>
      <div className="text-[10px] text-app-subtext">
        {runId ? `Run ID: ${runId.slice(0, 8)}…` : "No active run"}
      </div>
      {loading && (
        <div className="text-[11px] text-app-subtext">Loading run stream...</div>
      )}
      {error && <div className="text-[11px] text-red-300">{error}</div>}
      {!loading && events.length === 0 && !error && (
        <div className="text-[11px] text-app-subtext">No stream events yet.</div>
      )}
      <div className="space-y-2 max-h-48 overflow-auto">
        {events.map((event) => (
          <div
            key={event.id}
            className="rounded border border-app-border bg-black/20 px-3 py-2 text-[11px]">
            <div className="flex items-center justify-between gap-2 text-app-subtext">
              <span className="uppercase tracking-wide">{event.channel}</span>
              <span>{formatTimestamp(event.ts)}</span>
            </div>
            <div className="mt-1 text-app-text">
              {event.level.toUpperCase()}: {event.message}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
