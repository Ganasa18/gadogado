import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { EvaluationMetric } from "../types";

export function useEvaluationMetrics(versionId: string) {
  const [metrics, setMetrics] = useState<EvaluationMetric[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!versionId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listVersionMetrics(versionId);
        if (mounted) {
          setMetrics(data);
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
  }, [versionId, refreshIndex]);

  return { metrics, loading, error, refetch: () => setRefreshIndex((p) => p + 1) };
}
