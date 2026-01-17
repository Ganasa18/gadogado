import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ToastType } from "../../../shared/components/Toast";
import type { QaEvent, QaEventPage } from "../../../types/qa/types";

type AddToast = (message: string, type?: ToastType, duration?: number) => void;

type UseQaEventsOptions = {
  sessionId: string;
  isRecording: boolean;
  isTauriApp: boolean;
  pollIntervalMs: number;
  addToast: AddToast;
};

export default function useQaEvents({
  sessionId,
  isRecording,
  isTauriApp,
  pollIntervalMs,
  addToast,
}: UseQaEventsOptions) {
  const [events, setEvents] = useState<QaEvent[]>([]);
  const [eventsLoading, setEventsLoading] = useState(true);
  const [eventsError, setEventsError] = useState<string | null>(null);
  const [eventsTotal, setEventsTotal] = useState(0);
  const [eventsPage, setEventsPage] = useState(1);
  const [eventsPageSize, setEventsPageSize] = useState(30);
  const [expandedEventIds, setExpandedEventIds] = useState<
    Record<string, boolean>
  >({});
  const [selectedEventIds, setSelectedEventIds] = useState<Set<string>>(
    () => new Set()
  );
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const pollInFlightRef = useRef(false);

  const totalPages = useMemo(
    () => Math.max(1, Math.ceil(eventsTotal / eventsPageSize)),
    [eventsTotal, eventsPageSize]
  );
  const selectedCount = selectedEventIds.size;
  const pageEventIds = useMemo(
    () => events.map((event) => event.id),
    [events]
  );
  const isPageFullySelected =
    pageEventIds.length > 0 &&
    pageEventIds.every((id) => selectedEventIds.has(id));

  const fetchEventsPage = (page: number, pageSize: number) =>
    invoke<QaEventPage>("qa_list_events_page", {
      sessionId,
      page,
      pageSize,
    });

  const loadEvents = async (silent = false, pinToEnd = false) => {
    if (!sessionId || !isTauriApp) return;
    if (silent && pollInFlightRef.current) return;
    pollInFlightRef.current = true;
    if (!silent) {
      setEventsLoading(true);
    }
    setEventsError(null);
    try {
      const data = await fetchEventsPage(eventsPage, eventsPageSize);
      const lastPage = Math.max(1, Math.ceil(data.total / eventsPageSize));
      if (pinToEnd && eventsPage !== lastPage) {
        setEventsPage(lastPage);
        return;
      }
      setEvents(data.events);
      setEventsTotal(data.total);
    } catch (err) {
      console.error(err);
      setEventsError("Failed to load session events.");
    } finally {
      pollInFlightRef.current = false;
      if (!silent) {
        setEventsLoading(false);
      }
    }
  };

  const toggleEventDetails = (eventId: string) => {
    setExpandedEventIds((prev) => ({
      ...prev,
      [eventId]: !prev[eventId],
    }));
  };

  const toggleSelectEvent = (eventId: string) => {
    setSelectedEventIds((prev) => {
      const next = new Set(prev);
      if (next.has(eventId)) {
        next.delete(eventId);
      } else {
        next.add(eventId);
      }
      return next;
    });
  };

  const toggleSelectPage = () => {
    setSelectedEventIds((prev) => {
      const next = new Set(prev);
      if (isPageFullySelected) {
        pageEventIds.forEach((id) => next.delete(id));
      } else {
        pageEventIds.forEach((id) => next.add(id));
      }
      return next;
    });
  };

  const clearSelection = () => {
    setSelectedEventIds(new Set());
  };

  const handleDeleteSelected = async () => {
    if (!sessionId || selectedCount === 0) return;
    const confirmed = window.confirm(
      `Delete ${selectedCount} selected event${
        selectedCount === 1 ? "" : "s"
      }? This cannot be undone.`
    );
    if (!confirmed) return;

    setDeleteLoading(true);
    setDeleteError(null);
    try {
      const ids = Array.from(selectedEventIds);
      const deleted = await invoke<number>("qa_delete_events", {
        sessionId,
        eventIds: ids,
      });
      addToast(
        `Deleted ${deleted} event${deleted === 1 ? "" : "s"}.`,
        "success"
      );
      clearSelection();
      const nextTotal = Math.max(0, eventsTotal - deleted);
      const nextLastPage = Math.max(1, Math.ceil(nextTotal / eventsPageSize));
      if (eventsPage > nextLastPage) {
        setEventsPage(nextLastPage);
      } else {
        void loadEvents(true);
      }
    } catch (err) {
      console.error(err);
      setDeleteError("Failed to delete selected events.");
    } finally {
      setDeleteLoading(false);
    }
  };

  const handlePrevPage = () => {
    setEventsPage((page) => Math.max(1, page - 1));
  };

  const handleNextPage = () => {
    setEventsPage((page) => Math.min(totalPages, page + 1));
  };

  const handlePageSizeChange = (value: number) => {
    setEventsPageSize(value);
    setEventsPage(1);
  };

  useEffect(() => {
    setEvents([]);
    setEventsError(null);
    setEventsTotal(0);
    setEventsPage(1);
    setExpandedEventIds({});
    setSelectedEventIds(new Set());
    setDeleteError(null);
  }, [sessionId]);

  useEffect(() => {
    if (!sessionId) return;
    void loadEvents();
  }, [sessionId, eventsPage, eventsPageSize]);

  useEffect(() => {
    if (!isRecording) return;
    const timer = window.setInterval(() => {
      void loadEvents(true, true);
    }, pollIntervalMs);
    return () => window.clearInterval(timer);
  }, [isRecording, sessionId, eventsPage, eventsPageSize, pollIntervalMs]);

  useEffect(() => {
    if (!isTauriApp || !sessionId) return;
    let unlisten: (() => void) | null = null;
    const start = async () => {
      unlisten = await listen<QaEvent>("qa-event-recorded", (event) => {
        if (event.payload.session_id !== sessionId) return;
        void loadEvents(true, true);
      });
    };
    void start();
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [isTauriApp, sessionId, eventsPage, eventsPageSize]);

  return {
    events,
    eventsLoading,
    eventsError,
    eventsTotal,
    eventsPage,
    eventsPageSize,
    expandedEventIds,
    selectedEventIds,
    deleteLoading,
    deleteError,
    totalPages,
    selectedCount,
    isPageFullySelected,
    loadEvents,
    toggleEventDetails,
    toggleSelectEvent,
    toggleSelectPage,
    clearSelection,
    handleDeleteSelected,
    handlePrevPage,
    handleNextPage,
    handlePageSizeChange,
  };
}
