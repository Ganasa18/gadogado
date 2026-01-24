import {
  BarChart3,
  ChevronDown,
  FileText,
  RefreshCw,
} from "lucide-react";
import { qualityEmoji, fmtPct } from "../../ragAnalyticsUtils";
import type { RagCollection, RagDocument } from "../../types";
import type { CollectionQualityMetrics } from "../../api";

type Props = {
  collections: RagCollection[];
  selectedCollectionId: number | null;
  onSelectCollectionId: (id: number | null) => void;
  refreshing: boolean;
  onRefresh: () => void;
  collectionMetricsById: Record<number, CollectionQualityMetrics | null | undefined>;
  lowQualityDocs: RagDocument[];
  onToggleDocExpanded: (docId: number) => void;
};

export function RagAnalyticsSidebar(props: Props) {
  const {
    collections,
    selectedCollectionId,
    onSelectCollectionId,
    refreshing,
    onRefresh,
    collectionMetricsById,
    lowQualityDocs,
    onToggleDocExpanded,
  } = props;

  return (
    <aside className="w-[340px] flex-shrink-0 bg-app-panel border-r border-app-border flex flex-col">
      <div className="h-16 flex items-center justify-between px-5 border-b border-app-border">
        <div className="flex items-center gap-2 text-app-text font-bold tracking-tight">
          <BarChart3 className="w-5 h-5 text-emerald-500" />
          <span>Import Quality Debugger</span>
        </div>
        <button
          onClick={onRefresh}
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
              onChange={(e) => onSelectCollectionId(e.target.value ? parseInt(e.target.value) : null)}
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
                          onClick={() => onSelectCollectionId(c.id)}
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
                    onClick={() => onToggleDocExpanded(doc.id)}
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
  );
}
