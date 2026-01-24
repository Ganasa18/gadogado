import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import type { NavigateFunction } from "react-router";
import type { CollectionKind, DbAllowlistProfile, DbConnection, RagCollection, RagDocument } from "../types";
import {
  createRagCollection,
  dbGetSelectedTables,
  dbListAllowlistProfiles,
  dbListAllowlistedTables,
  dbListConnections,
  deleteRagCollection,
  deleteRagDocument,
  importRagFile,
  importRagWeb,
  listRagCollections,
  listRagDocuments,
  ragCreateDbCollection,
} from "../api";
import {
  getFileExtension,
  getStatusMessage,
  isSupportedFile,
  SUPPORTED_EXTENSIONS,
  type ImportProgress,
} from "../ragTabUtils";

type WebCrawlMode = "html" | "ocr";

export function useRagTabController(navigate: NavigateFunction) {
  const [collections, setCollections] = useState<RagCollection[]>([]);
  const [documents, setDocuments] = useState<RagDocument[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(null);
  const [newCollectionName, setNewCollectionName] = useState("");
  const [newCollectionDescription, setNewCollectionDescription] = useState("");
  const [isCreatingCollection, setIsCreatingCollection] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const hasDocuments = documents.length > 0;

  // Collection kind state
  const [collectionKind, setCollectionKind] = useState<CollectionKind>("files");

  // DB connection state
  const [dbConnections, setDbConnections] = useState<DbConnection[]>([]);
  const [dbConnId, setDbConnId] = useState<number | null>(null);
  const [allowlistProfiles, setAllowlistProfiles] = useState<DbAllowlistProfile[]>([]);
  const [allowlistProfileId, setAllowlistProfileId] = useState<number>(1);
  const [selectedTables, setSelectedTables] = useState<string[]>([]);
  const [collectionTables, setCollectionTables] = useState<string[]>([]);
  const [availableTables, setAvailableTables] = useState<string[]>([]);
  const [defaultLimit, setDefaultLimit] = useState(50);
  const [isLoadingDbData, setIsLoadingDbData] = useState(false);

  // Web import state
  const [webUrl, setWebUrl] = useState("");
  const [maxPages, setMaxPages] = useState(10);
  const [maxDepth, setMaxDepth] = useState(2);
  const [isCrawling, setIsCrawling] = useState(false);
  const [showWebImport, setShowWebImport] = useState(false);
  const [webCrawlMode, setWebCrawlMode] = useState<WebCrawlMode>("html");

  // Import progress state with real-time feedback
  const [importProgress, setImportProgress] = useState<ImportProgress>({
    status: "idle",
    message: "",
  });

  const selectedCollection = useMemo(() => {
    return collections.find((c) => c.id === selectedCollectionId) ?? null;
  }, [collections, selectedCollectionId]);

  const isImporting =
    importProgress.status !== "idle" &&
    importProgress.status !== "complete" &&
    importProgress.status !== "error";

  const loadCollections = useCallback(async () => {
    try {
      const data = await listRagCollections(50);
      setCollections(data);
    } catch (err) {
      console.error("Failed to load collections:", err);
    }
  }, []);

  useEffect(() => {
    void loadCollections();
  }, [loadCollections]);

  // Load DB connections and tables when create form is shown
  useEffect(() => {
    if (!showCreateForm) return;

    const loadDbData = async () => {
      setIsLoadingDbData(true);
      try {
        const [connections, profiles] = await Promise.all([
          dbListConnections(),
          dbListAllowlistProfiles(),
        ]);
        setDbConnections(connections);
        setAllowlistProfiles(profiles);
      } catch (err) {
        console.error("Failed to load DB connections:", err);
      } finally {
        setIsLoadingDbData(false);
      }
    };

    void loadDbData();
  }, [showCreateForm]);

  // Load available tables when DB connection is selected
  useEffect(() => {
    if (!dbConnId || collectionKind !== "db") {
      setAvailableTables([]);
      return;
    }

    const loadTables = async () => {
      try {
        const tables = await dbListAllowlistedTables(allowlistProfileId);
        setAvailableTables(tables);
        setSelectedTables((prev) => prev.filter((t) => tables.includes(t)));
      } catch (err) {
        console.error("Failed to load tables:", err);
        setAvailableTables([]);
      }
    };

    void loadTables();
  }, [allowlistProfileId, collectionKind, dbConnId]);

  // Load collection tables for display
  useEffect(() => {
    if (!selectedCollectionId || !selectedCollection || selectedCollection.kind !== "db") {
      setCollectionTables([]);
      return;
    }

    const loadTables = async () => {
      try {
        const tables = await dbGetSelectedTables(selectedCollectionId);
        setCollectionTables(tables);
      } catch (err) {
        console.error("Failed to load collection tables:", err);
        setCollectionTables([]);
      }
    };

    void loadTables();
  }, [selectedCollection, selectedCollectionId]);

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

  const resetCreateFormState = useCallback(() => {
    setNewCollectionName("");
    setNewCollectionDescription("");
    setCollectionKind("files");
    setDbConnId(null);
    setSelectedTables([]);
    setAvailableTables([]);
    setDefaultLimit(50);
  }, []);

  const handleCreateCollection = useCallback(async () => {
    if (!newCollectionName.trim()) {
      alert("Collection name is required");
      return;
    }

    if (collectionKind === "db") {
      if (!dbConnId) {
        alert("Please select a database connection");
        return;
      }
      if (selectedTables.length === 0) {
        alert("Please select at least one table to query");
        return;
      }
    }

    setIsCreatingCollection(true);
    try {
      if (collectionKind === "db") {
        await ragCreateDbCollection(newCollectionName.trim(), newCollectionDescription.trim() || undefined, {
          db_conn_id: dbConnId!,
          allowlist_profile_id: allowlistProfileId,
          selected_tables: selectedTables,
          default_limit: defaultLimit,
          max_limit: 200,
          external_llm_policy: "block",
        });
      } else {
        await createRagCollection({
          name: newCollectionName.trim(),
          description: newCollectionDescription.trim() || undefined,
        });
      }

      resetCreateFormState();
      setShowCreateForm(false);
      await loadCollections();
    } catch (err) {
      console.error("Failed to create collection:", err);
      alert("Failed to create collection: " + (err as Error).message);
    } finally {
      setIsCreatingCollection(false);
    }
  }, [
    allowlistProfileId,
    collectionKind,
    dbConnId,
    defaultLimit,
    loadCollections,
    newCollectionDescription,
    newCollectionName,
    resetCreateFormState,
    selectedTables,
  ]);

  const handleCancelCreate = useCallback(() => {
    setShowCreateForm(false);
    resetCreateFormState();
  }, [resetCreateFormState]);

  const importFile = useCallback(
    async (filePath: string) => {
      if (!filePath || typeof filePath !== "string") {
        alert("Invalid file path. Please choose a file again.");
        return;
      }

      const fileName = filePath.split(/[/\\]/).pop() || filePath;

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

      setImportProgress({
        status: "processing",
        message: getStatusMessage("processing", fileName),
        fileName,
      });

      const isCsv = getFileExtension(filePath) === "csv";
      if (isCsv) {
        setImportProgress({
          status: "preprocessing",
          message: getStatusMessage("preprocessing", fileName),
          fileName,
        });
      }

      try {
        const preprocessingTimer = isCsv
          ? setTimeout(() => {
              setImportProgress({
                status: "processing",
                message: "Converting CSV to optimal format for embeddings...",
                fileName,
              });
            }, 1000)
          : null;

        const progressTimer = setTimeout(
          () => {
            setImportProgress({
              status: "chunking",
              message: getStatusMessage("chunking", fileName),
              fileName,
            });
          },
          isCsv ? 2000 : 500
        );

        const embeddingTimer = setTimeout(
          () => {
            setImportProgress({
              status: "embedding",
              message: getStatusMessage("embedding", fileName),
              fileName,
            });
          },
          isCsv ? 3000 : 1500
        );

        await importRagFile(filePath, selectedCollectionId);

        if (preprocessingTimer) clearTimeout(preprocessingTimer);
        clearTimeout(progressTimer);
        clearTimeout(embeddingTimer);

        setImportProgress({
          status: "complete",
          message: getStatusMessage("complete", fileName),
          fileName,
        });

        await loadDocuments(selectedCollectionId);

        setTimeout(() => {
          setImportProgress((prev) =>
            prev.status === "complete" ? { status: "idle", message: "" } : prev
          );
        }, 5000);
      } catch (err) {
        const fallbackMessage =
          err instanceof Error ? err.message : typeof err === "string" ? err : JSON.stringify(err);
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

  const handleFilePicker = useCallback(async () => {
    if (selectedCollectionId === null) return;

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
  }, [importFile, selectedCollectionId]);

  const handleDeleteCollection = useCallback(
    async (id: number) => {
      if (!confirm("Are you sure you want to delete this collection?")) {
        return;
      }

      try {
        await deleteRagCollection(id);
        await loadCollections();
        if (selectedCollectionId === id) {
          setSelectedCollectionId(null);
          setDocuments([]);
        }
      } catch (err) {
        console.error("Failed to delete collection:", err);
        alert("Failed to delete collection: " + (err as Error).message);
      }
    },
    [loadCollections, selectedCollectionId]
  );

  const handleDeleteDocument = useCallback(
    async (docId: number) => {
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
    },
    [loadDocuments, selectedCollectionId]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
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
        alert("Drag and drop path unavailable. Please use the file picker instead.");
      } else {
        alert(
          `Unsupported file type. Supported formats: ${SUPPORTED_EXTENSIONS.join(", ").toUpperCase()}`
        );
      }
    },
    [importFile, selectedCollectionId]
  );

  const handleWebImport = useCallback(async () => {
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
      await importRagWeb(webUrl.trim(), selectedCollectionId, maxPages, maxDepth, webCrawlMode);
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
      await loadDocuments(selectedCollectionId);
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
  }, [loadDocuments, maxDepth, maxPages, selectedCollectionId, webCrawlMode, webUrl]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setup = async () => {
      unlisten = await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        const paths = event.payload.paths ?? [];
        if (!paths.length) return;
        if (selectedCollectionId === null) {
          alert("Please select a collection first");
          return;
        }

        void importFile(paths[0]);
      });
    };

    void setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, [importFile, selectedCollectionId]);

  const gotoDbConnections = useCallback(() => {
    navigate("/database");
  }, [navigate]);

  return {
    // main state
    collections,
    documents,
    hasDocuments,
    selectedCollectionId,
    selectedCollection,
    setSelectedCollectionId,

    // sidebar: create collection
    showCreateForm,
    setShowCreateForm,
    newCollectionName,
    setNewCollectionName,
    newCollectionDescription,
    setNewCollectionDescription,
    isCreatingCollection,
    collectionKind,
    setCollectionKind,
    handleCreateCollection,
    handleCancelCreate,
    handleDeleteCollection,

    // db fields
    dbConnections,
    dbConnId,
    setDbConnId,
    allowlistProfiles,
    allowlistProfileId,
    setAllowlistProfileId,
    selectedTables,
    setSelectedTables,
    availableTables,
    defaultLimit,
    setDefaultLimit,
    isLoadingDbData,
    gotoDbConnections,

    // db display
    collectionTables,

    // docs
    handleDeleteDocument,

    // imports
    handleFilePicker,
    importProgress,
    setImportProgress,
    isImporting,
    handleDragOver,
    handleDrop,

    // web import
    showWebImport,
    setShowWebImport,
    webUrl,
    setWebUrl,
    maxPages,
    setMaxPages,
    maxDepth,
    setMaxDepth,
    webCrawlMode,
    setWebCrawlMode,
    isCrawling,
    handleWebImport,
  };
}
