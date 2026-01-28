import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { Dataset } from "../types";

export function useDatasets(filters?: { type?: "corrections" | "golden" | "synthetic" }) {
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listDatasets(filters?.type);
        if (mounted) {
          setDatasets(data);
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
  }, [filters?.type, refreshIndex]);

  return { datasets, loading, error, refetch: () => setRefreshIndex((p) => p + 1) };
}
