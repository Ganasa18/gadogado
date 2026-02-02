import { AnimatePresence, motion } from "framer-motion";
import {
  AlertTriangle,
  ExternalLink,
  FileText,
  Target,
  TrendingUp,
  ChevronRight,
} from "lucide-react";
import type { AnalyticsEvent, AnalyticsSummary, ChunkWithQuality, DocumentQualityAnalysis, RagCollection, RagConfig, RagDocument } from "../../types";
import type { CollectionQualityMetrics, DocumentWarning, RetrievalGap } from "../../api";
import {
  fmtDateTime,
  fmtMaybeInt,
  fmtPct,
  gapLabel,
  gapSuggestion,
  qualityEmoji,
  shortHash,
} from "../../ragAnalyticsUtils";

type DocDetails = {
  loading: boolean;
  error: string | null;
  warnings: DocumentWarning[] | null;
  chunks: ChunkWithQuality[] | null;
  analysis: DocumentQualityAnalysis | null;
};

type Props = {
  selectedCollectionId: number | null;
  selectedCollection: RagCollection | null;
  lastSyncAt: number | null;
  onComputeMetrics: () => void;

  analyticsSummary: AnalyticsSummary | null;
  recentEvents: AnalyticsEvent[];

  qualityMetrics: CollectionQualityMetrics | null;
  aggregate: {
    docCount: number;
    warningTotal: number;
    lowQualityCount: number;
    missingOcrCount: number;
    gapCount: number;
    gapsByType: Record<string, number>;
  };
  actionableSuggestions: { title: string; why: string; action: string }[];
  ragConfig: RagConfig | null;

  filteredDocuments: RagDocument[];
  expandedDocIds: Set<number>;
  docDetailsById: Record<number, DocDetails>;
  onToggleDocExpanded: (docId: number) => void;
  onEnsureDocDetails: (docId: number) => void;

  retrievalGaps: RetrievalGap[];
};

export function RagAnalyticsMainPanel(props: Props) {
  const {
    selectedCollectionId,
    selectedCollection,
    lastSyncAt,
    onComputeMetrics,
    analyticsSummary,
    recentEvents,
    qualityMetrics,
    aggregate,
    actionableSuggestions,
    ragConfig,
    filteredDocuments,
    expandedDocIds,
    docDetailsById,
    onToggleDocExpanded,
    onEnsureDocDetails,
    retrievalGaps,
  } = props;

  return (
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
              onClick={onComputeMetrics}
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
              <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Global Telemetry</h2>
            </div>
            <div className="text-sm text-app-subtext">
              Select a collection to see per-document diagnostics and retrieval gaps.
            </div>
            <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Events</div>
                <div className="text-2xl font-mono font-bold text-app-text">
                  {analyticsSummary ? analyticsSummary.total_events : "—"}
                </div>
              </div>
              <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Retrievals</div>
                <div className="text-2xl font-mono font-bold text-app-text">
                  {analyticsSummary ? analyticsSummary.retrieval_count : "—"}
                </div>
              </div>
              <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Avg Retrieval</div>
                <div className="text-2xl font-mono font-bold text-app-text">
                  {analyticsSummary ? `${analyticsSummary.avg_retrieval_ms.toFixed(0)}ms` : "—"}
                </div>
              </div>
              <div className="bg-app-bg/30 rounded-lg border border-app-border p-4">
                <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Success</div>
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
                    <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Collection Quality</h2>
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
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Avg Doc Quality</div>
                  </div>
                  <div className="text-center md:border-l border-app-border">
                    <div className="text-3xl font-mono font-bold text-app-text mb-1">
                      {qualityEmoji(qualityMetrics?.avg_chunk_quality)} {fmtPct(qualityMetrics?.avg_chunk_quality)}
                    </div>
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Avg Chunk Quality</div>
                  </div>
                  <div className="text-center md:border-l border-app-border">
                    <div className="text-3xl font-mono font-bold text-app-text mb-1">
                      {qualityEmoji(qualityMetrics?.avg_ocr_confidence)} {fmtPct(qualityMetrics?.avg_ocr_confidence)}
                    </div>
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Avg OCR</div>
                  </div>
                  <div className="text-center md:border-l border-app-border">
                    <div className="text-3xl font-mono font-bold text-app-text mb-1">
                      {fmtMaybeInt(qualityMetrics?.documents_with_warnings)}
                    </div>
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Docs With Warnings</div>
                  </div>
                </div>

                <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="rounded-lg border border-app-border bg-app-bg/30 p-4">
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Best Reranker</div>
                    <div className="mt-1 text-sm text-app-text">
                      {qualityMetrics?.best_reranker ? (
                        <span className="font-mono">
                          {qualityMetrics.best_reranker} ({fmtPct(qualityMetrics.reranker_score)})
                        </span>
                      ) : (
                        <span className="text-app-subtext">—</span>
                      )}
                    </div>
                    <div className="mt-1 text-[10px] text-app-subtext">Note: per-document reranker metrics are not stored yet.</div>
                  </div>

                  <div className="rounded-lg border border-app-border bg-app-bg/30 p-4">
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Retrieval Gaps (from chat)</div>
                    <div className="mt-1 text-sm text-app-text font-mono">{aggregate.gapCount}</div>
                    <div className="mt-1 text-[10px] text-app-subtext">
                      no_results={aggregate.gapsByType.no_results ?? 0} • low_confidence={aggregate.gapsByType.low_confidence ?? 0} • partial_match={aggregate.gapsByType.partial_match ?? 0}
                    </div>
                  </div>
                </div>
              </div>

              <div className="bg-app-card rounded-xl border border-app-border p-6">
                <div className="flex items-center gap-2 mb-4">
                  <AlertTriangle className="w-4 h-4 text-amber-500" />
                  <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Actionable Next Steps</h2>
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
                    <div className="text-[10px] font-bold text-app-subtext uppercase tracking-wider">Current Chunking Config</div>
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
                  <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Document Diagnostics</h2>
                </div>
                <div className="text-xs text-app-subtext font-mono">
                  {filteredDocuments.length} docs • {aggregate.warningTotal} total warnings
                </div>
              </div>

              <div className="overflow-x-auto">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="border-b border-app-border bg-app-bg/50">
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Doc</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Quality</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">OCR</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden lg:table-cell">Chunks</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Warnings</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">Details</th>
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
                                    onClick={() => onToggleDocExpanded(doc.id)}
                                    className="p-1 rounded hover:bg-app-bg/60 transition-colors"
                                    title={expanded ? "Collapse" : "Expand"}>
                                    <motion.div animate={{ rotate: expanded ? 90 : 0 }} transition={{ duration: 0.15 }}>
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
                                  onClick={() => onToggleDocExpanded(doc.id)}
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
                                          <div className="text-xs font-bold text-app-text uppercase tracking-wider">Parsing Warnings</div>
                                          <button
                                            onClick={() => onEnsureDocDetails(doc.id)}
                                            className="text-[10px] text-app-subtext hover:text-app-text">
                                            Refresh
                                          </button>
                                        </div>

                                        {details?.loading ? (
                                          <div className="text-xs text-app-subtext animate-pulse">Loading warnings…</div>
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
                                              <div className="text-[10px] text-app-subtext">Showing 6 of {details.warnings.length} warnings.</div>
                                            )}
                                          </div>
                                        ) : (
                                          <div className="text-xs text-app-subtext">No warnings stored.</div>
                                        )}
                                      </div>

                                      <div className="rounded-lg border border-app-border bg-app-card p-4">
                                        <div className="flex items-center justify-between mb-3">
                                          <div className="text-xs font-bold text-app-text uppercase tracking-wider">Chunk Preview & Coherence</div>
                                          <div className="text-[10px] text-app-subtext">
                                            {details?.analysis ? `Extraction: ${details.analysis.extraction_quality}` : ""}
                                          </div>
                                        </div>

                                        {details?.loading ? (
                                          <div className="text-xs text-app-subtext animate-pulse">Loading chunks…</div>
                                        ) : details?.chunks && details.chunks.length > 0 ? (
                                          <div className="space-y-3">
                                            {details.chunks.map((c) => {
                                              const q = c.quality_score;
                                              const coherence = q >= 0.75 && c.has_embedding ? "good" : q >= 0.5 ? "ok" : "bad";
                                              const coherenceText =
                                                coherence === "good" ? "coherent" : coherence === "ok" ? "mixed" : "fragmented";

                                              return (
                                                <div key={c.chunk.id} className="rounded-lg border border-app-border bg-app-bg/20 p-3">
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
                                                    <span>{c.has_embedding ? "embedding=yes" : "embedding=no"}</span>
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

                                            <div className="text-[10px] text-app-subtext">Dev note: per-document reranker score is not stored yet.</div>
                                          </div>
                                        ) : (
                                          <div className="text-xs text-app-subtext">No chunks found for this document.</div>
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
                  <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Retrieval Gap Explorer</h2>
                </div>
                <div className="text-xs text-app-subtext">query hashes only (privacy-safe)</div>
              </div>

              <div className="overflow-x-auto">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="border-b border-app-border bg-app-bg/50">
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Type</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Query</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">Results</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">Confidence</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Suggestion</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">Time</th>
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
                          <td className="py-4 px-6 text-xs text-app-subtext font-mono">{gapLabel(g.gap_type)}</td>
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
                          <td className="py-4 px-6 text-xs text-app-subtext">{gapSuggestion(g.gap_type)}</td>
                          <td className="py-4 px-6 text-right text-xs text-app-subtext font-mono">{fmtDateTime(g.created_at)}</td>
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
                  <h2 className="text-sm font-bold text-app-text uppercase tracking-wider">Recent Retrieval Telemetry</h2>
                </div>
                <div className="text-xs text-app-subtext">hashed queries • source counts</div>
              </div>

              <div className="overflow-x-auto">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="border-b border-app-border bg-app-bg/50">
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Type</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider">Query</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">Sources</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider hidden md:table-cell">Confidence</th>
                      <th className="py-3 px-6 text-[10px] font-bold text-app-subtext uppercase tracking-wider text-right">Time</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-app-border">
                    {recentEvents.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="py-10 text-center text-sm text-app-subtext">No recent telemetry.</td>
                      </tr>
                    ) : (
                      recentEvents
                        .filter((e) => e.event_type === "retrieval" || e.event_type === "chat")
                        .slice(0, 20)
                        .map((e, idx) => (
                          <tr key={idx} className="hover:bg-app-bg/40 transition-colors">
                            <td className="py-4 px-6 text-xs text-app-subtext font-mono">{e.event_type}</td>
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
  );
}
