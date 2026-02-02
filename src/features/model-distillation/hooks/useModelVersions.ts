import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { ModelVersion } from "../types";

export function useModelVersions(filters?: { modelId?: string; isPromoted?: boolean }) {
  const [versions, setVersions] = useState<ModelVersion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listModelVersions(filters?.modelId);
        const filtered = data.filter((v) => {
          if (
            filters?.isPromoted != null &&
            v.isPromoted !== filters.isPromoted
          ) {
            return false;
          }
          return true;
        });
        if (mounted) {
          setVersions(filtered);
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
  }, [filters?.modelId, filters?.isPromoted, refreshIndex]);

  return {
    versions,
    loading,
    error,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
