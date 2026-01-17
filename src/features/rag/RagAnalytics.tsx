import { useCallback, useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
  BarChart3,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  FileText,
  RefreshCw,
  Target,
  TrendingUp,
} from "lucide-react";
import type {
  AnalyticsEvent,
  AnalyticsSummary,
  ChunkWithQuality,
  DocumentQualityAnalysis,
  RagCollection,
  RagConfig,
  RagDocument,
} from "./types";
import {
  analyzeDocumentQuality,
  computeCollectionQuality,
  getAnalyticsSummary,
  getChunksWithQuality,
  getCollectionQuality,
  getDocumentWarnings,
  getLowQualityDocuments,
  getRecentAnalytics,
  getRetrievalGaps,
  getRagConfig,
  listRagCollections,
  listRagDocuments,
  type CollectionQualityMetrics,
  type DocumentWarning,
  type RetrievalGap,
} from "./api";

type QualityLevel = "good" | "ok" | "bad" | "unknown";

function qualityLevel(score: number | null | undefined): QualityLevel {
  if (score === null || score === undefined) return "unknown";
  if (score >= 0.75) return "good";
  if (score >= 0.5) return "ok";
  return "bad";
}

function qualityEmoji(score: number | null | undefined): string {
  const level = qualityLevel(score);
  if (level === "good") return "🟢";
  if (level === "ok") return "🟡";
  if (level === "bad") return "🔴";
  return "—";
}

function fmtPct(score: number | null | undefined): string {
  if (score === null || score === undefined) return "—";
  return `${(score * 100).toFixed(0)}%`;
}

function fmtMaybeInt(v: number | null | undefined): string {
  if (v === null || v === undefined) return "—";
  return `${v}`;
}

function fmtDateTime(dt: string): string {
  // dt is stored in DB as ISO-ish; keep it readable and non-sensitive
  const d = new Date(dt);
  if (Number.isNaN(d.getTime())) return dt;
  return d.toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function shortHash(hash: string): string {
  if (!hash) return "—";
  return hash.length <= 10 ? hash : `${hash.slice(0, 10)}…`;
}

function gapLabel(gapType: string | null | undefined): string {
  if (!gapType) return "unknown";
  return gapType.replace(/_/g, " ");
}

function gapSuggestion(gapType: string | null | undefined): string {
  switch (gapType) {
    case "no_results":
      return "No matching chunks retrieved. Likely missing coverage or wrong collection.";
    case "low_confidence":
      return "Retrieved chunks scored low. Improve OCR or chunk coherence.";
    case "partial_match":
      return "Some matches, but weak coverage. Add docs or adjust chunking settings.";
    default:
      return "Investigate query coverage and chunk quality.";
  }
}

type DocDetails = {
  loading: boolean;
  error: string | null;
  warnings: DocumentWarning[] | null;
  chunks: ChunkWithQuality[] | null;
  analysis: DocumentQualityAnalysis | null;
};

function emptyDocDetails(): DocDetails {
  return {
    loading: false,
    error: null,
    warnings: null,
    chunks: null,
    analysis: null,
  };
}

function sum(xs: number[]): number {
  return xs.reduce((a, b) => a + b, 0);
}

export default function RagAnalytics() {
  const [collections, setCollections] = useState<RagCollection[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(null);

  const [refreshing, setRefreshing] = useState(false);
  const [lastSyncAt, setLastSyncAt] = useState<number | null>(null);

  // Selected collection data
  const [documents, setDocuments] = useState<RagDocument[]>([]);
  const [qualityMetrics, setQualityMetrics] = useState<CollectionQualityMetrics | null>(null);
  const [lowQualityDocs, setLowQualityDocs] = useState<RagDocument[]>([]);
  const [retrievalGaps, setRetrievalGaps] = useState<RetrievalGap[]>([]);
  const [ragConfig, setRagConfig] = useState<RagConfig | null>(null);

  // Telemetry (useful for debugging gaps)
  const [analyticsSummary, setAnalyticsSummary] = useState<AnalyticsSummary | null>(null);
  const [recentEvents, setRecentEvents] = useState<AnalyticsEvent[]>([]);

  // Collection comparison map (loaded lazily, no spinner)
  const [collectionMetricsById, setCollectionMetricsById] = useState<
    Record<number, CollectionQualityMetrics | null | undefined>
  >({});

  // Per-document details (expand rows)
  const [expandedDocIds, setExpandedDocIds] = useState<Set<number>>(new Set());
  const [docDetailsById, setDocDetailsById] = useState<Record<number, DocDetails>>({});

  useEffect(() => {
    // Dev note: These are required UX items but not currently available through DB-backed metrics.
    // Keep message safe for logs.
    console.info(
      "[RagAnalytics] Dev note: per-document reranker score and per-document citation coverage are not exposed as DB metrics yet (collection-level only)."
    );
  }, []);

  const loadCollections = useCallback(async () => {
    try {
      const list = await listRagCollections(50);
      setCollections(list);

      // Keep current selection if still valid
      setSelectedCollectionId((prev) => {
        if (prev && list.some((c) => c.id === prev)) return prev;
        return prev ?? null;
      });

      // Opportunistically load stored quality metrics for comparison
      // (no ingestion logic, just reads; missing metrics show as "—")
      const ids = list.slice(0, 20).map((c) => c.id);
      const results = await Promise.all(
        ids.map(async (id) => ({ id, metrics: await getCollectionQuality(id) }))
      );
      setCollectionMetricsById((prev) => {
        const next = { ...prev };
        for (const r of results) next[r.id] = r.metrics;
        return next;
      });
    } catch (err) {
      console.error("Failed to list collections:", err);
      setCollections([]);
    }
  }, []);

  const loadSelectedCollection = useCallback(
    async (collectionId: number) => {
      try {
        const [docs, qm, lowQ, gaps, cfg, summary, events] = await Promise.all([
          listRagDocuments(collectionId, 200),
          getCollectionQuality(collectionId),
          getLowQualityDocuments(collectionId, 0.5, 20),
          getRetrievalGaps(collectionId, 200),
          getRagConfig(),
          getAnalyticsSummary(collectionId),
          getRecentAnalytics(30, collectionId),
        ]);

        setDocuments(docs);
        setQualityMetrics(qm);
        setLowQualityDocs(lowQ);
        setRetrievalGaps(gaps);
        setRagConfig(cfg);
        setAnalyticsSummary(summary);
        setRecentEvents(events);

        setLastSyncAt(Date.now());
      } catch (err) {
        console.error("Failed to load collection analytics:", err);
        // Keep previous UI (optimistic) but clear the most misleading pieces
        setRetrievalGaps([]);
      }
    },
    []
  );

  const loadGlobalTelemetry = useCallback(async () => {
    try {
      const [summary, events] = await Promise.all([
        getAnalyticsSummary(undefined),
        getRecentAnalytics(30, undefined),
      ]);
      setAnalyticsSummary(summary);
      setRecentEvents(events);
    } catch (err) {
      console.error("Failed to load global analytics:", err);
    }
  }, []);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await loadCollections();
      if (selectedCollectionId) {
        await loadSelectedCollection(selectedCollectionId);
      } else {
        await loadGlobalTelemetry();
        setDocuments([]);
        setQualityMetrics(null);
        setLowQualityDocs([]);
        setRetrievalGaps([]);
        setRagConfig(null);
      }
    } finally {
      setRefreshing(false);
    }
  }, [loadCollections, loadGlobalTelemetry, loadSelectedCollection, selectedCollectionId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!selectedCollectionId) return;
    void loadSelectedCollection(selectedCollectionId);
  }, [loadSelectedCollection, selectedCollectionId]);

  const ensureDocDetails = useCallback(async (docId: number) => {
    setDocDetailsById((prev) => ({
      ...prev,
      [docId]: {
        ...(prev[docId] ?? emptyDocDetails()),
        loading: true,
        error: null,
      },
    }));

    try {
      const [warnings, chunks, analysis] = await Promise.all([
        getDocumentWarnings(docId),
        getChunksWithQuality(docId, 6),
        analyzeDocumentQuality(docId),
      ]);

      setDocDetailsById((prev) => ({
        ...prev,
        [docId]: {
          loading: false,
          error: null,
          warnings,
          chunks,
          analysis,
        },
      }));
    } catch (err) {
      setDocDetailsById((prev) => ({
        ...prev,
        [docId]: {
          ...(prev[docId] ?? emptyDocDetails()),
          loading: false,
          error: err instanceof Error ? err.message : "Failed to load document details",
        },
      }));
    }
  }, []);

  const toggleDocExpanded = useCallback(
    (docId: number) => {
      setExpandedDocIds((prev) => {
        const next = new Set(prev);
        if (next.has(docId)) {
          next.delete(docId);
          return next;
        }
        next.add(docId);
        return next;
      });

      if (!docDetailsById[docId]?.warnings && !docDetailsById[docId]?.chunks) {
        void ensureDocDetails(docId);
      }
    },
    [docDetailsById, ensureDocDetails]
  );

  const selectedCollection = useMemo(() => {
    return collections.find((c) => c.id === selectedCollectionId) ?? null;
  }, [collections, selectedCollectionId]);

  const aggregate = useMemo(() => {
    const docs = documents;
    const warningTotal = sum(docs.map((d) => d.warning_count ?? 0));
    const lowQualityCount = docs.filter((d) => (d.quality_score ?? 1) < 0.5).length;
    const missingOcrCount = docs.filter((d) => d.file_type === "pdf" && d.ocr_confidence === null)
      .length;

    const gaps = retrievalGaps;
    const gapsByType: Record<string, number> = {};
    for (const g of gaps) {
      const t = g.gap_type ?? "unknown";
      gapsByType[t] = (gapsByType[t] ?? 0) + 1;
    }

    return {
      docCount: docs.length,
      warningTotal,
      lowQualityCount,
      missingOcrCount,
      gapCount: gaps.length,
      gapsByType,
    };
  }, [documents, retrievalGaps]);

  const actionableSuggestions = useMemo(() => {
    const out: { title: string; why: string; action: string }[] = [];

    if (!selectedCollectionId) {
      out.push({
        title: "Pick a collection to debug",
        why: "Import quality is collection-specific.",
        action: "Select a collection from the left panel to see document-level diagnostics.",
      });
      return out;
    }

    if (qualityMetrics?.documents_with_warnings && qualityMetrics.documents_with_warnings > 0) {
      out.push({
        title: "Review parsing warnings",
        why: `${qualityMetrics.documents_with_warnings} documents have warnings stored in DB.`,
        action: "Expand documents in the table and follow the per-warning suggestions.",
      });
    }

    if ((qualityMetrics?.avg_ocr_confidence ?? 1) < 0.7) {
      out.push({
        title: "OCR quality looks low",
        why: `Avg OCR confidence is ${fmtPct(qualityMetrics?.avg_ocr_confidence)}.`,
        action: "Re-import with OCR preprocessing enabled, or use OCR web mode for JS-heavy sites.",
      });
    }

    if (aggregate.gapCount > 0) {
      const noResults = aggregate.gapsByType.no_results ?? 0;
      out.push({
        title: "Retrieval gaps detected from chat",
        why: `${aggregate.gapCount} gap events recorded (no_results=${noResults}).`,
        action: "Open the gap list, then improve coverage by adding docs or tightening chunk boundaries.",
      });
    }

    if (ragConfig) {
      const cs = ragConfig.chunking.chunk_size;
      const overlap = ragConfig.chunking.overlap;
      const minQ = ragConfig.chunking.min_quality_score;
      if (cs < 450) {
        out.push({
          title: "Chunk size may be too small",
          why: `chunk_size=${cs} can produce fragmented chunks.`,
          action: "Try a larger chunk_size (e.g. 700-900) for prose-heavy PDFs.",
        });
      }
      if (overlap === 0) {
        out.push({
          title: "Zero overlap can create context breaks",
          why: "overlap=0 often hurts continuity across paragraph boundaries.",
          action: "Try overlap 50-120 if your docs contain long explanations.",
        });
      }
      if (minQ > 0.75) {
        out.push({
          title: "Min quality threshold is strict",
          why: `minQualityScore=${minQ} may drop borderline-but-useful chunks.`,
          action: "Lower min_quality_score if you see many no_results gaps.",
        });
      }
    }

    if (!qualityMetrics?.best_reranker) {
      out.push({
        title: "Reranker performance not recorded",
        why: "Collection metrics do not include a best reranker yet.",
        action: "Dev note: record reranker_name + score per import run and per document.",
      });
    }

    return out.slice(0, 6);
  }, [aggregate.gapCount, aggregate.gapsByType, qualityMetrics, ragConfig, selectedCollectionId]);

  const computeQualityNow = useCallback(async () => {
    if (!selectedCollectionId) return;
    // Optimistic UI: keep old metrics until new arrives.
    try {
      const metrics = await computeCollectionQuality(selectedCollectionId);
      setQualityMetrics(metrics);
      setCollectionMetricsById((prev) => ({ ...prev, [selectedCollectionId]: metrics }));
      setLastSyncAt(Date.now());
    } catch (err) {
      console.error("Failed to compute collection quality metrics:", err);
    }
  }, [selectedCollectionId]);

  const filteredDocuments = useMemo(() => {
    const docs = documents.slice();
    docs.sort((a, b) => (b.warning_count ?? 0) - (a.warning_count ?? 0));
    return docs;
  }, [documents]);

  return (
    <div className="flex h-full bg-app-bg text-app-text font-sans overflow-hidden">
      <aside className="w-[340px] flex-shrink-0 bg-app-panel border-r border-app-border flex flex-col">
        <div className="h-16 flex items-center justify-between px-5 border-b border-app-border">
          <div className="flex items-center gap-2 text-app-text font-bold tracking-tight">
            <BarChart3 className="w-5 h-5 text-emerald-500" />
            <span>Import Quality Debugger</span>
          </div>
          <button
            onClick={refresh}
            disabled={refreshing}
            className="flex items-center gap-2 text-xs text-app-subtext hover:text-app-text disabled:opacity-50">
            <RefreshCw className="w-4 h-4" />
            {refreshing ? "Refreshing…" : "Refresh"}
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-6">
          <div className="space-y-2">
            <label className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
              Workspace (Collection)
            </label>
            <div className="relative group">
              <select
                value={selectedCollectionId ?? ""}
                onChange={(e) =>
                  setSelectedCollectionId(e.target.value ? parseInt(e.target.value) : null)
                }
                className="w-full appearance-none bg-app-card border border-app-border text-app-text text-sm rounded-lg px-4 py-2.5 outline-none focus:border-emerald-500/50 transition-colors cursor-pointer">
                <option value="">All collections (read-only)</option>
                {collections.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-3 top-3 w-4 h-4 text-app-subtext pointer-events-none group-hover:text-emerald-500 transition-colors" />
            </div>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                Collection Comparison
              </label>
              <span className="text-[10px] text-app-subtext">stored metrics</span>
            </div>

            <div className="bg-app-card rounded-xl border border-app-border overflow-hidden">
              <div className="overflow-x-auto">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="border-b border-app-border bg-app-bg/50">
                      <th className="py-2.5 px-4 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Collection
                      </th>
                      <th className="py-2.5 px-4 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Quality
                      </th>
                      <th className="py-2.5 px-4 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">
                        Warnings
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-app-border">
                    {collections.length === 0 ? (
                      <tr>
                        <td colSpan={3} className="py-6 px-4 text-xs text-app-subtext">
                          No collections found.
                        </td>
                      </tr>
                    ) : (
                      collections.slice(0, 20).map((c) => {
                        const m = collectionMetricsById[c.id];
                        const isSelected = c.id === selectedCollectionId;
                        return (
                          <tr
                            key={c.id}
                            onClick={() => setSelectedCollectionId(c.id)}
                            className={`cursor-pointer hover:bg-app-bg/40 transition-colors ${
                              isSelected ? "bg-emerald-500/10" : ""
                            }`}>
                            <td className="py-3 px-4 text-xs font-medium text-app-text">
                              <div className="truncate max-w-[170px]" title={c.name}>
                                {c.name}
                              </div>
                            </td>
                            <td className="py-3 px-4 text-xs text-app-subtext font-mono">
                              {m === undefined ? (
                                <span className="text-app-subtext/60">loading…</span>
                              ) : m === null ? (
                                <span className="text-app-subtext/60">—</span>
                              ) : (
                                <span>
                                  {qualityEmoji(m.avg_quality_score)} {fmtPct(m.avg_quality_score)}
                                </span>
                              )}
                            </td>
                            <td className="py-3 px-4 text-xs text-app-subtext font-mono text-right">
                              {m && m !== null ? m.documents_with_warnings : "—"}
                            </td>
                          </tr>
                        );
                      })
                    )}
                  </tbody>
                </table>
              </div>
            </div>

            <div className="text-[10px] text-app-subtext leading-relaxed">
              Tip: metrics are stored snapshots. Use “Compute Metrics” in the main panel to refresh.
            </div>
          </div>

          {selectedCollectionId && (
            <div className="space-y-2">
              <label className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                Quick Triage
              </label>
              <div className="space-y-2">
                {lowQualityDocs.length === 0 ? (
                  <div className="p-3 rounded-lg border border-app-border bg-app-card/40 text-xs text-app-subtext">
                    No low-quality docs (threshold 50%).
                  </div>
                ) : (
                  lowQualityDocs.slice(0, 8).map((doc) => (
                    <button
                      key={doc.id}
                      onClick={() => toggleDocExpanded(doc.id)}
                      className="w-full text-left p-3 rounded-lg border border-app-border bg-app-card hover:border-amber-500/30 transition-colors">
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex items-center gap-2 min-w-0">
                          <FileText className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                          <span className="text-xs font-medium truncate">{doc.file_name}</span>
                        </div>
                        <span className="text-xs font-mono text-app-subtext shrink-0">
                          {qualityEmoji(doc.quality_score)} {fmtPct(doc.quality_score)}
                        </span>
                      </div>
                      {doc.warning_count > 0 && (
                        <div className="mt-1 text-[10px] text-amber-500/80">
                          {doc.warning_count} warning{doc.warning_count > 1 ? "s" : ""}
                        </div>
                      )}
                    </button>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <header className="h-20 border-b border-app-border px-8 flex items-center justify-between flex-shrink-0">
          <div className="min-w-0">
            <div className="flex items-center gap-3">
              <div className="w-1 h-6 bg-emerald-500 rounded-full" />
              <h1 className="text-xl font-bold text-app-text truncate">
                {selectedCollection ? selectedCollection.name : "Import Quality"}
              </h1>
            </div>
            <div className="text-xs text-app-subtext mt-1 pl-4">
              {lastSyncAt
                ? `Last sync ${new Date(lastSyncAt).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  })}`
                : "Loading…"}
            </div>
          </div>

          {selectedCollectionId && (
            <div className="flex items-center gap-3">
              <button
                onClick={computeQualityNow}
                className="px-3 py-2 text-xs bg-emerald-500/10 text-emerald-500 rounded-lg hover:bg-emerald-500/20 transition-colors">
                Compute Metrics
              </button>
              <a
                href="/rag-chat"
                className="px-3 py-2 text-xs bg-app-card border border-app-border rounded-lg text-app-subtext hover:text-app-text hover:border-emerald-500/30 transition-colors flex items-center gap-2">
                <ExternalLink className="w-4 h-4" />
                Open Chat
              </a>
            </div>
          )}
        </header>

        <div className="flex-1 overflow-y-auto p-8 space-y-6 bg-app-bg">
          {!selectedCollectionId ? (
            <div className="bg-app-card rounded-xl border border-app-border p-6">
              <div className="flex items-center gap-2 mb-2">
                <TrendingUp className="w-4 h-4 text-app-subtext" />
                <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                  Global Telemetry
                </h2>
              </div>
              <div className="text-sm text-app-subtext">
                Select a collection to see per-document diagnostics and retrieval gaps.
              </div>
              <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                  <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                    Events
                  </div>
                  <div className="text-2xl font-mono font-bold text-app-text">
                    {analyticsSummary ? analyticsSummary.total_events : "—"}
                  </div>
                </div>
                <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                  <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                    Retrievals
                  </div>
                  <div className="text-2xl font-mono font-bold text-app-text">
                    {analyticsSummary ? analyticsSummary.retrieval_count : "—"}
                  </div>
                </div>
                <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                  <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                    Avg Retrieval
                  </div>
                  <div className="text-2xl font-mono font-bold text-app-text">
                    {analyticsSummary ? `${analyticsSummary.avg_retrieval_ms.toFixed(0)}ms` : "—"}
                  </div>
                </div>
                <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                  <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                    Success
                  </div>
                  <div className="text-2xl font-mono font-bold text-app-text">
                    {analyticsSummary ? fmtPct(analyticsSummary.success_rate) : "—"}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <>
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div className="lg:col-span-2 bg-app-card rounded-xl border border-app-border p-6">
                  <div className="flex items-center justify-between mb-6">
                    <div className="flex items-center gap-2">
                      <Target className="w-4 h-4 text-emerald-500" />
                      <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                        Collection Quality
                      </h2>
                    </div>
                    <div className="text-xs text-app-subtext">
                      {qualityMetrics ? `Computed ${fmtDateTime(qualityMetrics.computed_at)}` : "Not computed"}
                    </div>
                  </div>

                  <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
                    <div className="text-center">
                      <div className="text-3xl font-mono font-bold text-app-text mb-1">
                        {qualityEmoji(qualityMetrics?.avg_quality_score)} {fmtPct(qualityMetrics?.avg_quality_score)}
                      </div>
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Avg Doc Quality
                      </div>
                    </div>
                    <div className="text-center md:border-l border-app-border">
                      <div className="text-3xl font-mono font-bold text-app-text mb-1">
                        {qualityEmoji(qualityMetrics?.avg_chunk_quality)} {fmtPct(qualityMetrics?.avg_chunk_quality)}
                      </div>
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Avg Chunk Quality
                      </div>
                    </div>
                    <div className="text-center md:border-l border-app-border">
                      <div className="text-3xl font-mono font-bold text-app-text mb-1">
                        {qualityEmoji(qualityMetrics?.avg_ocr_confidence)} {fmtPct(qualityMetrics?.avg_ocr_confidence)}
                      </div>
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Avg OCR
                      </div>
                    </div>
                    <div className="text-center md:border-l border-app-border">
                      <div className="text-3xl font-mono font-bold text-app-text mb-1">
                        {fmtMaybeInt(qualityMetrics?.documents_with_warnings)}
                      </div>
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Docs With Warnings
                      </div>
                    </div>
                  </div>

                  <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="rounded-lg border border-app-border bg-app-bg/30 p-4">
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Best Reranker
                      </div>
                      <div className="mt-1 text-sm text-app-text">
                        {qualityMetrics?.best_reranker ? (
                          <span className="font-mono">
                            {qualityMetrics.best_reranker} ({fmtPct(qualityMetrics.reranker_score)})
                          </span>
                        ) : (
                          <span className="text-app-subtext">—</span>
                        )}
                      </div>
                      <div className="mt-1 text-[10px] text-app-subtext">
                        Note: per-document reranker metrics are not stored yet.
                      </div>
                    </div>

                    <div className="rounded-lg border border-app-border bg-app-bg/30 p-4">
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Retrieval Gaps (from chat)
                      </div>
                      <div className="mt-1 text-sm text-app-text font-mono">
                        {aggregate.gapCount}
                      </div>
                      <div className="mt-1 text-[10px] text-app-subtext">
                        no_results={aggregate.gapsByType.no_results ?? 0} • low_confidence={aggregate.gapsByType.low_confidence ?? 0} • partial_match={aggregate.gapsByType.partial_match ?? 0}
                      </div>
                    </div>
                  </div>
                </div>

                <div className="bg-app-card rounded-xl border border-app-border p-6">
                  <div className="flex items-center gap-2 mb-4">
                    <AlertTriangle className="w-4 h-4 text-amber-500" />
                    <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                      Actionable Next Steps
                    </h2>
                  </div>

                  {actionableSuggestions.length === 0 ? (
                    <div className="text-sm text-app-subtext">No suggestions right now.</div>
                  ) : (
                    <div className="space-y-3">
                      {actionableSuggestions.map((s, idx) => (
                        <div key={idx} className="rounded-lg border border-app-border bg-app-bg/30 p-3">
                          <div className="text-xs font-semibold text-app-text">{s.title}</div>
                          <div className="mt-1 text-[11px] text-app-subtext">{s.why}</div>
                          <div className="mt-2 text-[11px] text-app-subtext">
                            <span className="font-semibold text-app-text">Do:</span> {s.action}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}

                  {ragConfig && (
                    <div className="mt-5 rounded-lg border border-app-border bg-app-bg/30 p-3">
                      <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                        Current Chunking Config
                      </div>
                      <div className="mt-2 text-[11px] text-app-subtext font-mono">
                        chunk_size={ragConfig.chunking.chunk_size} • overlap={ragConfig.chunking.overlap} • minQualityScore={ragConfig.chunking.min_quality_score}
                      </div>
                    </div>
                  )}
                </div>
              </div>

              <div className="bg-app-card rounded-xl border border-app-border overflow-hidden">
                <div className="px-6 py-4 border-b border-app-border flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="w-4 h-4 text-app-subtext" />
                    <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                      Document Diagnostics
                    </h2>
                  </div>
                  <div className="text-xs text-app-subtext font-mono">
                    {documents.length} docs • {aggregate.warningTotal} total warnings
                  </div>
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full text-left border-collapse">
                    <thead>
                      <tr className="border-b border-app-border bg-app-bg/50">
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Doc
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Quality
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">
                          OCR
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden lg:table-cell">
                          Chunks
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Warnings
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">
                          Details
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-app-border">
                      {filteredDocuments.length === 0 ? (
                        <tr>
                          <td colSpan={6} className="py-10 text-center text-sm text-app-subtext">
                            No documents found in this collection.
                          </td>
                        </tr>
                      ) : (
                        filteredDocuments.map((doc) => {
                          const expanded = expandedDocIds.has(doc.id);
                          const details = docDetailsById[doc.id];

                          return (
                            <>
                              <tr key={doc.id} className="hover:bg-app-bg/40 transition-colors">
                                <td className="py-4 px-6">
                                  <div className="flex items-center gap-3 min-w-0">
                                    <button
                                      onClick={() => toggleDocExpanded(doc.id)}
                                      className="p-1 rounded hover:bg-app-bg/60 transition-colors"
                                      title={expanded ? "Collapse" : "Expand"}>
                                      <motion.div
                                        animate={{ rotate: expanded ? 90 : 0 }}
                                        transition={{ duration: 0.15 }}>
                                        <ChevronRight className="w-4 h-4 text-app-subtext" />
                                      </motion.div>
                                    </button>
                                    <div className="min-w-0">
                                      <div className="text-xs font-medium text-app-text truncate" title={doc.file_name}>
                                        {doc.file_name}
                                      </div>
                                      <div className="text-[10px] text-app-subtext">
                                        {doc.file_type.toUpperCase()} • {doc.total_pages} pages
                                      </div>
                                    </div>
                                  </div>
                                </td>

                                <td className="py-4 px-6 text-xs font-mono text-app-subtext">
                                  {qualityEmoji(doc.quality_score)} {fmtPct(doc.quality_score)}
                                </td>

                                <td className="py-4 px-6 text-xs font-mono text-app-subtext hidden md:table-cell">
                                  {qualityEmoji(doc.ocr_confidence)} {fmtPct(doc.ocr_confidence)}
                                </td>

                                <td className="py-4 px-6 text-xs font-mono text-app-subtext hidden lg:table-cell">
                                  {doc.chunk_count}
                                </td>

                                <td className="py-4 px-6 text-xs font-mono text-app-subtext">
                                  {doc.warning_count > 0 ? (
                                    <span className="text-amber-500">{doc.warning_count}</span>
                                  ) : (
                                    <span className="text-emerald-500">0</span>
                                  )}
                                </td>

                                <td className="py-4 px-6 text-right">
                                  <button
                                    onClick={() => toggleDocExpanded(doc.id)}
                                    className="text-xs text-app-subtext hover:text-app-text transition-colors">
                                    {expanded ? "Hide" : "Show"}
                                  </button>
                                </td>
                              </tr>

                              <AnimatePresence>
                                {expanded && (
                                  <motion.tr
                                    initial={{ opacity: 0 }}
                                    animate={{ opacity: 1 }}
                                    exit={{ opacity: 0 }}
                                    className="bg-app-bg/20">
                                    <td colSpan={6} className="px-6 py-5">
                                      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
                                        <div className="rounded-lg border border-app-border bg-app-card p-4">
                                          <div className="flex items-center justify-between mb-3">
                                            <div className="text-xs font-bold text-app-text uppercase tracking-wider">
                                              Parsing Warnings
                                            </div>
                                            <button
                                              onClick={() => ensureDocDetails(doc.id)}
                                              className="text-[10px] text-app-subtext hover:text-app-text">
                                              Refresh
                                            </button>
                                          </div>

                                          {details?.loading ? (
                                            <div className="text-xs text-app-subtext animate-pulse">
                                              Loading warnings…
                                            </div>
                                          ) : details?.error ? (
                                            <div className="text-xs text-red-400">{details.error}</div>
                                          ) : details?.warnings && details.warnings.length > 0 ? (
                                            <div className="space-y-2">
                                              {details.warnings.slice(0, 6).map((w) => (
                                                <div
                                                  key={w.id}
                                                  className={`p-3 rounded-lg border ${
                                                    w.severity === "error"
                                                      ? "border-red-500/30 bg-red-500/5"
                                                      : w.severity === "warning"
                                                        ? "border-amber-500/30 bg-amber-500/5"
                                                        : "border-blue-500/30 bg-blue-500/5"
                                                  }`}>
                                                  <div className="flex items-center justify-between gap-3 mb-1">
                                                    <div className="text-[10px] font-bold uppercase tracking-wider text-app-subtext">
                                                      {w.warning_type.replace(/_/g, " ")}
                                                    </div>
                                                    <div className="text-[10px] text-app-subtext font-mono">
                                                      {w.page_number ? `p${w.page_number}` : ""}
                                                      {w.chunk_index !== null && w.chunk_index !== undefined
                                                        ? ` • c${w.chunk_index}`
                                                        : ""}
                                                    </div>
                                                  </div>
                                                  <div className="text-xs text-app-text">{w.message}</div>
                                                  {w.suggestion && (
                                                    <div className="mt-1 text-[11px] text-app-subtext">
                                                      <span className="font-semibold text-app-text">Suggestion:</span> {w.suggestion}
                                                    </div>
                                                  )}
                                                </div>
                                              ))}
                                              {details.warnings.length > 6 && (
                                                <div className="text-[10px] text-app-subtext">
                                                  Showing 6 of {details.warnings.length} warnings.
                                                </div>
                                              )}
                                            </div>
                                          ) : (
                                            <div className="text-xs text-app-subtext">No warnings stored.</div>
                                          )}
                                        </div>

                                        <div className="rounded-lg border border-app-border bg-app-card p-4">
                                          <div className="flex items-center justify-between mb-3">
                                            <div className="text-xs font-bold text-app-text uppercase tracking-wider">
                                              Chunk Preview & Coherence
                                            </div>
                                            <div className="text-[10px] text-app-subtext">
                                              {details?.analysis ? `Extraction: ${details.analysis.extraction_quality}` : ""}
                                            </div>
                                          </div>

                                          {details?.loading ? (
                                            <div className="text-xs text-app-subtext animate-pulse">
                                              Loading chunks…
                                            </div>
                                          ) : details?.chunks && details.chunks.length > 0 ? (
                                            <div className="space-y-3">
                                              {details.chunks.map((c) => {
                                                const q = c.quality_score;
                                                const coherence =
                                                  q >= 0.75 && c.has_embedding ? "good" : q >= 0.5 ? "ok" : "bad";
                                                const coherenceText =
                                                  coherence === "good" ? "coherent" : coherence === "ok" ? "mixed" : "fragmented";
                                                return (
                                                  <div
                                                    key={c.chunk.id}
                                                    className="rounded-lg border border-app-border bg-app-bg/20 p-3">
                                                    <div className="flex items-center justify-between gap-3">
                                                      <div className="text-[10px] text-app-subtext font-mono">
                                                        chunk#{c.chunk.chunk_index + 1}
                                                        {c.chunk.page_number ? ` • page ${c.chunk.page_number}` : ""}
                                                        {c.chunk.content_type ? ` • ${c.chunk.content_type}` : ""}
                                                      </div>
                                                      <div className="text-[10px] text-app-subtext font-mono">
                                                        {qualityEmoji(q)} {fmtPct(q)} • {coherenceText}
                                                      </div>
                                                    </div>

                                                    <div className="mt-2 text-xs text-app-text whitespace-pre-wrap line-clamp-3">
                                                      {c.chunk.content}
                                                    </div>

                                                    <div className="mt-2 flex items-center justify-between text-[10px] text-app-subtext font-mono">
                                                      <span>
                                                        id={c.chunk.id} • ~{c.token_estimate} tok
                                                      </span>
                                                      <span>
                                                        {c.has_embedding ? "embedding=yes" : "embedding=no"}
                                                      </span>
                                                    </div>
                                                  </div>
                                                );
                                              })}

                                              {details.analysis && details.analysis.issues.length > 0 && (
                                                <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3">
                                                  <div className="text-[10px] font-bold uppercase tracking-wider text-amber-500 mb-1">
                                                    Extraction Issues
                                                  </div>
                                                  <div className="text-xs text-app-subtext">
                                                    {details.analysis.issues.slice(0, 5).join(" • ")}
                                                  </div>
                                                </div>
                                              )}

                                              <div className="text-[10px] text-app-subtext">
                                                Dev note: per-document reranker score is not stored yet.
                                              </div>
                                            </div>
                                          ) : (
                                            <div className="text-xs text-app-subtext">
                                              No chunks found for this document.
                                            </div>
                                          )}
                                        </div>
                                      </div>
                                    </td>
                                  </motion.tr>
                                )}
                              </AnimatePresence>
                            </>
                          );
                        })
                      )}
                    </tbody>
                  </table>
                </div>
              </div>

              <div className="bg-app-card rounded-xl border border-app-border overflow-hidden">
                <div className="px-6 py-4 border-b border-app-border flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="w-4 h-4 text-blue-400" />
                    <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                      Retrieval Gap Explorer
                    </h2>
                  </div>
                  <div className="text-xs text-app-subtext">
                    query hashes only (privacy-safe)
                  </div>
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full text-left border-collapse">
                    <thead>
                      <tr className="border-b border-app-border bg-app-bg/50">
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Type
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Query
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">
                          Results
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">
                          Confidence
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Suggestion
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">
                          Time
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-app-border">
                      {retrievalGaps.length === 0 ? (
                        <tr>
                          <td colSpan={6} className="py-10 text-center text-sm text-app-subtext">
                            No retrieval gaps recorded yet.
                          </td>
                        </tr>
                      ) : (
                        retrievalGaps.slice(0, 50).map((g) => (
                          <tr key={g.id} className="hover:bg-app-bg/40 transition-colors">
                            <td className="py-4 px-6 text-xs text-app-subtext font-mono">
                              {gapLabel(g.gap_type)}
                            </td>
                            <td className="py-4 px-6 text-xs text-app-subtext font-mono">
                              {shortHash(g.query_hash)}
                              <span className="text-app-subtext/60"> • len={fmtMaybeInt(g.query_length)}</span>
                            </td>
                            <td className="py-4 px-6 text-xs text-app-subtext font-mono hidden md:table-cell">
                              {fmtMaybeInt(g.result_count)}
                            </td>
                            <td className="py-4 px-6 text-xs text-app-subtext font-mono hidden md:table-cell">
                              max={fmtPct(g.max_confidence)} • avg={fmtPct(g.avg_confidence)}
                            </td>
                            <td className="py-4 px-6 text-xs text-app-subtext">
                              {gapSuggestion(g.gap_type)}
                            </td>
                            <td className="py-4 px-6 text-right text-xs text-app-subtext font-mono">
                              {fmtDateTime(g.created_at)}
                            </td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>

              <div className="bg-app-card rounded-xl border border-app-border overflow-hidden">
                <div className="px-6 py-4 border-b border-app-border flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <TrendingUp className="w-4 h-4 text-app-subtext" />
                    <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">
                      Recent Retrieval Telemetry
                    </h2>
                  </div>
                  <div className="text-xs text-app-subtext">hashed queries • source counts</div>
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full text-left border-collapse">
                    <thead>
                      <tr className="border-b border-app-border bg-app-bg/50">
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Type
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
                          Query
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">
                          Sources
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">
                          Confidence
                        </th>
                        <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">
                          Time
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-app-border">
                      {recentEvents.length === 0 ? (
                        <tr>
                          <td colSpan={5} className="py-10 text-center text-sm text-app-subtext">
                            No recent telemetry.
                          </td>
                        </tr>
                      ) : (
                        recentEvents
                          .filter((e) => e.event_type === "retrieval" || e.event_type === "chat")
                          .slice(0, 20)
                          .map((e, idx) => (
                            <tr key={idx} className="hover:bg-app-bg/40 transition-colors">
                              <td className="py-4 px-6 text-xs text-app-subtext font-mono">
                                {e.event_type}
                              </td>
                              <td className="py-4 px-6 text-xs text-app-subtext font-mono">
                                {e.metadata.query_hash ? shortHash(e.metadata.query_hash) : "—"}
                                {e.metadata.query_length ? (
                                  <span className="text-app-subtext/60"> • len={e.metadata.query_length}</span>
                                ) : null}
                              </td>
                              <td className="py-4 px-6 text-xs text-app-subtext font-mono hidden md:table-cell">
                                {e.metadata.sources ?? "—"}
                              </td>
                              <td className="py-4 px-6 text-xs text-app-subtext font-mono hidden md:table-cell">
                                {e.metadata.confidence !== undefined && e.metadata.confidence !== null
                                  ? fmtPct(e.metadata.confidence)
                                  : "—"}
                              </td>
                              <td className="py-4 px-6 text-right text-xs text-app-subtext font-mono">
                                {new Date(e.timestamp_ms).toLocaleTimeString([], {
                                  hour12: false,
                                  hour: "2-digit",
                                  minute: "2-digit",
                                  second: "2-digit",
                                })}
                              </td>
                            </tr>
                          ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            </>
          )}
        </div>
      </main>
    </div>
  );
}
