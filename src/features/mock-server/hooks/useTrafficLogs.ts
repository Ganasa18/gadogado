// =============================================================================
// Traffic Logs Hook
// Manages loading and refreshing of traffic logs
// =============================================================================

import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "../../../utils/tauri";
import { getMockServerLogs } from "../api/client";
import type { LogEntry } from "../types";

export interface UseTrafficLogsReturn {
  logs: LogEntry[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export interface UseTrafficLogsProps {
  enabled: boolean;
  refreshInterval?: number;
}

/**
 * Hook for managing traffic logs with auto-refresh
 */
export function useTrafficLogs({
  enabled,
  refreshInterval = 2000,
}: UseTrafficLogsProps): UseTrafficLogsReturn {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeLogRequest = useRef(0);

  const loadLogs = useCallback(async () => {
    if (!isTauri()) return;
    const requestId = Date.now();
    activeLogRequest.current = requestId;
    setLoading(true);
    setError(null);
    try {
      const entries = await getMockServerLogs();
      if (activeLogRequest.current !== requestId) return;
      setLogs(entries);
    } catch (err: any) {
      if (activeLogRequest.current !== requestId) return;
      console.error("Failed to load logs:", err);
      setError(err?.message || "Failed to load logs.");
    } finally {
      if (activeLogRequest.current === requestId) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;
    void loadLogs();
    const intervalId = window.setInterval(loadLogs, refreshInterval);
    return () => window.clearInterval(intervalId);
  }, [loadLogs, enabled, refreshInterval]);

  return {
    logs,
    loading,
    error,
    refresh: loadLogs,
  };
}
