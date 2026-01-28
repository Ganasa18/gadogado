import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { SoftLabel } from "../types";

export function useSoftLabel(softLabelId: string) {
  const [softLabel, setSoftLabel] = useState<SoftLabel | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!softLabelId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getSoftLabel(softLabelId);
        if (mounted) {
          setSoftLabel(data);
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
  }, [softLabelId]);

  return { softLabel, loading, error };
}
