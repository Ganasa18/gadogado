import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  AnalyticsEvent,
  AnalyticsSummary,
  ChunkWithQuality,
  DocumentQualityAnalysis,
  RagCollection,
  RagConfig,
  RagDocument,
} from "../types";
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
} from "../api";
import { fmtPct, sum } from "../ragAnalyticsUtils";

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

export function useRagAnalyticsController() {
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

  // Telemetry
  const [analyticsSummary, setAnalyticsSummary] = useState<AnalyticsSummary | null>(null);
  const [recentEvents, setRecentEvents] = useState<AnalyticsEvent[]>([]);

  // Collection comparison map
  const [collectionMetricsById, setCollectionMetricsById] = useState<
    Record<number, CollectionQualityMetrics | null | undefined>
  >({});

  // Per-document details (expand rows)
  const [expandedDocIds, setExpandedDocIds] = useState<Set<number>>(new Set());
  const [docDetailsById, setDocDetailsById] = useState<Record<number, DocDetails>>({});

  useEffect(() => {
    console.info(
      "[RagAnalytics] Dev note: per-document reranker score and per-document citation coverage are not exposed as DB metrics yet (collection-level only)."
    );
  }, []);

  const selectedCollection = useMemo(() => {
    return collections.find((c) => c.id === selectedCollectionId) ?? null;
  }, [collections, selectedCollectionId]);

  const loadCollections = useCallback(async () => {
    try {
      const list = await listRagCollections(50);
      setCollections(list);

      setSelectedCollectionId((prev) => {
        if (prev === null) return null;
        return list.some((c) => c.id === prev) ? prev : null;
      });

      // Opportunistically load stored quality metrics for comparison
      const ids = list.slice(0, 20).map((c) => c.id);
      const results = await Promise.all(ids.map(async (id) => ({ id, metrics: await getCollectionQuality(id) })));
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

  const loadSelectedCollection = useCallback(async (collectionId: number) => {
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
      setRetrievalGaps([]);
    }
  }, []);

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
    if (!selectedCollectionId) {
      void loadGlobalTelemetry();
      setDocuments([]);
      setQualityMetrics(null);
      setLowQualityDocs([]);
      setRetrievalGaps([]);
      setRagConfig(null);
      return;
    }
    void loadSelectedCollection(selectedCollectionId);
  }, [loadGlobalTelemetry, loadSelectedCollection, selectedCollectionId]);

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
        if (next.has(docId)) next.delete(docId);
        else next.add(docId);
        return next;
      });

      if (!docDetailsById[docId]?.warnings && !docDetailsById[docId]?.chunks) {
        void ensureDocDetails(docId);
      }
    },
    [docDetailsById, ensureDocDetails]
  );

  const aggregate = useMemo(() => {
    const docs = documents;
    const warningTotal = sum(docs.map((d) => d.warning_count ?? 0));
    const lowQualityCount = docs.filter((d) => (d.quality_score ?? 1) < 0.5).length;
    const missingOcrCount = docs.filter((d) => d.file_type === "pdf" && d.ocr_confidence === null).length;

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

  return {
    collections,
    selectedCollectionId,
    setSelectedCollectionId,
    refreshing,
    refresh,
    lastSyncAt,
    selectedCollection,
    documents,
    filteredDocuments,
    qualityMetrics,
    lowQualityDocs,
    retrievalGaps,
    ragConfig,
    analyticsSummary,
    recentEvents,
    collectionMetricsById,
    expandedDocIds,
    docDetailsById,
    toggleDocExpanded,
    ensureDocDetails,
    aggregate,
    actionableSuggestions,
    computeQualityNow,
  };
}
