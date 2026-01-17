import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { QaRunStreamEvent } from "../../../types/qa/types";

const DEFAULT_LIMIT = 50;

export default function useQaRunStream({
  runId,
  isTauriApp,
}: {
  runId: string | null;
  isTauriApp: boolean;
}) {
  const [streamEvents, setStreamEvents] = useState<QaRunStreamEvent[]>([]);
  const [streamLoading, setStreamLoading] = useState(false);
  const [streamError, setStreamError] = useState<string | null>(null);
  const loadedRunRef = useRef<string | null>(null);

  const loadStream = async () => {
    if (!runId || !isTauriApp) return;
    setStreamLoading(true);
    setStreamError(null);
    try {
      const data = await invoke<QaRunStreamEvent[]>("qa_list_run_stream_events", {
        runId,
        limit: DEFAULT_LIMIT,
      });
      const sorted = data.slice().sort((left, right) => left.seq - right.seq);
      setStreamEvents(sorted);
      loadedRunRef.current = runId;
    } catch (err) {
      console.error(err);
      setStreamError("Failed to load run stream.");
    } finally {
      setStreamLoading(false);
    }
  };

  useEffect(() => {
    if (!runId || !isTauriApp) {
      setStreamEvents([]);
      setStreamError(null);
      loadedRunRef.current = null;
      return;
    }
    if (loadedRunRef.current !== runId) {
      void loadStream();
    }
  }, [runId, isTauriApp]);

  useEffect(() => {
    if (!runId || !isTauriApp) return;
    let unlisten: (() => void) | null = null;
    const start = async () => {
      unlisten = await listen<QaRunStreamEvent>("qa-run-stream", (event) => {
        if (event.payload.runId !== runId) return;
        setStreamEvents((prev) => {
          if (prev.some((entry) => entry.id === event.payload.id)) {
            return prev;
          }
          const next = [...prev, event.payload].sort(
            (left, right) => left.seq - right.seq
          );
          return next.slice(-DEFAULT_LIMIT);
        });
      });
    };
    void start();
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [runId, isTauriApp]);

  return {
    streamEvents,
    streamLoading,
    streamError,
    reloadStream: loadStream,
  };
}
