import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { SoftLabel } from "../types";

export function useSoftLabels(filters?: { teacherModelId?: string; limit?: number }) {
  const [softLabels, setSoftLabels] = useState<SoftLabel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listSoftLabels(
          filters?.teacherModelId,
          filters?.limit || 100,
        );
        if (mounted) {
          setSoftLabels(data);
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
  }, [filters?.teacherModelId, filters?.limit, refreshIndex]);

  return {
    softLabels,
    loading,
    error,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
