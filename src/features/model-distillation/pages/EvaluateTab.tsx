import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
  BarChart3,
  CheckCircle,
  FileText,
  Download,
  ArrowLeftRight,
  TrendingUp,
  Grid,
  List,
  AlertCircle,
  RefreshCcw,
  GitCompare,
  ArrowUp,
  ArrowDown,
  Minus,
} from "lucide-react";
import {
  Card,
  Select,
  Button,
  MetricCard,
  InfoBox,
} from "../components/UI";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  LineChart,
  Line,
} from "recharts";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ModelDistillationAPI,
  type Dataset,
  type DistillEvalConfig,
  type Model,
  type ModelVersion,
} from "../api";
import type { EvaluationMetric } from "../types";

function downloadBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  URL.revokeObjectURL(url);
}

async function exportSvg(container: HTMLDivElement | null, fileName: string) {
  if (!container) return;
  const svg = container.querySelector("svg");
  if (!svg) return;
  const serializer = new XMLSerializer();
  const source = serializer.serializeToString(svg);
  const blob = new Blob([source], { type: "image/svg+xml;charset=utf-8" });
  downloadBlob(blob, fileName);
}

async function exportPng(container: HTMLDivElement | null, fileName: string) {
  if (!container) return;
  const svg = container.querySelector("svg");
  if (!svg) return;
  const serializer = new XMLSerializer();
  const source = serializer.serializeToString(svg);
  const svgBlob = new Blob([source], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(svgBlob);

  const img = new Image();
  img.onload = () => {
    const canvas = document.createElement("canvas");
    canvas.width = img.width;
    canvas.height = img.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.drawImage(img, 0, 0);
    canvas.toBlob((blob) => {
      if (blob) downloadBlob(blob, fileName);
      URL.revokeObjectURL(url);
    });
  };
  img.src = url;
}

function formatValue(_name: string, value: number) {
  if (value <= 1 && value >= 0) {
    return { display: `${(value * 100).toFixed(2)}%`, chartValue: value * 100 };
  }
  return { display: value.toFixed(4), chartValue: value };
}

export default function EvaluateTab() {
  const [models, setModels] = useState<Model[]>([]);
  const [modelsLoading, setModelsLoading] = useState(true);
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [datasetsLoading, setDatasetsLoading] = useState(true);
  const [versions, setVersions] = useState<ModelVersion[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(true);

  const [selectedModelId, setSelectedModelId] = useState("");
  const [selectedVersionId, setSelectedVersionId] = useState("");
  const [baselineVersionId, setBaselineVersionId] = useState("");
  const [selectedDatasetId, setSelectedDatasetId] = useState("");

  const [candidateMetrics, setCandidateMetrics] = useState<EvaluationMetric[]>([]);
  const [baselineMetrics, setBaselineMetrics] = useState<EvaluationMetric[]>([]);
  const [metricsByVersion, setMetricsByVersion] = useState<Record<string, EvaluationMetric[]>>({});

  const [viewMode, setViewMode] = useState<"summary" | "trend" | "heatmap" | "compare">("summary");
  const [trendMetric, setTrendMetric] = useState("exact_match");
  const [evalError, setEvalError] = useState<string | null>(null);
  const [evalStatus, setEvalStatus] = useState<"idle" | "running" | "error" | "complete">("idle");
  const [evalProgress, setEvalProgress] = useState({ processed: 0, total: 0 });
  const [activeEvalId, setActiveEvalId] = useState<string | null>(null);
  const activeEvalIdRef = useRef<string | null>(null);
  const [computeTeacherAgreement, setComputeTeacherAgreement] = useState(false);

  const [datasetName, setDatasetName] = useState("");
  const [datasetDescription, setDatasetDescription] = useState("");
  const [datasetImporting, setDatasetImporting] = useState(false);

  const barRef = useRef<HTMLDivElement>(null);
  const lineRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    loadModels();
    loadDatasets();
  }, []);

  useEffect(() => {
    activeEvalIdRef.current = activeEvalId;
  }, [activeEvalId]);

  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen("distill-eval-stream", (event) => {
        const msg = event.payload as { kind: string; payload: Record<string, unknown> };
        handleEvalEvent(msg);
      });
    };

    setupListener();

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    if (selectedModelId) {
      loadVersions(selectedModelId);
    }
  }, [selectedModelId]);

  useEffect(() => {
    if (selectedVersionId) {
      loadVersionMetrics(selectedVersionId, setCandidateMetrics);
    }
  }, [selectedVersionId]);

  useEffect(() => {
    if (baselineVersionId) {
      loadVersionMetrics(baselineVersionId, setBaselineMetrics);
    }
  }, [baselineVersionId]);

  useEffect(() => {
    if (versions.length === 0) return;
    const names = new Set<string>();
    candidateMetrics.forEach((m) => names.add(m.metricName));
    baselineMetrics.forEach((m) => names.add(m.metricName));
    if (!names.has(trendMetric)) {
      const first = names.values().next().value;
      if (first) setTrendMetric(first);
    }
  }, [candidateMetrics, baselineMetrics, trendMetric, versions.length]);

  useEffect(() => {
    if (!selectedDatasetId || versions.length === 0) return;
    let mounted = true;
    const loadAll = async () => {
      const entries = await Promise.all(
        versions.map((v) => ModelDistillationAPI.listVersionMetrics(v.versionId))
      );
      if (!mounted) return;
      const next: Record<string, EvaluationMetric[]> = {};
      versions.forEach((v, idx) => {
        next[v.versionId] = entries[idx].filter((m) => m.datasetId === selectedDatasetId);
      });
      setMetricsByVersion(next);
    };
    loadAll();
    return () => {
      mounted = false;
    };
  }, [versions, selectedDatasetId]);

  const loadModels = async () => {
    try {
      setModelsLoading(true);
      const data = await ModelDistillationAPI.listModels();
      setModels(data);
      if (!selectedModelId && data.length > 0) {
        setSelectedModelId(data[0].modelId);
      }
    } catch (err) {
      console.error("Failed to load models:", err);
    } finally {
      setModelsLoading(false);
    }
  };

  const loadDatasets = async () => {
    try {
      setDatasetsLoading(true);
      const data = await ModelDistillationAPI.listDatasets();
      setDatasets(data);
      if (!selectedDatasetId && data.length > 0) {
        const golden = data.find((d) => d.type === "golden");
        setSelectedDatasetId(golden?.datasetId || data[0].datasetId);
      }
    } catch (err) {
      console.error("Failed to load datasets:", err);
    } finally {
      setDatasetsLoading(false);
    }
  };

  const loadVersions = async (modelId: string) => {
    try {
      setVersionsLoading(true);
      const data = await ModelDistillationAPI.listModelVersions(modelId);
      setVersions(data);
      if (data.length > 0) {
        setSelectedVersionId((prev) => prev || data[0].versionId);
        setBaselineVersionId((prev) => prev || data[Math.min(1, data.length - 1)].versionId);
      }
    } catch (err) {
      console.error("Failed to load versions:", err);
    } finally {
      setVersionsLoading(false);
    }
  };

  const loadVersionMetrics = async (
    versionId: string,
    setter: (metrics: EvaluationMetric[]) => void
  ) => {
    try {
      const data = await ModelDistillationAPI.listVersionMetrics(versionId);
      setter(data);
    } catch (err) {
      console.error("Failed to load metrics:", err);
    }
  };

  const handleEvalEvent = (msg: { kind: string; payload: Record<string, unknown> }) => {
    const { kind, payload } = msg;
    const evalId = payload.eval_id as string | undefined;
    const activeId = activeEvalIdRef.current;
    if (activeId && evalId && evalId !== activeId) return;

    if (kind === "progress") {
      setEvalProgress({
        processed: (payload.processed as number) || 0,
        total: (payload.total as number) || 0,
      });
    }

    if (kind === "status") {
      const level = payload.level as string;
      const message = payload.message as string;
      if (level === "error") {
        setEvalStatus("error");
        setEvalError(message);
      }
      if (level === "info" && message === "evaluator completed") {
        setEvalStatus("complete");
        if (selectedVersionId) loadVersionMetrics(selectedVersionId, setCandidateMetrics);
        if (baselineVersionId) loadVersionMetrics(baselineVersionId, setBaselineMetrics);
      }
    }
  };

  const evaluateVersion = async (versionId: string) => {
    if (!versionId || !selectedDatasetId) return;
    setEvalError(null);
    setEvalStatus("running");
    setEvalProgress({ processed: 0, total: 0 });

    const config: DistillEvalConfig = {
      versionId,
      datasetId: selectedDatasetId,
      maxSamples: 200,
      maxNewTokens: 128,
      temperature: 0,
      topP: 1,
      computeTeacherAgreement,
    };

    try {
      const evalId = await ModelDistillationAPI.evaluateVersion(config);
      setActiveEvalId(evalId);
    } catch (err) {
      setEvalStatus("error");
      setEvalError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleImportDataset = async () => {
    try {
      setDatasetImporting(true);
      const selection = await open({
        multiple: false,
        filters: [{ name: "JSONL", extensions: ["jsonl"] }],
      });
      if (!selection || Array.isArray(selection)) return;

      const path = selection as string;
      const name = datasetName || path.split(/[/\\]/).filter(Boolean).pop() || "Golden Dataset";
      await ModelDistillationAPI.importDatasetJsonl({
        name,
        datasetType: "golden",
        description: datasetDescription || undefined,
        path,
      });
      setDatasetName("");
      setDatasetDescription("");
      await loadDatasets();
    } catch (err) {
      setEvalError(err instanceof Error ? err.message : String(err));
    } finally {
      setDatasetImporting(false);
    }
  };

  const filteredCandidate = useMemo(() => {
    return candidateMetrics.filter((m) => m.datasetId === selectedDatasetId);
  }, [candidateMetrics, selectedDatasetId]);

  const filteredBaseline = useMemo(() => {
    return baselineMetrics.filter((m) => m.datasetId === selectedDatasetId);
  }, [baselineMetrics, selectedDatasetId]);

  const metricNames = useMemo(() => {
    const set = new Set<string>();
    filteredCandidate.forEach((m) => set.add(m.metricName));
    filteredBaseline.forEach((m) => set.add(m.metricName));
    return Array.from(set.values());
  }, [filteredCandidate, filteredBaseline]);

  const candidateMap = useMemo(() => {
    const map = new Map<string, number>();
    filteredCandidate.forEach((m) => map.set(m.metricName, m.metricValue));
    return map;
  }, [filteredCandidate]);

  const baselineMap = useMemo(() => {
    const map = new Map<string, number>();
    filteredBaseline.forEach((m) => map.set(m.metricName, m.metricValue));
    return map;
  }, [filteredBaseline]);

  const barChartData = metricNames.map((name) => {
    const baselineValue = baselineMap.get(name);
    const candidateValue = candidateMap.get(name);
    const baselineFormatted = baselineValue != null ? formatValue(name, baselineValue) : null;
    const candidateFormatted = candidateValue != null ? formatValue(name, candidateValue) : null;
    return {
      name,
      baseline: baselineFormatted?.chartValue ?? 0,
      current: candidateFormatted?.chartValue ?? 0,
    };
  });

  const trendData = versions.map((version) => {
    const metrics = metricsByVersion[version.versionId] || [];
    const metric = metrics.find((m) => m.metricName === trendMetric);
    const value = metric ? formatValue(trendMetric, metric.metricValue).chartValue : 0;
    return { version: version.versionId, value };
  });

  const heatmapMax = Math.max(
    1,
    ...Object.values(metricsByVersion).flatMap((metrics) =>
      metrics.map((m) => formatValue(m.metricName, m.metricValue).chartValue)
    )
  );

  const primaryMetric = candidateMap.get("exact_match") ?? filteredCandidate[0]?.metricValue ?? 0;
  const baselineMetric = baselineMap.get("exact_match") ?? filteredBaseline[0]?.metricValue ?? 0;
  const improvement = primaryMetric && baselineMetric
    ? ((primaryMetric - baselineMetric) / Math.max(1e-9, baselineMetric)) * 100
    : 0;

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-6 flex flex-col gap-6">
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="mb-4">
          <h1 className="text-3xl font-bold mb-2 text-app-text">
            Evaluation Dashboard
          </h1>
          <p className="text-app-subtext">
            Compare model versions and analyze performance metrics
          </p>
        </motion.div>

        {evalError && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mb-4">
            <InfoBox type="error" icon={AlertCircle}>
              <div className="font-semibold mb-1">Evaluation Error</div>
              <div>{evalError}</div>
            </InfoBox>
          </motion.div>
        )}

        <div className="grid grid-cols-4 gap-4">
          <MetricCard
            icon={CheckCircle}
            label="Primary Metric"
            value={formatValue("primary", primaryMetric).display}
            delay={0}
          />
          <MetricCard
            icon={TrendingUp}
            label="Improvement"
            value={isFinite(improvement) ? improvement.toFixed(1) : "0"}
            unit="%"
            delay={0.1}
          />
          <MetricCard
            icon={FileText}
            label="Candidate"
            value={selectedVersionId || "None"}
            delay={0.2}
          />
          <MetricCard
            icon={Grid}
            label="Samples Evaluated"
            value={evalProgress.total || "-"}
            delay={0.3}
          />
        </div>

        <Card title="Evaluation Configuration" icon={BarChart3} iconColor="text-blue-400">
          <div className="grid grid-cols-4 gap-6">
            <Select
              label="Model"
              value={selectedModelId}
              onChange={setSelectedModelId}
              options={modelsLoading ? [{ value: "", label: "Loading..." }] : models.map((m) => ({
                value: m.modelId,
                label: m.displayName,
              }))}
            />
            <Select
              label="Candidate Version"
              value={selectedVersionId}
              onChange={setSelectedVersionId}
              options={versionsLoading ? [{ value: "", label: "Loading..." }] : versions.map((v) => ({
                value: v.versionId,
                label: v.versionId,
              }))}
            />
            <Select
              label="Baseline Version"
              value={baselineVersionId}
              onChange={setBaselineVersionId}
              options={versionsLoading ? [{ value: "", label: "Loading..." }] : versions.map((v) => ({
                value: v.versionId,
                label: v.versionId,
              }))}
            />
            <Select
              label="Dataset"
              value={selectedDatasetId}
              onChange={setSelectedDatasetId}
              options={datasetsLoading ? [{ value: "", label: "Loading..." }] : datasets.map((d) => ({
                value: d.datasetId,
                label: `${d.name} (${d.type})`,
              }))}
            />
          </div>

          <div className="flex gap-2 mt-4">
            <Button
              variant="primary"
              size="sm"
              icon={BarChart3}
              onClick={() => evaluateVersion(selectedVersionId)}
              disabled={!selectedVersionId || !selectedDatasetId}>
              Evaluate Candidate
            </Button>
            <Button
              variant="default"
              size="sm"
              icon={ArrowLeftRight}
              onClick={() => evaluateVersion(baselineVersionId)}
              disabled={!baselineVersionId || !selectedDatasetId}>
              Evaluate Baseline
            </Button>
            <Button
              variant="ghost"
              size="sm"
              icon={RefreshCcw}
              onClick={() => {
                if (selectedVersionId) loadVersionMetrics(selectedVersionId, setCandidateMetrics);
                if (baselineVersionId) loadVersionMetrics(baselineVersionId, setBaselineMetrics);
              }}>
              Refresh Metrics
            </Button>
            <div className="text-xs text-app-subtext flex items-center">
              {evalStatus === "running" && evalProgress.total > 0
                ? `Running ${evalProgress.processed}/${evalProgress.total}`
                : evalStatus === "complete"
                  ? "Evaluation complete"
                  : ""}
            </div>
          </div>
          <div className="mt-3 flex items-center gap-2 text-xs text-app-subtext">
            <input
              type="checkbox"
              checked={computeTeacherAgreement}
              onChange={(e) => setComputeTeacherAgreement(e.target.checked)}
              className="rounded border-app-border"
            />
            <span>Compute teacher agreement (KD proxy)</span>
          </div>
        </Card>

        <Card title="Import Golden Dataset" icon={Download} iconColor="text-green-400">
          <div className="grid grid-cols-3 gap-4">
            <div>
              <label className="text-xs text-app-subtext block mb-1">Dataset Name</label>
              <input
                className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs outline-none"
                value={datasetName}
                onChange={(e) => setDatasetName(e.target.value)}
                placeholder="golden-v1"
              />
            </div>
            <div>
              <label className="text-xs text-app-subtext block mb-1">Description</label>
              <input
                className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs outline-none"
                value={datasetDescription}
                onChange={(e) => setDatasetDescription(e.target.value)}
                placeholder="Golden dataset import"
              />
            </div>
            <div className="flex items-end">
              <Button
                variant="primary"
                size="sm"
                icon={Download}
                onClick={handleImportDataset}
                loading={datasetImporting}>
                Import JSONL
              </Button>
            </div>
          </div>
        </Card>

        <div className="flex gap-2 mb-2">
          <Button
            variant={viewMode === "summary" ? "primary" : "default"}
            size="sm"
            icon={BarChart3}
            onClick={() => setViewMode("summary")}>Summary</Button>
          <Button
            variant={viewMode === "compare" ? "primary" : "default"}
            size="sm"
            icon={GitCompare}
            onClick={() => setViewMode("compare")}>Compare</Button>
          <Button
            variant={viewMode === "trend" ? "primary" : "default"}
            size="sm"
            icon={List}
            onClick={() => setViewMode("trend")}>Trend</Button>
          <Button
            variant={viewMode === "heatmap" ? "primary" : "default"}
            size="sm"
            icon={Grid}
            onClick={() => setViewMode("heatmap")}>Heatmap</Button>
        </div>

        {viewMode === "summary" && (
          <Card title="Metrics Comparison" icon={ArrowLeftRight} iconColor="text-purple-400">
            <div className="flex justify-end gap-2 mb-2">
              <Button variant="ghost" size="sm" onClick={() => exportSvg(barRef.current, "metrics.svg")}>Export SVG</Button>
              <Button variant="ghost" size="sm" onClick={() => exportPng(barRef.current, "metrics.png")}>Export PNG</Button>
            </div>
            <div className="h-72" ref={barRef}>
              {barChartData.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={barChartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
                    <XAxis dataKey="name" stroke="#a1a1aa" fontSize={12} tickLine={false} axisLine={false} />
                    <YAxis stroke="#a1a1aa" fontSize={12} tickLine={false} axisLine={false} />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "#252526",
                        border: "1px solid #3f3f46",
                        borderRadius: "8px",
                      }}
                      itemStyle={{ color: "#e4e4e7" }}
                    />
                    <Bar dataKey="baseline" fill="#6b7280" radius={[4, 4, 0, 0]} />
                    <Bar dataKey="current" fill="#3b82f6" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-full flex items-center justify-center text-app-subtext">
                  No metrics yet. Run evaluation to populate results.
                </div>
              )}
            </div>

            <div className="mt-4 border-t border-app-border pt-4">
              <div className="grid grid-cols-4 text-xs text-app-subtext mb-2">
                <div>Metric</div>
                <div>Baseline</div>
                <div>Candidate</div>
                <div>Delta</div>
              </div>
              <div className="space-y-2">
                {metricNames.map((name) => {
                  const baseVal = baselineMap.get(name);
                  const candVal = candidateMap.get(name);
                  const delta = baseVal != null && candVal != null ? candVal - baseVal : null;
                  const baseFmt = baseVal != null ? formatValue(name, baseVal).display : "-";
                  const candFmt = candVal != null ? formatValue(name, candVal).display : "-";
                  return (
                    <div key={name} className="grid grid-cols-4 text-xs border border-app-border rounded px-3 py-2">
                      <div className="text-app-text">{name}</div>
                      <div className="text-app-subtext">{baseFmt}</div>
                      <div className="text-app-text">{candFmt}</div>
                      <div className={delta != null && delta >= 0 ? "text-green-400" : "text-red-400"}>
                        {delta != null ? delta.toFixed(4) : "-"}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </Card>
        )}

        {viewMode === "trend" && (
          <Card title="Metric Trend" icon={TrendingUp} iconColor="text-green-400">
            <div className="flex items-center justify-between mb-3">
              <Select
                label="Metric"
                value={trendMetric}
                onChange={setTrendMetric}
                options={metricNames.map((name) => ({ value: name, label: name }))}
              />
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" onClick={() => exportSvg(lineRef.current, "trend.svg")}>Export SVG</Button>
                <Button variant="ghost" size="sm" onClick={() => exportPng(lineRef.current, "trend.png")}>Export PNG</Button>
              </div>
            </div>
            <div className="h-72" ref={lineRef}>
              {trendData.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={trendData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
                    <XAxis dataKey="version" stroke="#a1a1aa" fontSize={12} tickLine={false} axisLine={false} />
                    <YAxis stroke="#a1a1aa" fontSize={12} tickLine={false} axisLine={false} />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "#252526",
                        border: "1px solid #3f3f46",
                        borderRadius: "8px",
                      }}
                      itemStyle={{ color: "#e4e4e7" }}
                    />
                    <Line type="monotone" dataKey="value" stroke="#22c55e" strokeWidth={2} />
                  </LineChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-full flex items-center justify-center text-app-subtext">
                  No trend data available yet.
                </div>
              )}
            </div>
          </Card>
        )}

        {viewMode === "heatmap" && (
          <Card title="Metrics Heatmap" icon={Grid} iconColor="text-orange-400">
            {versions.length === 0 || metricNames.length === 0 ? (
              <div className="text-app-subtext">No metrics to display.</div>
            ) : (
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div
                  className="grid gap-1"
                  style={{ gridTemplateColumns: `160px repeat(${metricNames.length}, minmax(80px, 1fr))` }}>
                  <div />
                  {metricNames.map((name) => (
                    <div key={name} className="text-xs text-center text-app-subtext py-2">
                      {name}
                    </div>
                  ))}
                  {versions.map((version) => {
                    const metrics = metricsByVersion[version.versionId] || [];
                    const map = new Map(metrics.map((m) => [m.metricName, m.metricValue]));
                    return (
                      <div key={version.versionId} className="contents">
                        <div className="text-xs text-app-subtext py-2 pr-2 truncate">
                          {version.versionId}
                        </div>
                        {metricNames.map((name) => {
                          const value = map.get(name) ?? 0;
                          const scaled = formatValue(name, value).chartValue;
                          const intensity = Math.min(1, scaled / heatmapMax);
                          return (
                            <div
                              key={`${version.versionId}-${name}`}
                              className="text-xs text-center font-mono py-2 rounded"
                              style={{
                                backgroundColor: `rgba(59, 130, 246, ${intensity})`,
                                color: intensity > 0.5 ? "white" : "#e4e4e7",
                              }}>
                              {scaled.toFixed(2)}
                            </div>
                          );
                        })}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </Card>
        )}

        {viewMode === "compare" && (
          <Card title="Side-by-Side Comparison" icon={GitCompare} iconColor="text-cyan-400">
            <div className="grid grid-cols-2 gap-6">
              {/* Baseline Version */}
              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="flex items-center gap-2 mb-4">
                  <div className="w-3 h-3 rounded-full bg-gray-500" />
                  <div className="text-sm font-medium text-app-text">Baseline</div>
                  <div className="text-xs text-app-subtext font-mono truncate flex-1">
                    {baselineVersionId || "Not selected"}
                  </div>
                </div>
                {filteredBaseline.length === 0 ? (
                  <div className="text-xs text-app-subtext py-8 text-center">
                    No metrics available. Run evaluation first.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {metricNames.map((name) => {
                      const value = baselineMap.get(name);
                      const formatted = value != null ? formatValue(name, value) : null;
                      return (
                        <div key={name} className="flex justify-between items-center">
                          <span className="text-xs text-app-subtext">{name}</span>
                          <span className="text-sm font-mono text-app-text">
                            {formatted?.display ?? "-"}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Candidate Version */}
              <div className="bg-background rounded-lg p-4 border border-blue-500/30">
                <div className="flex items-center gap-2 mb-4">
                  <div className="w-3 h-3 rounded-full bg-blue-500" />
                  <div className="text-sm font-medium text-app-text">Candidate</div>
                  <div className="text-xs text-app-subtext font-mono truncate flex-1">
                    {selectedVersionId || "Not selected"}
                  </div>
                </div>
                {filteredCandidate.length === 0 ? (
                  <div className="text-xs text-app-subtext py-8 text-center">
                    No metrics available. Run evaluation first.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {metricNames.map((name) => {
                      const candValue = candidateMap.get(name);
                      const baseValue = baselineMap.get(name);
                      const formatted = candValue != null ? formatValue(name, candValue) : null;
                      const delta = candValue != null && baseValue != null ? candValue - baseValue : null;
                      const deltaPercent = delta != null && baseValue != null && baseValue !== 0
                        ? (delta / baseValue) * 100
                        : null;

                      return (
                        <div key={name} className="flex justify-between items-center">
                          <span className="text-xs text-app-subtext">{name}</span>
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-mono text-app-text">
                              {formatted?.display ?? "-"}
                            </span>
                            {delta != null && (
                              <span className={`flex items-center text-xs ${
                                delta > 0 ? "text-green-400" : delta < 0 ? "text-red-400" : "text-app-subtext"
                              }`}>
                                {delta > 0 ? <ArrowUp className="w-3 h-3" /> :
                                 delta < 0 ? <ArrowDown className="w-3 h-3" /> :
                                 <Minus className="w-3 h-3" />}
                                {deltaPercent != null && (
                                  <span className="ml-1">
                                    {Math.abs(deltaPercent).toFixed(1)}%
                                  </span>
                                )}
                              </span>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            {/* Delta Summary */}
            {metricNames.length > 0 && (filteredCandidate.length > 0 || filteredBaseline.length > 0) && (
              <div className="mt-6 pt-4 border-t border-app-border">
                <div className="text-sm font-medium text-app-text mb-3">Delta Analysis</div>
                <div className="grid grid-cols-4 gap-4">
                  {metricNames.slice(0, 4).map((name) => {
                    const candValue = candidateMap.get(name);
                    const baseValue = baselineMap.get(name);
                    const delta = candValue != null && baseValue != null ? candValue - baseValue : null;
                    const deltaPercent = delta != null && baseValue != null && baseValue !== 0
                      ? (delta / baseValue) * 100
                      : null;

                    return (
                      <div
                        key={name}
                        className={`bg-background rounded-lg p-3 border ${
                          delta != null && delta > 0
                            ? "border-green-500/30"
                            : delta != null && delta < 0
                              ? "border-red-500/30"
                              : "border-app-border"
                        }`}
                      >
                        <div className="text-xs text-app-subtext mb-1">{name}</div>
                        <div className={`text-lg font-bold ${
                          delta != null && delta > 0 ? "text-green-400" :
                          delta != null && delta < 0 ? "text-red-400" : "text-app-text"
                        }`}>
                          {delta != null ? (
                            <>
                              {delta > 0 ? "+" : ""}{delta.toFixed(4)}
                              {deltaPercent != null && (
                                <span className="text-xs ml-1">
                                  ({deltaPercent > 0 ? "+" : ""}{deltaPercent.toFixed(1)}%)
                                </span>
                              )}
                            </>
                          ) : "-"}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </Card>
        )}

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="flex gap-4">
          <Button variant="default" className="flex-1" icon={Download}>
            Download CSV
          </Button>
          <Button variant="primary" className="flex-1" icon={Download}>
            Export Report
          </Button>
        </motion.div>
      </aside>
    </div>
  );
}
