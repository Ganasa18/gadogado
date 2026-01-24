import { RagAnalyticsMainPanel } from "./components/analytics/RagAnalyticsMainPanel";
import { RagAnalyticsSidebar } from "./components/analytics/RagAnalyticsSidebar";
import { useRagAnalyticsController } from "./hooks/useRagAnalyticsController";

export default function RagAnalytics() {
  const c = useRagAnalyticsController();

  return (
    <div className="flex h-full bg-app-bg text-app-text font-sans overflow-hidden">
      <RagAnalyticsSidebar
        collections={c.collections}
        selectedCollectionId={c.selectedCollectionId}
        onSelectCollectionId={c.setSelectedCollectionId}
        refreshing={c.refreshing}
        onRefresh={c.refresh}
        collectionMetricsById={c.collectionMetricsById}
        lowQualityDocs={c.lowQualityDocs}
        onToggleDocExpanded={c.toggleDocExpanded}
      />
      <RagAnalyticsMainPanel
        selectedCollectionId={c.selectedCollectionId}
        selectedCollection={c.selectedCollection}
        lastSyncAt={c.lastSyncAt}
        onComputeMetrics={c.computeQualityNow}
        analyticsSummary={c.analyticsSummary}
        recentEvents={c.recentEvents}
        qualityMetrics={c.qualityMetrics}
        aggregate={c.aggregate}
        actionableSuggestions={c.actionableSuggestions}
        ragConfig={c.ragConfig}
        filteredDocuments={c.filteredDocuments}
        expandedDocIds={c.expandedDocIds}
        docDetailsById={c.docDetailsById}
        onToggleDocExpanded={c.toggleDocExpanded}
        onEnsureDocDetails={c.ensureDocDetails}
        retrievalGaps={c.retrievalGaps}
      />
    </div>
  );
}
