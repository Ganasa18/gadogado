import { Globe, Loader2, Upload } from "lucide-react";
import type { RagCollection } from "../types";

type Props = {
  selectedCollection: RagCollection | null;
  selectedCollectionId: number | null;
  isImporting: boolean;
  onImportFile: () => void;
  showWebImport: boolean;
  onToggleWebImport: () => void;
};

export function RagTabHeader(props: Props) {
  const {
    selectedCollection,
    selectedCollectionId,
    isImporting,
    onImportFile,
    showWebImport,
    onToggleWebImport,
  } = props;

  const isDbCollection = selectedCollection?.kind === "db";

  return (
    <div className="p-6 border-b border-app-border flex flex-wrap items-center justify-between gap-4">
      <div className="min-w-0">
        <h2 className="text-2xl font-bold text-app-text truncate">
          {selectedCollection ? selectedCollection.name : "Select a collection"}
        </h2>
        <p className="text-sm text-app-text-muted mt-1">
          {selectedCollection
            ? selectedCollection.description || "No description provided."
            : "Choose a collection to view and import documents."}
        </p>
      </div>
      {!isDbCollection && (
        <div className="flex items-center gap-2">
          <button
            onClick={onImportFile}
            disabled={selectedCollectionId === null || isImporting}
            className="flex items-center gap-2 px-4 py-2 bg-app-accent text-white rounded-lg text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
            {isImporting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Upload className="w-4 h-4" />}
            Import file
          </button>
          <button
            onClick={onToggleWebImport}
            disabled={selectedCollectionId === null}
            className="flex items-center gap-2 px-4 py-2 border border-app-border rounded-lg text-sm text-app-text hover:border-app-accent/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all">
            <Globe className="w-4 h-4" />
            {showWebImport ? "Hide web import" : "Web import"}
          </button>
        </div>
      )}
    </div>
  );
}
