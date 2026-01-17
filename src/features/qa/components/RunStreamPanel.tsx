import type { QaEvent, QaRunStreamEvent } from "../../../types/qa/types";
import { formatEventTime, getEventBadgeClasses, getEventDetails } from "../utils/eventFormatting";

const originStyles: Record<string, string> = {
  user: "border-emerald-500/30 bg-emerald-500/10",
  ai: "border-sky-500/30 bg-sky-500/10",
  system: "border-amber-500/30 bg-amber-500/10",
};

const channelStyles: Record<string, string> = {
  browser: "border-emerald-500/30 bg-emerald-500/10",
  api: "border-blue-500/30 bg-blue-500/10",
  system: "border-amber-500/30 bg-amber-500/10",
};

const normalizeOrigin = (value?: string | null) =>
  value?.trim().toLowerCase() || "system";

const parseEventPayload = (payload?: string | null): QaEvent | null => {
  if (!payload) return null;
  try {
    const parsed = JSON.parse(payload);
    if (parsed && typeof parsed === "object" && "event_type" in parsed) {
      return parsed as QaEvent;
    }
  } catch {
    return null;
  }
  return null;
};

const parseEventMeta = (event: QaEvent) => {
  if (!event.meta_json) return null;
  try {
    const parsed = JSON.parse(event.meta_json);
    return parsed && typeof parsed === "object"
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
};

const buildApiSummary = (event: QaEvent) => {
  const meta = parseEventMeta(event);
  const status = typeof meta?.status === "number" ? meta.status : null;
  const latency =
    typeof meta?.timing_ms === "number" ? `${meta.timing_ms}ms` : "latency n/a";
  const statusLabel = status ? `status ${status}` : "status n/a";
  return `${statusLabel} • ${latency}`;
};

export default function RunStreamPanel({
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
          Live Run Stream
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
        <div className="text-[11px] text-app-subtext">Loading stream...</div>
      )}
      {error && <div className="text-[11px] text-red-300">{error}</div>}
      {!loading && events.length === 0 && !error && (
        <div className="text-[11px] text-app-subtext">No stream events yet.</div>
      )}
      <div className="space-y-2 max-h-60 overflow-auto">
        {events.map((event) => {
          const payloadEvent = parseEventPayload(event.payloadJson);
          const origin = normalizeOrigin(payloadEvent?.origin);
          const channel = event.channel?.toLowerCase() || "system";
          const details = payloadEvent ? getEventDetails(payloadEvent) : null;
          const isApi = payloadEvent?.event_type?.toLowerCase().includes("api") ||
            payloadEvent?.event_type?.toLowerCase().includes("curl");
          const badgeClass = payloadEvent
            ? getEventBadgeClasses(payloadEvent.event_type)
            : "bg-slate-500/10 text-slate-300 border-slate-500/20";
          const originStyle = originStyles[origin] ?? originStyles.system;
          const channelStyle = channelStyles[channel] ?? channelStyles.system;

          return (
            <div
              key={event.id}
              className={`rounded border px-3 py-2 text-[11px] ${channelStyle}`}>
              <div className="flex items-center justify-between gap-2 text-app-subtext">
                <div className="flex items-center gap-2">
                  <span
                    className={`rounded-full border px-2 py-0.5 text-[9px] uppercase tracking-wide ${originStyle}`}>
                    {origin}
                  </span>
                  <span className="uppercase tracking-wide">{channel}</span>
                </div>
                <span>{formatEventTime(event.ts)}</span>
              </div>
              <div className="mt-2 text-app-text">
                <span className={`rounded-full border px-2 py-0.5 text-[9px] ${badgeClass}`}>
                  {payloadEvent?.event_type ?? event.level.toUpperCase()}
                </span>
                <span className="ml-2">{event.message}</span>
              </div>
              {payloadEvent && details && (
                <div className="mt-2 text-[10px] text-app-subtext space-y-1">
                  <div>{details.primary}</div>
                  <div>{details.secondary}</div>
                  {isApi && <div>{buildApiSummary(payloadEvent)}</div>}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
