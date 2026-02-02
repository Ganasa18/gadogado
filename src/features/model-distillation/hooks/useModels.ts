import { useEffect, useState } from "react";
import { ModelDistillationAPI, type Model } from "../api";

export function useModels(provider?: string) {
  const [models, setModels] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listModels(provider);
        if (mounted) {
          setModels(data);
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
  }, [provider, refreshIndex]);

  return {
    models,
    loading,
    error,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
