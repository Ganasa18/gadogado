export interface Tag {
  tagId: number;
  name: string;
}

export interface Correction {
  correctionId: string;
  prompt: string;
  studentOutput: string;
  correctedOutput: string;
  accuracyRating: number;
  relevanceRating?: number;
  safetyRating?: number;
  domainNotes?: string;
  createdAt?: string;
}

export interface CorrectionWithTags extends Correction {
  tags: Tag[];
}

export interface TrainingRun {
  runId: string;
  studentModelId: string;
  baseVersionId?: string;
  teacherModelId?: string;
  method: "fine_tune" | "knowledge_distillation" | "hybrid";
  status: "queued" | "running" | "completed" | "failed" | "cancelled" | "rolled_back";
  startTime: string;
  endTime?: string;
  hyperparams: {
    epochs: number;
    batchSize: number;
    learningRate: number;
    temperature?: number;
    alpha?: number;
  };
  seed?: number;
  failureReason?: string;
}

export interface ModelVersion {
  versionId: string;
  modelId: string;
  runId?: string;
  parentVersionId?: string;
  createdAt: string;
  isPromoted: boolean;
  promotedAt?: string;
  artifactPath: string;
  artifactHash?: string;
  artifactSizeBytes?: number;
  notes?: string;
}

export interface EvaluationMetric {
  metricId: number;
  versionId: string;
  datasetId: string;
  metricName: string;
  metricValue: number;
  evaluatedAt: string;
}

export interface TrainingLog {
  logId: number;
  runId: string;
  epoch: number;
  step: number;
  loss?: number;
  lr?: number;
  temperature?: number;
  cpuUtil?: number;
  ramUsageMb?: number;
  gpuUtil?: number;
  timestamp: string;
}

export interface Dataset {
  datasetId: string;
  name: string;
  type: "corrections" | "golden" | "synthetic";
  description?: string;
  createdAt: string;
}

export interface RunArtifact {
  artifactId: string;
  runId: string;
  kind: "config" | "log" | "checkpoint" | "adapter" | "merged_model" | "gguf";
  path: string;
  hash?: string;
  sizeBytes?: number;
  createdAt: string;
}

// ============================================================================
// Soft Labels Types (Knowledge Distillation - Phase 1: Data Preparation)
// ============================================================================

export type SoftLabelType = "logits" | "one_hot" | "text_only";

export interface SoftLabel {
  softLabelId: string;
  prompt: string;
  promptHash: string;
  teacherModelId: string;
  teacherOutput: string;
  softLabelType: SoftLabelType;
  temperature: number;
  metadataJson?: string;
  createdAt?: string;
}

export interface SoftLabelGenerationInput {
  prompts: string[];
  teacherModelId: string;
  temperature: number;
  softLabelType: SoftLabelType;
}

export interface SoftLabelGenerationResult {
  softLabelIds: string[];
  cachedCount: number;
  generatedCount: number;
  failedCount: number;
  errors: string[];
}
