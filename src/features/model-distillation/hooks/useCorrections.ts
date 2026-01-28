import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { Correction } from "../types";

export function useCorrections(filters?: {
  minAccuracyRating?: number;
  tags?: string[];
  startDate?: string;
  endDate?: string;
}) {
  const [corrections, setCorrections] = useState<Correction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        console.log("[useCorrections] Fetching corrections...", {
          refreshIndex,
          filters,
        });
        setLoading(true);
        const data = await ModelDistillationAPI.listCorrections(500);
        console.log(
          "[useCorrections] Received data:",
          data.length,
          "corrections",
        );
        const filtered = data.filter((c) => {
          if (
            filters?.minAccuracyRating != null &&
            c.accuracyRating < filters.minAccuracyRating
          ) {
            return false;
          }
          return true;
        });
        console.log(
          "[useCorrections] Filtered to:",
          filtered.length,
          "corrections",
        );
        if (mounted) {
          setCorrections(filtered);
          setError(null);
        }
      } catch (e) {
        console.error("[useCorrections] Error fetching corrections:", e);
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
  }, [
    filters?.minAccuracyRating,
    JSON.stringify(filters?.tags),
    filters?.startDate,
    filters?.endDate,
    refreshIndex,
  ]);

  return {
    corrections,
    loading,
    error,
    refetch: () => {
      console.log("[useCorrections] Refetch called, incrementing refreshIndex");
      setRefreshIndex((prev) => prev + 1);
    },
  };
}
