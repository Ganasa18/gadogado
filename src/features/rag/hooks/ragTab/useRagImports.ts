import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { importRagFile, importRagWeb } from "../../api";
import {
  getFileExtension,
  getStatusMessage,
  isSupportedFile,
  SUPPORTED_EXTENSIONS,
  type ImportProgress,
} from "../../ragTabUtils";

type WebCrawlMode = "html" | "ocr";

type UseRagImportsArgs = {
  selectedCollectionId: number | null;
  loadDocuments: (collectionId: number) => Promise<void>;
};

export function useRagImports({
  selectedCollectionId,
  loadDocuments,
}: UseRagImportsArgs) {
  const [importProgress, setImportProgress] = useState<ImportProgress>({
    status: "idle",
    message: "",
  });

  // Web import state
  const [webUrl, setWebUrl] = useState("");
  const [maxPages, setMaxPages] = useState(10);
  const [maxDepth, setMaxDepth] = useState(2);
  const [isCrawling, setIsCrawling] = useState(false);
  const [showWebImport, setShowWebImport] = useState(false);
  const [webCrawlMode, setWebCrawlMode] = useState<WebCrawlMode>("html");

  const isImporting = useMemo(
    () =>
      importProgress.status !== "idle" &&
      importProgress.status !== "complete" &&
      importProgress.status !== "error",
    [importProgress.status],
  );

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
          isCsv ? 2000 : 500,
        );

        const embeddingTimer = setTimeout(
          () => {
            setImportProgress({
              status: "embedding",
              message: getStatusMessage("embedding", fileName),
              fileName,
            });
          },
          isCsv ? 3000 : 1500,
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
          setImportProgress((prev: ImportProgress) =>
            prev.status === "complete" ? { status: "idle", message: "" } : prev,
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
    [loadDocuments, selectedCollectionId],
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
          `Unsupported file type. Supported formats: ${SUPPORTED_EXTENSIONS.join(", ").toUpperCase()}`,
        );
      }
    },
    [importFile, selectedCollectionId],
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
      await importRagWeb(
        webUrl.trim(),
        selectedCollectionId,
        maxPages,
        maxDepth,
        webCrawlMode,
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

  return {
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
  };
}
