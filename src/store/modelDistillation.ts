import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { TrainingRun, ModelVersion } from "../features/model-distillation/types";

export interface ModelDistillationState {
  // Training configuration
  studentModel: string;
  teacherModel: string;
  baseVersion: string;
  trainingMethod: "fine_tune" | "knowledge_distillation" | "hybrid";
  datasetSize: number;
  minCorrections: number;
  epochs: number;
  batchSize: number;
  learningRate: number;
  temperature: number;
  alpha: number;

  // Active session state
  activeRunId: string | null;
  activeRun: TrainingRun | null;
  selectedVersionId: string | null;
  selectedVersion: ModelVersion | null;

  // Export settings
  exportFormat: "adapter" | "merged_model" | "gguf";
  exportPath: string;

  // UI state
  logsAutoRefresh: boolean;

  // Setters
  setStudentModel: (model: string) => void;
  setTeacherModel: (model: string) => void;
  setBaseVersion: (version: string) => void;
  setTrainingMethod: (method: "fine_tune" | "knowledge_distillation" | "hybrid") => void;
  setDatasetSize: (size: number) => void;
  setMinCorrections: (count: number) => void;
  setEpochs: (epochs: number) => void;
  setBatchSize: (size: number) => void;
  setLearningRate: (rate: number) => void;
  setTemperature: (temp: number) => void;
  setAlpha: (alpha: number) => void;

  setActiveRunId: (runId: string | null) => void;
  setActiveRun: (run: TrainingRun | null) => void;
  setSelectedVersionId: (versionId: string | null) => void;
  setSelectedVersion: (version: ModelVersion | null) => void;

  setExportFormat: (format: "adapter" | "merged_model" | "gguf") => void;
  setExportPath: (path: string) => void;

  setLogsAutoRefresh: (enabled: boolean) => void;

  // Actions
  resetTrainingConfig: () => void;
  resetActiveSession: () => void;
}

const DEFAULT_CONFIG = {
  studentModel: "phi3-mini-4k",
  teacherModel: "gemini-2.5-flash-lite",
  baseVersion: "v1.0.0",
  trainingMethod: "hybrid" as const,
  datasetSize: 50,
  minCorrections: 30,
  epochs: 5,
  batchSize: 4,
  learningRate: 0.0001,
  temperature: 3.0,
  alpha: 0.7,
};

export const useModelDistillationStore = create<ModelDistillationState>()(
  persist(
    (set) => ({
      ...DEFAULT_CONFIG,
      activeRunId: null,
      activeRun: null,
      selectedVersionId: null,
      selectedVersion: null,
      exportFormat: "adapter",
      exportPath: "",
      logsAutoRefresh: true,

      setStudentModel: (studentModel) => set({ studentModel }),
      setTeacherModel: (teacherModel) => set({ teacherModel }),
      setBaseVersion: (baseVersion) => set({ baseVersion }),
      setTrainingMethod: (trainingMethod) => set({ trainingMethod }),
      setDatasetSize: (datasetSize) => set({ datasetSize }),
      setMinCorrections: (minCorrections) => set({ minCorrections }),
      setEpochs: (epochs) => set({ epochs }),
      setBatchSize: (batchSize) => set({ batchSize }),
      setLearningRate: (learningRate) => set({ learningRate }),
      setTemperature: (temperature) => set({ temperature }),
      setAlpha: (alpha) => set({ alpha }),

      setActiveRunId: (activeRunId) => set({ activeRunId }),
      setActiveRun: (activeRun) => set({ activeRun }),
      setSelectedVersionId: (selectedVersionId) => set({ selectedVersionId }),
      setSelectedVersion: (selectedVersion) => set({ selectedVersion }),

      setExportFormat: (exportFormat) => set({ exportFormat }),
      setExportPath: (exportPath) => set({ exportPath }),

      setLogsAutoRefresh: (logsAutoRefresh) => set({ logsAutoRefresh }),

      resetTrainingConfig: () => set(DEFAULT_CONFIG),
      resetActiveSession: () =>
        set({
          activeRunId: null,
          activeRun: null,
          selectedVersionId: null,
          selectedVersion: null,
        }),
    }),
    {
      name: "gadogado-model-distillation",
      version: 1,
      partialize: (state) => ({
        studentModel: state.studentModel,
        teacherModel: state.teacherModel,
        baseVersion: state.baseVersion,
        trainingMethod: state.trainingMethod,
        datasetSize: state.datasetSize,
        minCorrections: state.minCorrections,
        epochs: state.epochs,
        batchSize: state.batchSize,
        learningRate: state.learningRate,
        temperature: state.temperature,
        alpha: state.alpha,
        exportFormat: state.exportFormat,
        exportPath: state.exportPath,
        logsAutoRefresh: state.logsAutoRefresh,
      }),
    }
  )
);
