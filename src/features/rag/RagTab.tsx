import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Upload,
  FileText,
  Trash2,
  Plus,
  X,
  Database,
  FileCode,
  FileSpreadsheet,
  Globe,
  Loader2,
  Info,
  ArrowRight,
  CheckCircle2,
  AlertCircle,
  FileCheck,
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import type { RagCollection, RagDocument } from "./types";
import {
  deleteRagDocument,
  importRagFile,
  listRagCollections,
  listRagDocuments,
} from "./api";
import AnimatedContainer from "../../shared/components/AnimatedContainer";

const SUPPORTED_EXTENSIONS = ["pdf", "docx", "xlsx", "csv", "txt", "web"];

// Import status types for user feedback
type ImportStatus = "idle" | "validating" | "preprocessing" | "processing" | "chunking" | "embedding" | "complete" | "error";

interface ImportProgress {
  status: ImportStatus;
  message: string;
  fileName?: string;
  error?: string;
}

function getFileExtension(filePath: string): string {
  const parts = filePath.split(".");
  return parts[parts.length - 1].toLowerCase();
}

function isSupportedFile(filePath: string): boolean {
  const ext = getFileExtension(filePath);
  return SUPPORTED_EXTENSIONS.includes(ext);
}

function getFileIcon(fileType: string) {
  switch (fileType.toLowerCase()) {
    case "pdf":
      return <FileText className="w-5 h-5 text-red-500" />;
    case "docx":
      return <FileCode className="w-5 h-5 text-blue-500" />;
    case "xlsx":
      return <FileSpreadsheet className="w-5 h-5 text-green-500" />;
    case "csv":
      return <FileSpreadsheet className="w-5 h-5 text-emerald-500" />;
    default:
      return <FileText className="w-5 h-5 text-gray-500" />;
  }
}

function getStatusIcon(status: ImportStatus) {
  switch (status) {
    case "validating":
    case "preprocessing":
    case "processing":
    case "chunking":
    case "embedding":
      return <Loader2 className="w-5 h-5 text-app-accent animate-spin" />;
    case "complete":
      return <CheckCircle2 className="w-5 h-5 text-green-500" />;
    case "error":
      return <AlertCircle className="w-5 h-5 text-red-500" />;
    default:
      return <FileCheck className="w-5 h-5 text-app-text-muted" />;
  }
}

function getStatusMessage(status: ImportStatus, fileName?: string): string {
  const name = fileName ? `"${fileName}"` : "file";
  switch (status) {
    case "validating":
      return `Validating ${name}...`;
    case "preprocessing":
      return `Preprocessing CSV (detecting content type)...`;
    case "processing":
      return `Parsing ${name}...`;
    case "chunking":
      return `Creating smart chunks...`;
    case "embedding":
      return `Generating embeddings...`;
    case "complete":
      return `${name} imported successfully. Ready for chat.`;
    case "error":
      return `Failed to import ${name}`;
    default:
      return "";
  }
}

export default function RagTab() {
  const [collections, setCollections] = useState<RagCollection[]>([]);
  const [documents, setDocuments] = useState<RagDocument[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(null);
  const [newCollectionName, setNewCollectionName] = useState("");
  const [newCollectionDescription, setNewCollectionDescription] = useState("");
  const [isCreatingCollection, setIsCreatingCollection] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const hasDocuments = documents.length > 0;

  // Web import state
  const [webUrl, setWebUrl] = useState("");
  const [maxPages, setMaxPages] = useState(10);
  const [maxDepth, setMaxDepth] = useState(2);
  const [isCrawling, setIsCrawling] = useState(false);
  const [showWebImport, setShowWebImport] = useState(false);
  const [webCrawlMode, setWebCrawlMode] = useState<"html" | "ocr">("html");

  // Import progress state with real-time feedback
  const [importProgress, setImportProgress] = useState<ImportProgress>({
    status: "idle",
    message: "",
  });

  const selectedCollection = useMemo(() => {
    return collections.find((c) => c.id === selectedCollectionId);
  }, [collections, selectedCollectionId]);

  const isImporting = importProgress.status !== "idle" && importProgress.status !== "complete" && importProgress.status !== "error";

  const handleCreateCollection = async () => {
    if (!newCollectionName.trim()) {
      alert("Collection name is required");
      return;
    }

    setIsCreatingCollection(true);
    try {
      await import("./api").then((m) =>
        m.createRagCollection({
          name: newCollectionName.trim(),
          description: newCollectionDescription.trim() || undefined,
        })
      );
      setNewCollectionName("");
      setNewCollectionDescription("");
      setShowCreateForm(false);
      await loadCollections();
    } catch (err) {
      console.error("Failed to create collection:", err);
      alert("Failed to create collection: " + (err as Error).message);
    } finally {
      setIsCreatingCollection(false);
    }
  };

  const handleCancelCreate = () => {
    setShowCreateForm(false);
    setNewCollectionName("");
    setNewCollectionDescription("");
  };

  const handleFilePicker = async () => {
    if (selectedCollectionId === null) {
      return;
    }

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Select file to import",
        filters: [
          {
            name: "Supported Files",
            extensions: SUPPORTED_EXTENSIONS,
          },
        ],
      });

      if (typeof selected === "string") {
        await importFile(selected);
      } else if (Array.isArray(selected) && selected[0]) {
        await importFile(selected[0]);
      }
    } catch (err) {
      console.error("Failed to open file picker:", err);
    }
  };

  const handleDeleteCollection = async (id: number) => {
    if (!confirm("Are you sure you want to delete this collection?")) {
      return;
    }

    try {
      await import("./api").then((m) => m.deleteRagCollection(id));
      await loadCollections();
      if (selectedCollectionId === id) {
        setSelectedCollectionId(null);
        setDocuments([]);
      }
    } catch (err) {
      console.error("Failed to delete collection:", err);
      alert("Failed to delete collection: " + (err as Error).message);
    }
  };

  const handleDeleteDocument = async (docId: number) => {
    if (!confirm("Delete this document?")) {
      return;
    }

    try {
      await deleteRagDocument(docId);
      if (selectedCollectionId !== null) {
        await loadDocuments(selectedCollectionId);
      }
    } catch (err) {
      console.error("Failed to delete document:", err);
      alert("Failed to delete document: " + (err as Error).message);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();

    if (selectedCollectionId === null) {
      alert("Please select a collection first");
      return;
    }

    const files = Array.from(e.dataTransfer.files);
    const filePath = files[0] ? (files[0] as any).path : undefined;

    if (filePath && isSupportedFile(filePath)) {
      await importFile(filePath);
    } else if (!filePath) {
      alert(
        "Drag and drop path unavailable. Please use the file picker instead."
      );
    } else {
      alert(
        `Unsupported file type. Supported formats: ${SUPPORTED_EXTENSIONS.join(
          ", "
        ).toUpperCase()}`
      );
    }
  };

  const handleWebImport = async () => {
    if (!webUrl.trim() || !selectedCollectionId) {
      alert("Please enter a URL and select a collection");
      return;
    }

    setIsCrawling(true);
    setImportProgress({
      status: "processing",
      message: "Crawling website...",
      fileName: webUrl,
    });

    try {
      await import("./api").then((m) =>
        m.importRagWeb(
          webUrl.trim(),
          selectedCollectionId,
          maxPages,
          maxDepth,
          webCrawlMode
        )
      );
      setWebUrl("");
      setMaxPages(10);
      setMaxDepth(2);
      setWebCrawlMode("html");
      setShowWebImport(false);
      setImportProgress({
        status: "complete",
        message: "Web import completed successfully. Ready for chat.",
        fileName: webUrl,
      });
      if (selectedCollectionId !== null) {
        await loadDocuments(selectedCollectionId);
      }
    } catch (err) {
      console.error("Failed to import web:", err);
      setImportProgress({
        status: "error",
        message: "Failed to import web content",
        error: (err as Error).message,
      });
    } finally {
      setIsCrawling(false);
    }
  };

  const loadCollections = async () => {
    try {
      const data = await listRagCollections(50);
      setCollections(data);
    } catch (err) {
      console.error("Failed to load collections:", err);
    }
  };

  useEffect(() => {
    loadCollections();
  }, []);

  const loadDocuments = useCallback(
    async (collectionId: number) => {
      try {
        const data = await listRagDocuments(collectionId, 50);
        if (collectionId !== selectedCollectionId) return;
        setDocuments(data);
      } catch (err) {
        console.error("Failed to load documents:", err);
      }
    },
    [selectedCollectionId]
  );

  useEffect(() => {
    if (selectedCollectionId === null) {
      setDocuments([]);
      return;
    }

    setDocuments([]);
    void loadDocuments(selectedCollectionId);
  }, [loadDocuments, selectedCollectionId]);

  /**
   * File import with integrated RAG pipeline:
   * 1. File validation (extension, size)
   * 2. Content parsing (PDF/DOCX/XLSX/TXT)
   * 3. Smart chunking (content-aware)
   * 4. Local embedding generation (Fastembed)
   * 5. Storage to encrypted SQLite
   */
  const importFile = useCallback(
    async (filePath: string) => {
      if (!filePath || typeof filePath !== "string") {
        alert("Invalid file path. Please choose a file again.");
        return;
      }

      const fileName = filePath.split(/[/\\]/).pop() || filePath;

      // Step 1: Validation
      setImportProgress({
        status: "validating",
        message: getStatusMessage("validating", fileName),
        fileName,
      });

      if (!isSupportedFile(filePath)) {
        setImportProgress({
          status: "error",
          message: `Unsupported file type. Supported formats: ${SUPPORTED_EXTENSIONS.join(", ").toUpperCase()}`,
          fileName,
          error: "Unsupported file type",
        });
        return;
      }

      if (selectedCollectionId === null) {
        setImportProgress({
          status: "error",
          message: "Please select a collection first",
          fileName,
          error: "No collection selected",
        });
        return;
      }

      // Step 2: Processing (parsing)
      setImportProgress({
        status: "processing",
        message: getStatusMessage("processing", fileName),
        fileName,
      });

      // CSV-specific: Show preprocessing status
      const isCsv = getFileExtension(filePath) === "csv";
      if (isCsv) {
        console.log("📊 CSV Import: Starting preprocessing", {
          fileName,
          collectionId: selectedCollectionId,
        });

        setImportProgress({
          status: "preprocessing",
          message: getStatusMessage("preprocessing", fileName),
          fileName,
        });
      }

      try {
        // The backend handles the full pipeline:
        // - CSV: Content type detection → formatting → chunking → embedding
        // - Other files: Parsing → chunking → embedding
        // - Storage to SQLite

        // Simulate progress updates for better UX
        const preprocessingTimer = isCsv
          ? setTimeout(() => {
              setImportProgress({
                status: "processing",
                message: `Converting CSV to optimal format for embeddings...`,
                fileName,
              });
            }, 1000)
          : null;

        const progressTimer = setTimeout(() => {
          setImportProgress({
            status: "chunking",
            message: getStatusMessage("chunking", fileName),
            fileName,
          });
        }, isCsv ? 2000 : 500);

        const embeddingTimer = setTimeout(() => {
          setImportProgress({
            status: "embedding",
            message: getStatusMessage("embedding", fileName),
            fileName,
          });
        }, isCsv ? 3000 : 1500);

        console.info("RAG import requested", {
          filePath,
          collectionId: selectedCollectionId,
          fileType: getFileExtension(filePath),
          isCsv,
        });

        await importRagFile(filePath, selectedCollectionId);

        if (preprocessingTimer) clearTimeout(preprocessingTimer);
        clearTimeout(progressTimer);
        clearTimeout(embeddingTimer);

        console.info("RAG import completed", {
          filePath,
          fileType: getFileExtension(filePath),
          isCsv,
        });

        setImportProgress({
          status: "complete",
          message: getStatusMessage("complete", fileName),
          fileName,
        });

        if (selectedCollectionId !== null) {
          await loadDocuments(selectedCollectionId);
        }

        // Auto-clear success message after 5 seconds
        setTimeout(() => {
          setImportProgress((prev) =>
            prev.status === "complete" ? { status: "idle", message: "" } : prev
          );
        }, 5000);
      } catch (err) {
        const fallbackMessage =
          err instanceof Error
            ? err.message
            : typeof err === "string"
            ? err
            : JSON.stringify(err);
        console.error("Failed to import file:", err);
        setImportProgress({
          status: "error",
          message: getStatusMessage("error", fileName),
          fileName,
          error: fallbackMessage,
        });
      }
    },
    [loadDocuments, selectedCollectionId]
  );

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setup = async () => {
      unlisten = await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type !== "drop") {
          return;
        }

        const paths = event.payload.paths ?? [];
        if (!paths.length) {
          return;
        }

        if (selectedCollectionId === null) {
          alert("Please select a collection first");
          return;
        }

        void importFile(paths[0]);
      });
    };

    void setup();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [importFile, selectedCollectionId]);

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-80 border-r border-app-border flex flex-col">
        <div className="p-4 border-b border-app-border">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Database className="w-5 h-5 text-app-accent" />
              <h2 className="text-lg font-semibold text-app-text">
                Collections
              </h2>
            </div>
            {!showCreateForm && (
              <button
                onClick={() => setShowCreateForm(true)}
                className="flex items-center gap-1 px-3 py-1.5 text-sm bg-app-accent text-white rounded-md hover:opacity-90 transition-opacity">
                <Plus className="w-4 h-4" />
                New
              </button>
            )}
          </div>

          {showCreateForm && (
            <div className="space-y-3">
              <div>
                <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                  Name
                </label>
                <input
                  value={newCollectionName}
                  onChange={(e) => setNewCollectionName(e.target.value)}
                  placeholder="Marketing docs"
                  className="w-full bg-app-card border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
                />
              </div>
              <div>
                <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                  Description
                </label>
                <textarea
                  value={newCollectionDescription}
                  onChange={(e) => setNewCollectionDescription(e.target.value)}
                  placeholder="Optional description"
                  rows={2}
                  className="w-full bg-app-card border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all resize-none"
                />
              </div>
              <div className="flex gap-2">
                <button
                  onClick={handleCreateCollection}
                  disabled={isCreatingCollection || !newCollectionName.trim()}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-app-accent text-white rounded-md text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
                  {isCreatingCollection ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <ArrowRight className="w-4 h-4" />
                  )}
                  Create
                </button>
                <button
                  onClick={handleCancelCreate}
                  className="flex items-center justify-center px-3 py-2 border border-app-border rounded-md text-sm text-app-text-muted hover:text-app-text transition-colors">
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          {collections.length === 0 ? (
            <div className="text-sm text-app-text-muted">
              No collections yet. Create one to start importing.
            </div>
          ) : (
            collections.map((collection) => (
              <button
                key={collection.id}
                onClick={() => setSelectedCollectionId(collection.id)}
                className={`w-full text-left p-3 rounded-lg border transition-all ${
                  selectedCollectionId === collection.id
                    ? "border-app-accent bg-app-accent/10"
                    : "border-app-border bg-app-card hover:border-app-accent/40"
                }`}>
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium text-app-text">
                      {collection.name}
                    </div>
                    {collection.description && (
                      <div className="text-xs text-app-text-muted mt-1">
                        {collection.description}
                      </div>
                    )}
                    <div className="text-[10px] text-app-text-muted mt-2">
                      Added{" "}
                      {new Date(collection.created_at).toLocaleDateString(
                        undefined,
                        {
                          month: "short",
                          day: "numeric",
                          year: "numeric",
                        }
                      )}
                    </div>
                  </div>
                  <button
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleDeleteCollection(collection.id);
                    }}
                    className="p-1.5 text-app-text-muted hover:text-red-500 transition-colors"
                    title="Delete collection">
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </button>
            ))
          )}
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0">
        <div className="p-6 border-b border-app-border flex flex-wrap items-center justify-between gap-4">
          <div className="min-w-0">
            <h2 className="text-2xl font-bold text-app-text truncate">
              {selectedCollection
                ? selectedCollection.name
                : "Select a collection"}
            </h2>
            <p className="text-sm text-app-text-muted mt-1">
              {selectedCollection
                ? selectedCollection.description || "No description provided."
                : "Choose a collection to view and import documents."}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleFilePicker}
              disabled={selectedCollectionId === null || isImporting}
              className="flex items-center gap-2 px-4 py-2 bg-app-accent text-white rounded-lg text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
              {isImporting ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Upload className="w-4 h-4" />
              )}
              Import file
            </button>
            <button
              onClick={() => setShowWebImport((prev) => !prev)}
              disabled={selectedCollectionId === null}
              className="flex items-center gap-2 px-4 py-2 border border-app-border rounded-lg text-sm text-app-text hover:border-app-accent/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all">
              <Globe className="w-4 h-4" />
              Web import
            </button>
          </div>
        </div>

        {/* Import Status Feedback */}
        {importProgress.status !== "idle" && (
          <div
            className={`mx-6 mt-4 p-4 rounded-lg border flex items-center gap-3 ${
              importProgress.status === "complete"
                ? "bg-green-500/10 border-green-500/30"
                : importProgress.status === "error"
                ? "bg-red-500/10 border-red-500/30"
                : "bg-app-accent/10 border-app-accent/30"
            }`}>
            {getStatusIcon(importProgress.status)}
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-app-text">
                {importProgress.message}
              </div>
              {importProgress.error && (
                <div className="text-xs text-red-500 mt-1">
                  {importProgress.error}
                </div>
              )}
            </div>
            {(importProgress.status === "complete" || importProgress.status === "error") && (
              <button
                onClick={() => setImportProgress({ status: "idle", message: "" })}
                className="p-1 text-app-text-muted hover:text-app-text transition-colors">
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
        )}

        {showWebImport && (
          <div className="p-6 border-b border-app-border bg-app-card/30">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <Globe className="w-4 h-4 text-app-accent" />
                <h3 className="text-sm font-semibold">Import from website</h3>
              </div>
              <button
                onClick={() => setShowWebImport(false)}
                className="p-1.5 text-app-text-muted hover:text-app-text transition-colors">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
              <div className="md:col-span-2">
                <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                  URL
                </label>
                <input
                  value={webUrl}
                  onChange={(e) => setWebUrl(e.target.value)}
                  placeholder="https://docs.example.com"
                  className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
                />
              </div>
              <div>
                <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                  Max pages
                </label>
                <input
                  type="number"
                  min={1}
                  value={maxPages}
                  onChange={(e) => setMaxPages(Number(e.target.value) || 1)}
                  className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
                />
              </div>
              <div>
                <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                  Max depth
                </label>
                <input
                  type="number"
                  min={1}
                  value={maxDepth}
                  onChange={(e) => setMaxDepth(Number(e.target.value) || 1)}
                  className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
                />
              </div>
            </div>
            <div className="mt-4">
              <label className="text-[10px] text-app-subtext block mb-2 uppercase tracking-wider">
                Crawl Mode
              </label>
              <div className="flex gap-3">
                <button
                  onClick={() => setWebCrawlMode("html")}
                  className={`flex-1 px-4 py-2.5 rounded-lg text-sm border transition-all ${
                    webCrawlMode === "html"
                      ? "bg-app-accent text-white border-app-accent"
                      : "bg-app-bg border-app-border text-app-text hover:border-app-accent/50"
                  }`}>
                  <div className="font-medium">HTML (Fast)</div>
                  <div className="text-[10px] opacity-75 mt-0.5">
                    Best for simple sites
                  </div>
                </button>
                <button
                  onClick={() => setWebCrawlMode("ocr")}
                  className={`flex-1 px-4 py-2.5 rounded-lg text-sm border transition-all ${
                    webCrawlMode === "ocr"
                      ? "bg-app-accent text-white border-app-accent"
                      : "bg-app-bg border-app-border text-app-text hover:border-app-accent/50"
                  }`}>
                  <div className="font-medium">OCR (Accurate)</div>
                  <div className="text-[10px] opacity-75 mt-0.5">
                    For JS-heavy sites
                  </div>
                </button>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2 text-xs text-app-text-muted">
                <Info className="w-3.5 h-3.5" />
                {webCrawlMode === "html"
                  ? "Crawls only same-domain links to keep the import scoped."
                  : "Uses Playwright screenshots + Tesseract OCR. Requires Node.js."}
              </div>
              <button
                onClick={handleWebImport}
                disabled={
                  isCrawling || selectedCollectionId === null || !webUrl.trim()
                }
                className="flex items-center gap-2 px-4 py-2 bg-app-accent text-white rounded-lg text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
                {isCrawling ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <ArrowRight className="w-4 h-4" />
                )}
                {webCrawlMode === "ocr" ? "Start OCR capture" : "Start crawl"}
              </button>
            </div>
          </div>
        )}

        <div
          className="flex-1 overflow-y-auto p-6 space-y-6"
          onDragOver={handleDragOver}
          onDrop={handleDrop}>
          {selectedCollectionId === null ? (
            <AnimatedContainer
              animation="fadeIn"
              className="h-full flex items-center justify-center">
              <div className="text-center max-w-md">
                <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-app-card border border-app-border flex items-center justify-center">
                  <Database className="w-10 h-10 text-app-text-muted/70" />
                </div>
                <h3 className="text-xl font-semibold text-app-text mb-2">
                  Select a collection
                </h3>
                <p className="text-app-text-muted text-sm">
                  Pick a collection to browse and import documents.
                </p>
              </div>
            </AnimatedContainer>
          ) : (
            <>
              <div className="rounded-2xl border border-dashed border-app-border bg-app-card/20 p-6 text-center">
                <div className="flex flex-col items-center gap-3">
                  <div className="w-12 h-12 rounded-full bg-app-accent/10 flex items-center justify-center">
                    {isImporting ? (
                      <Loader2 className="w-6 h-6 text-app-accent animate-spin" />
                    ) : (
                      <Upload className="w-6 h-6 text-app-accent" />
                    )}
                  </div>
                  <div>
                    <h4 className="text-base font-semibold text-app-text">
                      Drag & drop files to import
                    </h4>
                    <p className="text-sm text-app-text-muted mt-1">
                      Supported formats: PDF, DOCX, XLSX, CSV, TXT, WEB
                    </p>
                    <p className="text-xs text-app-text-muted mt-2">
                      CSV files are auto-detected for optimal embedding format
                    </p>
                  </div>
                  <button
                    onClick={handleFilePicker}
                    disabled={isImporting}
                    className="px-4 py-2 border border-app-border rounded-lg text-sm text-app-text hover:border-app-accent/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all">
                    Browse files
                  </button>
                </div>
              </div>

              {!hasDocuments ? (
                <AnimatedContainer animation="fadeIn">
                  <div className="text-center py-20">
                    <div className="w-24 h-24 mx-auto mb-6 rounded-full from-app-accent/5 to-app-accent/10 border border-app-border/50 flex items-center justify-center">
                      <FileText className="w-12 h-12 text-app-text-muted/70" />
                    </div>
                    <h3 className="text-2xl font-bold text-app-text mb-3">
                      No Documents Yet
                    </h3>
                    <p className="text-app-text-muted max-w-md mx-auto text-base leading-relaxed">
                      Your collection is ready to store documents. Drag & drop
                      files above to import them.
                    </p>
                  </div>
                </AnimatedContainer>
              ) : (
                <>
                  <div className="flex items-center justify-between">
                    <div>
                      <h2 className="text-2xl font-bold text-app-text">
                        Documents
                      </h2>
                      <p className="text-sm text-app-text-muted mt-1">
                        {documents.length}{" "}
                        {documents.length === 1 ? "document" : "documents"}{" "}
                        stored in this collection
                      </p>
                    </div>
                  </div>
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {documents.map((document) => (
                      <div
                        key={document.id}
                        className="group relative from-app-card to-app-card/80 rounded-2xl border border-app-border/50 p-6 hover:shadow-2xl hover:border-app-accent/30 hover:-translate-y-1 transition-all duration-300">
                        <div className="absolute top-4 right-4">
                          <div className="p-2 rounded-full bg-app-bg/80 backdrop-blur-sm border border-app-border/30 group-hover:border-app-accent/50 transition-colors">
                            {getFileIcon(document.file_type)}
                          </div>
                        </div>

                        <div className="mb-4 pr-10">
                          <h3
                            className="font-semibold text-app-text truncate pr-8"
                            title={document.file_name}>
                            {document.file_name}
                          </h3>
                          <div className="mt-2 flex items-center justify-between gap-2">
                            <span className="inline-flex items-center px-2.5 py-1 rounded-md bg-app-accent/10 text-app-accent text-xs font-medium">
                              {document.file_type.toUpperCase()}
                            </span>
                            <button
                              onClick={() =>
                                void handleDeleteDocument(document.id)
                              }
                              className="text-app-text-muted hover:text-red-500 transition-colors"
                              title="Delete document">
                              <Trash2 className="w-4 h-4" />
                            </button>
                          </div>
                        </div>

                        <div className="space-y-2.5">
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-app-text-muted/70">
                              Pages
                            </span>
                            <span className="font-medium text-app-text">
                              {document.total_pages}
                            </span>
                          </div>
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-app-text-muted/70">
                              Language
                            </span>
                            <span className="font-medium text-app-text capitalize">
                              {document.language === "auto"
                                ? "Auto"
                                : document.language}
                            </span>
                          </div>
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-app-text-muted/70">
                              Added
                            </span>
                            <span className="font-medium text-app-text">
                              {new Date(document.created_at).toLocaleDateString(
                                undefined,
                                {
                                  month: "short",
                                  day: "numeric",
                                  year: "numeric",
                                }
                              )}
                            </span>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              )}
            </>
          )}
        </div>
      </main>
    </div>
  );
}
