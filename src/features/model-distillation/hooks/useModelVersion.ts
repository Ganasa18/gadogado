import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { ModelVersion } from "../types";

export function useModelVersion(versionId: string) {
  const [version, setVersion] = useState<ModelVersion | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!versionId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getModelVersion(versionId);
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
  }, [versionId]);

  return { version, loading, error };
}
