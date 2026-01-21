import { useEffect, useState } from "react";
import { ModelDistillationAPI, type Model } from "../api";
import type {
  Correction,
  TrainingRun,
  ModelVersion,
  EvaluationMetric,
  TrainingLog,
  Dataset,
  RunArtifact,
  SoftLabel,
  SoftLabelGenerationResult,
} from "../types";
import type { BaseModelEntry } from "../api";

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
        console.log('[useCorrections] Fetching corrections...', { refreshIndex, filters });
        setLoading(true);
        const data = await ModelDistillationAPI.listCorrections(500);
        console.log('[useCorrections] Received data:', data.length, 'corrections');
        const filtered = data.filter((c) => {
          if (
            filters?.minAccuracyRating != null &&
            c.accuracyRating < filters.minAccuracyRating
          ) {
            return false;
          }
          return true;
        });
        console.log('[useCorrections] Filtered to:', filtered.length, 'corrections');
        if (mounted) {
          setCorrections(filtered);
          setError(null);
        }
      } catch (e) {
        console.error('[useCorrections] Error fetching corrections:', e);
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
      console.log('[useCorrections] Refetch called, incrementing refreshIndex');
      setRefreshIndex((prev) => prev + 1);
    },
  };
}

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

export function useTrainingRuns(filters?: {
  status?: string;
  studentModelId?: string;
  startDate?: string;
  endDate?: string;
}) {
  const [runs, setRuns] = useState<TrainingRun[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refreshIndex, setRefreshIndex] = useState(0);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listTrainingRuns(200);
        const filtered = data.filter((run) => {
          if (filters?.status && run.status !== filters.status) {
            return false;
          }
          if (filters?.studentModelId && run.studentModelId !== filters.studentModelId) {
            return false;
          }
          return true;
        });
        if (mounted) {
          setRuns(filtered);
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
  }, [filters?.status, filters?.studentModelId, filters?.startDate, filters?.endDate, refreshIndex]);

  return { runs, loading, error, refetch: () => setRefreshIndex((prev) => prev + 1) };
}

export function useTrainingRun(runId: string) {
  const [run, setRun] = useState<TrainingRun | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

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
  }, [runId]);

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

  return { run, loading, error, start, pause, resume, cancel, refetch: () => setLoading(true) };
}

export function useTrainingLogs(runId: string, poll: boolean = false) {
  const [logs, setLogs] = useState<TrainingLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;
    let interval: number | null = null;

    async function fetch() {
      if (!runId) return;

      try {
        const data = await ModelDistillationAPI.listTrainingLogs(runId);
        if (mounted) {
          setLogs(data);
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

    if (poll) {
      interval = window.setInterval(fetch, 1000);
    }

    return () => {
      mounted = false;
      if (interval) {
        window.clearInterval(interval);
      }
    };
  }, [runId, poll]);

  return { logs, loading, error, refetch: () => setLoading(true) };
}

export function useModelVersions(filters?: {
  modelId?: string;
  isPromoted?: boolean;
}) {
  const [versions, setVersions] = useState<ModelVersion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listModelVersions(filters?.modelId);
        const filtered = data.filter((v) => {
          if (filters?.isPromoted != null && v.isPromoted !== filters.isPromoted) {
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
  }, [filters?.modelId, filters?.isPromoted]);

  return { versions, loading, error, refetch: () => setLoading(true) };
}

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

export function useActiveVersion(modelId: string) {
  const [version, setVersion] = useState<ModelVersion | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!modelId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.getActiveVersion(modelId);
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
  }, [modelId]);

  return { version, loading, error, refetch: () => setLoading(true) };
}

export function useEvaluationMetrics(versionId: string) {
  const [metrics, setMetrics] = useState<EvaluationMetric[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      if (!versionId) return;

      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listVersionMetrics(versionId);
        if (mounted) {
          setMetrics(data);
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

  return { metrics, loading, error, refetch: () => setLoading(true) };
}

export function useDatasets(filters?: {
  type?: "corrections" | "golden" | "synthetic";
}) {
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

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
  }, [filters?.type]);

  return { datasets, loading, error, refetch: () => setLoading(true) };
}

export function useRunArtifacts(runId: string) {
  const [artifacts, setArtifacts] = useState<RunArtifact[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

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
  }, [runId]);

  return { artifacts, loading, error, refetch: () => setLoading(true) };
}

export function useBaseModels() {
  const [baseModels, setBaseModels] = useState<BaseModelEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetch() {
      try {
        setLoading(true);
        const data = await ModelDistillationAPI.listBaseModels();
        if (mounted) {
          setBaseModels(data);
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
  }, []);

  return { baseModels, loading, error, refetch: () => setLoading(true) };
}

// ============================================================================
// Soft Labels Hooks (Phase 1: Data Preparation)
// ============================================================================

export function useSoftLabels(filters?: {
  teacherModelId?: string;
  limit?: number;
}) {
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
          filters?.limit || 100
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

  return { softLabels, loading, error, refetch: () => setRefreshIndex((prev) => prev + 1) };
}

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

export function useSoftLabelGeneration() {
  const [result, setResult] = useState<SoftLabelGenerationResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const generate = async (input: {
    prompts: string[];
    teacherModelId: string;
    temperature: number;
    softLabelType: "logits" | "one_hot" | "text_only";
  }) => {
    try {
      setLoading(true);
      setError(null);
      const data = await ModelDistillationAPI.generateSoftLabels(input);
      setResult(data);
      return data;
    } catch (e) {
      const err = e as Error;
      setError(err);
      throw err;
    } finally {
      setLoading(false);
    }
  };

  const reset = () => {
    setResult(null);
    setError(null);
  };

  return { result, loading, error, generate, reset };
}

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
    refetch: () => setRefreshIndex((prev) => prev) 
  };
}
