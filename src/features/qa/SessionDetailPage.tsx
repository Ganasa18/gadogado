import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Camera,
  ChevronLeft,
  ChevronRight,
  Maximize2,
  PauseCircle,
  Plus,
  PlayCircle,
  RefreshCcw,
  AlertTriangle,
  ScreenShare,
  Trash2,
} from "lucide-react";
import { useToastStore } from "../../store/toast";
import { useQaSessionStore } from "../../store/qaSession";
import { isTauri } from "../../utils/tauri";

interface QaSession {
  id: string;
  title: string;
  goal: string;
  is_positive_case: boolean;
  app_version?: string | null;
  os?: string | null;
  started_at: number;
  ended_at?: number | null;
  notes?: string | null;
}

interface QaEvent {
  id: string;
  session_id: string;
  seq: number;
  ts: number;
  event_type: string;
  selector?: string | null;
  element_text?: string | null;
  value?: string | null;
  url?: string | null;
  screenshot_id?: string | null;
  meta_json?: string | null;
}

type ScreenshotResult = {
  path?: string | null;
  dataUrl?: string | null;
};

type QaEventPage = {
  events: QaEvent[];
  total: number;
  page: number;
  pageSize: number;
};

const PREVIEW_LOAD_TIMEOUT_MS = 12000;
const EVENTS_POLL_INTERVAL_MS = 2000;

export default function SessionDetailPage() {
  const { id } = useParams();
  const sessionId = id ?? "";
  const navigate = useNavigate();
  const { addToast } = useToastStore();
  const {
    recordingSessionId,
    setActiveSessionId,
    setRecordingSessionId,
    recordingMode,
    setRecordingMode,
    recordingDelay,
    setRecordingDelay,
    isRecordingArmed,
    setIsRecordingArmed,
  } = useQaSessionStore();

  const [session, setSession] = useState<QaSession | null>(null);
  const [sessionLoading, setSessionLoading] = useState(true);
  const [sessionError, setSessionError] = useState<string | null>(null);

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

  const [previewLoading, setPreviewLoading] = useState(true);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewReloadToken, setPreviewReloadToken] = useState(0);

  const [screenshotLoading, setScreenshotLoading] = useState(false);
  const [screenshotError, setScreenshotError] = useState<string | null>(null);
  const [screenshotSrc, setScreenshotSrc] = useState<string | null>(null);
  const [fabOpen, setFabOpen] = useState(false);

  const previewContainerRef = useRef<HTMLDivElement | null>(null);
  const previewFrameRef = useRef<HTMLIFrameElement | null>(null);
  const previewTimeoutRef = useRef<number | null>(null);
  const lastEventIdRef = useRef<string | null>(null);

  const previewUrl = useMemo(() => extractPreviewUrl(session), [session]);
  const previewUrlValid = useMemo(
    () => (previewUrl ? isValidUrl(previewUrl) : false),
    [previewUrl]
  );

  // Check if preview URL is cross-origin and needs proxy
  const proxiedPreviewUrl = useMemo(() => {
    if (!previewUrl || !previewUrlValid) return null;

    try {
      const url = new URL(previewUrl);
      const currentOrigin = window.location.origin;
      const previewOrigin = `${url.protocol}//${url.host}`;

      // If EXACT same origin (protocol + host + port), use direct URL
      if (previewOrigin === currentOrigin) {
        console.log(
          `[QA Session] Same origin, using direct URL: ${previewUrl}`
        );
        return previewUrl;
      }

      // Different origin (including different ports like localhost:3000): use proxy
      console.log(
        `[QA Session] Cross-origin detected (${previewOrigin} !== ${currentOrigin}), using proxy`
      );
      return `http://localhost:3001/api/qa/proxy?url=${encodeURIComponent(
        previewUrl
      )}`;
    } catch {
      return previewUrl;
    }
  }, [previewUrl, previewUrlValid]);

  const isRecording = recordingSessionId === sessionId;
  const canStartRecording =
    Boolean(session) && !session?.ended_at && !isRecording;
  const canStopRecording = isRecording;
  const canEndSession = Boolean(session) && !session?.ended_at;

  const loadSession = async () => {
    if (!sessionId) {
      setSessionError("Missing session ID.");
      setSessionLoading(false);
      return;
    }
    if (!isTauri()) {
      setSessionError("QA sessions are only available in the Tauri app.");
      setSessionLoading(false);
      return;
    }

    setSessionLoading(true);
    setSessionError(null);
    setSession(null);
    try {
      const data = await invoke<QaSession>("qa_get_session", {
        sessionId,
      });
      setSession(data);
    } catch (err) {
      console.error(err);
      setSessionError("Failed to load QA session.");
    } finally {
      setSessionLoading(false);
    }
  };

  const fetchEventsPage = (page: number, pageSize: number) =>
    invoke<QaEventPage>("qa_list_events_page", {
      sessionId,
      page,
      pageSize,
    });

  const loadEvents = async (silent = false, pinToEnd = false) => {
    if (!sessionId || !isTauri()) return;
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
      if (!silent) {
        setEventsLoading(false);
      }
    }
  };

  const resolveScreenshotSrc = (payload: ScreenshotResult | string) => {
    if (typeof payload === "string") {
      return payload.startsWith("data:")
        ? payload
        : isTauri()
        ? convertFileSrc(payload)
        : payload;
    }
    if (payload.dataUrl) return payload.dataUrl;
    if (payload.path) {
      return isTauri() ? convertFileSrc(payload.path) : payload.path;
    }
    return null;
  };

  const requestIframeCapture = async (frame: HTMLIFrameElement) =>
    new Promise<string>((resolve, reject) => {
      const requestId = `capture-${Date.now()}-${Math.random()
        .toString(36)
        .slice(2, 8)}`;
      const timeoutId = window.setTimeout(() => {
        window.removeEventListener("message", handleMessage);
        reject(new Error("Preview capture timed out."));
      }, 8000);

      const handleMessage = (event: MessageEvent) => {
        if (event.source !== frame.contentWindow) return;
        if (!event.data || event.data.requestId !== requestId) return;
        if (event.data.type === "qa-recorder-capture") {
          window.clearTimeout(timeoutId);
          window.removeEventListener("message", handleMessage);
          if (typeof event.data.dataUrl !== "string") {
            reject(new Error("Preview capture returned invalid data."));
            return;
          }
          resolve(event.data.dataUrl);
          return;
        }
        if (event.data.type === "qa-recorder-capture-error") {
          window.clearTimeout(timeoutId);
          window.removeEventListener("message", handleMessage);
          reject(
            new Error(event.data.error || "Preview capture failed in iframe.")
          );
        }
      };

      window.addEventListener("message", handleMessage);
      frame.contentWindow?.postMessage(
        { type: "qa-recorder-command", action: "capture", requestId },
        "*"
      );
    });

  const capturePreviewDataUrl = async () => {
    const frame = previewFrameRef.current;
    if (!frame) {
      throw new Error("Preview frame is not available.");
    }
    const doc = frame.contentDocument;
    if (!doc) {
      return requestIframeCapture(frame);
    }
    const { width, height } = frame.getBoundingClientRect();
    const safeWidth = Math.max(1, Math.floor(width));
    const safeHeight = Math.max(1, Math.floor(height));

    try {
      return await renderDocumentToDataUrl(
        doc.documentElement,
        safeWidth,
        safeHeight
      );
    } catch (err) {
      if (isTaintedCanvasError(err)) {
        const sanitized = sanitizeDocumentElement(doc.documentElement);
        return renderDocumentToDataUrl(sanitized, safeWidth, safeHeight);
      }
      throw err;
    }
  };

  const captureScreenshot = async (silent = false, eventId?: string | null) => {
    if (!sessionId) return;
    if (!isTauri()) {
      if (!silent) {
        setScreenshotError("Screenshot capture requires the Tauri app.");
      }
      return;
    }

    setScreenshotLoading(true);
    if (!silent) {
      setScreenshotError(null);
    }
    try {
      const dataUrl = await capturePreviewDataUrl();
      const result = await invoke<ScreenshotResult | string>(
        "qa_capture_screenshot",
        {
          sessionId,
          dataUrl,
          eventId: eventId ?? undefined,
        }
      );
      const nextSrc = resolveScreenshotSrc(result);
      if (!nextSrc) {
        throw new Error("Screenshot payload missing.");
      }
      setScreenshotSrc(nextSrc);
      setScreenshotError(null);
    } catch (err) {
      console.error(err);
      if (!silent) {
        const message =
          err instanceof Error && err.message
            ? err.message
            : "Failed to capture screenshot.";
        setScreenshotError(message);
      }
    } finally {
      setScreenshotLoading(false);
    }
  };

  const handleStartRecording = () => {
    if (!session) return;
    if (!isTauri()) {
      addToast("QA recording is only available in the Tauri app", "error");
      return;
    }
    if (session.ended_at) {
      addToast("This session has already ended", "error");
      return;
    }
    if (recordingSessionId && recordingSessionId !== session.id) {
      addToast("Another QA session is recording", "error");
      return;
    }
    setActiveSessionId(session.id);
    setRecordingSessionId(session.id);
    addToast("QA recording started", "success");
  };

  const handleStopRecording = () => {
    if (!isRecording) return;
    setRecordingSessionId(null);
    addToast("QA recording paused", "info");
  };

  const handleReloadPreview = () => {
    setPreviewReloadToken((value) => value + 1);
  };

  const handleEndSession = async () => {
    if (!session) return;
    if (!isTauri()) {
      addToast("QA sessions are only available in the Tauri app", "error");
      return;
    }
    if (session.ended_at) {
      addToast("This session has already ended", "info");
      return;
    }
    const confirmed = window.confirm(
      "End this QA session? Recording will stop."
    );
    if (!confirmed) return;
    try {
      const updated = await invoke<QaSession>("qa_end_session", {
        sessionId: session.id,
      });
      setSession(updated);
      setRecordingSessionId(null);
      setActiveSessionId(null);
      addToast("QA session ended", "success");
    } catch (err) {
      console.error(err);
      addToast("Failed to end QA session", "error");
    }
  };

  const toggleEventDetails = (eventId: string) => {
    setExpandedEventIds((prev) => ({
      ...prev,
      [eventId]: !prev[eventId],
    }));
  };

  const handleToggleFullscreen = () => {
    if (!previewContainerRef.current) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void previewContainerRef.current.requestFullscreen();
    }
  };

  const sendPreviewCommand = (action: "back" | "refocus") => {
    const frame = previewFrameRef.current;
    if (!frame?.contentWindow) {
      addToast("Preview is not ready yet.", "error");
      return;
    }
    frame.contentWindow.postMessage(
      { type: "qa-recorder-command", action },
      "*"
    );
  };

  const handleManualRecordNext = () => {
    if (isRecordingArmed) {
      setIsRecordingArmed(false);
      return;
    }
    setIsRecordingArmed(true);
    sendPreviewCommand("refocus");
  };

  const totalPages = Math.max(1, Math.ceil(eventsTotal / eventsPageSize));
  const selectedCount = selectedEventIds.size;
  const pageEventIds = events.map((event) => event.id);
  const isPageFullySelected =
    pageEventIds.length > 0 &&
    pageEventIds.every((id) => selectedEventIds.has(id));

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
    void loadSession();
  }, [sessionId]);

  useEffect(() => {
    setEvents([]);
    setEventsError(null);
    setEventsTotal(0);
    setEventsPage(1);
    setScreenshotSrc(null);
    setScreenshotError(null);
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
    }, EVENTS_POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [isRecording, sessionId, eventsPage, eventsPageSize]);

  useEffect(() => {
    if (!isRecording || !lastEventIdRef.current) return;
    void captureScreenshot(true, lastEventIdRef.current);
  }, [isRecording]);

  useEffect(() => {
    if (!previewUrlValid) {
      setPreviewLoading(false);
      setPreviewError(null);
      if (previewTimeoutRef.current) {
        window.clearTimeout(previewTimeoutRef.current);
      }
      return;
    }

    setPreviewLoading(true);
    setPreviewError(null);
    if (previewTimeoutRef.current) {
      window.clearTimeout(previewTimeoutRef.current);
    }
    previewTimeoutRef.current = window.setTimeout(() => {
      setPreviewLoading(false);
      setPreviewError("Preview failed to load.");
    }, PREVIEW_LOAD_TIMEOUT_MS);

    return () => {
      if (previewTimeoutRef.current) {
        window.clearTimeout(previewTimeoutRef.current);
      }
    };
  }, [previewUrlValid, previewReloadToken, previewUrl]);

  useEffect(() => {
    if (!isRecording) {
      lastEventIdRef.current = null;
      return;
    }
    if (events.length === 0) {
      lastEventIdRef.current = null;
      return;
    }
    const latestId = events[events.length - 1]?.id ?? null;
    if (!latestId) return;
    if (lastEventIdRef.current !== latestId) {
      lastEventIdRef.current = latestId;
      void captureScreenshot(true, latestId);
    }
  }, [events, isRecording]);

  const showLivePreview = isRecording && Boolean(screenshotSrc);
  const showLivePreviewLoading =
    isRecording && screenshotLoading && !screenshotSrc;

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-4 flex flex-col gap-4">
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={() => navigate("/qa/history")}
                className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-3 py-2 text-[11px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition">
                <ArrowLeft className="w-3.5 h-3.5" />
                Back to History
              </button>
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
                onClick={loadSession}
                className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
                <RefreshCcw className="w-3 h-3" />
                Retry
              </button>
            </div>
          )}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-1 xl:grid-cols-3 gap-4">
          <section className="space-y-4">
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
                  <div className="text-[10px] text-gray-500">Started</div>
                  <div className="text-gray-300">
                    {session?.started_at
                      ? formatTimestamp(session.started_at)
                      : "n/a"}
                  </div>
                </div>
                <div className="rounded-md border border-app-border bg-black/20 p-2">
                  <div className="text-[10px] text-gray-500">Ended</div>
                  <div className="text-gray-300">
                    {session?.ended_at
                      ? formatTimestamp(session.ended_at)
                      : "Still running"}
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
              <div className="flex items-center justify-between gap-2">
                <div className="text-[11px] font-semibold text-app-text">
                  Events
                </div>
                <div className="text-[10px] text-app-subtext">
                  {eventsLoading ? "Loading..." : `${eventsTotal} events`}
                </div>
              </div>
              <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-[10px] text-app-subtext">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="rounded-full border border-app-border px-2 py-1">
                    Selected {selectedCount}
                  </span>
                  <button
                    type="button"
                    onClick={toggleSelectPage}
                    disabled={events.length === 0}
                    className="rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
                    {isPageFullySelected ? "Unselect Page" : "Select Page"}
                  </button>
                  <button
                    type="button"
                    onClick={clearSelection}
                    disabled={selectedCount === 0}
                    className="rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
                    Clear
                  </button>
                  <button
                    type="button"
                    onClick={handleDeleteSelected}
                    disabled={selectedCount === 0 || deleteLoading}
                    className="flex items-center gap-1 rounded border border-red-800/50 px-2 py-1 text-red-200 hover:border-red-600/70 transition disabled:opacity-50">
                    <Trash2 className="w-3 h-3" />
                    Delete Selected
                  </button>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <button
                    type="button"
                    onClick={handlePrevPage}
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
                    onClick={handleNextPage}
                    disabled={eventsPage >= totalPages}
                    className="flex items-center gap-1 rounded border border-app-border px-2 py-1 hover:border-emerald-600/60 transition disabled:opacity-50">
                    Next
                    <ChevronRight className="w-3 h-3" />
                  </button>
                  <label className="flex items-center gap-1">
                    <span>Size</span>
                    <select
                      value={eventsPageSize}
                      onChange={(e) =>
                        handlePageSizeChange(Number(e.target.value))
                      }
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
                    onClick={() => void loadEvents(false)}
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
                    onClick={handleDeleteSelected}
                    className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
                    <Trash2 className="w-3 h-3" />
                    Retry Delete
                  </button>
                </div>
              )}
              {!eventsError && (
                <div className="mt-3 max-h-[520px] overflow-y-auto">
                  <div className="event-timeline px-6 py-4">
                    <h2 className="text-[10px] font-mono uppercase tracking-wide text-app-subtext mb-4 sticky top-0 bg-app-card z-10 py-2 -mx-6 px-6 border-b border-app-border -mt-4">
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
                          const isExpanded = Boolean(
                            expandedEventIds[event.id]
                          );
                          const isSelected = selectedEventIds.has(event.id);
                          const { primary, secondary } = getEventDetails(event);
                          return (
                            <div key={event.id}>
                              <button
                                type="button"
                                onClick={() => toggleEventDetails(event.id)}
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
                                      onChange={() =>
                                        toggleSelectEvent(event.id)
                                      }
                                      className="h-3 w-3 rounded border-app-border bg-black/40 text-emerald-400 focus:ring-emerald-500/40"
                                    />
                                  </div>
                                  <div
                                    className={`font-mono text-[11px] w-12 shrink-0 ${
                                      isExpanded
                                        ? "text-emerald-200"
                                        : "text-app-subtext"
                                    }`}>
                                    {formatEventSeq(event.seq)}
                                  </div>
                                  <div
                                    className={`font-mono text-[11px] w-28 shrink-0 ${
                                      isExpanded
                                        ? "text-app-text"
                                        : "text-app-subtext"
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
                                      isExpanded
                                        ? "text-app-text"
                                        : "text-app-subtext"
                                    }`}
                                    title={primary}>
                                    {primary}
                                  </div>
                                  <div
                                    className={`font-mono text-[11px] truncate flex-1 ${
                                      isExpanded
                                        ? "text-app-text"
                                        : "text-app-subtext"
                                    }`}
                                    title={secondary}>
                                    {secondary}
                                  </div>
                                </div>
                              </button>
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
              )}
            </div>

            <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-app-text font-medium">
                  <Camera className="w-4 h-4 text-amber-300" />
                  <h4>Latest Screenshot</h4>
                </div>
              </div>
              <div className="mt-3 rounded-md border border-app-border bg-black/30 h-[320px] overflow-hidden flex items-center justify-center">
                {sessionLoading && (
                  <div className="text-[11px] text-app-subtext">
                    Loading screenshot...
                  </div>
                )}
                {!sessionLoading && screenshotLoading && (
                  <div className="text-[11px] text-app-subtext">
                    Capturing screenshot...
                  </div>
                )}
                {!sessionLoading && !screenshotLoading && screenshotSrc && (
                  <img
                    src={screenshotSrc}
                    alt="Latest QA screenshot"
                    className="w-full h-full object-contain"
                  />
                )}
                {!sessionLoading && !screenshotLoading && !screenshotSrc && (
                  <div className="text-[11px] text-app-subtext text-center px-6">
                    No screenshot captured yet.
                  </div>
                )}
              </div>
              {screenshotError && (
                <div className="mt-2 flex items-center gap-2 text-[11px] text-red-200">
                  <span>{screenshotError}</span>
                  <button
                    type="button"
                    onClick={() => void captureScreenshot(false)}
                    className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
                    <RefreshCcw className="w-3 h-3" />
                    Retry
                  </button>
                </div>
              )}
            </div>
          </section>
          <section className="space-y-4 col-span-2">
            <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm h-full flex flex-col">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-app-text font-medium">
                  <ScreenShare className="w-4 h-4 text-sky-300" />
                  <h4>Preview</h4>
                </div>
                <div className="text-[10px] text-app-subtext">
                  {isRecording ? "Live capture" : "Idle"}
                </div>
              </div>
              <div
                ref={previewContainerRef}
                data-qa-record-root
                className="mt-3 rounded-md border border-app-border bg-black/30 h-full overflow-hidden flex items-center justify-center relative">
                {/* LIST ACTION BUTTON */}
                <div
                  className="absolute top-2 right-2 z-20"
                  data-qa-record-ignore>
                  <div className="relative flex items-start justify-end">
                    <button
                      type="button"
                      onClick={() => setFabOpen((open) => !open)}
                      aria-label={
                        fabOpen ? "Close action menu" : "Open action menu"
                      }
                      aria-expanded={fabOpen}
                      className="flex h-9 w-9 items-center justify-center rounded-full border border-emerald-500/40 bg-emerald-900/60 text-emerald-100 shadow-md shadow-emerald-900/20 transition hover:border-emerald-400/70 hover:bg-emerald-900/50">
                      <Plus
                        className={`h-4 w-4 transition-transform duration-200 ${
                          fabOpen ? "rotate-45" : "rotate-0"
                        }`}
                      />
                    </button>

                    <div
                      className={`absolute right-0 top-full mt-2 flex flex-col gap-2 transition-all duration-200 ease-out ${
                        fabOpen
                          ? "pointer-events-auto translate-y-0 opacity-100"
                          : "pointer-events-none -translate-y-2 opacity-0"
                      }`}>
                      {!isRecording && (
                        <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
                          <button
                            type="button"
                            onClick={() => setRecordingMode("auto")}
                            className={`px-2 py-0.5 text-[9px] rounded transition ${
                              recordingMode === "auto"
                                ? "bg-emerald-700/40 text-emerald-100 border border-emerald-500/50"
                                : "text-app-subtext hover:text-app-text"
                            }`}>
                            Auto
                          </button>
                          <button
                            type="button"
                            onClick={() => setRecordingMode("manual")}
                            className={`px-2 py-0.5 text-[9px] rounded transition ${
                              recordingMode === "manual"
                                ? "bg-blue-700/40 text-blue-100 border border-blue-500/50"
                                : "text-app-subtext hover:text-app-text"
                            }`}>
                            Manual
                          </button>
                        </div>
                      )}

                      {!isRecording && (
                        <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
                          <label
                            htmlFor="recording-delay"
                            className="text-[9px] text-app-subtext">
                            Delay:
                          </label>
                          <input
                            id="recording-delay"
                            type="number"
                            min="0"
                            max="5000"
                            step="100"
                            value={recordingDelay}
                            onChange={(e) =>
                              setRecordingDelay(Number(e.target.value))
                            }
                            className="w-12 bg-black/30 border border-app-border rounded px-1 py-0.5 text-[9px] text-app-text focus:outline-none focus:border-emerald-500/50"
                          />
                          <span className="text-[9px] text-app-subtext">
                            ms
                          </span>
                        </div>
                      )}

                      {isRecording && recordingMode === "manual" && (
                        <button
                          type="button"
                          onClick={handleManualRecordNext}
                          className={`flex items-center gap-2 border rounded px-2 py-1 text-[10px] transition ${
                            isRecordingArmed
                              ? "bg-blue-900/40 border-blue-500/60 text-blue-100"
                              : "bg-[#1a2a3a] border-blue-800/40 text-blue-200 hover:border-blue-500/60"
                          }`}>
                          <PlayCircle className="w-3 h-3" />
                          {isRecordingArmed ? "Armed (Cancel)" : "Record Next"}
                        </button>
                      )}

                      <button
                        type="button"
                        onClick={handleStartRecording}
                        disabled={!canStartRecording}
                        className="flex items-center gap-2 bg-[#133122] border border-emerald-800/40 rounded px-2 py-1 text-[10px] text-emerald-200 hover:border-emerald-500/60 transition disabled:opacity-50">
                        <PlayCircle className="w-3 h-3" />
                        {isRecording ? "Recording..." : "Start Record"}
                      </button>
                      <button
                        type="button"
                        onClick={handleStopRecording}
                        disabled={!canStopRecording}
                        className="flex items-center gap-2 bg-[#2a1d1d] border border-red-900/40 rounded px-2 py-1 text-[10px] text-red-200 hover:border-red-700/60 transition disabled:opacity-50">
                        <PauseCircle className="w-3 h-3" />
                        Stop Record
                      </button>
                      <button
                        type="button"
                        onClick={handleEndSession}
                        disabled={!canEndSession}
                        className="flex items-center gap-2 bg-[#2a1414] border border-red-800/50 rounded px-2 py-1 text-[10px] text-red-200 hover:border-red-500/70 hover:text-red-100 transition disabled:opacity-50 shadow-sm">
                        <AlertTriangle className="w-3 h-3" />
                        End Session
                      </button>
                    </div>

                    <div
                      className={`absolute right-full top-0 mr-2 flex items-center gap-2 transition-all duration-200 ease-out ${
                        fabOpen
                          ? "pointer-events-auto translate-x-0 opacity-100"
                          : "pointer-events-none translate-x-2 opacity-0"
                      }`}>
                      <button
                        type="button"
                        onClick={() =>
                          void captureScreenshot(false, lastEventIdRef.current)
                        }
                        disabled={screenshotLoading}
                        className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
                        <Camera className="w-3 h-3" />
                        Capture
                      </button>
                      <button
                        type="button"
                        onClick={handleReloadPreview}
                        disabled={!previewUrlValid}
                        className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
                        <RefreshCcw className="w-3 h-3" />
                        Reload
                      </button>
                      <button
                        type="button"
                        onClick={() => sendPreviewCommand("back")}
                        disabled={!previewUrlValid}
                        className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
                        <ChevronLeft className="w-3 h-3" />
                        Back
                      </button>
                      <button
                        type="button"
                        onClick={handleToggleFullscreen}
                        disabled={!previewUrlValid}
                        className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50 justify-center">
                        <Maximize2 className="w-3 h-3" />
                        Full
                      </button>
                    </div>
                  </div>
                </div>
                {isRecording && (
                  <div className="absolute top-2 left-2 z-20 flex items-center gap-2 rounded-full border border-red-900/60 bg-red-900/20 px-2 py-0.5 text-[10px] text-red-100">
                    <span className="h-2 w-2 rounded-full bg-red-400" />
                    REC
                  </div>
                )}
                {sessionLoading && (
                  <div className="text-[11px] text-app-subtext">
                    Loading preview...
                  </div>
                )}
                {!sessionLoading && showLivePreviewLoading && (
                  <div className="text-[11px] text-app-subtext text-center px-6">
                    Capturing live preview...
                  </div>
                )}
                {!sessionLoading && showLivePreview && screenshotSrc && (
                  <img
                    src={screenshotSrc}
                    alt="Live QA preview"
                    className="absolute inset-0 w-full h-full object-contain"
                  />
                )}
                {!sessionLoading && !previewUrlValid && !showLivePreview && (
                  <div className="text-[11px] text-app-subtext text-center px-6">
                    Preview URL is missing or invalid. Add a valid URL when
                    creating the session to enable the preview.
                  </div>
                )}
                {!sessionLoading && previewUrlValid && (
                  <iframe
                    key={`${proxiedPreviewUrl}-${previewReloadToken}`}
                    src={proxiedPreviewUrl ?? undefined}
                    title="QA session preview"
                    data-qa-preview-frame
                    ref={previewFrameRef}
                    className={`w-full h-full border-none ${
                      showLivePreview ? "opacity-0 pointer-events-none" : ""
                    }`}
                    onLoad={() => {
                      if (previewTimeoutRef.current) {
                        window.clearTimeout(previewTimeoutRef.current);
                      }
                      setPreviewLoading(false);
                      setPreviewError(null);
                    }}
                    onError={() => {
                      if (previewTimeoutRef.current) {
                        window.clearTimeout(previewTimeoutRef.current);
                      }
                      setPreviewLoading(false);
                      setPreviewError("Preview failed to load.");
                    }}
                  />
                )}
              </div>
              {!sessionLoading &&
                previewUrlValid &&
                previewLoading &&
                !showLivePreview && (
                  <div className="mt-2 text-[10px] text-app-subtext">
                    Loading preview...
                  </div>
                )}
              {!sessionLoading &&
                previewUrlValid &&
                previewError &&
                !showLivePreview && (
                  <div className="mt-2 flex items-center gap-2 text-[11px] text-red-200">
                    <span>{previewError}</span>
                    <button
                      type="button"
                      onClick={handleReloadPreview}
                      className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
                      <RefreshCcw className="w-3 h-3" />
                      Reload Preview
                    </button>
                  </div>
                )}
            </div>
          </section>
        </div>
      </aside>
    </div>
  );
}

function extractPreviewUrl(session: QaSession | null): string | null {
  if (!session?.notes) return null;
  const trimmed = session.notes.trim();
  if (!trimmed) return null;
  try {
    const parsed = JSON.parse(trimmed);
    if (parsed && typeof parsed === "object") {
      const url =
        typeof (parsed as { preview_url?: unknown }).preview_url === "string"
          ? (parsed as { preview_url: string }).preview_url
          : typeof (parsed as { target_url?: unknown }).target_url === "string"
          ? (parsed as { target_url: string }).target_url
          : null;
      return url ?? null;
    }
  } catch {
    return trimmed;
  }
  return null;
}

function isValidUrl(value: string) {
  try {
    const url = new URL(value);
    return Boolean(url.protocol && url.hostname);
  } catch {
    return false;
  }
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp).toLocaleString();
}

function pickFirstString(values: Array<string | null | undefined>) {
  return values.find(
    (value) => typeof value === "string" && value.trim().length > 0
  );
}

function formatEventSeq(seq: number) {
  return seq.toString().padStart(3, "0");
}

function formatEventTime(timestamp: number) {
  const date = new Date(timestamp);
  const time = date.toLocaleTimeString([], {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  const ms = String(date.getMilliseconds()).padStart(3, "0");
  return `${time}.${ms}`;
}

function getEventDetails(event: QaEvent) {
  const primary =
    pickFirstString([event.selector, event.element_text, event.url]) || "n/a";
  const secondary =
    pickFirstString(
      [event.value, event.url, event.element_text, event.selector].filter(
        (value) => value !== primary
      )
    ) || "n/a";
  return { primary, secondary };
}

function getEventBadgeClasses(eventType: string) {
  const normalized = eventType.toLowerCase();
  if (normalized.includes("navigate") || normalized.includes("route")) {
    return "bg-sky-500/10 text-sky-300 border-sky-500/20";
  }
  if (normalized.includes("input") || normalized.includes("change")) {
    return "bg-purple-500/10 text-purple-300 border-purple-500/20";
  }
  if (normalized.includes("click")) {
    return "bg-emerald-500/10 text-emerald-300 border-emerald-500/20";
  }
  if (normalized.includes("submit")) {
    return "bg-amber-500/10 text-amber-300 border-amber-500/20";
  }
  return "bg-slate-500/10 text-slate-300 border-slate-500/20";
}

function formatEventMetadata(event: QaEvent) {
  let meta: unknown = null;
  if (event.meta_json) {
    try {
      meta = JSON.parse(event.meta_json);
    } catch {
      meta = event.meta_json;
    }
  }
  const payload = {
    event_type: event.event_type,
    selector: event.selector ?? undefined,
    element_text: event.element_text ?? undefined,
    value: event.value ?? undefined,
    url: event.url ?? undefined,
    screenshot_id: event.screenshot_id ?? undefined,
    timestamp: event.ts,
    meta,
  };
  return JSON.stringify(payload, null, 2);
}

function isTaintedCanvasError(err: unknown): boolean {
  if (!(err instanceof Error)) return false;
  return (
    err.message.includes("Tainted canvases") ||
    err.message.includes("SecurityError")
  );
}

function sanitizeDocumentElement(root: HTMLElement): HTMLElement {
  const clone = root.cloneNode(true) as HTMLElement;
  const stripSelectors = [
    "img",
    "picture",
    "source",
    "video",
    "audio",
    "canvas",
    "iframe",
    "svg",
    'link[rel="stylesheet"]',
  ];
  clone.querySelectorAll(stripSelectors.join(",")).forEach((el) => el.remove());

  clone.querySelectorAll("style").forEach((style) => {
    if (!style.textContent) return;
    let text = style.textContent;
    text = text.replace(/@font-face\s*\{[\s\S]*?\}/g, "");
    text = text.replace(/url\(([^)]+)\)/g, "none");
    style.textContent = text;
  });

  clone.querySelectorAll("[style]").forEach((el) => {
    const inline = el.getAttribute("style");
    if (!inline || !inline.includes("url(")) return;
    const cleaned = inline.replace(/url\(([^)]+)\)/g, "none");
    el.setAttribute("style", cleaned);
  });

  return clone;
}

async function renderDocumentToDataUrl(
  root: HTMLElement,
  width: number,
  height: number
): Promise<string> {
  const safeWidth = Math.max(1, Math.floor(width));
  const safeHeight = Math.max(1, Math.floor(height));
  const serialized = new XMLSerializer().serializeToString(root);
  const wrapped = `<div xmlns="http://www.w3.org/1999/xhtml">${serialized}</div>`;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${safeWidth}" height="${safeHeight}"><foreignObject width="100%" height="100%">${wrapped}</foreignObject></svg>`;
  const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(blob);

  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () =>
        reject(new Error("Failed to render preview snapshot."));
      image.src = url;
    });

    const canvas = document.createElement("canvas");
    canvas.width = safeWidth;
    canvas.height = safeHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas is not available for screenshot.");
    }
    ctx.drawImage(img, 0, 0, safeWidth, safeHeight);
    return canvas.toDataURL("image/png");
  } finally {
    URL.revokeObjectURL(url);
  }
}
