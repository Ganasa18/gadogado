import { useCallback, useEffect, useMemo, useState } from "react";
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
  listRagCollections,
  listRagDocuments,
  ragCreateDbCollection,
} from "../api";
import { useRagImports } from "./ragTab/useRagImports";

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
  const [collectionKind, setCollectionKind] = useState<CollectionKind>("Files");

  // DB connection state
  const [dbConnections, setDbConnections] = useState<DbConnection[]>([]);
  const [dbConnId, setDbConnId] = useState<number | null>(null);
  const [allowlistProfiles, setAllowlistProfiles] = useState<DbAllowlistProfile[]>([]);
  const [allowlistProfileId, setAllowlistProfileId] = useState<number>(1);
  const [selectedTables, setSelectedTables] = useState<string[]>([]);
  const [collectionTables, setCollectionTables] = useState<string[]>([]);
  const [collectionConfig, setCollectionConfig] = useState<any>(null);
  const [availableTables, setAvailableTables] = useState<string[]>([]);
  const [isLoadingDbData, setIsLoadingDbData] = useState(false);

  // Imports (file + web) + progress

  const selectedCollection = useMemo(() => {
    const collection = collections.find((c) => c.id === selectedCollectionId) ?? null;
    if (collection) {
      console.log("[FRONTEND] Selected collection:", {
        id: collection.id,
        name: collection.name,
        kind: collection.kind,
        kindType: typeof collection.kind,
      });
    }
    return collection;
  }, [collections, selectedCollectionId]);

  // (imports hook is initialized after loadDocuments is declared)

  const loadCollections = useCallback(async () => {
    try {
      const data = await listRagCollections(50);
      console.log("[FRONTEND] Loaded collections:", data.map(c => ({
        id: c.id,
        name: c.name,
        kind: c.kind,
        kindType: typeof c.kind,
      })));
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
    if (!dbConnId || collectionKind !== "Db") {
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
    if (!selectedCollectionId || !selectedCollection || selectedCollection.kind !== "Db") {
      setCollectionTables([]);
      setCollectionConfig(null);
      return;
    }

    const loadTables = async () => {
      try {
        const tables = await dbGetSelectedTables(selectedCollectionId);
        setCollectionTables(tables);

        // Parse full config from collection's config_json
        try {
          const config = JSON.parse(selectedCollection.config_json);
          setCollectionConfig(config);
        } catch {
          setCollectionConfig(null);
        }
      } catch (err) {
        console.error("Failed to load collection tables:", err);
        setCollectionTables([]);
        setCollectionConfig(null);
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

    // Don't load documents for DB collections - they don't have documents
    if (selectedCollection && selectedCollection.kind === "Db") {
      setDocuments([]);
      return;
    }

    setDocuments([]);
    void loadDocuments(selectedCollectionId);
  }, [loadDocuments, selectedCollectionId, selectedCollection]);

  const {
    importProgress,
    setImportProgress,
    isImporting,
    handleFilePicker,
    handleDragOver,
    handleDrop,
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
  } = useRagImports({ selectedCollectionId, loadDocuments });

  const resetCreateFormState = useCallback(() => {
    setNewCollectionName("");
    setNewCollectionDescription("");
    setCollectionKind("Files");
    setDbConnId(null);
    setSelectedTables([]);
    setAvailableTables([]);
  }, []);

  const handleCreateCollection = useCallback(async () => {
    if (!newCollectionName.trim()) {
      alert("Collection name is required");
      return;
    }

    if (collectionKind === "Db") {
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
      let createdCollection: RagCollection;
      if (collectionKind === "Db") {
        createdCollection = await ragCreateDbCollection(newCollectionName.trim(), newCollectionDescription.trim() || undefined, {
          db_conn_id: dbConnId!,
          allowlist_profile_id: allowlistProfileId,
          selected_tables: selectedTables,
          max_limit: 200,
          external_llm_policy: "block",
        });
      } else {
        createdCollection = await createRagCollection({
          name: newCollectionName.trim(),
          description: newCollectionDescription.trim() || undefined,
        });
      }

      resetCreateFormState();
      setShowCreateForm(false);
      await loadCollections();

      // Auto-select the newly created collection
      setSelectedCollectionId(createdCollection.id);
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
    isLoadingDbData,
    gotoDbConnections,

    // db display
    collectionTables,
    collectionConfig,

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
