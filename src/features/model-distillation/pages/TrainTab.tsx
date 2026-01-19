import { useState, useEffect, useRef, useCallback } from "react";
import { motion } from "framer-motion";
import {
  Play,
  Pause,
  Square,
  Activity,
  Clock,
  CheckCircle,
  Loader2,
  Zap,
  Terminal,
  AlertCircle,
} from "lucide-react";
import {
  Card,
  Button,
  MetricCard,
  ProgressBar,
  InfoBox,
} from "../components/UI";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ModelDistillationAPI, type DistillTrainConfig } from "../api";
import { useModelDistillationStore } from "../../../store/modelDistillation";

interface TrainingLog {
  timestamp: string;
  epoch: number;
  step: number;
  loss: number;
  lr: number;
  stage: string;
  ceLoss?: number;
  kdLoss?: number;
  alpha?: number;
  temperature?: number;
  cpuPercent?: number;
  ramBytes?: number;
  gpuPercent?: number;
  isStderr?: boolean;
  stderrMessage?: string;
}

interface DistillPythonMessage {
  kind: string;
  payload: Record<string, unknown>;
}

type TrainingStatus = "idle" | "queued" | "running" | "paused" | "completed" | "failed" | "cancelled";

export default function TrainTab() {
  const store = useModelDistillationStore();
  const [status, setStatus] = useState<TrainingStatus>("idle");
  const [logs, setLogs] = useState<TrainingLog[]>([]);
  const [currentEpoch, setCurrentEpoch] = useState(0);
  const [currentStep, setCurrentStep] = useState(0);
  const [totalSteps, setTotalSteps] = useState(100);
  const [currentLoss, setCurrentLoss] = useState(0);
  const [learningRate, setLearningRate] = useState(0);
  const [cpuUsage, setCpuUsage] = useState(0);
  const [ramUsage, setRamUsage] = useState(0);
  const [gpuUsage, setGpuUsage] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [startTime, setStartTime] = useState<number | null>(null);

  const logsEndRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Auto-scroll logs
  useEffect(() => {
    if (logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

  // Listen to training events from Tauri
  useEffect(() => {
    let mounted = true;

    const setupListener = async () => {
      try {
        unlistenRef.current = await listen<DistillPythonMessage>(
          "distill-train-stream",
          (event) => {
            if (!mounted) return;

            const msg = event.payload;
            handleTrainEvent(msg);
          }
        );
      } catch (err) {
        console.error("Failed to set up event listener:", err);
      }
    };

    setupListener();

    return () => {
      mounted = false;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  const handleTrainEvent = useCallback((msg: DistillPythonMessage) => {
    const { kind, payload } = msg;

    // Log all events for debugging
    console.log("[TrainTab] Received event:", kind, payload);

    switch (kind) {
      case "status": {
        const level = payload.level as string;
        const message = payload.message as string;

        console.log(`[Trainer Status] ${level}: ${message}`);

        // Add status messages to logs for visibility
        const statusLog: TrainingLog = {
          timestamp: new Date().toISOString().split("T")[1].split(".")[0],
          epoch: 0,
          step: currentStep,
          loss: 0,
          lr: 0,
          stage: "Info",
          isStderr: level === "error" || level === "warn",
          stderrMessage: level === "error" || level === "warn" ? message : undefined,
        };
        setLogs((prev) => [...prev, statusLog].slice(-200));

        if (level === "info") {
          if (message === "trainer started") {
            setStatus("running");
            setStartTime(Date.now());
          } else if (message === "trainer completed") {
            setStatus("completed");
          } else if (message === "trainer cancelled") {
            setStatus("cancelled");
          } else if (message === "trainer exited") {
            const cancelled = payload.cancelled as boolean;
            if (cancelled) {
              setStatus("cancelled");
            }
          }
        } else if (level === "error") {
          setStatus("failed");
          setError(message);
        } else if (level === "warn") {
          // Log warnings but don't change status
          console.warn("[Trainer]", message);
        }
        break;
      }

      case "progress": {
        const epoch = (payload.epoch as number) ?? 0;
        const step = (payload.step as number) ?? 0;
        const total = (payload.total_steps as number) ?? totalSteps;
        const loss = payload.loss as number;
        const lr = payload.lr as number;
        const ceLoss = payload.ce_loss as number | undefined;
        const kdLoss = payload.kd_loss as number | undefined;
        const alpha = payload.alpha as number | undefined;
        const temperature = payload.temperature as number | undefined;
        const mode = (payload.mode as string) ?? "fine_tune";

        setCurrentEpoch(epoch);
        setCurrentStep(step);
        setTotalSteps(total);
        if (loss != null) setCurrentLoss(loss);
        if (lr != null) setLearningRate(lr);

        // Resource stats
        const resources = payload.resources as Record<string, unknown> | undefined;
        if (resources) {
          const cpu = resources.cpu_percent as number | undefined;
          const ramBytes = resources.ram_rss_bytes as number | undefined;
          const gpu = resources.gpu_util_percent as number | undefined;

          if (cpu != null) setCpuUsage(cpu);
          if (ramBytes != null) setRamUsage(ramBytes / (1024 * 1024 * 1024)); // Convert to GB
          if (gpu != null) setGpuUsage(gpu);
        }

        // Determine stage
        let stage = "Training";
        if (mode === "knowledge_distillation") {
          stage = "Distillation";
        } else if (mode === "hybrid") {
          stage = alpha && alpha > 0.5 ? "Distillation" : "Fine-tuning";
        } else if (epoch < 1) {
          stage = "Warmup";
        }

        const newLog: TrainingLog = {
          timestamp: new Date().toISOString().split("T")[1].split(".")[0],
          epoch,
          step,
          loss: loss ?? 0,
          lr: lr ?? 0,
          stage,
          ceLoss,
          kdLoss,
          alpha,
          temperature,
          cpuPercent: resources?.cpu_percent as number | undefined,
          ramBytes: resources?.ram_rss_bytes as number | undefined,
          gpuPercent: resources?.gpu_util_percent as number | undefined,
        };

        setLogs((prev) => [...prev, newLog].slice(-200)); // Keep last 200 logs
        break;
      }

      case "env": {
        // Environment info (python version, GPU info, etc.)
        console.log("[Trainer] Environment:", payload);
        break;
      }

      case "model": {
        // Model info loaded
        console.log("[Trainer] Model:", payload);
        break;
      }

      case "dataset": {
        // Dataset info
        const counts = payload.counts as Record<string, number> | undefined;
        console.log("[Trainer] Dataset:", payload);
        if (counts?.train) {
          // Could update UI with dataset info
        }
        break;
      }

      case "artifact": {
        // Artifact produced
        console.log("[Trainer] Artifact:", payload);
        break;
      }

      case "metric": {
        // Evaluation metric
        console.log("[Trainer] Metric:", payload);
        break;
      }

      case "stderr": {
        // Stderr output from Python - display in logs
        const message = payload.message as string;
        console.error("[Trainer stderr]:", message);

        // Also update error state for critical errors
        if (message.includes("GGUF models are not supported") ||
            message.includes("Error") ||
            message.includes("error")) {
          setError(message);
        }

        // Add to logs as stderr entry
        const stderrLog: TrainingLog = {
          timestamp: new Date().toISOString().split("T")[1].split(".")[0],
          epoch: 0,
          step: currentStep,
          loss: 0,
          lr: 0,
          stage: "Error",
          isStderr: true,
          stderrMessage: message,
        };
        setLogs((prev) => [...prev, stderrLog].slice(-200));
        break;
      }
    }
  }, [totalSteps, currentStep]);

  const handleStart = async () => {
    if (!store.activeRunId) {
      setError("No training run selected. Please create a training run first.");
      return;
    }

    try {
      setError(null);
      setStatus("queued");
      setLogs([]);
      setCurrentStep(0);
      setCurrentEpoch(0);
      setCurrentLoss(0);

      const config: DistillTrainConfig = {
        runId: store.activeRunId,
        runDir: "", // Will be resolved by backend
        mode: store.trainingMethod,
        seed: Math.floor(Math.random() * 100000),
        steps: store.epochs * 100, // Rough estimate
        emitEvery: 1,
        hyperparams: {
          epochs: store.epochs,
          batch_size: store.batchSize,
          learning_rate: store.learningRate,
          temperature: store.temperature,
          alpha: store.alpha,
        },
      };

      await ModelDistillationAPI.startPythonTraining(config);
    } catch (err) {
      setStatus("failed");
      // Handle Tauri invoke errors which come as objects
      let errorMessage: string;
      if (err instanceof Error) {
        errorMessage = err.message;
      } else if (typeof err === "object" && err !== null) {
        errorMessage = JSON.stringify(err);
      } else {
        errorMessage = String(err);
      }
      console.error("Start training error:", err);
      setError(errorMessage);
    }
  };

  const handleCancel = async () => {
    if (!store.activeRunId) return;

    try {
      await ModelDistillationAPI.cancelPythonTraining(store.activeRunId);
      setStatus("cancelled");
    } catch (err) {
      // Handle Tauri invoke errors which come as objects
      let errorMessage: string;
      if (err instanceof Error) {
        errorMessage = err.message;
      } else if (typeof err === "object" && err !== null) {
        errorMessage = JSON.stringify(err);
      } else {
        errorMessage = String(err);
      }
      console.error("Cancel error:", err);
      setError(errorMessage);
    }
  };

  const handleReset = () => {
    setStatus("idle");
    setLogs([]);
    setCurrentStep(0);
    setCurrentEpoch(0);
    setCurrentLoss(0);
    setLearningRate(0);
    setCpuUsage(0);
    setRamUsage(0);
    setGpuUsage(null);
    setError(null);
    setStartTime(null);
  };

  // Calculate progress and ETA
  const progress = totalSteps > 0 ? (currentStep / totalSteps) * 100 : 0;
  const eta = (() => {
    if (!startTime || currentStep === 0 || status !== "running") return "—";
    const elapsed = Date.now() - startTime;
    const msPerStep = elapsed / currentStep;
    const remaining = (totalSteps - currentStep) * msPerStep;
    const mins = Math.floor(remaining / 60000);
    const secs = Math.floor((remaining % 60000) / 1000);
    if (mins > 60) {
      return `~${Math.floor(mins / 60)}h ${mins % 60}m`;
    }
    return `~${mins}m ${secs}s`;
  })();

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-6 flex flex-col gap-6">
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="mb-4">
          <h1 className="text-3xl font-bold mb-2 text-app-text">
            Training Monitor
          </h1>
          <p className="text-app-subtext">
            Start, monitor, and manage model distillation training
          </p>
          {store.activeRunId && (
            <p className="text-xs text-app-subtext mt-1">
              Active Run: <code className="bg-background px-1 rounded">{store.activeRunId}</code>
            </p>
          )}
        </motion.div>

        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mb-4">
            <InfoBox type="error" icon={AlertCircle}>
              <div className="font-semibold mb-1">Training Error</div>
              <div>{error}</div>
            </InfoBox>
          </motion.div>
        )}

        <div className="grid grid-cols-4 gap-4">
          <MetricCard
            icon={Activity}
            label="Status"
            value={status === "running" ? "Running" : status === "queued" ? "Starting" : status === "completed" ? "Done" : status === "failed" ? "Failed" : status === "cancelled" ? "Cancelled" : "Idle"}
            delay={0}
          />
          <MetricCard
            icon={Zap}
            label="Progress"
            value={`${currentStep}`}
            unit={`/ ${totalSteps}`}
            delay={0.1}
          />
          <MetricCard
            icon={Activity}
            label="Loss"
            value={currentLoss > 0 ? currentLoss.toFixed(4) : "—"}
            delay={0.2}
          />
          <MetricCard
            icon={Clock}
            label="ETA"
            value={eta}
            delay={0.3}
          />
        </div>

        <Card title="Training Control" icon={Play} iconColor="text-blue-400">
          <div className="space-y-6">
            <div className="flex items-center gap-4">
              <div className="flex-1">
                <ProgressBar
                  value={currentStep}
                  max={totalSteps}
                  label={`Step ${currentStep} / ${totalSteps}${currentEpoch > 0 ? ` (Epoch ${currentEpoch})` : ""}`}
                  color="bg-blue-500"
                  animated={status === "running"}
                />
              </div>

              <div className="flex gap-2">
                {status === "idle" || status === "completed" || status === "failed" || status === "cancelled" ? (
                  <Button
                    variant="primary"
                    size="md"
                    onClick={handleStart}
                    icon={Play}
                    disabled={!store.activeRunId}>
                    Start
                  </Button>
                ) : status === "running" || status === "queued" ? (
                  <Button variant="danger" size="md" onClick={handleCancel} icon={Square}>
                    Cancel
                  </Button>
                ) : null}

                {(status === "completed" || status === "failed" || status === "cancelled") && (
                  <Button variant="ghost" size="md" onClick={handleReset}>
                    Reset
                  </Button>
                )}
              </div>
            </div>

            <div className="grid grid-cols-4 gap-4">
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="text-xs text-app-subtext mb-2">Learning Rate</div>
                <div className="text-2xl font-bold text-app-text">
                  {learningRate > 0 ? learningRate.toExponential(2) : "—"}
                </div>
              </div>
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="text-xs text-app-subtext mb-2">CPU Usage</div>
                <div className="text-2xl font-bold text-app-text">
                  {cpuUsage > 0 ? `${cpuUsage.toFixed(0)}%` : "—"}
                </div>
              </div>
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="text-xs text-app-subtext mb-2">RAM Usage</div>
                <div className="text-2xl font-bold text-app-text">
                  {ramUsage > 0 ? `${ramUsage.toFixed(1)} GB` : "—"}
                </div>
              </div>
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="text-xs text-app-subtext mb-2">GPU Usage</div>
                <div className="text-2xl font-bold text-app-text">
                  {gpuUsage != null ? `${gpuUsage.toFixed(0)}%` : "—"}
                </div>
              </div>
            </div>
          </div>
        </Card>

        <div className="grid grid-cols-2 gap-6">
          <Card title="Loss Curve" icon={Activity} iconColor="text-purple-400">
            <div className="h-64">
              {logs.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <AreaChart data={logs}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
                    <XAxis
                      dataKey="step"
                      stroke="#a1a1aa"
                      fontSize={12}
                      tickLine={false}
                      axisLine={false}
                    />
                    <YAxis
                      stroke="#a1a1aa"
                      fontSize={12}
                      tickLine={false}
                      axisLine={false}
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#252526',
                        border: '1px solid #3f3f46',
                        borderRadius: '8px',
                      }}
                      itemStyle={{ color: '#e4e4e7' }}
                    />
                    <Area
                      type="monotone"
                      dataKey="loss"
                      stroke="#3b82f6"
                      strokeWidth={2}
                      fill="#3b82f6"
                      fillOpacity={0.1}
                    />
                  </AreaChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-full flex items-center justify-center text-app-subtext">
                  <div className="text-center">
                    <Terminal className="w-12 h-12 mx-auto mb-3 opacity-50" />
                    <div>Start training to see loss curve</div>
                  </div>
                </div>
              )}
            </div>
          </Card>

          <Card title="Live Logs" icon={Terminal} iconColor="text-orange-400">
            <div className="h-64 overflow-y-auto space-y-2">
              {logs.length === 0 ? (
                <div className="h-full flex items-center justify-center text-app-subtext">
                  <div className="text-center">
                    <Terminal className="w-12 h-12 mx-auto mb-3 opacity-50" />
                    <div>Waiting for training to start...</div>
                  </div>
                </div>
              ) : (
                logs.slice(-50).map((log, index) => (
                  <motion.div
                    key={`${log.step}-${index}`}
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ duration: 0.2 }}
                    className={`rounded-lg p-3 border text-xs font-mono ${
                      log.isStderr
                        ? 'bg-red-500/5 border-red-500/30'
                        : log.stage === 'Info'
                          ? 'bg-blue-500/5 border-blue-500/30'
                          : 'bg-background border-app-border'
                    }`}>
                    <div className="flex justify-between mb-1">
                      <span className="text-app-subtext">{log.timestamp}</span>
                      <span className={`${
                        log.isStderr
                          ? 'text-red-400'
                          : log.stage === 'Info'
                            ? 'text-blue-400'
                            : log.stage === 'Warmup'
                              ? 'text-orange-400'
                              : log.stage === 'Distillation'
                                ? 'text-purple-400'
                                : 'text-blue-400'
                      }`}>
                        {log.isStderr ? 'ERROR' : log.stage}
                      </span>
                    </div>
                    <div className={log.isStderr ? 'text-red-300' : 'text-app-text'}>
                      {log.isStderr ? (
                        <div className="break-words">{log.stderrMessage}</div>
                      ) : log.stage === 'Info' && log.stderrMessage ? (
                        <div className="break-words text-blue-300">{log.stderrMessage}</div>
                      ) : (
                        <>
                          Epoch {log.epoch} • Step {log.step} • Loss: {log.loss.toFixed(4)}
                          {log.alpha != null && <span className="text-app-subtext"> • α: {log.alpha.toFixed(2)}</span>}
                        </>
                      )}
                    </div>
                  </motion.div>
                ))
              )}
              <div ref={logsEndRef} />
            </div>
          </Card>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="mt-6">
          <Card title="Status Overview" icon={Activity} iconColor="text-blue-400">
            <div className="flex items-center gap-4">
              <div className={`p-3 rounded-xl transition-colors ${
                status === "running" || status === "queued" ? 'bg-blue-500/10 text-blue-400' :
                status === "completed" ? 'bg-green-500/10 text-green-400' :
                status === "failed" ? 'bg-red-500/10 text-red-400' :
                status === "cancelled" ? 'bg-orange-500/10 text-orange-400' : 
                'bg-app-card text-app-subtext'
              }`}>
                {status === "running" || status === "queued" ? (
                  <Loader2 className="w-6 h-6 animate-spin" />
                ) : status === "completed" ? (
                  <CheckCircle className="w-6 h-6" />
                ) : status === "failed" ? (
                  <AlertCircle className="w-6 h-6" />
                ) : status === "cancelled" ? (
                  <Pause className="w-6 h-6" />
                ) : (
                  <Activity className="w-6 h-6" />
                )}
              </div>
              <div className="flex-1">
                <div className="text-sm font-semibold text-app-text mb-1">
                  {status === "running" ? "Training in Progress" :
                   status === "queued" ? "Starting Training..." :
                   status === "completed" ? "Training Completed" :
                   status === "failed" ? "Training Failed" :
                   status === "cancelled" ? "Training Cancelled" : "Ready to Start"}
                </div>
                <div className="text-xs text-app-subtext">
                  {status === "running" ? `Step ${currentStep}/${totalSteps} • ${logs.length} logs recorded` :
                   status === "queued" ? "Initializing Python trainer..." :
                   status === "completed" ? "Training completed successfully! Check the Export tab for artifacts." :
                   status === "failed" ? error || "An error occurred during training." :
                   status === "cancelled" ? "Training was cancelled." :
                   store.activeRunId ? "Click Start to begin training." : "Create a training run in Setup first."}
                </div>
              </div>
              <div className="text-right">
                <div className="text-xs text-app-subtext mb-1">Progress</div>
                <div className="text-2xl font-bold text-app-text">
                  {progress.toFixed(0)}%
                </div>
              </div>
            </div>
          </Card>
        </motion.div>
      </aside>
    </div>
  );
}
