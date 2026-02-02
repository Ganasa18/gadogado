import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { Correction } from "../types";

export function useCorrection(correctionId: string) {
  const [correction, setCorrection] = useState<Correction | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!correctionId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getCorrection(correctionId);
        if (mounted) {
          setCorrection(data);
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
  }, [correctionId]);

  return { correction, loading, error };
}
