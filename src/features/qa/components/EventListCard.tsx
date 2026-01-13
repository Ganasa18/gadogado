import { ChevronLeft, ChevronRight, RefreshCcw, Trash2 } from "lucide-react";
import type { QaEvent } from "../../../types/qa/types";
import EventTimeline from "./EventTimeline";

type EventListCardProps = {
  events: QaEvent[];
  eventsLoading: boolean;
  eventsError: string | null;
  eventsTotal: number;
  eventsPage: number;
  eventsPageSize: number;
  totalPages: number;
  selectedCount: number;
  isPageFullySelected: boolean;
  expandedEventIds: Record<string, boolean>;
  selectedEventIds: Set<string>;
  deleteLoading: boolean;
  deleteError: string | null;
  isReplaying: boolean;
  onToggleSelectPage: () => void;
  onClearSelection: () => void;
  onDeleteSelected: () => void;
  onReplaySession: () => void;
  onPrevPage: () => void;
  onNextPage: () => void;
  onPageSizeChange: (value: number) => void;
  onRetryLoad: () => void;
  onRetryDelete: () => void;
  onToggleEventDetails: (eventId: string) => void;
  onToggleSelectEvent: (eventId: string) => void;
  onReplayEvent: (event: QaEvent) => void;
};

export default function EventListCard({
  events,
  eventsLoading,
  eventsError,
  eventsTotal,
  eventsPage,
  eventsPageSize,
  totalPages,
  selectedCount,
  isPageFullySelected,
  expandedEventIds,
  selectedEventIds,
  deleteLoading,
  deleteError,
  isReplaying,
  onToggleSelectPage,
  onClearSelection,
  onDeleteSelected,
  onReplaySession,
  onPrevPage,
  onNextPage,
  onPageSizeChange,
  onRetryLoad,
  onRetryDelete,
  onToggleEventDetails,
  onToggleSelectEvent,
  onReplayEvent,
}: EventListCardProps) {
  return (
    <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold text-app-text">Events</div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onReplaySession}
            disabled={events.length === 0 || eventsLoading || isReplaying}
            className="rounded border border-app-border px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-500/60 transition disabled:opacity-50">
            {isReplaying ? "Replaying..." : "Replay Session"}
          </button>
          <div className="text-[10px] text-app-subtext">
            {eventsLoading ? "Loading..." : `${eventsTotal} events`}
          </div>
        </div>
      </div>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-[10px] text-app-subtext">
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-full border border-app-border px-2 py-1">
            Selected {selectedCount}
          </span>
          <button
            type="button"
            onClick={onToggleSelectPage}
            disabled={events.length === 0}
            className="rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
            {isPageFullySelected ? "Unselect Page" : "Select Page"}
          </button>
          <button
            type="button"
            onClick={onClearSelection}
            disabled={selectedCount === 0}
            className="rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
            Clear
          </button>
          <button
            type="button"
            onClick={onDeleteSelected}
            disabled={selectedCount === 0 || deleteLoading}
            className="flex items-center gap-1 rounded border border-red-800/50 px-2 py-1 text-red-200 hover:border-red-600/70 transition disabled:opacity-50">
            <Trash2 className="w-3 h-3" />
            Delete Selected
          </button>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={onPrevPage}
            disabled={eventsPage <= 1}
            className="flex items-center gap-1 rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
            <ChevronLeft className="w-3 h-3" />
            Prev
          </button>
          <span className="rounded-full border border-app-border px-2 py-1">
            Page {eventsPage} / {totalPages}
          </span>
          <button
            type="button"
            onClick={onNextPage}
            disabled={eventsPage >= totalPages}
            className="flex items-center gap-1 rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
            Next
            <ChevronRight className="w-3 h-3" />
          </button>
          <label className="flex items-center gap-1">
            <span>Size</span>
            <select
              value={eventsPageSize}
              onChange={(e) => onPageSizeChange(Number(e.target.value))}
              className="rounded border border-app-border bg-black/30 px-2 py-1 text-[10px] text-app-text focus:outline-none focus:border-emerald-500/50">
              {[20, 30, 50, 100].map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>
      {eventsError && (
        <div className="mt-3 flex items-center justify-between gap-3 rounded-md border border-red-900/50 bg-red-900/10 px-3 py-2 text-[11px] text-red-200">
          <span>{eventsError}</span>
          <button
            type="button"
            onClick={onRetryLoad}
            className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
            <RefreshCcw className="w-3 h-3" />
            Retry
          </button>
        </div>
      )}
      {deleteError && (
        <div className="mt-2 flex items-center justify-between gap-3 rounded-md border border-red-900/50 bg-red-900/10 px-3 py-2 text-[11px] text-red-200">
          <span>{deleteError}</span>
          <button
            type="button"
            onClick={onRetryDelete}
            className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
            <Trash2 className="w-3 h-3" />
            Retry Delete
          </button>
        </div>
      )}
      {!eventsError && (
          <EventTimeline
            events={events}
            eventsLoading={eventsLoading}
            expandedEventIds={expandedEventIds}
            selectedEventIds={selectedEventIds}
            onToggleEventDetails={onToggleEventDetails}
            onToggleSelectEvent={onToggleSelectEvent}
            onReplayEvent={onReplayEvent}
          />

      )}
    </div>
  );
}
