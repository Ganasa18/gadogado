import { invoke } from "@tauri-apps/api/core";
import type {
  Correction,
  CorrectionWithTags,
  Tag,
  TrainingRun,
  TrainingLog,
  ModelVersion,
  EvaluationMetric,
  Dataset,
  RunArtifact,
  SoftLabel,
  SoftLabelGenerationInput,
  SoftLabelGenerationResult,
} from "../types";

// ============================================================================
// Correction Input Types
// ============================================================================

export interface CorrectionInput {
  correctionId: string;
  prompt: string;
  studentOutput: string;
  correctedOutput: string;
  accuracyRating: number;
  relevanceRating?: number;
  safetyRating?: number;
  domainNotes?: string;
}

// ============================================================================
// Dataset Input Types
// ============================================================================

export interface DatasetInput {
  datasetId: string;
  name: string;
  datasetType: string;
  description?: string;
}

export interface DatasetItem {
  itemId: string;
  datasetId: string;
  prompt: string;
  expectedOutput?: string;
  metadataJson?: string;
  sourceCorrectionId?: string;
  createdAt?: string;
}

export interface DatasetItemInput {
  itemId: string;
  datasetId: string;
  prompt: string;
  expectedOutput?: string;
  metadataJson?: string;
  sourceCorrectionId?: string;
}

// ============================================================================
// Model Input Types
// ============================================================================

export interface ModelInput {
  modelId: string;
  displayName: string;
  provider: "local" | "api";
  modelFamily?: string;
  defaultArtifactPath?: string;
}

export interface Model {
  modelId: string;
  displayName: string;
  provider: string;
  modelFamily?: string;
  defaultArtifactPath?: string;
  createdAt?: string;
}

// ============================================================================
// Training Run Input Types
// ============================================================================

export interface TrainingRunInput {
  runId: string;
  studentModelId: string;
  baseVersionId?: string;
  teacherModelId?: string;
  method: "fine_tune" | "knowledge_distillation" | "hybrid";
  hyperparamsJson: string;
  seed?: number;
}

export interface TrainingLogInput {
  runId: string;
  epoch: number;
  step: number;
  loss?: number;
  lr?: number;
  temperature?: number;
  cpuUtil?: number;
  ramUsageMb?: number;
  gpuUtil?: number;
}

// ============================================================================
// Python Trainer Types
// ============================================================================

export interface DistillTrainConfig {
  runId: string;
  runDir: string;
  mode?: "fine_tune" | "knowledge_distillation" | "hybrid";
  seed?: number;
  steps?: number;
  emitEvery?: number;
  hyperparams?: unknown;
}

export interface DistillEvalConfig {
  versionId: string;
  datasetId: string;
  evalId?: string;
  maxSamples?: number;
  maxNewTokens?: number;
  temperature?: number;
  topP?: number;
  seed?: number;
  computeTeacherAgreement?: boolean;
}

// ============================================================================
// Model Version Input Types
// ============================================================================

export interface ModelVersionInput {
  versionId: string;
  modelId: string;
  runId?: string;
  parentVersionId?: string;
  artifactPath: string;
  artifactHash?: string;
  artifactSizeBytes?: number;
  notes?: string;
}

// ============================================================================
// Evaluation Input Types
// ============================================================================

export interface EvaluationMetricInput {
  versionId: string;
  datasetId: string;
  metricName: string;
  metricValue: number;
}

// ============================================================================
// Run Artifact Input Types
// ============================================================================

export interface RunArtifactInput {
  artifactId: string;
  runId: string;
  kind: "config" | "log" | "checkpoint" | "adapter" | "merged_model" | "gguf";
  path: string;
  hash?: string;
  sizeBytes?: number;
}

// ============================================================================
// Base Model Types
// ============================================================================

export interface BaseModelEntry {
  name: string;
  path: string;
  source: "resource" | "app_data";
  kind: "file" | "dir";
  format: "gguf" | "hf" | "unknown";
}

export interface BaseModelImportInput {
  sourcePath: string;
  displayName?: string;
  modelId?: string;
  modelFamily?: string;
}

export interface BaseModelImportResult {
  model: Model;
  entry: BaseModelEntry;
}

// ============================================================================
// Backup Types
// ============================================================================

export interface BackupInfo {
  path: string;
  fileName: string;
  sizeBytes: number;
  isPromotionBackup: boolean;
}

export interface ArtifactLayoutInfo {
  root: string;
  modelsBase: string;
  modelsVersions: string;
  runs: string;
  evaluations: string;
}

// ============================================================================
// Promotion & Rollback Types
// ============================================================================

export interface PromotionGuardrails {
  /** Minimum exact_match score required (0.0 - 1.0) */
  minExactMatch?: number;
  /** Minimum BLEU score required (0.0 - 1.0) */
  minBleu?: number;
  /** Minimum F1 score required (0.0 - 1.0) */
  minF1?: number;
  /** Whether to require at least one evaluation before promotion */
  requireEvaluation?: boolean;
  /** Skip guardrail checks (use with caution) */
  force?: boolean;
}

export interface GuardrailCheck {
  metricName: string;
  required: number;
  actual: number | null;
  passed: boolean;
}

export interface PromotionResult {
  success: boolean;
  versionId: string;
  modelId: string;
  guardrailChecks: GuardrailCheck[];
  backupCreated: boolean;
}

export interface RollbackResult {
  previousVersionId: string | null;
  rolledBackTo: ModelVersion;
  backupCreated: boolean;
}

// ============================================================================
// API Class
// ============================================================================

export class ModelDistillationAPI {
  // -------------------------------------------------------------------------
  // Corrections (Flow A)
  // -------------------------------------------------------------------------

  static async saveCorrection(
    input: CorrectionInput,
    tags?: string[]
  ): Promise<Correction> {
    return await invoke("distill_save_correction", { input, tags });
  }

  static async getCorrection(correctionId: string): Promise<CorrectionWithTags> {
    return await invoke("distill_get_correction", { correctionId });
  }

  static async listCorrections(limit?: number): Promise<Correction[]> {
    console.log('[ModelDistillationAPI] listCorrections called with limit:', limit);
    const result = await invoke("distill_list_corrections", { limit }) as Correction[];
    console.log('[ModelDistillationAPI] listCorrections result:', result);
    return result;
  }

  static async deleteCorrection(correctionId: string): Promise<number> {
    return await invoke("distill_delete_correction", { correctionId });
  }

  static async updateCorrectionTags(
    correctionId: string,
    tags: string[]
  ): Promise<Tag[]> {
    return await invoke("distill_update_correction_tags", { correctionId, tags });
  }

  static async listTags(): Promise<Tag[]> {
    return await invoke("distill_list_tags");
  }

  // -------------------------------------------------------------------------
  // Datasets (Flow B)
  // -------------------------------------------------------------------------

  static async createDataset(input: DatasetInput): Promise<Dataset> {
    return await invoke("distill_create_dataset", { input });
  }

  static async getDataset(datasetId: string): Promise<Dataset> {
    return await invoke("distill_get_dataset", { datasetId });
  }

  static async listDatasets(datasetType?: string): Promise<Dataset[]> {
    return await invoke("distill_list_datasets", { datasetType });
  }

  static async deleteDataset(datasetId: string): Promise<number> {
    return await invoke("distill_delete_dataset", { datasetId });
  }

  static async addDatasetItem(item: DatasetItemInput): Promise<void> {
    await invoke("distill_add_dataset_item", { item });
  }

  static async listDatasetItems(datasetId: string): Promise<DatasetItem[]> {
    return await invoke("distill_list_dataset_items", { datasetId });
  }

  // -------------------------------------------------------------------------
  // Models
  // -------------------------------------------------------------------------

  static async registerModel(input: ModelInput): Promise<Model> {
    return await invoke("distill_register_model", { input });
  }

  static async listModels(provider?: string): Promise<Model[]> {
    return await invoke("distill_list_models", { provider });
  }

  static async listBaseModels(): Promise<BaseModelEntry[]> {
    return await invoke("distill_list_base_models");
  }

  static async importBaseModel(input: BaseModelImportInput): Promise<BaseModelImportResult> {
    return await invoke("distill_import_base_model", { input });
  }

  static async downloadDefaultModel(): Promise<BaseModelImportResult> {
    return await invoke("distill_download_default_model");
  }

  static async getModel(modelId: string): Promise<Model> {
    return await invoke("distill_get_model", { modelId });
  }

  // -------------------------------------------------------------------------
  // Training Runs (Flow C)
  // -------------------------------------------------------------------------

  static async createTrainingRun(
    input: TrainingRunInput,
    correctionIds: [string, string, number][], // [correction_id, split, weight]
    datasetIds?: [string, string, number][] // [dataset_id, split, weight]
  ): Promise<TrainingRun> {
    return await invoke("distill_create_training_run", {
      input,
      correctionIds,
      datasetIds,
    });
  }

  static async updateRunStatus(
    runId: string,
    status: string,
    failureReason?: string
  ): Promise<TrainingRun> {
    return await invoke("distill_update_run_status", {
      runId,
      status,
      failureReason,
    });
  }

  static async getTrainingRun(runId: string): Promise<TrainingRun> {
    return await invoke("distill_get_training_run", { runId });
  }

  static async listTrainingRuns(limit?: number): Promise<TrainingRun[]> {
    return await invoke("distill_list_training_runs", { limit });
  }

  static async logTrainingStep(log: TrainingLogInput): Promise<void> {
    await invoke("distill_log_training_step", { log });
  }

  static async listTrainingLogs(runId: string, limit?: number): Promise<TrainingLog[]> {
    return await invoke("distill_list_training_logs", { runId, limit });
  }

  static async startPythonTraining(config: DistillTrainConfig): Promise<string> {
    return await invoke("distill_start_python_training", { config });
  }

  static async cancelPythonTraining(runId: string): Promise<void> {
    await invoke("distill_cancel_python_training", { runId });
  }

  // -------------------------------------------------------------------------
  // Model Versions (Flow E)
  // -------------------------------------------------------------------------

  static async createModelVersion(input: ModelVersionInput): Promise<ModelVersion> {
    return await invoke("distill_create_model_version", { input });
  }

  static async listModelVersions(modelId?: string): Promise<ModelVersion[]> {
    return await invoke("distill_list_model_versions", { modelId });
  }

  static async getModelVersion(versionId: string): Promise<ModelVersion> {
    return await invoke("distill_get_model_version", { versionId });
  }

  static async promoteVersion(
    modelId: string,
    versionId: string,
    guardrails?: PromotionGuardrails
  ): Promise<PromotionResult> {
    return await invoke("distill_promote_version", { modelId, versionId, guardrails });
  }

  static async getActiveVersion(modelId: string): Promise<ModelVersion | null> {
    return await invoke("distill_get_active_version", { modelId });
  }

  static async rollbackVersion(modelId: string, targetVersionId: string): Promise<RollbackResult> {
    return await invoke("distill_rollback_version", { modelId, targetVersionId });
  }

  static async getVersionHistory(
    modelId: string,
    currentVersionId?: string,
    limit?: number
  ): Promise<ModelVersion[]> {
    return await invoke("distill_get_version_history", { modelId, currentVersionId, limit });
  }

  // -------------------------------------------------------------------------
  // Evaluation (Flow D)
  // -------------------------------------------------------------------------

  static async recordMetric(metric: EvaluationMetricInput): Promise<void> {
    await invoke("distill_record_metric", { metric });
  }

  static async listVersionMetrics(versionId: string): Promise<EvaluationMetric[]> {
    return await invoke("distill_list_version_metrics", { versionId });
  }

  static async evaluateVersion(config: DistillEvalConfig): Promise<string> {
    return await invoke("distill_evaluate_version", { config });
  }

  // -------------------------------------------------------------------------
  // Run Artifacts
  // -------------------------------------------------------------------------

  static async recordArtifact(artifact: RunArtifactInput): Promise<void> {
    await invoke("distill_record_artifact", { artifact });
  }

  static async listRunArtifacts(runId: string, kind?: string): Promise<RunArtifact[]> {
    return await invoke("distill_list_run_artifacts", { runId, kind });
  }

  // -------------------------------------------------------------------------
  // Backups
  // -------------------------------------------------------------------------

  static async createBackup(reason?: string): Promise<BackupInfo> {
    return await invoke("distill_create_backup", { reason });
  }

  static async listBackups(): Promise<BackupInfo[]> {
    return await invoke("distill_list_backups");
  }

  static async restoreBackup(backupPath: string): Promise<BackupInfo> {
    return await invoke("distill_restore_backup", { backupPath });
  }

  static async cleanupOldBackups(): Promise<string[]> {
    return await invoke("distill_cleanup_old_backups");
  }

  // -------------------------------------------------------------------------
  // Dataset Import
  // -------------------------------------------------------------------------

  static async importDatasetJsonl(input: {
    datasetId?: string;
    name: string;
    datasetType: string;
    description?: string;
    path: string;
  }): Promise<Dataset> {
    return await invoke("distill_import_dataset_jsonl", { input });
  }

  // -------------------------------------------------------------------------
  // Soft Labels (Phase 1: Data Preparation)
  // -------------------------------------------------------------------------

  static async generateSoftLabels(input: SoftLabelGenerationInput): Promise<SoftLabelGenerationResult> {
    return await invoke("distill_generate_soft_labels", { input });
  }

  static async listSoftLabels(teacherModelId?: string, limit?: number): Promise<SoftLabel[]> {
    return await invoke("distill_list_soft_labels", { teacherModelId, limit });
  }

  static async getSoftLabel(softLabelId: string): Promise<SoftLabel> {
    return await invoke("distill_get_soft_label", { softLabelId });
  }

  static async deleteSoftLabel(softLabelId: string): Promise<number> {
    return await invoke("distill_delete_soft_label", { softLabelId });
  }

  static async linkSoftLabelsToRun(runId: string, softLabelIds: string[]): Promise<void> {
    return await invoke("distill_link_soft_labels_to_run", { runId, softLabelIds });
  }

  // -------------------------------------------------------------------------
  // Artifact Layout
  // -------------------------------------------------------------------------

  static async getArtifactLayout(): Promise<ArtifactLayoutInfo> {
    return await invoke("distill_get_artifact_layout");
  }
}

// Re-export for convenience
export {
  type Correction,
  type CorrectionWithTags,
  type Tag,
  type TrainingRun,
  type TrainingLog,
  type ModelVersion,
  type EvaluationMetric,
  type Dataset,
  type RunArtifact,
  type SoftLabel,
  type SoftLabelType,
  type SoftLabelGenerationInput,
  type SoftLabelGenerationResult,
} from "../types";
