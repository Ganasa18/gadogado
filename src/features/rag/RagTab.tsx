import { Database } from "lucide-react";
import { useNavigate } from "react-router";
import AnimatedContainer from "../../shared/components/AnimatedContainer";
import { RagTabDbCollectionInfo } from "./components/RagTabDbCollectionInfo";
import { RagTabDocumentsGrid } from "./components/RagTabDocumentsGrid";
import { RagTabHeader } from "./components/RagTabHeader";
import { RagTabImportStatus } from "./components/RagTabImportStatus";
import { RagTabSidebar } from "./components/RagTabSidebar";
import { RagTabWebImportPanel } from "./components/RagTabWebImportPanel";
import { useRagTabController } from "./hooks/useRagTabController";

export default function RagTab() {
  const navigate = useNavigate();
  const c = useRagTabController(navigate);

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <RagTabSidebar
        collections={c.collections}
        selectedCollectionId={c.selectedCollectionId}
        onSelectCollection={c.setSelectedCollectionId}
        onDeleteCollection={c.handleDeleteCollection}
        showCreateForm={c.showCreateForm}
        onShowCreateForm={c.setShowCreateForm}
        collectionKind={c.collectionKind}
        onChangeCollectionKind={(v) => {
          c.setCollectionKind(v);
          c.setDbConnId(null);
          c.setSelectedTables([]);
        }}
        newCollectionName={c.newCollectionName}
        onChangeNewCollectionName={c.setNewCollectionName}
        newCollectionDescription={c.newCollectionDescription}
        onChangeNewCollectionDescription={c.setNewCollectionDescription}
        isCreatingCollection={c.isCreatingCollection}
        onCreateCollection={c.handleCreateCollection}
        onCancelCreate={c.handleCancelCreate}
        dbConnections={c.dbConnections}
        dbConnId={c.dbConnId}
        onChangeDbConnId={c.setDbConnId}
        allowlistProfiles={c.allowlistProfiles}
        allowlistProfileId={c.allowlistProfileId}
        onChangeAllowlistProfileId={c.setAllowlistProfileId}
        availableTables={c.availableTables}
        selectedTables={c.selectedTables}
        onChangeSelectedTables={c.setSelectedTables}
        defaultLimit={c.defaultLimit}
        onChangeDefaultLimit={c.setDefaultLimit}
        isLoadingDbData={c.isLoadingDbData}
        onOpenDbConnections={c.gotoDbConnections}
      />

      <main className="flex-1 flex flex-col min-w-0">
        <RagTabHeader
          selectedCollection={c.selectedCollection}
          selectedCollectionId={c.selectedCollectionId}
          isImporting={c.isImporting}
          onImportFile={c.handleFilePicker}
          showWebImport={c.showWebImport}
          onToggleWebImport={() => c.setShowWebImport((prev) => !prev)}
        />

        <RagTabImportStatus
          progress={c.importProgress}
          onClear={() => c.setImportProgress({ status: "idle", message: "" })}
        />

        {c.showWebImport && (
          <RagTabWebImportPanel
            selectedCollectionId={c.selectedCollectionId}
            webUrl={c.webUrl}
            onChangeWebUrl={c.setWebUrl}
            maxPages={c.maxPages}
            onChangeMaxPages={c.setMaxPages}
            maxDepth={c.maxDepth}
            onChangeMaxDepth={c.setMaxDepth}
            webCrawlMode={c.webCrawlMode}
            onChangeWebCrawlMode={c.setWebCrawlMode}
            isCrawling={c.isCrawling}
            onStart={c.handleWebImport}
            onClose={() => c.setShowWebImport(false)}
          />
        )}

        <div
          className="flex-1 overflow-y-auto p-6 space-y-6"
          {...(c.selectedCollection?.kind !== "db" && {
            onDragOver: c.handleDragOver,
            onDrop: c.handleDrop,
          })}>
          {c.selectedCollectionId === null ? (
            <AnimatedContainer animation="fadeIn" className="h-full flex items-center justify-center">
              <div className="text-center max-w-md">
                <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-app-card border border-app-border flex items-center justify-center">
                  <Database className="w-10 h-10 text-app-text-muted/70" />
                </div>
                <h3 className="text-xl font-semibold text-app-text mb-2">Select a collection</h3>
                <p className="text-app-text-muted text-sm">Pick a collection to browse and import documents.</p>
              </div>
            </AnimatedContainer>
          ) : (
            <>
              {c.selectedCollection?.kind === "db" ? <RagTabDbCollectionInfo tables={c.collectionTables} /> : null}
              {c.selectedCollection?.kind !== "db" ? (
                <RagTabDocumentsGrid documents={c.documents} onDeleteDocument={c.handleDeleteDocument} />
              ) : null}
            </>
          )}
        </div>
      </main>
    </div>
  );
}
