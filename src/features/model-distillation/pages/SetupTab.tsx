import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Cog,
  Database,
  CheckCircle,
  Plus,
  Loader2,
  AlertCircle,
  Trash2,
  ExternalLink,
  Cloud,
  Cpu,
  Zap,
  Activity,
} from "lucide-react";
import {
  Card,
  Select,
  Button,
  MetricCard,
  InfoBox,
  Input,
  ProgressBar,
} from "../components/UI";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  ModelDistillationAPI,
  type BaseModelEntry,
  type TrainingRunInput,
} from "../api";
import { useModelDistillationStore } from "../../../store/modelDistillation";
import {
  useCorrections,
  useDatasets,
  useTrainingRuns,
  useModels,
} from "../hooks";
import { useSettingsStore, type LLMProvider } from "../../../store/settings";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";
import { LOCAL_LLM_BASE_URL } from "../../../shared/api/llmConfig";

// Generate a UUID v4 (simple implementation)
function uuid(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// Model options for each provider
const PROVIDER_MODELS: Record<LLMProvider, string[]> = {
  local: [],
  ollama: ["llama3", "llama3.1", "mistral", "codellama", "phi3"],
  llama_cpp: ["llama-3-8b-instruct", "phi-3-mini", "mistral-7b"],
  openai: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
  gemini: ["gemini-2.5-flash-lite", "gemini-2.0-flash-lite", "gemini-1.5-pro"],
  dll: ["custom-model"],
};

// Provider display names
const PROVIDER_LABELS: Record<LLMProvider, string> = {
  local: "Local (LM Studio)",
  ollama: "Ollama",
  llama_cpp: "Llama.cpp",
  openai: "OpenAI",
  gemini: "Gemini",
  dll: "DLL",
};

export default function SetupTab() {
  const store = useModelDistillationStore();
  const {
    corrections,
    loading: correctionsLoading,
    refetch: refetchCorrections,
  } = useCorrections();
  const { datasets } = useDatasets();
  const { refetch: refetchRuns } = useTrainingRuns();
  const { models: registeredModels, refetch: refetchModels } = useModels();

  // Settings store - for LLM provider configuration
  const { provider, model, localModels, apiKey, baseUrl } = useSettingsStore();
  const buildConfig = useLlmConfigBuilder();

  // Base models state (student models from resources)
  const [baseModels, setBaseModels] = useState<BaseModelEntry[]>([]);
  const [baseModelsLoading, setBaseModelsLoading] = useState(true);

  // Form state
  const [selectedStudentPath, setSelectedStudentPath] = useState("");
  const [teacherModelSelection, setTeacherModelSelection] = useState("");
  const [selectedDatasetId, setSelectedDatasetId] = useState("");
  const [selectedCorrections, setSelectedCorrections] = useState<string[]>([]);
  const [trainingMethod, setTrainingMethod] = useState<
    "fine_tune" | "knowledge_distillation" | "hybrid"
  >("hybrid");

  // Check if LLM provider is properly configured
  const isLocalProvider =
    provider === "local" || provider === "ollama" || provider === "llama_cpp";
  const requiresApiKey =
    provider === "openai" || provider === "gemini" || provider === "dll";
  const isProviderConfigured =
    isLocalProvider || (requiresApiKey && apiKey && baseUrl);
  const teacherRequired = trainingMethod !== "fine_tune";
  const teacherReady = !teacherRequired || isProviderConfigured;

  // UI state
  const [saveStatus, setSaveStatus] = useState<
    "idle" | "saving" | "success" | "error"
  >("idle");
  const [error, setError] = useState<string | null>(null);

  // Load base models on mount
  useEffect(() => {
    loadBaseModels();
  }, []);

  // Set default teacher model when provider changes
  useEffect(() => {
    if (!isProviderConfigured) return;
    // For local providers, use the model from settings; for API providers, use the first available
    const isLocal =
      provider === "local" || provider === "ollama" || provider === "llama_cpp";
    const availableModels = isLocal
      ? provider === "local"
        ? localModels
        : PROVIDER_MODELS[provider]
      : PROVIDER_MODELS[provider];
    if (availableModels.length === 0) return;
    if (
      !teacherModelSelection ||
      !availableModels.includes(teacherModelSelection)
    ) {
      const nextModel = availableModels.includes(model)
        ? model
        : availableModels[0];
      setTeacherModelSelection(nextModel);
    }
  }, [
    isProviderConfigured,
    provider,
    model,
    localModels,
    teacherModelSelection,
  ]);

  const loadBaseModels = async () => {
    try {
      setBaseModelsLoading(true);
      console.log('[SetupTab] Loading base models...');
      const data = await ModelDistillationAPI.listBaseModels();
      console.log('[SetupTab] Base models loaded:', data);
      console.log('[SetupTab] Number of models:', data.length);
      setBaseModels(data);
    } catch (err) {
      console.error("[SetupTab] Failed to load base models:", err);
    } finally {
      setBaseModelsLoading(false);
    }
  };

  // Get available teacher models based on provider
  const getTeacherModels = (): string[] => {
    if (provider === "local") {
      return localModels.length > 0 ? localModels : [];
    }
    return PROVIDER_MODELS[provider] || [];
  };

  const buildCorrectionSplits = (
    ids: string[],
  ): [string, "train" | "val" | "test", number][] => {
    if (ids.length === 0) return [];
    const sorted = [...ids].sort();
    return sorted.map((id, idx) => {
      const ratio = idx / sorted.length;
      const split = ratio < 0.9 ? "train" : ratio < 0.95 ? "val" : "test";
      return [id, split, 1.0];
    });
  };

  const buildHyperparams = () => {
    const teacherModel = teacherModelSelection || model;
    return {
      epochs: store.epochs,
      batch_size: store.batchSize,
      learning_rate: store.learningRate,
      temperature: store.temperature,
      alpha: store.alpha,
      ...(teacherRequired && teacherModel
        ? { teacher: { provider, model: teacherModel } }
        : {}),
      lora: {
        enabled: true,
        r: 8,
        alpha: 16,
        dropout: 0.05,
      },
      training: {
        max_seq_len: 512,
        grad_accum: 1,
        warmup_steps: 10,
      },
    };
  };

  // State for training data generator
  const [showDataGenerator, setShowDataGenerator] = useState(false);
  const [promptsInput, setPromptsInput] = useState("");
  const [generatingData, setGeneratingData] = useState(false);
  const [generationProgress, setGenerationProgress] = useState({
    current: 0,
    total: 0,
  });

  const handleGenerateTrainingData = async () => {
    if (teacherRequired && !isProviderConfigured) {
      setError(
        "Please configure an LLM provider in General Settings for KD/Hybrid training",
      );
      return;
    }

    if (!selectedStudentPath) {
      setError("Please select a student model first");
      return;
    }

    const prompts = promptsInput
      .split("\n")
      .map((p) => p.trim())
      .filter((p) => p.length > 0);

    if (prompts.length === 0) {
      setError("Please enter at least one prompt");
      return;
    }

    try {
      setGeneratingData(true);
      setError(null);
      setGenerationProgress({ current: 0, total: prompts.length });

      // Import LLM API
      const { invoke } = await import("@tauri-apps/api/core");
      const teacherConfig = buildConfig({
        model: teacherModelSelection || model,
        maxTokens: 512,
        temperature: 0.7,
      });

      // Log the start of generation
      await invoke("add_log_message", {
        level: "INFO",
        source: "Distillation",
        message: `Starting training data generation: ${prompts.length} prompts, teacher: ${provider}/${teacherModelSelection || model}, student: ${selectedStudentPath}`,
      }).catch((e) => console.error("Failed to log:", e));

      for (let i = 0; i < prompts.length; i++) {
        const prompt = prompts[i];
        setGenerationProgress({ current: i + 1, total: prompts.length });

        // Log processing each prompt
        await invoke("add_log_message", {
          level: "DEBUG",
          source: "Distillation",
          message: `Processing prompt ${i + 1}/${prompts.length}: ${prompt.substring(0, 50)}...`,
        }).catch((e) => console.error("Failed to log:", e));

        // Call teacher model (from LLM settings)
        const teacherResponse = (await invoke("llm_chat", {
          config: teacherConfig,
          messages: [{ role: "user", content: prompt }],
        })) as string;

        await invoke("add_log_message", {
          level: "INFO",
          source: "Distillation",
          message: `Teacher response received for prompt ${i + 1}: ${teacherResponse.substring(0, 50)}...`,
        }).catch((e) => console.error("Failed to log:", e));

        // Call student model (local model)
        // For now, we'll simulate student output or use a placeholder
        // In production, you'd call the actual student model
        // Extract just the model name from the path (e.g., "openai/gpt-oss-20b" -> "gpt-oss-20b")
        const studentModelName =
          selectedStudentPath.split(/[\\/]/).pop() || selectedStudentPath;
        const studentResponse = (await invoke("llm_chat", {
          config: {
            provider: "local",
            model: studentModelName,
            baseUrl: LOCAL_LLM_BASE_URL,
            maxTokens: 512,
            temperature: 0.7,
          },
          messages: [{ role: "user", content: prompt }],
        }).catch((studentErr) => {
          const studentErrMsg =
            studentErr instanceof Error
              ? studentErr.message
              : typeof studentErr === "string"
                ? studentErr
                : JSON.stringify(studentErr);

          // Log student model error
          invoke("add_log_message", {
            level: "WARN",
            source: "Distillation",
            message: `Student model call failed for prompt ${i + 1}: ${studentErrMsg}`,
          }).catch((e) => console.error("Failed to log:", e));

          // If student model fails, use a placeholder
          return "[Student model output - not yet trained]";
        })) as string;

        // Save as correction
        const correctionInput = {
          correctionId: uuid(),
          prompt: prompt,
          studentOutput: studentResponse,
          correctedOutput: teacherResponse,
          accuracyRating: 3, // Default rating
          relevanceRating: 4,
          safetyRating: 5,
          domainNotes: `Auto-generated from teacher model: ${PROVIDER_LABELS[provider]}/${teacherModelSelection || model}`,
        };

        await invoke("add_log_message", {
          level: "DEBUG",
          source: "Distillation",
          message: `Saving correction ${i + 1}/${prompts.length}: ${correctionInput.correctionId}`,
        }).catch((e) => console.error("Failed to log:", e));

        await ModelDistillationAPI.saveCorrection(correctionInput, [
          "auto-generated",
          "distillation",
        ]);

        await invoke("add_log_message", {
          level: "INFO",
          source: "Distillation",
          message: `Correction saved successfully: ${correctionInput.correctionId}`,
        }).catch((e) => console.error("Failed to log:", e));
      }

      // Refresh corrections list
      setGeneratingData(false);
      setShowDataGenerator(false);
      setPromptsInput("");
      refetchCorrections();

      await invoke("add_log_message", {
        level: "INFO",
        source: "Distillation",
        message: `Training data generation completed: ${prompts.length} corrections created`,
      }).catch((e) => console.error("Failed to log:", e));
    } catch (err) {
      setGeneratingData(false);

      // Log the error
      await invoke("add_log_message", {
        level: "ERROR",
        source: "Distillation",
        message: `Training data generation failed: ${err instanceof Error ? err.message : JSON.stringify(err)}`,
      }).catch((e) => console.error("Failed to log:", e));

      // Handle different error types
      let errorMessage = "An error occurred";
      if (err instanceof Error) {
        errorMessage = err.message;
      } else if (typeof err === "string") {
        errorMessage = err;
      } else if (err && typeof err === "object") {
        // Try to extract message from error object
        errorMessage = (err as any).message || JSON.stringify(err);
      }
      setError(errorMessage);
    }
  };

  const handleCreateTrainingRun = async () => {
    if (!selectedStudentPath) {
      setError("Please select a student model");
      return;
    }

    if (selectedCorrections.length === 0 && !selectedDatasetId) {
      setError("Please select corrections or a dataset for training");
      return;
    }

    if (!isProviderConfigured) {
      setError("Please configure an LLM provider in General Settings first");
      return;
    }

    try {
      setSaveStatus("saving");
      setError(null);

      // Prepare correction IDs with split assignments (90% train, 5% val, 5% test)
      const correctionIds = buildCorrectionSplits(selectedCorrections);

      // Prepare dataset IDs
      const datasetIds: [string, string, number][] | undefined =
        selectedDatasetId ? [[selectedDatasetId, "train", 1.0]] : undefined;

      const hyperparams = buildHyperparams();

      // Ensure student model is registered in the database
      let studentModelId = "";
      const existingModel = registeredModels.find(
        (m) => m.defaultArtifactPath === selectedStudentPath,
      );

      console.log("[SetupTab] Existing model check:", {
        selectedStudentPath,
        existingModel: existingModel ? existingModel.modelId : "none",
        allRegisteredModels: registeredModels.map((m) => ({
          id: m.modelId,
          path: m.defaultArtifactPath,
        })),
      });

      if (existingModel) {
        studentModelId = existingModel.modelId;
        console.log("[SetupTab] Using existing model:", studentModelId);
      } else {
        // Check if this is a resource model (don't copy, just register)
        const selectedModel = baseModels.find(
          (m) => m.path === selectedStudentPath,
        );
        const isResourceModel = selectedModel?.source === "resource";

        const modelName =
          selectedStudentPath.split(/[\\/]/).pop() || "student-model";
        const sanitized = modelName
          .replace(/[^a-zA-Z0-9-]/g, "-")
          .toLowerCase();

        console.log("[SetupTab] Registering new model:", {
          sourcePath: selectedStudentPath,
          displayName: modelName,
          modelId: sanitized,
          isResourceModel,
        });

        if (isResourceModel) {
          // For resource models, just register without copying
          // Use the resource path directly
          console.log(
            "[SetupTab] Resource model detected, registering without copy",
          );
          const model = await ModelDistillationAPI.registerModel({
            modelId: sanitized,
            displayName: modelName,
            provider: "local",
            modelFamily: "llama",
            defaultArtifactPath: selectedStudentPath,
          });
          studentModelId = model.modelId;
        } else {
          // For user-imported models, copy to app_data
          console.log("[SetupTab] User model detected, importing with copy");
          const registered = await ModelDistillationAPI.importBaseModel({
            sourcePath: selectedStudentPath,
            displayName: modelName,
            modelId: sanitized,
          });
          studentModelId = registered.model.modelId;
        }

        console.log(
          "[SetupTab] Model registered, studentModelId:",
          studentModelId,
        );
        await refetchModels();
      }

      console.log("[SetupTab] Final studentModelId:", studentModelId);

      const input: TrainingRunInput = {
        runId: uuid(),
        studentModelId: studentModelId,
        method: trainingMethod,
        hyperparamsJson: JSON.stringify(hyperparams),
        seed: Math.floor(Math.random() * 100000),
      };

      console.log("[SetupTab] Creating training run with input:", input);
      console.log("[SetupTab] Correction IDs:", correctionIds);
      console.log("[SetupTab] Dataset IDs:", datasetIds);

      const run = await ModelDistillationAPI.createTrainingRun(
        input,
        correctionIds,
        datasetIds,
      );

      console.log("[SetupTab] Training run created successfully:", run);

      // Update store with the new run
      store.setActiveRunId(run.runId);

      setSaveStatus("success");
      refetchRuns();

      // Reset after short delay
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      // Handle different error types (Tauri errors are objects)
      let errorMessage = "An error occurred";
      if (err instanceof Error) {
        errorMessage = err.message;
      } else if (typeof err === "string") {
        errorMessage = err;
      } else if (err && typeof err === "object") {
        // Tauri error object - try to extract message
        console.error("[SetupTab] Error object:", err);
        console.error("[SetupTab] Error keys:", Object.keys(err));
        console.error("[SetupTab] Error stringified:", JSON.stringify(err));
        errorMessage =
          (err as any).message || (err as any).error || JSON.stringify(err);
      }
      console.error("[SetupTab] Error creating training run:", errorMessage);
      setError(errorMessage);
    }
  };

  const selectAllCorrections = () => {
    setSelectedCorrections(corrections.map((c) => c.correctionId));
  };

  const clearCorrections = () => {
    setSelectedCorrections([]);
  };

  const datasetOptions = [
    { value: "", label: "No dataset (use corrections only)" },
    ...datasets.map((d) => ({
      value: d.datasetId,
      label: `${d.name} (${d.type})`,
    })),
  ];

  // Build options for select dropdowns

  // Get selected student model info
  const selectedStudentModel = baseModels.find(
    (m) => m.path === selectedStudentPath,
  );

  return (
    <div className="bg-app-bg text-app-text min-h-full overflow-y-auto p-8">
      <div className="max-w-7xl mx-auto space-y-8">
        {/* Header Section */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="flex justify-between items-start border-b border-app-border/50 pb-6">
          <div>
            <h1 className="text-3xl font-bold mb-2 text-app-text tracking-tight">
              Model Configuration
            </h1>
            <p className="text-app-subtext">
              Define your teacher-student architecture
            </p>
          </div>
          <div className="px-3 py-1.5 bg-app-card border border-app-border rounded-md text-xs font-mono text-app-subtext shadow-sm">
            V1.2.0-BETA
          </div>
        </motion.div>

        {/* Error Display */}
        <AnimatePresence>
          {error && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              className="mb-4 overflow-hidden">
              <InfoBox type="error" icon={AlertCircle}>
                <div className="flex justify-between items-start w-full">
                  <div>
                    <div className="font-semibold mb-1">
                      Configuration Error
                    </div>
                    <div>{error}</div>
                  </div>
                  <button
                    onClick={() => setError(null)}
                    className="p-1 hover:bg-red-500/20 rounded transition-colors">
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </InfoBox>
            </motion.div>
          )}
        </AnimatePresence>

        <div className="grid grid-cols-12 gap-8">
          {/* LEFT COLUMN (Teacher & Student Models) */}
          <div className="col-span-12 lg:col-span-7 space-y-8">
            {/* Teacher Model Selection */}
            <Card
              title="Teacher Model Selection"
              icon={Cloud}
              iconColor="text-purple-400">
              <div className="space-y-4">
                {!isProviderConfigured ? (
                  <InfoBox type="warning" icon={AlertCircle}>
                    <div className="font-semibold mb-1">
                      No LLM Provider Configured
                    </div>
                    <p className="mb-3 text-app-subtext">
                      Configure an LLM provider in General Settings to use as
                      the teacher model.
                    </p>
                    <Button
                      variant="default"
                      size="sm"
                      icon={ExternalLink}
                      onClick={() => {
                        window.location.hash = "#/settings";
                      }}>
                      Go to Settings
                    </Button>
                  </InfoBox>
                ) : (
                  <>
                    <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-3">
                      <div className="flex items-center gap-2 mb-1">
                        <CheckCircle className="w-4 h-4 text-green-400" />
                        <span className="text-sm font-medium text-green-400">
                          Provider Configured: {PROVIDER_LABELS[provider]}
                        </span>
                      </div>
                      <div className="text-xs text-green-300/70 ml-6">
                        {isLocalProvider
                          ? "Local Model Provider"
                          : "Cloud API Provider"}
                      </div>
                    </div>

                    <Select
                      label="Select Teacher Model"
                      value={teacherModelSelection}
                      onChange={setTeacherModelSelection}
                      options={[
                        { value: "", label: "Default (defined in settings)" },
                        ...getTeacherModels().map((m) => ({
                          value: m,
                          label: m,
                        })),
                      ]}
                    />
                  </>
                )}

                <div className="grid grid-cols-2 gap-4 mt-4">
                  <div className="bg-background rounded-lg p-3 border border-app-border">
                    <div className="text-xs text-app-subtext mb-1">
                      Provider
                    </div>
                    <div className="text-sm font-mono text-app-text">
                      {PROVIDER_LABELS[provider]}
                    </div>
                  </div>
                  <div className="bg-background rounded-lg p-3 border border-app-border">
                    <div className="text-xs text-app-subtext mb-1">Status</div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${isProviderConfigured ? "bg-green-500" : "bg-red-500"}`}></span>
                      <span className="text-sm text-app-text">
                        {isProviderConfigured ? "Connected" : "Disconnected"}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </Card>

            {/* Student Architecture (Base Models) */}
            <Card
              title="Student Architecture"
              icon={Cpu}
              iconColor="text-blue-400">
              <div className="flex justify-between items-center mb-4">
                <div>
                  <div className="text-sm text-app-subtext">
                    Select a base model to distill knowledge into
                  </div>
                  {baseModels.some((m) => m.source === "resource") && (
                    <div className="text-xs text-green-400/80 mt-1">
                      Built-in models from resources folder available
                    </div>
                  )}
                </div>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={async () => {
                      try {
                        const dir = await open({
                          directory: true,
                          multiple: false,
                        });
                        if (dir) {
                          setBaseModelsLoading(true);
                          await ModelDistillationAPI.importBaseModel({
                            sourcePath: dir,
                          });
                          await loadBaseModels();
                        }
                      } catch (err) {
                        console.error("Failed to import model:", err);
                        setError(
                          err instanceof Error ? err.message : String(err),
                        );
                      }
                    }}>
                    Import External Model
                  </Button>
                </div>
              </div>

              {baseModelsLoading ? (
                <div className="flex justify-center py-8">
                  <Loader2 className="w-8 h-8 animate-spin text-blue-400" />
                </div>
              ) : baseModels.length === 0 ? (
                <div className="text-center py-8 rounded-lg border-2 border-dashed border-app-border hover:border-app-subtext/50 transition-colors">
                  <p className="text-sm text-app-subtext mb-2">
                    No base models found
                  </p>
                  <p className="text-xs text-app-subtext/70 mb-4">
                    Models from resources/models/base should appear
                    automatically
                  </p>
                  {/* <Button
                    size="sm"
                    variant="ghost"
                    onClick={async () => {
                      try {
                        setBaseModelsLoading(true);
                        await ModelDistillationAPI.downloadDefaultModel();
                        await loadBaseModels();
                      } catch (err) {
                        console.error(
                          "Failed to download default model:",
                          err,
                        );
                        setError(
                          err instanceof Error ? err.message : String(err),
                        );
                      } finally {
                        setBaseModelsLoading(false);
                      }
                    }}
                    loading={baseModelsLoading}>
                    Download Default Model
                  </Button> */}
                </div>
              ) : (
                <div className="space-y-3">
                  {baseModels.map((entry) => {
                    const isSelected = selectedStudentPath === entry.path;
                    return (
                      <div
                        key={entry.path}
                        onClick={() => setSelectedStudentPath(entry.path)}
                        className={`relative p-4 rounded-lg border cursor-pointer transition-all duration-200 flex items-center justify-between ${
                          isSelected
                            ? "bg-blue-500/10 border-blue-500/50 shadow-md shadow-blue-500/10"
                            : "bg-background border-app-border hover:border-app-subtext/50"
                        }`}>
                        <div>
                          <div className="text-sm font-semibold text-app-text mb-1">
                            {entry.name}
                          </div>
                          <div className="flex items-center gap-2">
                            <span className="text-xs px-2 py-0.5 rounded bg-app-bg border border-app-border text-app-subtext font-mono">
                              {entry.format.toUpperCase()}
                            </span>
                            <span className="text-xs text-app-subtext">
                              {entry.source}
                            </span>
                          </div>
                        </div>
                        {isSelected && (
                          <CheckCircle className="w-5 h-5 text-blue-400" />
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>
          </div>

          {/* RIGHT COLUMN (Data & Settings) */}
          <div className="col-span-12 lg:col-span-5 space-y-8">
            {/* Data Ingestion */}
            <Card
              title="Data Ingestion"
              icon={Database}
              iconColor="text-green-400">
              {/* Training Data Gen Modal Inline/Conditional */}
              <AnimatePresence>
                {showDataGenerator && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    className="mb-6 overflow-hidden border-b border-app-border pb-6">
                    <div className="bg-app-bg rounded-lg border border-app-border p-4">
                      <div className="flex justify-between items-center mb-4">
                        <h4 className="text-sm font-semibold text-app-text">
                          Training Data Generator
                        </h4>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setShowDataGenerator(false)}>
                          Close
                        </Button>
                      </div>

                      <div className="space-y-4">
                        <InfoBox type="info">
                          Enter prompts (one per line). System runs them through
                          Teacher & Student models to generate distillation
                          data.
                        </InfoBox>

                        <textarea
                          value={promptsInput}
                          onChange={(e) => setPromptsInput(e.target.value)}
                          placeholder={`Translate: Hello world\nExplain Quantum Physics\n...`}
                          rows={5}
                          className="w-full bg-app-card border border-app-border rounded-lg p-3 text-sm font-mono focus:border-blue-500 outline-none"
                        />

                        <div className="flex items-center justify-between">
                          <span className="text-xs text-app-subtext">
                            {
                              promptsInput.split("\n").filter((p) => p.trim())
                                .length
                            }{" "}
                            prompts
                          </span>
                          <Button
                            variant="primary"
                            size="sm"
                            onClick={handleGenerateTrainingData}
                            loading={generatingData}
                            disabled={generatingData || !promptsInput.trim()}>
                            Generate
                          </Button>
                        </div>

                        {generatingData && (
                          <ProgressBar
                            value={generationProgress.current}
                            max={generationProgress.total}
                            label="Generating..."
                          />
                        )}
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>

              <div className="space-y-6">
                <Select
                  label="Dataset (Optional)"
                  value={selectedDatasetId}
                  onChange={setSelectedDatasetId}
                  options={datasetOptions}
                />

                {/* Corrections List */}
                <div>
                  <div className="flex items-center justify-between mb-3">
                    <label className="text-xs font-semibold text-app-text">
                      Selected Corrections ({selectedCorrections.length})
                    </label>
                    <div className="flex gap-2">
                      <button
                        onClick={selectAllCorrections}
                        className="text-xs text-blue-400 hover:text-blue-300">
                        Select All
                      </button>
                      <button
                        onClick={clearCorrections}
                        className="text-xs text-app-subtext hover:text-app-text">
                        Clear
                      </button>
                      <Button
                        variant="default"
                        size="sm"
                        icon={Plus}
                        onClick={() => setShowDataGenerator(true)}>
                        Generate Data
                      </Button>
                    </div>
                  </div>

                  {correctionsLoading ? (
                    <div className="flex justify-center py-4">
                      <Loader2 className="animate-spin w-5 h-5 text-app-subtext" />
                    </div>
                  ) : corrections.length === 0 ? (
                    <div className="text-center py-6 bg-app-bg rounded-lg border border-dashed border-app-border">
                      <p className="text-sm text-app-subtext mb-2">
                        No training data found
                      </p>
                    </div>
                  ) : (
                    <div className="max-h-60 overflow-y-auto space-y-2 pr-1">
                      {corrections.slice(0, 50).map((c) => (
                        <div
                          key={c.correctionId}
                          className="flex items-start gap-3 p-2 rounded bg-app-bg border border-app-border hover:border-app-subtext/30">
                          <input
                            type="checkbox"
                            checked={selectedCorrections.includes(
                              c.correctionId,
                            )}
                            onChange={(e) => {
                              if (e.target.checked)
                                setSelectedCorrections([
                                  ...selectedCorrections,
                                  c.correctionId,
                                ]);
                              else
                                setSelectedCorrections(
                                  selectedCorrections.filter(
                                    (id) => id !== c.correctionId,
                                  ),
                                );
                            }}
                            className="mt-1"
                          />
                          <div className="min-w-0 flex-1">
                            <p className="text-xs text-app-text truncate font-mono">
                              {c.prompt}
                            </p>
                            <div className="flex justify-between items-center mt-1">
                              <span className="text-[10px] text-app-subtext">
                                Rating: {c.accuracyRating}/5
                              </span>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </Card>

            {/* Training Configuration */}
            <Card
              title="Training Configuration"
              icon={Cog}
              iconColor="text-orange-400">
              <div className="space-y-4">
                <Select
                  label="Training Method"
                  // @ts-ignore - store.trainingMethod mismatch handling
                  value={trainingMethod}
                  onChange={(val) => setTrainingMethod(val as any)}
                  options={[
                    {
                      value: "fine_tune",
                      label: "Fine-Tune (Supervised Only)",
                    },
                    {
                      value: "knowledge_distillation",
                      label: "Knowledge Distillation (Teacher Only)",
                    },
                    {
                      value: "hybrid",
                      label: "Hybrid (Teacher + Data) - Recommended",
                    },
                  ]}
                />

                <div className="grid grid-cols-2 gap-4">
                  <Input
                    label="Epochs"
                    type="number"
                    value={store.epochs}
                    onChange={(value) =>
                      store.setEpochs(
                        typeof value === "number"
                          ? value
                          : parseInt(String(value)) || 1,
                      )
                    }
                    min={1}
                  />
                  <Input
                    label="Batch Size"
                    type="number"
                    value={store.batchSize}
                    onChange={(value) =>
                      store.setBatchSize(
                        typeof value === "number"
                          ? value
                          : parseInt(String(value)) || 1,
                      )
                    }
                    min={1}
                  />
                  <Input
                    label="Learning Rate"
                    type="number"
                    step={0.0001}
                    value={store.learningRate}
                    onChange={(value) =>
                      store.setLearningRate(
                        typeof value === "number"
                          ? value
                          : parseFloat(String(value)) || 0.0001,
                      )
                    }
                  />
                  <Input
                    label="Temperature"
                    type="number"
                    step={0.1}
                    value={store.temperature}
                    onChange={(value) =>
                      store.setTemperature(
                        typeof value === "number"
                          ? value
                          : parseFloat(String(value)) || 1.0,
                      )
                    }
                  />
                </div>
              </div>
            </Card>
          </div>
        </div>

        {/* Action Bar */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="mt-6">
          <div className="bg-app-card rounded-xl border border-app-border p-6 shadow-xl">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-app-text">
                Ready to Train?
              </h3>
              <div className="flex gap-2">
                <MetricCard
                  icon={Cpu}
                  label="Student"
                  value={selectedStudentModel?.name || "Not Selected"}
                  delay={0}
                />
                <MetricCard
                  icon={Cloud}
                  label="Teacher"
                  value={
                    teacherRequired
                      ? isProviderConfigured
                        ? PROVIDER_LABELS[provider]
                        : "Not Set"
                      : "Skipped"
                  }
                  delay={0.1}
                />
                <MetricCard
                  icon={Database}
                  label="Data"
                  value={
                    selectedCorrections.length > 0
                      ? `${selectedCorrections.length} Samples`
                      : selectedDatasetId
                        ? "Dataset Configured"
                        : "No Data"
                  }
                  delay={0.2}
                />
              </div>
            </div>

            <div className="grid grid-cols-3 gap-6 mb-6">
              <div className="col-span-2">
                <div className="bg-background rounded-lg p-4 border border-app-border h-full flex flex-col justify-center">
                  <div className="flex items-center gap-3">
                    <Activity className="w-5 h-5 text-blue-400" />
                    <div>
                      <div className="text-sm font-semibold text-app-text">
                        System Status Check
                      </div>
                      <div className="text-xs text-app-subtext">
                        {saveStatus === "success"
                          ? "Training Run Created"
                          : saveStatus === "saving"
                            ? "Creating Training Run..."
                            : saveStatus === "error"
                              ? "Creation Failed"
                              : !selectedStudentPath ||
                                  !teacherReady ||
                                  (selectedCorrections.length === 0 &&
                                    !selectedDatasetId)
                                ? "Missing Requirements"
                                : "Ready to Create Run"}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div>
                <div
                  className={`rounded-lg p-4 border h-full flex items-center justify-center text-center ${
                    !selectedStudentPath ||
                    !teacherReady ||
                    (selectedCorrections.length === 0 && !selectedDatasetId)
                      ? "bg-yellow-500/10 border-yellow-500/30 text-yellow-400"
                      : "bg-green-500/10 border-green-500/30 text-green-400"
                  }`}>
                  <div className="text-sm font-medium">
                    {saveStatus === "success"
                      ? "Success"
                      : saveStatus === "error"
                        ? "Error"
                        : !selectedStudentPath
                          ? "Select a student model from the Base Models section"
                          : !teacherReady
                            ? "Configure an LLM provider for KD/Hybrid training"
                            : selectedCorrections.length === 0 &&
                                !selectedDatasetId
                              ? "Select corrections or a dataset for training data"
                              : `Ready: ${selectedStudentModel?.name || "Model"} + ${teacherRequired ? PROVIDER_LABELS[provider] : "No Teacher"} + ${selectedCorrections.length} corrections`}
                  </div>
                </div>
              </div>
            </div>

            <Button
              variant="primary"
              size="lg"
              onClick={handleCreateTrainingRun}
              disabled={
                !selectedStudentPath ||
                (selectedCorrections.length === 0 && !selectedDatasetId) ||
                !teacherReady
              }
              loading={saveStatus === "saving"}
              icon={Zap}
              className="w-full shadow-blue-500/20 shadow-lg">
              {saveStatus === "success" ? "Created!" : "Create Training Run"}
            </Button>
          </div>
        </motion.div>
      </div>
    </div>
  );
}
