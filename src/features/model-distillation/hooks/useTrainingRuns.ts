import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { TrainingRun } from "../types";

export function useTrainingRuns(filters?: {
  status?: string;
  studentModelId?: string;
  startDate?: string;
  endDate?: string;
}) {
  const [runs, setRuns] = useState<TrainingRun[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listTrainingRuns(200);
        const filtered = data.filter((run) => {
          if (filters?.status && run.status !== filters.status) {
            return false;
          }
          if (
            filters?.studentModelId &&
            run.studentModelId !== filters.studentModelId
          ) {
            return false;
          }
          return true;
        });
        if (mounted) {
          setRuns(filtered);
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

    return () => {
      mounted = false;
    };
  }, [
    filters?.status,
    filters?.studentModelId,
    filters?.startDate,
    filters?.endDate,
    refreshIndex,
  ]);

  return {
    runs,
    loading,
    error,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
