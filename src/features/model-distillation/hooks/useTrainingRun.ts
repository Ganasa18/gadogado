import { useEffect, useState } from "react";
import { ModelDistillationAPI } from "../api";
import type { TrainingRun } from "../types";

export function useTrainingRun(runId: string) {
  const [run, setRun] = useState<TrainingRun | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!runId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getTrainingRun(runId);
        if (mounted) {
          setRun(data);
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

  const start = () =>
    ModelDistillationAPI.startPythonTraining({
      runId,
      runDir: "",
    });

  const pause = async () => {
    throw new Error("pauseTraining is not supported");
  };
  const resume = async () => {
    throw new Error("resumeTraining is not supported");
  };
  const cancel = () => ModelDistillationAPI.cancelPythonTraining(runId);

  return {
    run,
    loading,
    error,
    start,
    pause,
    resume,
    cancel,
    refetch: () => setRefreshIndex((prev) => prev + 1),
  };
}
