import type { QaEvent } from "../../../types/qa/types";
import { PlayCircle } from "lucide-react";
import {
  formatEventMetadata,
  formatEventSeq,
  formatEventTime,
  getEventBadgeClasses,
  getEventDetails,
} from "../utils/eventFormatting";

type EventTimelineProps = {
  events: QaEvent[];
  eventsLoading: boolean;
  expandedEventIds: Record<string, boolean>;
  selectedEventIds: Set<string>;
  onToggleEventDetails: (eventId: string) => void;
  onToggleSelectEvent: (eventId: string) => void;
  onReplayEvent: (event: QaEvent) => void;
};

export default function EventTimeline({
  events,
  eventsLoading,
  expandedEventIds,
  selectedEventIds,
  onToggleEventDetails,
  onToggleSelectEvent,
  onReplayEvent,
}: EventTimelineProps) {
  return (
    <div className="mt-3 max-h-[520px] overflow-y-auto">
      <div className="event-timeline px-6 py-4">
        <h2 className="text-[10px] font-mono uppercase tracking-wide text-app-subtext mb-4 sticky top-0 bg-app-card py-2 -mx-6 px-6 border-b border-app-border">
          Activity Stream
        </h2>
        <div className="divide-y divide-app-border -mx-6">
          {eventsLoading && (
            <div className="px-6 py-4 text-[11px] text-app-subtext">
              Loading events...
            </div>
          )}
          {!eventsLoading && events.length === 0 && (
            <div className="px-6 py-4 text-[11px] text-app-subtext">
              No events yet.
            </div>
          )}
          {!eventsLoading &&
            events.map((event) => {
              const isExpanded = Boolean(expandedEventIds[event.id]);
              const isSelected = selectedEventIds.has(event.id);
              const { primary, secondary } = getEventDetails(event);
              const canReplay =
                [
                  "click",
                  "input",
                  "change",
                  "submit",
                  "focus",
                  "blur",
                  "navigation",
                  "api_request",
                  "api_response",
                ].includes(event.event_type) &&
                (event.event_type === "navigation" ||
                  Boolean(event.selector) ||
                  event.event_type.startsWith("api_"));
              return (
                <div key={event.id}>
                  <div
                    role="button"
                    tabIndex={0}
                    onClick={() => onToggleEventDetails(event.id)}
                    onKeyDown={(keyboardEvent) => {
                      if (
                        keyboardEvent.key === "Enter" ||
                        keyboardEvent.key === " "
                      ) {
                        keyboardEvent.preventDefault();
                        onToggleEventDetails(event.id);
                      }
                    }}
                    aria-expanded={isExpanded}
                    className={`group event-card w-full text-left border-l-2 pl-6 py-3 cursor-pointer relative transition-colors ${
                      isExpanded
                        ? "bg-black/20 border-emerald-500/60"
                        : "border-app-border hover:bg-black/10"
                    } ${
                      isSelected
                        ? "bg-emerald-900/10 border-emerald-500/60"
                        : ""
                    }`}>
                    <div className="timeline-dot absolute left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 w-3.5 h-3.5 rounded-full bg-app-card border border-app-border flex items-center justify-center" />
                    <div className="flex items-center gap-4">
                      <div
                        className="flex items-center"
                        onClick={(clickEvent) => {
                          clickEvent.stopPropagation();
                        }}>
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => onToggleSelectEvent(event.id)}
                          className="h-3 w-3 rounded border-app-border bg-black/40 text-emerald-400 focus:ring-emerald-500/40"
                        />
                      </div>
                      <div
                        className={`font-mono text-[11px] w-12 shrink-0 ${
                          isExpanded ? "text-emerald-200" : "text-app-subtext"
                        }`}>
                        {formatEventSeq(event.seq)}
                      </div>
                      <div
                        className={`font-mono text-[11px] w-28 shrink-0 ${
                          isExpanded ? "text-app-text" : "text-app-subtext"
                        }`}>
                        {formatEventTime(event.ts)}
                      </div>
                      <div className="w-28 shrink-0">
                        <span
                          className={`inline-flex items-center px-2 py-0.5 rounded text-[10px] font-mono font-medium border ${getEventBadgeClasses(
                            event.event_type
                          )}`}>
                          {event.event_type.toUpperCase()}
                        </span>
                      </div>
                      <div
                        className={`font-mono text-[11px] truncate flex-1 ${
                          isExpanded ? "text-app-text" : "text-app-subtext"
                        }`}
                        title={primary}>
                        {primary}
                      </div>
                      <div
                        className={`font-mono text-[11px] truncate flex-1 ${
                          isExpanded ? "text-app-text" : "text-app-subtext"
                        }`}
                        title={secondary}>
                        {secondary}
                      </div>
                      <button
                        type="button"
                        onClick={(clickEvent) => {
                          clickEvent.stopPropagation();
                          if (canReplay) {
                            onReplayEvent(event);
                          }
                        }}
                        disabled={!canReplay}
                        title={
                          canReplay ? "Replay event" : "Replay not available"
                        }
                        className="ml-auto inline-flex items-center justify-center rounded border border-app-border p-1 text-app-subtext transition hover:border-emerald-500/60 hover:text-emerald-300 disabled:opacity-40 disabled:hover:border-app-border">
                        <PlayCircle className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  </div>
                  {isExpanded && (
                    <div className="json-metadata-expanded p-4 border-b border-app-border -mx-6">
                      <div className="flex-1 flex flex-col min-w-0 bg-black/10 border border-app-border rounded-lg overflow-hidden">
                        <div className="px-3 py-2 border-b border-app-border bg-black/20 flex justify-between items-center">
                          <span className="text-[10px] font-mono uppercase tracking-wide text-app-subtext">
                            Event Metadata
                          </span>
                          <span className="text-[10px] font-mono text-app-subtext">
                            application/json
                          </span>
                        </div>
                        <div className="p-3 overflow-auto font-mono text-[11px] leading-relaxed text-app-text whitespace-pre-wrap">
                          {formatEventMetadata(event)}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
        </div>
      </div>
    </div>
  );
}
