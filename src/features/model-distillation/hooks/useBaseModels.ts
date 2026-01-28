import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { BaseModelEntry } from "../api";

export function useBaseModels() {
  const [baseModels, setBaseModels] = useState<BaseModelEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listBaseModels();
        if (mounted) {
          setBaseModels(data);
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
  }, [refreshIndex]);

  return { baseModels, loading, error, refetch: () => setRefreshIndex((p) => p + 1) };
}
