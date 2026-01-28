import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { TrainingLog } from "../types";

export function useTrainingLogs(runId: string, poll: boolean = false) {
  const [logs, setLogs] = useState<TrainingLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;
    let interval: number | null = null;

    async function fetch() {
      if (!runId) return;

      try {
        const data = await ModelDistillationAPI.listTrainingLogs(runId);
        if (mounted) {
          setLogs(data);
          setError(null);
        }
      } catch (e) {
        if (mounted) {
          setError(e as Error);
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    fetch();

    if (poll) {
      interval = window.setInterval(fetch, 1000);
    }

    return () => {
      mounted = false;
      if (interval) {
        window.clearInterval(interval);
      }
    };
  }, [runId, poll, refreshIndex]);

  return {
    logs,
    loading,
    error,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
