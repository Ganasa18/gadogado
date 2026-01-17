import React, { useState, useCallback, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import type { RagDocument, RagCollection } from "../types";
import { importRagFile, getLogs } from "../api";

interface DocumentUploaderProps {
  collections: RagCollection[];
  selectedCollectionId?: number;
  onDocumentUploaded: (doc: RagDocument) => void;
  onClose?: () => void;
}

interface UploadProgress {
  stage: "idle" | "reading" | "parsing" | "chunking" | "embedding" | "complete" | "error";
  progress: number;
  message: string;
}

const STAGE_PROGRESS: Record<UploadProgress["stage"], number> = {
  idle: 0,
  reading: 10,
  parsing: 30,
  chunking: 50,
  embedding: 70,
  complete: 100,
  error: 0,
};

export const DocumentUploader: React.FC<DocumentUploaderProps> = ({
  collections,
  selectedCollectionId,
  onDocumentUploaded,
  onClose,
}) => {
  const [collectionId, setCollectionId] = useState<number | undefined>(selectedCollectionId);
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress>({
    stage: "idle",
    progress: 0,
    message: "Ready to upload",
  });
  const [error, setError] = useState<string | null>(null);
  const [uploadedDocs, setUploadedDocs] = useState<RagDocument[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const logPollIntervalRef = useRef<number | null>(null);

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      const files = Array.from(e.target.files);
      setSelectedFiles(files);
      setError(null);
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const files = Array.from(e.dataTransfer.files);
    const supportedFiles = files.filter((f) => {
      const ext = f.name.toLowerCase().split(".").pop();
      return ["pdf", "docx", "xlsx", "csv", "txt", "md"].includes(ext || "");
    });

    if (supportedFiles.length !== files.length) {
      setError("Some files were skipped. Supported formats: PDF, DOCX, XLSX, CSV, TXT, MD");
    }

    setSelectedFiles(supportedFiles);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);

  const detectStageFromLogs = useCallback(async () => {
    try {
      const logs = await getLogs();
      // Get the most recent RAG log
      const ragLogs = logs
        .filter((l) => l.source === "RAG")
        .slice(-5);

      for (const log of ragLogs.reverse()) {
        const msg = log.message.toLowerCase();
        if (msg.includes("embedding") || msg.includes("embed")) {
          setUploadProgress((prev) => ({
            ...prev,
            stage: "embedding",
            progress: STAGE_PROGRESS.embedding,
            message: "Generating embeddings...",
          }));
          return;
        }
        if (msg.includes("chunk")) {
          setUploadProgress((prev) => ({
            ...prev,
            stage: "chunking",
            progress: STAGE_PROGRESS.chunking,
            message: "Splitting into chunks...",
          }));
          return;
        }
        if (msg.includes("pars") || msg.includes("extract")) {
          setUploadProgress((prev) => ({
            ...prev,
            stage: "parsing",
            progress: STAGE_PROGRESS.parsing,
            message: "Parsing document content...",
          }));
          return;
        }
      }
    } catch {
      // Ignore log polling errors
    }
  }, []);

  const startLogPolling = useCallback(() => {
    logPollIntervalRef.current = setInterval(detectStageFromLogs, 500);
  }, [detectStageFromLogs]);

  const stopLogPolling = useCallback(() => {
    if (logPollIntervalRef.current) {
      clearInterval(logPollIntervalRef.current);
      logPollIntervalRef.current = null;
    }
  }, []);

  const uploadFile = useCallback(
    async (file: File): Promise<RagDocument | null> => {
      // Tauri needs file path, not File object
      // In a real implementation, you would use dialog.open to get the path
      // For now, we'll show the path input approach
      const filePath = (file as unknown as { path?: string }).path;

      if (!filePath) {
        throw new Error(
          "File path not available. Please use the file browser to select files."
        );
      }

      const doc = await importRagFile(filePath, collectionId);
      return doc;
    },
    [collectionId]
  );

  const handleUpload = useCallback(async () => {
    if (selectedFiles.length === 0) return;

    setError(null);
    setUploadedDocs([]);
    startLogPolling();

    try {
      for (let i = 0; i < selectedFiles.length; i++) {
        const file = selectedFiles[i];

        setUploadProgress({
          stage: "reading",
          progress: STAGE_PROGRESS.reading,
          message: `Reading ${file.name} (${i + 1}/${selectedFiles.length})...`,
        });

        try {
          const doc = await uploadFile(file);
          if (doc) {
            setUploadedDocs((prev) => [...prev, doc]);
            onDocumentUploaded(doc);
          }
        } catch (err) {
          setError(
            `Failed to upload ${file.name}: ${
              err instanceof Error ? err.message : "Unknown error"
            }`
          );
        }
      }

      setUploadProgress({
        stage: "complete",
        progress: 100,
        message: "Upload complete!",
      });
    } finally {
      stopLogPolling();
    }
  }, [selectedFiles, uploadFile, onDocumentUploaded, startLogPolling, stopLogPolling]);

  const getStageIcon = (stage: UploadProgress["stage"]) => {
    switch (stage) {
      case "idle":
        return "📄";
      case "reading":
        return "📖";
      case "parsing":
        return "🔍";
      case "chunking":
        return "✂️";
      case "embedding":
        return "🧠";
      case "complete":
        return "✅";
      case "error":
        return "❌";
    }
  };

  const getFileTypeIcon = (fileName: string) => {
    const ext = fileName.toLowerCase().split(".").pop();
    switch (ext) {
      case "pdf":
        return "📕";
      case "docx":
        return "📘";
      case "xlsx":
        return "📊";
      case "csv":
        return "📋";
      case "txt":
        return "📝";
      case "md":
        return "📑";
      default:
        return "📄";
    }
  };

  return (
    <div className="bg-neutral-900 rounded-lg p-6 border border-neutral-700">
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-lg font-semibold text-white">Upload Documents</h3>
        {onClose && (
          <button
            onClick={onClose}
            className="text-neutral-400 hover:text-white transition-colors"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>

      {/* Collection Selector */}
      <div className="mb-4">
        <label className="block text-sm text-neutral-400 mb-1">Target Collection</label>
        <select
          value={collectionId ?? ""}
          onChange={(e) => setCollectionId(e.target.value ? parseInt(e.target.value) : undefined)}
          className="w-full bg-neutral-800 text-white rounded px-3 py-2 border border-neutral-600"
        >
          <option value="">No collection (standalone)</option>
          {collections.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </div>

      {/* Drop Zone */}
      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onClick={() => fileInputRef.current?.click()}
        className="border-2 border-dashed border-neutral-600 rounded-lg p-8 text-center cursor-pointer hover:border-blue-500 hover:bg-blue-500/5 transition-colors"
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept=".pdf,.docx,.xlsx,.csv,.txt,.md"
          onChange={handleFileSelect}
          className="hidden"
        />
        <div className="text-4xl mb-2">📁</div>
        <p className="text-neutral-300 mb-1">
          Drag & drop files here or click to browse
        </p>
        <p className="text-neutral-500 text-sm">
          Supported: PDF, DOCX, XLSX, CSV, TXT, MD
        </p>
      </div>

      {/* Selected Files */}
      {selectedFiles.length > 0 && (
        <div className="mt-4 space-y-2">
          <div className="text-sm text-neutral-400 mb-2">
            Selected Files ({selectedFiles.length}):
          </div>
          {selectedFiles.map((file, i) => (
            <div
              key={i}
              className="flex items-center justify-between bg-neutral-800 rounded px-3 py-2"
            >
              <div className="flex items-center gap-2">
                <span>{getFileTypeIcon(file.name)}</span>
                <span className="text-neutral-200 text-sm">{file.name}</span>
                <span className="text-neutral-500 text-xs">
                  ({(file.size / 1024).toFixed(1)} KB)
                </span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setSelectedFiles((prev) => prev.filter((_, idx) => idx !== i));
                }}
                className="text-neutral-500 hover:text-red-400"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Progress */}
      {uploadProgress.stage !== "idle" && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          className="mt-4"
        >
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xl">{getStageIcon(uploadProgress.stage)}</span>
            <span className="text-neutral-200">{uploadProgress.message}</span>
          </div>
          <div className="w-full bg-neutral-700 rounded-full h-2 overflow-hidden">
            <motion.div
              className={`h-full ${
                uploadProgress.stage === "error" ? "bg-red-500" : "bg-blue-500"
              }`}
              initial={{ width: 0 }}
              animate={{ width: `${uploadProgress.progress}%` }}
              transition={{ duration: 0.3 }}
            />
          </div>
          {uploadProgress.stage === "complete" && uploadedDocs.length > 0 && (
            <div className="mt-3 text-sm text-green-400">
              Successfully uploaded {uploadedDocs.length} document(s)
            </div>
          )}
        </motion.div>
      )}

      {/* Error */}
      {error && (
        <div className="mt-4 p-3 bg-red-500/10 border border-red-500/30 rounded">
          <p className="text-red-400 text-sm">{error}</p>
        </div>
      )}

      {/* Upload Button */}
      <button
        onClick={handleUpload}
        disabled={
          selectedFiles.length === 0 ||
          (uploadProgress.stage !== "idle" && uploadProgress.stage !== "complete" && uploadProgress.stage !== "error")
        }
        className="mt-4 w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:cursor-not-allowed text-white rounded transition-colors font-medium"
      >
        {uploadProgress.stage !== "idle" && uploadProgress.stage !== "complete" && uploadProgress.stage !== "error"
          ? "Uploading..."
          : `Upload ${selectedFiles.length} File(s)`}
      </button>

      {/* Uploaded Documents */}
      <AnimatePresence>
        {uploadedDocs.length > 0 && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-4"
          >
            <div className="text-sm text-neutral-400 mb-2">Uploaded Documents:</div>
            <div className="space-y-2">
              {uploadedDocs.map((doc) => (
                <div
                  key={doc.id}
                  className="flex items-center gap-2 bg-green-500/10 border border-green-500/30 rounded px-3 py-2"
                >
                  <span className="text-green-400">✓</span>
                  <span className="text-green-300 text-sm">{doc.file_name}</span>
                  <span className="text-green-400/60 text-xs">
                    ({doc.total_pages} pages)
                  </span>
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default DocumentUploader;
