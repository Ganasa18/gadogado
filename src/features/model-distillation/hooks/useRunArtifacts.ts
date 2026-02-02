import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { RunArtifact } from "../types";

export function useRunArtifacts(runId: string) {
  const [artifacts, setArtifacts] = useState<RunArtifact[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!runId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listRunArtifacts(runId);
        if (mounted) {
          setArtifacts(data);
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
  }, [runId, refreshIndex]);

  return { artifacts, loading, error, refetch: () => setRefreshIndex((p) => p + 1) };
}
