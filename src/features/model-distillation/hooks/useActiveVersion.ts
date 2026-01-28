import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { ModelVersion } from "../types";

export function useActiveVersion(modelId: string) {
  const [version, setVersion] = useState<ModelVersion | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!modelId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getActiveVersion(modelId);
        if (mounted) {
          setVersion(data);
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
  }, [modelId, refreshIndex]);

  return { version, loading, error, refetch: () => setRefreshIndex((p) => p + 1) };
}
