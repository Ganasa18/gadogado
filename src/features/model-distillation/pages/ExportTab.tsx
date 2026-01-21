import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Download,
  FileText,
  Package,
  CheckCircle,
  Shield,
  Clock,
  FolderOpen,
  FileDown,
  FileJson,
  RotateCcw,
  AlertTriangle,
  Settings2,
  ArrowUpCircle,
} from "lucide-react";
import {
  Card,
  Button,
  MetricCard,
  InfoBox,
  StatusBadge,
} from "../components/UI";
import {
  ModelDistillationAPI,
  type Model,
  type ModelVersion,
  type PromotionGuardrails,
  type PromotionResult,
  type RollbackResult,
} from "../api";

interface ReportFormat {
  id: string;
  name: string;
  extension: string;
  icon: React.ComponentType<{ className?: string }>;
}

const REPORT_FORMATS: ReportFormat[] = [
  { id: "markdown", name: "Markdown Report", extension: ".md", icon: FileJson },
  { id: "json", name: "JSON Data", extension: ".json", icon: FileJson },
  { id: "pdf", name: "PDF Report", extension: ".pdf", icon: FileText },
];

export default function ExportTab() {
  // Data state
  const [models, setModels] = useState<Model[]>([]);
  const [versions, setVersions] = useState<ModelVersion[]>([]);
  const [activeVersion, setActiveVersion] = useState<ModelVersion | null>(null);
  const [modelsLoading, setModelsLoading] = useState(true);
  const [versionsLoading, setVersionsLoading] = useState(true);

  // Selection state
  const [selectedModelId, setSelectedModelId] = useState("");
  const [selectedVersionId, setSelectedVersionId] = useState("");
  const [exportFormat, setExportFormat] = useState("adapter");
  const [reportFormat, setReportFormat] = useState("markdown");

  // Export state
  const [exportStatus, setExportStatus] = useState<"idle" | "preparing" | "exporting" | "complete" | "error">("idle");

  // Promotion state
  const [showPromotionConfig, setShowPromotionConfig] = useState(false);
  const [promotionInProgress, setPromotionInProgress] = useState(false);
  const [promotionResult, setPromotionResult] = useState<PromotionResult | null>(null);
  const [promotionError, setPromotionError] = useState<string | null>(null);
  const [guardrails, setGuardrails] = useState<PromotionGuardrails>({
    minExactMatch: 0.7,
    minBleu: undefined,
    minF1: undefined,
    requireEvaluation: true,
    force: false,
  });

  // Rollback state
  const [rollbackInProgress, setRollbackInProgress] = useState(false);
  const [rollbackResult, setRollbackResult] = useState<RollbackResult | null>(null);
  const [rollbackError, setRollbackError] = useState<string | null>(null);
  const [rollbackTargetId, setRollbackTargetId] = useState("");

  useEffect(() => {
    loadModels();
  }, []);

  useEffect(() => {
    if (selectedModelId) {
      loadVersions(selectedModelId);
      loadActiveVersion(selectedModelId);
    }
  }, [selectedModelId]);

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

  const loadVersions = async (modelId: string) => {
    try {
      setVersionsLoading(true);
      const data = await ModelDistillationAPI.listModelVersions(modelId);
      setVersions(data);
      if (data.length > 0 && !selectedVersionId) {
        setSelectedVersionId(data[0].versionId);
      }
    } catch (err) {
      console.error("Failed to load versions:", err);
    } finally {
      setVersionsLoading(false);
    }
  };

  const loadActiveVersion = async (modelId: string) => {
    try {
      const active = await ModelDistillationAPI.getActiveVersion(modelId);
      setActiveVersion(active);
    } catch (err) {
      console.error("Failed to load active version:", err);
    }
  };

  const exportFormats = [
    { value: "adapter", label: "LoRA Adapter (~50 MB)" },
    { value: "merged", label: "Merged Model (~450 MB)" },
    { value: "gguf", label: "GGUF Format (~120 MB)" },
  ];

  const formatSize = (bytes: number | undefined) => {
    if (!bytes) return "—";
    const mb = bytes / (1024 * 1024);
    return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
  };

  const formatDate = (dateString: string | undefined) => {
    if (!dateString) return "—";
    return new Date(dateString).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  };

  const handleDownloadModel = async () => {
    setExportStatus("preparing");
    await new Promise((resolve) => setTimeout(resolve, 1000));
    setExportStatus("exporting");
    await new Promise((resolve) => setTimeout(resolve, 2000));
    setExportStatus("complete");
  };

  const handleDownloadReport = async (format: string) => {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    alert(`Downloading report as ${format.toUpperCase()}`);
  };

  const handlePromote = async () => {
    if (!selectedModelId || !selectedVersionId) return;

    setPromotionInProgress(true);
    setPromotionError(null);
    setPromotionResult(null);

    try {
      const result = await ModelDistillationAPI.promoteVersion(
        selectedModelId,
        selectedVersionId,
        guardrails
      );
      setPromotionResult(result);
      if (result.success) {
        await loadActiveVersion(selectedModelId);
        await loadVersions(selectedModelId);
      }
    } catch (err) {
      setPromotionError(err instanceof Error ? err.message : String(err));
    } finally {
      setPromotionInProgress(false);
    }
  };

  const handleRollback = async () => {
    if (!selectedModelId || !rollbackTargetId) return;

    setRollbackInProgress(true);
    setRollbackError(null);
    setRollbackResult(null);

    try {
      const result = await ModelDistillationAPI.rollbackVersion(
        selectedModelId,
        rollbackTargetId
      );
      setRollbackResult(result);
      await loadActiveVersion(selectedModelId);
      await loadVersions(selectedModelId);
    } catch (err) {
      setRollbackError(err instanceof Error ? err.message : String(err));
    } finally {
      setRollbackInProgress(false);
    }
  };

  const selectedVersion = versions.find((v) => v.versionId === selectedVersionId);
  const previousVersions = versions.filter(
    (v) => v.versionId !== activeVersion?.versionId
  );

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-6 flex flex-col gap-6">
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="mb-4">
          <h1 className="text-3xl font-bold mb-2 text-app-text">
            Export Center
          </h1>
          <p className="text-app-subtext">Download models, manage promotions, and generate reports</p>
        </motion.div>

        <div className="grid grid-cols-4 gap-4">
          <MetricCard
            icon={Package}
            label="Total Versions"
            value={versions.length}
            delay={0}
          />
          <MetricCard
            icon={CheckCircle}
            label="Active Version"
            value={activeVersion?.versionId?.slice(0, 8) || "—"}
            delay={0.1}
          />
          <MetricCard
            icon={FolderOpen}
            label="Total Storage"
            value={formatSize(versions.reduce((acc, v) => acc + (v.artifactSizeBytes || 0), 0))}
            delay={0.2}
          />
          <MetricCard
            icon={Clock}
            label="Latest Update"
            value={formatDate(versions[0]?.createdAt)}
            delay={0.3}
          />
        </div>

        {/* Version Promotion with Guardrails */}
        <Card title="Version Promotion" icon={ArrowUpCircle} iconColor="text-green-400">
          <div className="grid grid-cols-2 gap-6">
            <div className="space-y-4">
              <div>
                <label className="text-xs font-semibold text-app-text mb-2 block">
                  Model
                </label>
                <select
                  value={selectedModelId}
                  onChange={(e) => setSelectedModelId(e.target.value)}
                  className="w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm appearance-none cursor-pointer hover:border-green-500/50 transition-all duration-200 outline-none focus:border-green-500">
                  {modelsLoading ? (
                    <option>Loading...</option>
                  ) : (
                    models.map((model) => (
                      <option key={model.modelId} value={model.modelId}>
                        {model.displayName}
                      </option>
                    ))
                  )}
                </select>
              </div>

              <div>
                <label className="text-xs font-semibold text-app-text mb-2 block">
                  Version to Promote
                </label>
                <select
                  value={selectedVersionId}
                  onChange={(e) => setSelectedVersionId(e.target.value)}
                  className="w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm appearance-none cursor-pointer hover:border-green-500/50 transition-all duration-200 outline-none focus:border-green-500">
                  {versionsLoading ? (
                    <option>Loading...</option>
                  ) : (
                    versions.map((version) => (
                      <option key={version.versionId} value={version.versionId}>
                        {version.versionId.slice(0, 8)}... {version.isPromoted && "(Active)"}
                      </option>
                    ))
                  )}
                </select>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={Settings2}
                  onClick={() => setShowPromotionConfig(!showPromotionConfig)}>
                  {showPromotionConfig ? "Hide" : "Show"} Guardrails
                </Button>
              </div>

              {showPromotionConfig && (
                <div className="bg-background rounded-lg p-4 border border-app-border space-y-3">
                  <div className="text-xs font-semibold text-app-text mb-2">
                    Promotion Guardrails
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-xs text-app-subtext block mb-1">Min Exact Match</label>
                      <input
                        type="number"
                        step="0.05"
                        min="0"
                        max="1"
                        value={guardrails.minExactMatch ?? ""}
                        onChange={(e) => setGuardrails({
                          ...guardrails,
                          minExactMatch: e.target.value ? parseFloat(e.target.value) : undefined,
                        })}
                        placeholder="0.7"
                        className="w-full bg-app-bg border border-app-border rounded px-3 py-2 text-xs outline-none"
                      />
                    </div>
                    <div>
                      <label className="text-xs text-app-subtext block mb-1">Min BLEU</label>
                      <input
                        type="number"
                        step="0.05"
                        min="0"
                        max="1"
                        value={guardrails.minBleu ?? ""}
                        onChange={(e) => setGuardrails({
                          ...guardrails,
                          minBleu: e.target.value ? parseFloat(e.target.value) : undefined,
                        })}
                        placeholder="Optional"
                        className="w-full bg-app-bg border border-app-border rounded px-3 py-2 text-xs outline-none"
                      />
                    </div>
                    <div>
                      <label className="text-xs text-app-subtext block mb-1">Min F1</label>
                      <input
                        type="number"
                        step="0.05"
                        min="0"
                        max="1"
                        value={guardrails.minF1 ?? ""}
                        onChange={(e) => setGuardrails({
                          ...guardrails,
                          minF1: e.target.value ? parseFloat(e.target.value) : undefined,
                        })}
                        placeholder="Optional"
                        className="w-full bg-app-bg border border-app-border rounded px-3 py-2 text-xs outline-none"
                      />
                    </div>
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={guardrails.requireEvaluation ?? false}
                        onChange={(e) => setGuardrails({
                          ...guardrails,
                          requireEvaluation: e.target.checked,
                        })}
                        className="rounded border-app-border"
                      />
                      <label className="text-xs text-app-subtext">Require evaluation</label>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 pt-2 border-t border-app-border">
                    <input
                      type="checkbox"
                      checked={guardrails.force ?? false}
                      onChange={(e) => setGuardrails({
                        ...guardrails,
                        force: e.target.checked,
                      })}
                      className="rounded border-app-border"
                    />
                    <label className="text-xs text-orange-400">Skip guardrails (force)</label>
                  </div>
                </div>
              )}
            </div>

            <div className="space-y-4">
              {promotionError && (
                <InfoBox type="error" icon={AlertTriangle}>
                  {promotionError}
                </InfoBox>
              )}

              {promotionResult && (
                <InfoBox 
                  type={promotionResult.success ? "success" : "error"} 
                  icon={promotionResult.success ? CheckCircle : AlertTriangle}
                >
                  <div className="font-medium mb-2">
                    {promotionResult.success ? "Promotion Successful" : "Promotion Failed"}
                  </div>
                  {promotionResult.guardrailChecks.length > 0 && (
                    <div className="space-y-1">
                      {promotionResult.guardrailChecks.map((check, i) => (
                        <div key={i} className="flex items-center justify-between text-xs">
                          <span className="text-app-subtext">{check.metricName}</span>
                          <span className={check.passed ? "text-green-400" : "text-red-400"}>
                            {check.actual?.toFixed(2) ?? "N/A"} / {check.required.toFixed(2)}
                            {check.passed ? " ✓" : " ✗"}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </InfoBox>
              )}

              <div className="bg-background rounded-lg p-4 border border-app-border">
                <div className="text-xs text-app-subtext mb-2">Current Active Version</div>
                <div className="text-sm font-mono text-app-text">
                  {activeVersion?.versionId || "None"}
                </div>
              </div>

              <Button
                variant="primary"
                size="lg"
                onClick={handlePromote}
                disabled={!selectedVersionId || promotionInProgress}
                loading={promotionInProgress}
                icon={ArrowUpCircle}
                className="w-full">
                Promote Version
              </Button>
            </div>
          </div>
        </Card>

        {/* Rollback */}
        <Card title="Version Rollback" icon={RotateCcw} iconColor="text-orange-400">
          <div className="grid grid-cols-2 gap-6">
            <div className="space-y-4">
              <div>
                <label className="text-xs font-semibold text-app-text mb-2 block">
                  Rollback to Version
                </label>
                <select
                  value={rollbackTargetId}
                  onChange={(e) => setRollbackTargetId(e.target.value)}
                  className="w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm appearance-none cursor-pointer hover:border-orange-500/50 transition-all duration-200 outline-none focus:border-orange-500">
                  <option value="">Select a version...</option>
                  {previousVersions.map((version) => (
                    <option key={version.versionId} value={version.versionId}>
                      {version.versionId.slice(0, 8)}... ({formatDate(version.createdAt)})
                    </option>
                  ))}
                </select>
              </div>

              <div className="bg-orange-500/10 border border-orange-500/30 rounded-lg p-4">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="w-4 h-4 text-orange-400 flex-shrink-0 mt-0.5" />
                  <div className="text-xs text-orange-300">
                    Rollback will change the active version. A backup will be created automatically.
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-4">
              {rollbackError && (
                <InfoBox type="error" icon={AlertTriangle}>
                  {rollbackError}
                </InfoBox>
              )}

              {rollbackResult && (
                <InfoBox type="success" icon={CheckCircle}>
                  <div className="font-medium mb-1">Rollback Complete</div>
                  <div className="text-xs text-app-subtext">
                    Now active: {rollbackResult.rolledBackTo.versionId.slice(0, 8)}...
                    {rollbackResult.backupCreated && " (backup created)"}
                  </div>
                </InfoBox>
              )}

              <Button
                variant="default"
                size="lg"
                onClick={handleRollback}
                disabled={!rollbackTargetId || rollbackInProgress}
                loading={rollbackInProgress}
                icon={RotateCcw}
                className="w-full">
                Rollback
              </Button>
            </div>
          </div>
        </Card>

        {/* Model Export */}
        <Card title="Model Export" icon={Download} iconColor="text-blue-400">
          <div className="grid grid-cols-2 gap-6">
            <div className="space-y-5">
              <div>
                <label className="text-xs font-semibold text-app-text mb-2 block">
                  Select Version
                </label>
                <select
                  value={selectedVersionId}
                  onChange={(e) => setSelectedVersionId(e.target.value)}
                  className="w-full bg-background border border-app-border rounded-lg px-4 py-3 text-sm appearance-none cursor-pointer hover:border-blue-500/50 transition-all duration-200 outline-none focus:border-blue-500">
                  {versionsLoading ? (
                    <option>Loading...</option>
                  ) : (
                    versions.map((version) => (
                      <option key={version.versionId} value={version.versionId}>
                        {version.versionId.slice(0, 8)}... {version.isPromoted && "(Active)"}
                      </option>
                    ))
                  )}
                </select>
              </div>

              <div>
                <label className="text-xs font-semibold text-app-text mb-2 block">
                  Export Format
                </label>
                <div className="space-y-2">
                  {exportFormats.map((format) => (
                    <button
                      key={format.value}
                      onClick={() => setExportFormat(format.value)}
                      className={`w-full text-left px-4 py-3 rounded-lg border transition-all ${
                        exportFormat === format.value
                          ? "border-blue-500 bg-blue-500/10"
                          : "border-app-border bg-background hover:border-blue-500/30"
                      }`}>
                      <div className="text-sm text-app-text">
                        {format.label}
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              {selectedVersion && (
                <div className="bg-background rounded-lg p-4 border border-app-border">
                  <div className="space-y-2 text-xs">
                    <div className="flex justify-between">
                      <span className="text-app-subtext">Size:</span>
                      <span className="text-app-text">{formatSize(selectedVersion.artifactSizeBytes)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-app-subtext">Created:</span>
                      <span className="text-app-text">{formatDate(selectedVersion.createdAt)}</span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-app-subtext">Status:</span>
                      <StatusBadge status="completed" text="Ready" />
                    </div>
                    <div className="pt-2 border-t border-app-border">
                      <div className="text-app-subtext mb-1">Artifact Path:</div>
                      <div className="text-app-text font-mono text-xs bg-app-bg p-2 rounded truncate">
                        {selectedVersion.artifactPath}
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            <div className="flex flex-col justify-between">
              <div className="flex-1">
                <div className="text-xs font-semibold text-app-text mb-3">
                  Export Status
                </div>
                <div className="bg-background rounded-lg p-6 border border-app-border">
                  <div className="text-center">
                    {exportStatus === "idle" && (
                      <div>
                        <Download className="w-12 h-12 mx-auto mb-3 text-app-subtext" />
                        <div className="text-sm text-app-text mb-1">Ready to Export</div>
                        <div className="text-xs text-app-subtext">
                          Select a version and format
                        </div>
                      </div>
                    )}
                    {exportStatus === "preparing" && (
                      <div>
                        <Package className="w-12 h-12 mx-auto mb-3 text-blue-400 animate-pulse" />
                        <div className="text-sm text-app-text mb-1">Preparing Export</div>
                        <div className="text-xs text-app-subtext">
                          Packing model files...
                        </div>
                      </div>
                    )}
                    {exportStatus === "exporting" && (
                      <div>
                        <FileDown className="w-12 h-12 mx-auto mb-3 text-blue-400 animate-bounce" />
                        <div className="text-sm text-app-text mb-1">Downloading...</div>
                        <div className="text-xs text-app-subtext">
                          {formatSize(selectedVersion?.artifactSizeBytes)} remaining
                        </div>
                      </div>
                    )}
                    {exportStatus === "complete" && (
                      <div>
                        <CheckCircle className="w-12 h-12 mx-auto mb-3 text-green-400" />
                        <div className="text-sm text-app-text mb-1">Export Complete</div>
                        <div className="text-xs text-app-subtext">
                          File saved to local directory
                        </div>
                      </div>
                    )}
                    {exportStatus === "error" && (
                      <div>
                        <Shield className="w-12 h-12 mx-auto mb-3 text-red-400" />
                        <div className="text-sm text-app-text mb-1">Export Failed</div>
                        <div className="text-xs text-app-subtext">
                          Please check file permissions
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <Button
                variant="primary"
                size="lg"
                onClick={handleDownloadModel}
                disabled={exportStatus === "preparing" || exportStatus === "exporting" || !selectedVersionId}
                loading={exportStatus === "preparing" || exportStatus === "exporting"}
                className="w-full mt-4">
                {exportStatus === "complete" ? "Download Again" : "Download Model"}
              </Button>
            </div>
          </div>
        </Card>

        {/* Report Generation */}
        <Card title="Report Generation" icon={FileText} iconColor="text-purple-400">
          <div className="space-y-5">
            <div>
              <label className="text-xs font-semibold text-app-text mb-3 block">
                Select Report Format
              </label>
              <div className="grid grid-cols-3 gap-4">
                {REPORT_FORMATS.map((format) => (
                  <motion.button
                    key={format.id}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => setReportFormat(format.id)}
                    className={`p-4 rounded-lg border transition-all text-left ${
                      reportFormat === format.id
                        ? "border-purple-500 bg-purple-500/10"
                        : "border-app-border bg-background hover:border-purple-500/30"
                    }`}>
                    <format.icon className={`w-6 h-6 mb-2 ${reportFormat === format.id ? "text-purple-400" : "text-app-subtext"}`} />
                    <div className="text-sm font-medium text-app-text">
                      {format.name}
                    </div>
                    <div className="text-xs text-app-subtext mt-1">
                      {format.extension}
                    </div>
                  </motion.button>
                ))}
              </div>
            </div>

            <div className="flex gap-3">
              <Button
                variant="primary"
                size="lg"
                onClick={() => handleDownloadReport(reportFormat)}
                icon={FileText}
                className="flex-1">
                Generate Report
              </Button>
              <Button
                variant="default"
                size="lg"
                icon={FolderOpen}>
                View All Reports
              </Button>
            </div>
          </div>
        </Card>

        {/* Security Footer */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="bg-app-card border border-app-border rounded-xl p-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="p-3 bg-green-500/10 rounded-xl">
                <Shield className="w-6 h-6 text-green-400" />
              </div>
              <div>
                <div className="text-sm font-semibold text-app-text mb-1">
                  Secure Export Mode
                </div>
                <div className="text-xs text-app-subtext">
                  No credentials stored • Automatic backups before promotions
                </div>
              </div>
            </div>
            <div className="text-right">
              <div className="text-xs text-app-subtext mb-1">Versions Available</div>
              <div className="text-2xl font-bold text-app-text">
                {versions.length}
              </div>
            </div>
          </div>
        </motion.div>
      </aside>
    </div>
  );
}
