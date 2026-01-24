import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router";
import {
  RefreshCcw,
  Wand2,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  AlertTriangle,
  ShieldAlert,
  Search,
  BookOpen,
  LayoutList,
  Fingerprint,
  Layers,
  Clock,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../../../store/toast";
import {
  PROVIDER_MODEL_OPTIONS,
  useSettingsStore,
} from "../../../store/settings";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";
import { useModelsQuery } from "../../../hooks/useLlmApi";
import { isTauri } from "../../../utils/tauri";
import useQaSession from "../hooks/useQaSession";
import type {
  QaCheckpoint,
  QaCheckpointSummary,
  QaLlmRun,
  QaTestCase,
} from "../../../types/qa/types";
import SessionDetailHeader from "../components/SessionDetailHeader";

const LANGUAGES = [
  "English",
  "Indonesian",
  "Spanish",
  "French",
  "German",
  "Chinese",
  "Japanese",
  "Arabic",
  "Russian",
  "Portuguese",
  "Hindi",
  "Korean",
];

export default function AiResultsPage() {
  const { id } = useParams();
  const sessionId = id ?? "";
  const navigate = useNavigate();
  const { addToast } = useToastStore();
  const {
    provider,
    model,
    localModels,
    setLocalModels,
    // apiKey,
    aiOutputLanguage,
    setAiOutputLanguage,
  } = useSettingsStore();
  const buildConfig = useLlmConfigBuilder();
  const isTauriApp = isTauri();

  const [checkpoints, setCheckpoints] = useState<QaCheckpoint[]>([]);
  const [summaries, setSummaries] = useState<QaCheckpointSummary[]>([]);
  const [testCases, setTestCases] = useState<QaTestCase[]>([]);
  const [_, setLlmRuns] = useState<QaLlmRun[]>([]);
  const [loading, setLoading] = useState(false);
  const [actionCheckpoint, setActionCheckpoint] = useState<string | null>(null);
  const [manualTitle, setManualTitle] = useState("");
  const [selectedModel, setSelectedModel] = useState(model);
  const [activeCheckpointId, setActiveCheckpointId] = useState<string | null>(
    null,
  );

  const { session, sessionLoading, sessionError, loadSession } = useQaSession({
    sessionId,
    isTauriApp,
  });

  const isLocalProvider =
    provider === "local" || provider === "ollama" || provider === "llama_cpp";
  // const isOpenRouter = provider === "openrouter";
  // const canFetchRemoteModels = isOpenRouter && apiKey.trim().length > 0;
  const localConfig = useMemo(
    () => buildConfig({ maxTokens: 1024, temperature: 0.4 }),
    [buildConfig],
  );
  const modelsQuery = useModelsQuery(localConfig, isLocalProvider);

  useEffect(() => {
    if (!isLocalProvider) return;
    if (!modelsQuery.data) return;
    setLocalModels(modelsQuery.data);
    if (!selectedModel && modelsQuery.data.length > 0) {
      setSelectedModel(modelsQuery.data[0]);
    }
  }, [isLocalProvider, modelsQuery.data, setLocalModels, selectedModel]);

  useEffect(() => {
    setSelectedModel(model);
  }, [model]);

  // Set initial active checkpoint
  useEffect(() => {
    if (checkpoints.length > 0 && !activeCheckpointId) {
      setActiveCheckpointId(checkpoints[0].id);
    }
  }, [checkpoints, activeCheckpointId]);

  const modelOptions = useMemo(() => {
    if (isLocalProvider) {
      return localModels.length > 0 ? localModels : ["No models found"];
    }
    if (provider === "gemini") {
      return PROVIDER_MODEL_OPTIONS.gemini ?? ["gemini-2.0-flash"];
    }
    if (provider === "openai") {
      return PROVIDER_MODEL_OPTIONS.openai ?? ["gpt-4o", "gpt-4o-mini"];
    }
    if (provider === "openrouter") {
      const models =
        (modelsQuery.data && modelsQuery.data.length > 0
          ? modelsQuery.data
          : PROVIDER_MODEL_OPTIONS.openrouter) ?? [];
      return models.length > 0 ? models : ["custom-model"];
    }
    return ["custom-model"];
  }, [isLocalProvider, localModels, provider, modelsQuery.data]);

  const canGenerate =
    selectedModel !== "" && selectedModel !== "No models found";

  const summariesByCheckpoint = useMemo(() => {
    return summaries.reduce<Record<string, QaCheckpointSummary>>(
      (acc, summary) => {
        acc[summary.checkpointId] = summary;
        return acc;
      },
      {},
    );
  }, [summaries]);

  const testCasesByCheckpoint = useMemo(() => {
    return testCases.reduce<Record<string, QaTestCase[]>>((acc, testCase) => {
      const key = testCase.checkpointId ?? "unassigned";
      if (!acc[key]) acc[key] = [];
      acc[key].push(testCase);
      return acc;
    }, {});
  }, [testCases]);

  const loadOutputs = async () => {
    if (!sessionId || !isTauriApp) return;
    setLoading(true);
    try {
      const [checkpointData, summaryData, testCaseData, runData] =
        await Promise.all([
          invoke<QaCheckpoint[]>("qa_list_checkpoints", { sessionId }),
          invoke<QaCheckpointSummary[]>("qa_list_checkpoint_summaries", {
            sessionId,
          }),
          invoke<QaTestCase[]>("qa_list_test_cases", { sessionId }),
          invoke<QaLlmRun[]>("qa_list_llm_runs", { sessionId }),
        ]);
      setCheckpoints(checkpointData);
      setSummaries(summaryData);
      setTestCases(testCaseData);
      setLlmRuns(runData);
    } catch (err) {
      console.error(err);
      addToast("Failed to load AI outputs", "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!isTauriApp) return;
    void loadOutputs();
  }, [sessionId, isTauriApp]);

  const handleCreateCheckpoint = async () => {
    if (!sessionId) return;
    setActionCheckpoint("create");
    try {
      const newCheckpoint = await invoke<QaCheckpoint>("qa_create_checkpoint", {
        sessionId,
        title: manualTitle.trim() || null,
      });
      setManualTitle("");
      addToast("Checkpoint created", "success");
      await loadOutputs();
      // Auto-select new checkpoint
      setActiveCheckpointId(newCheckpoint.id);
    } catch (err) {
      console.error(err);
      addToast("Failed to create checkpoint", "error");
    } finally {
      setActionCheckpoint(null);
    }
  };

  const handleGenerateSummary = async (checkpointId: string) => {
    if (!sessionId) return;
    if (!canGenerate) {
      addToast("Select a valid model before generating", "error");
      return;
    }
    setActionCheckpoint(checkpointId);
    try {
      const config = buildConfig({
        maxTokens: 2048,
        temperature: 0.3,
        model: selectedModel,
      });
      await invoke<QaCheckpointSummary>("qa_generate_checkpoint_summary", {
        sessionId,
        checkpointId,
        config,
        outputLanguage: aiOutputLanguage,
      });
      addToast("Checkpoint summary generated", "success");
      await loadOutputs();
    } catch (err) {
      console.error(err);
      addToast("Failed to generate summary", "error");
    } finally {
      setActionCheckpoint(null);
    }
  };

  const handleGenerateTests = async (checkpointId: string) => {
    if (!sessionId) return;
    if (!canGenerate) {
      addToast("Select a valid model before generating", "error");
      return;
    }
    setActionCheckpoint(checkpointId);
    try {
      const config = buildConfig({
        maxTokens: 4096,
        temperature: 0.4,
        model: selectedModel,
      });
      await invoke<QaTestCase[]>("qa_generate_test_cases", {
        sessionId,
        checkpointId,
        config,
        outputLanguage: aiOutputLanguage,
      });
      addToast("AI test cases generated", "success");
      await loadOutputs();
    } catch (err) {
      console.error(err);
      addToast("Failed to generate test cases", "error");
    } finally {
      setActionCheckpoint(null);
    }
  };

  const activeCheckpoint = checkpoints.find((c) => c.id === activeCheckpointId);
  const activeSummary = activeCheckpoint
    ? summariesByCheckpoint[activeCheckpoint.id]
    : null;
  const activeTestCases = activeCheckpoint
    ? testCasesByCheckpoint[activeCheckpoint.id] || []
    : [];

  const groupedCases = useMemo(() => {
    const groups = {
      positive: [] as QaTestCase[],
      negative: [] as QaTestCase[],
      edge: [] as QaTestCase[],
      exploratory: [] as QaTestCase[],
    };
    activeTestCases.forEach((tc) => {
      // @ts-ignore - Dynamic key access
      if (groups[tc.type]) {
        // @ts-ignore
        groups[tc.type].push(tc);
      } else {
        groups.exploratory.push(tc);
      }
    });
    return groups;
  }, [activeTestCases]);

  return (
    <div className="flex flex-col h-full bg-app-bg text-app-text overflow-hidden">
      {/* Header & Controls - Fixed */}
      <div className="flex-none bg-app-bg border-b border-app-border z-10 shadow-sm">
        <div className="px-4 pt-4">
          <SessionDetailHeader
            session={session}
            sessionLoading={sessionLoading}
            sessionError={sessionError}
            isRecording={false}
            backLabel="Back to Session"
            onBack={() => navigate(`/qa/session/${sessionId}`)}
            onRetry={loadSession}
          />
        </div>

        {/* Compact Toolbar */}
        <div className="px-5 pt-6 pb-6 grid grid-cols-1 md:grid-cols-12 gap-4 items-center">
          <div className="md:col-span-3 flex items-center gap-2 text-xs text-app-text">
            <LayoutList className="w-4 h-4 text-emerald-400" />
            <span className="font-semibold">
              {checkpoints.length} Checkpoints
            </span>
          </div>

          <div className="md:col-span-9 flex flex-wrap items-center justify-end gap-3">
            {/* Model Select */}
            <div className="relative min-w-[180px]">
              <select
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                className="w-full bg-app-panel border border-app-border rounded-md py-1.5 pl-2.5 pr-8 text-xs appearance-none focus:border-emerald-500/50 transition outline-none text-app-text">
                {modelOptions.map((opt) => (
                  <option
                    key={opt}
                    value={opt}
                    className="bg-app-panel text-app-text">
                    {opt}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-2 w-3.5 h-3.5 text-app-subtext pointer-events-none" />
            </div>

            {/* Language Select */}
            <div className="relative min-w-[120px]">
              <select
                value={aiOutputLanguage}
                onChange={(e) => setAiOutputLanguage(e.target.value)}
                className="w-full bg-app-panel border border-app-border rounded-md py-1.5 pl-2.5 pr-8 text-xs appearance-none focus:border-emerald-500/50 transition outline-none text-app-text">
                {LANGUAGES.map((lang) => (
                  <option
                    key={lang}
                    value={lang}
                    className="bg-app-panel text-app-text">
                    {lang}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-2 w-3.5 h-3.5 text-app-subtext pointer-events-none" />
            </div>

            <div className="h-5 w-px bg-app-border mx-1" />

            <div className="flex items-center gap-2">
              <input
                value={manualTitle}
                onChange={(e) => setManualTitle(e.target.value)}
                placeholder="New checkpoint..."
                className="bg-black/20 border border-app-border rounded-md py-1.5 px-3 text-xs focus:border-emerald-500/50 transition outline-none text-app-text w-32 focus:w-48 duration-200"
              />
              <button
                onClick={handleCreateCheckpoint}
                disabled={actionCheckpoint === "create" || !sessionId}
                className="p-1.5 bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 rounded-md transition disabled:opacity-50"
                title="Add Checkpoint">
                <LayoutList className="w-4 h-4" />
              </button>
              <button
                onClick={loadOutputs}
                className="p-1.5 bg-app-panel hover:bg-app-panel/80 text-app-subtext hover:text-app-text border border-app-border rounded-md transition"
                title="Refresh">
                <RefreshCcw
                  className={`w-4 h-4 ${loading ? "animate-spin" : ""}`}
                />
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content - Split View */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 min-h-0 divide-y lg:divide-y-0 lg:divide-x divide-app-border/50">
        {/* LEFT: Checkpoint Timeline / Navigation */}
        <div className="lg:col-span-3 overflow-y-auto bg-black/10 custom-scrollbar">
          {checkpoints.length === 0 ? (
            <div className="p-8 text-center text-app-subtext text-xs italic opacity-60">
              No checkpoints recorded.
            </div>
          ) : (
            <div className="flex flex-col">
              {checkpoints.map((checkpoint) => {
                const isActive = checkpoint.id === activeCheckpointId;
                const hasSummary = !!summariesByCheckpoint[checkpoint.id];
                const testCount = (testCasesByCheckpoint[checkpoint.id] || [])
                  .length;

                return (
                  <button
                    key={checkpoint.id}
                    onClick={() => setActiveCheckpointId(checkpoint.id)}
                    className={`text-left p-4 border-b border-app-border/40 transition-all hover:bg-white/5 relative group
                          ${isActive ? "bg-white/5" : ""}
                        `}>
                    {isActive && (
                      <div className="absolute left-0 top-0 bottom-0 w-1 bg-emerald-500" />
                    )}
                    <div className="flex justify-between items-start mb-1">
                      <div className="flex items-center gap-2 font-mono text-[10px] text-app-subtext opacity-70">
                        <span>#{checkpoint.seq}</span>
                        <span>
                          {new Date(checkpoint.createdAt).toLocaleTimeString(
                            [],
                            { hour: "2-digit", minute: "2-digit" },
                          )}
                        </span>
                      </div>
                      <div className="flex gap-1">
                        {hasSummary && (
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-sm shadow-emerald-500/50" />
                        )}
                        {testCount > 0 && (
                          <div className="w-1.5 h-1.5 rounded-full bg-sky-500 shadow-sm shadow-sky-500/50" />
                        )}
                      </div>
                    </div>
                    <div
                      className={`text-sm font-medium line-clamp-2 mb-2 ${
                        isActive
                          ? "text-app-text"
                          : "text-app-subtext group-hover:text-app-text"
                      }`}>
                      {checkpoint.title || `Checkpoint ${checkpoint.seq}`}
                    </div>
                    <div className="flex items-center gap-2 text-[10px] text-app-subtext/60">
                      <span className="flex items-center gap-1 bg-black/20 px-1.5 py-0.5 rounded border border-white/5">
                        <Fingerprint className="w-3 h-3" />
                        {checkpoint.startEventSeq}-{checkpoint.endEventSeq}
                      </span>
                      {testCount > 0 && <span>{testCount} tests</span>}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* RIGHT: Active Detail View */}
        <div className="lg:col-span-9 overflow-y-auto bg-app-bg custom-scrollbar relative">
          {!activeCheckpoint ? (
            <div className="h-full flex flex-col items-center justify-center text-app-subtext p-8 opacity-60">
              <Layers className="w-12 h-12 mb-4 text-app-border" />
              <p>Select a checkpoint to view details</p>
            </div>
          ) : (
            <div className="p-6 max-w-5xl mx-auto space-y-8 animate-in fade-in duration-300 transform-gpu key={activeCheckpoint.id}">
              {/* Detail Header */}
              <div className="flex items-start justify-between gap-4 border-b border-app-border/40 pb-4">
                <div>
                  <div className="flex items-center gap-3 mb-1">
                    <span className="font-mono text-2xl font-light text-emerald-400 opacity-80">
                      #{activeCheckpoint.seq}
                    </span>
                    <h2 className="text-xl font-semibold text-app-text">
                      {activeCheckpoint.title || "Untitled Checkpoint"}
                    </h2>
                  </div>
                  <div className="flex items-center gap-3 text-xs text-app-subtext">
                    <span className="flex items-center gap-1.5">
                      <Clock className="w-3.5 h-3.5" />{" "}
                      {new Date(activeCheckpoint.createdAt).toLocaleString()}
                    </span>
                    <span className="w-1 h-1 rounded-full bg-app-border" />
                    <span className="flex items-center gap-1.5">
                      <Fingerprint className="w-3.5 h-3.5" /> Events{" "}
                      {activeCheckpoint.startEventSeq} -{" "}
                      {activeCheckpoint.endEventSeq}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  {!activeSummary && (
                    <button
                      onClick={() => handleGenerateSummary(activeCheckpoint.id)}
                      disabled={!!actionCheckpoint || !canGenerate}
                      className="text-xs px-3 py-2 bg-app-panel hover:bg-app-panel/80 border border-app-border rounded-lg text-app-text transition disabled:opacity-50 flex items-center gap-2">
                      <BookOpen className="w-3.5 h-3.5 text-emerald-400" />
                      {actionCheckpoint === activeCheckpoint.id
                        ? "Analyzing..."
                        : "Generate Summary"}
                    </button>
                  )}
                  <button
                    onClick={() => handleGenerateTests(activeCheckpoint.id)}
                    disabled={!!actionCheckpoint || !canGenerate}
                    className="text-xs px-3 py-2 bg-sky-500/10 hover:bg-sky-500/20 border border-sky-500/30 text-sky-400 rounded-lg transition disabled:opacity-50 flex items-center gap-2">
                    <Wand2 className="w-3.5 h-3.5" />
                    {actionCheckpoint === activeCheckpoint.id
                      ? "Generating..."
                      : "Generate Tests"}
                  </button>
                </div>
              </div>

              {/* Summary Card */}
              {activeSummary && (
                <section className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="text-sm font-semibold uppercase tracking-wider text-app-subtext flex items-center gap-2">
                      <BookOpen className="w-4 h-4 text-emerald-400" /> Analysis
                    </h3>
                    <span className="text-[10px] text-app-subtext bg-app-panel px-2 py-0.5 rounded border border-app-border">
                      Auto-generated
                    </span>
                  </div>
                  <div className="bg-app-card/60 rounded-xl border border-app-border p-5 shadow-sm transition-all hover:border-app-border/80">
                    <div className="text-sm text-app-text/90 leading-relaxed font-light">
                      <SummaryText text={activeSummary.summaryText} />
                    </div>
                    {(activeSummary.entitiesJson ||
                      activeSummary.risksJson) && (
                      <div className="flex flex-wrap gap-2 mt-4 pt-4 border-t border-app-border/30">
                        <SummaryMeta
                          label="Entities"
                          jsonValue={activeSummary.entitiesJson}
                          icon={<Layers className="w-3 h-3" />}
                        />
                        <SummaryMeta
                          label="Risks"
                          jsonValue={activeSummary.risksJson}
                          icon={
                            <AlertTriangle className="w-3 h-3 text-amber-400" />
                          }
                        />
                      </div>
                    )}
                  </div>
                </section>
              )}

              {/* Test Cases List */}
              <section className="space-y-4">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-semibold uppercase tracking-wider text-app-subtext flex items-center gap-2">
                    <CheckCircle2 className="w-4 h-4 text-sky-400" /> Generated
                    Tests
                    <span className="ml-1 text-app-subtext/60 bg-white/5 px-2 py-0.5 rounded-full text-xs">
                      {activeTestCases.length}
                    </span>
                  </h3>
                </div>

                {activeTestCases.length > 0 ? (
                  <div className="space-y-8 pb-10">
                    {groupedCases.positive.length > 0 && (
                      <div className="space-y-3">
                        <div className="flex items-center gap-2 pb-1 border-b border-app-border/30">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]"></div>
                          <h4 className="text-xs font-bold uppercase tracking-wider text-emerald-400 opacity-90">
                            Positive Scenarios
                          </h4>
                          <span className="text-[10px] text-app-subtext">
                            ({groupedCases.positive.length})
                          </span>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                          {groupedCases.positive.map((tc) => (
                            <TestCaseCard key={tc.id} testCase={tc} />
                          ))}
                        </div>
                      </div>
                    )}

                    {groupedCases.negative.length > 0 && (
                      <div className="space-y-3">
                        <div className="flex items-center gap-2 pb-1 border-b border-app-border/30">
                          <div className="w-1.5 h-1.5 rounded-full bg-rose-500 shadow-[0_0_8px_rgba(244,63,94,0.5)]"></div>
                          <h4 className="text-xs font-bold uppercase tracking-wider text-rose-400 opacity-90">
                            Negative Scenarios
                          </h4>
                          <span className="text-[10px] text-app-subtext">
                            ({groupedCases.negative.length})
                          </span>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                          {groupedCases.negative.map((tc) => (
                            <TestCaseCard key={tc.id} testCase={tc} />
                          ))}
                        </div>
                      </div>
                    )}

                    {groupedCases.edge.length > 0 && (
                      <div className="space-y-3">
                        <div className="flex items-center gap-2 pb-1 border-b border-app-border/30">
                          <div className="w-1.5 h-1.5 rounded-full bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.5)]"></div>
                          <h4 className="text-xs font-bold uppercase tracking-wider text-amber-400 opacity-90">
                            Edge Cases
                          </h4>
                          <span className="text-[10px] text-app-subtext">
                            ({groupedCases.edge.length})
                          </span>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                          {groupedCases.edge.map((tc) => (
                            <TestCaseCard key={tc.id} testCase={tc} />
                          ))}
                        </div>
                      </div>
                    )}

                    {groupedCases.exploratory.length > 0 && (
                      <div className="space-y-3">
                        <div className="flex items-center gap-2 pb-1 border-b border-app-border/30">
                          <div className="w-1.5 h-1.5 rounded-full bg-violet-500 shadow-[0_0_8px_rgba(139,92,246,0.5)]"></div>
                          <h4 className="text-xs font-bold uppercase tracking-wider text-violet-400 opacity-90">
                            Exploratory / Other
                          </h4>
                          <span className="text-[10px] text-app-subtext">
                            ({groupedCases.exploratory.length})
                          </span>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                          {groupedCases.exploratory.map((tc) => (
                            <TestCaseCard key={tc.id} testCase={tc} />
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center py-10 border-2 border-dashed border-app-border/40 rounded-xl bg-app-panel/20 text-center">
                    <ShieldAlert className="w-8 h-8 text-app-subtext/40 mb-3" />
                    <p className="text-sm text-app-subtext">
                      No test cases generated yet.
                    </p>
                    <p className="text-xs text-app-subtext/50 mt-1">
                      Use "Generate Tests" to create scenarios.
                    </p>
                  </div>
                )}
              </section>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function SummaryText({ text }: { text: string }) {
  const lines = text
    .split(/\n|•/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length === 0) return null;

  // If it's a short text, just show it
  if (lines.length === 1 && text.length < 200) {
    return <p>{text}</p>;
  }

  return (
    <ul className="space-y-1.5 list-none">
      {lines.map((line, index) => (
        <li key={index} className="flex gap-2 items-start opacity-90">
          <span className="mt-1.5 w-1 h-1 rounded-full bg-emerald-500/50 flex-none" />
          <span>{line}</span>
        </li>
      ))}
    </ul>
  );
}

function SummaryMeta({
  label,
  jsonValue,
  icon,
}: {
  label: string;
  jsonValue?: string | null;
  icon?: React.ReactNode;
}) {
  if (!jsonValue) return null;
  let items: string[] = [];
  try {
    const parsed = JSON.parse(jsonValue);
    if (Array.isArray(parsed)) {
      items = parsed.map((entry) => String(entry));
    }
  } catch {
    items = [];
  }
  if (items.length === 0) return null;
  return (
    <div className="inline-flex items-center gap-2 text-xs bg-app-panel px-2.5 py-1 rounded-md border border-app-border/50 text-app-subtext">
      {icon}
      <span className="font-medium text-app-text">{label}:</span>
      <span>{items.join(", ")}</span>
    </div>
  );
}

function TestCaseCard({ testCase }: { testCase: QaTestCase }) {
  const [expanded, setExpanded] = useState(false);

  const typeConfig = useMemo(() => {
    switch (testCase.type) {
      case "positive":
        return { color: "emerald", icon: CheckCircle2 };
      case "negative":
        return { color: "rose", icon: ShieldAlert };
      case "edge":
        return { color: "amber", icon: AlertTriangle };
      case "exploratory":
        return { color: "violet", icon: Search };
      default:
        return { color: "slate", icon: BookOpen };
    }
  }, [testCase.type]);

  const Icon = typeConfig.icon;
  // const borderColor = `border-${typeConfig.color}-500/20`;

  // Explicit inline styles or tailwind classes? Tailwind dynamic classes can be tricky if not safelisted.
  // Using generic classes combined with specific border colors is safer.
  let badgeClass = "";
  if (testCase.type === "positive")
    badgeClass = "bg-emerald-500/10 text-emerald-400 border-emerald-500/20";
  else if (testCase.type === "negative")
    badgeClass = "bg-rose-500/10 text-rose-400 border-rose-500/20";
  else if (testCase.type === "edge")
    badgeClass = "bg-amber-500/10 text-amber-400 border-amber-500/20";
  else if (testCase.type === "exploratory")
    badgeClass = "bg-violet-500/10 text-violet-400 border-violet-500/20";
  else badgeClass = "bg-slate-500/10 text-slate-400 border-slate-500/20";

  return (
    <div
      className={`rounded-xl border ${
        expanded
          ? "border-app-border shadow-md bg-black/40"
          : "border-app-border/40 bg-black/20"
      } hover:border-app-border/80 transition-all duration-200`}>
      <div
        className="p-3.5 flex gap-3 cursor-pointer items-start"
        onClick={() => setExpanded(!expanded)}>
        <div className={`mt-0.5 p-1.5 rounded-lg ${badgeClass} border`}>
          <Icon className="w-4 h-4" />
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <h6 className="text-sm font-medium text-app-text truncate pr-2">
              {testCase.title}
            </h6>
            <div className="flex items-center gap-2">
              {testCase.priority && (
                <span className="text-[10px] uppercase font-bold tracking-wider text-app-subtext/70 bg-white/5 px-1.5 py-0.5 rounded">
                  {testCase.priority}
                </span>
              )}
              {expanded ? (
                <ChevronDown className="w-4 h-4 text-app-subtext" />
              ) : (
                <ChevronRight className="w-4 h-4 text-app-subtext" />
              )}
            </div>
          </div>
          {!expanded && (
            <div className="text-xs text-app-subtext mt-1 flex gap-3 items-center">
              <span className="capitalize opacity-80">{testCase.type}</span>
              {testCase.expected && (
                <>
                  <span className="text-app-border">•</span>
                  <span className="opacity-50 truncate max-w-[400px]">
                    Expects: {testCase.expected}
                  </span>
                </>
              )}
            </div>
          )}
        </div>
      </div>

      {expanded && (
        <div className="px-5 pb-5 pt-1 animate-in slide-in-from-top-1 duration-200 pl-[3.5rem]">
          <div className="space-y-4 pt-2 border-t border-app-border/30">
            <div>
              <div className="text-[10px] uppercase tracking-wider text-app-subtext mb-2 font-semibold flex items-center gap-2">
                <LayoutList className="w-3 h-3" /> Test Steps
              </div>
              <TestSteps stepsJson={testCase.stepsJson} />
            </div>

            {testCase.expected && (
              <div className="bg-emerald-500/5 rounded-lg p-3 border border-emerald-500/10 flex gap-3 items-start">
                <CheckCircle2 className="w-4 h-4 text-emerald-500/70 mt-0.5 flex-none" />
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-emerald-500/70 mb-1 font-semibold">
                    Expected Result
                  </div>
                  <div className="text-sm text-app-text/90">
                    {testCase.expected}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function TestSteps({ stepsJson }: { stepsJson: string }) {
  let steps: string[] = [];
  try {
    const parsed = JSON.parse(stepsJson);
    if (Array.isArray(parsed)) {
      steps = parsed.map((entry) => String(entry));
    }
  } catch {
    steps = [];
  }

  if (steps.length === 0)
    return (
      <div className="text-xs text-app-subtext italic">No steps defined.</div>
    );

  return (
    <div className="space-y-2">
      {steps.map((step, i) => (
        <div
          key={i}
          className="flex gap-3 text-xs text-app-subtext group hover:text-app-text transition-colors">
          <span className="font-mono opacity-40 select-none w-5 text-right flex-none tabular-nums text-[11px] pt-px">
            {i + 1}.
          </span>
          <span className="leading-normal">{step}</span>
        </div>
      ))}
    </div>
  );
}
