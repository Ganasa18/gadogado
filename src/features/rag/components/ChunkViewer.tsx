import React, { useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import type { ChunkWithQuality, RagDocument } from "../types";
import {
  getChunksWithQuality,
  deleteChunk,
  updateChunkContent,
  reembedChunk,
  filterLowQualityChunks,
  analyzeDocumentQuality,
} from "../api";

interface ChunkViewerProps {
  document: RagDocument;
  onClose: () => void;
}

export const ChunkViewer: React.FC<ChunkViewerProps> = ({ document, onClose }) => {
  const [chunks, setChunks] = useState<ChunkWithQuality[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [qualityFilter, setQualityFilter] = useState<number>(0);
  const [showLowQualityOnly, setShowLowQualityOnly] = useState(false);
  const [editingChunkId, setEditingChunkId] = useState<number | null>(null);
  const [editContent, setEditContent] = useState("");
  const [qualityAnalysis, setQualityAnalysis] = useState<{
    avg_chunk_quality: number;
    extraction_quality: string;
    issues: string[];
    total_chunks: number;
    low_quality_chunk_count: number;
  } | null>(null);

  const loadChunks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getChunksWithQuality(document.id);
      setChunks(data);

      // Also load quality analysis
      const analysis = await analyzeDocumentQuality(document.id);
      setQualityAnalysis(analysis);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load chunks");
    } finally {
      setLoading(false);
    }
  }, [document.id]);

  React.useEffect(() => {
    loadChunks();
  }, [loadChunks]);

  const handleFilterLowQuality = async () => {
    if (showLowQualityOnly) {
      // Reset to show all
      await loadChunks();
      setShowLowQualityOnly(false);
    } else {
      // Filter to low quality only
      setLoading(true);
      try {
        const lowQuality = await filterLowQualityChunks(document.id, qualityFilter || 0.5);
        setChunks(lowQuality);
        setShowLowQualityOnly(true);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to filter chunks");
      } finally {
        setLoading(false);
      }
    }
  };

  const handleDeleteChunk = async (chunkId: number) => {
    if (!confirm("Are you sure you want to delete this chunk?")) return;

    try {
      await deleteChunk(chunkId);
      setChunks((prev) => prev.filter((c) => c.chunk.id !== chunkId));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete chunk");
    }
  };

  const handleEditChunk = (chunk: ChunkWithQuality) => {
    setEditingChunkId(chunk.chunk.id);
    setEditContent(chunk.chunk.content);
  };

  const handleSaveEdit = async () => {
    if (editingChunkId === null) return;

    try {
      await updateChunkContent(editingChunkId, editContent);
      // Reload chunks to get updated quality scores
      await loadChunks();
      setEditingChunkId(null);
      setEditContent("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update chunk");
    }
  };

  const handleReembed = async (chunkId: number) => {
    try {
      await reembedChunk(chunkId);
      // Reload to update embedding status
      await loadChunks();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to re-embed chunk");
    }
  };

  const getQualityColor = (score: number): string => {
    if (score >= 0.8) return "bg-green-500";
    if (score >= 0.6) return "bg-yellow-500";
    if (score >= 0.4) return "bg-orange-500";
    return "bg-red-500";
  };

  const getQualityLabel = (score: number): string => {
    if (score >= 0.8) return "Excellent";
    if (score >= 0.6) return "Good";
    if (score >= 0.4) return "Fair";
    return "Poor";
  };

  const filteredChunks = chunks.filter((c) =>
    qualityFilter > 0 ? c.quality_score >= qualityFilter : true
  );

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.95, opacity: 0 }}
        className="bg-neutral-900 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-4 border-b border-neutral-700 flex justify-between items-center">
          <div>
            <h2 className="text-lg font-semibold text-white">
              Chunk Viewer: {document.file_name}
            </h2>
            {qualityAnalysis && (
              <div className="text-sm text-neutral-400 mt-1">
                {qualityAnalysis.total_chunks} chunks | Avg Quality:{" "}
                {(qualityAnalysis.avg_chunk_quality * 100).toFixed(0)}% |{" "}
                {qualityAnalysis.extraction_quality}
              </div>
            )}
          </div>
          <button
            onClick={onClose}
            className="text-neutral-400 hover:text-white transition-colors"
          >
            <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Quality Issues Alert */}
        {qualityAnalysis && qualityAnalysis.issues.length > 0 && (
          <div className="p-3 bg-yellow-500/10 border-b border-yellow-500/30">
            <div className="text-yellow-400 text-sm font-medium mb-1">Quality Issues Detected:</div>
            <ul className="text-yellow-300/80 text-xs space-y-0.5">
              {qualityAnalysis.issues.map((issue, i) => (
                <li key={i}>• {issue}</li>
              ))}
            </ul>
          </div>
        )}

        {/* Filters */}
        <div className="p-4 border-b border-neutral-700 flex gap-4 items-center flex-wrap">
          <div className="flex items-center gap-2">
            <label className="text-sm text-neutral-400">Min Quality:</label>
            <select
              value={qualityFilter}
              onChange={(e) => setQualityFilter(parseFloat(e.target.value))}
              className="bg-neutral-800 text-white text-sm rounded px-2 py-1 border border-neutral-600"
            >
              <option value={0}>All</option>
              <option value={0.3}>30%+</option>
              <option value={0.5}>50%+</option>
              <option value={0.7}>70%+</option>
              <option value={0.9}>90%+</option>
            </select>
          </div>

          <button
            onClick={handleFilterLowQuality}
            className={`px-3 py-1 rounded text-sm transition-colors ${
              showLowQualityOnly
                ? "bg-red-600 text-white"
                : "bg-neutral-700 text-neutral-300 hover:bg-neutral-600"
            }`}
          >
            {showLowQualityOnly ? "Show All" : "Show Low Quality Only"}
          </button>

          <button
            onClick={loadChunks}
            className="px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white rounded text-sm transition-colors"
          >
            Refresh
          </button>

          <div className="ml-auto text-sm text-neutral-400">
            Showing {filteredChunks.length} of {chunks.length} chunks
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="p-3 bg-red-500/10 border-b border-red-500/30">
            <div className="text-red-400 text-sm">{error}</div>
          </div>
        )}

        {/* Chunks List */}
        <div className="flex-1 overflow-y-auto p-4">
          {loading ? (
            <div className="flex items-center justify-center h-32">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            </div>
          ) : (
            <div className="space-y-3">
              <AnimatePresence>
                {filteredChunks.map((chunkData, index) => (
                  <motion.div
                    key={chunkData.chunk.id}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -10 }}
                    transition={{ delay: index * 0.02 }}
                    className="bg-neutral-800 rounded-lg p-4 border border-neutral-700"
                  >
                    {/* Chunk Header */}
                    <div className="flex justify-between items-start mb-2">
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-neutral-500">
                          #{chunkData.chunk.chunk_index + 1}
                        </span>
                        {chunkData.chunk.page_number && (
                          <span className="text-xs text-neutral-400 bg-neutral-700 px-2 py-0.5 rounded">
                            Page {chunkData.chunk.page_number}
                          </span>
                        )}
                        <div
                          className={`w-2 h-2 rounded-full ${getQualityColor(
                            chunkData.quality_score
                          )}`}
                          title={`Quality: ${(chunkData.quality_score * 100).toFixed(0)}%`}
                        />
                        <span className="text-xs text-neutral-400">
                          {getQualityLabel(chunkData.quality_score)} (
                          {(chunkData.quality_score * 100).toFixed(0)}%)
                        </span>
                        {chunkData.has_embedding ? (
                          <span className="text-xs text-green-400 bg-green-500/20 px-2 py-0.5 rounded">
                            Embedded
                          </span>
                        ) : (
                          <span className="text-xs text-yellow-400 bg-yellow-500/20 px-2 py-0.5 rounded">
                            No Embedding
                          </span>
                        )}
                      </div>

                      <div className="flex gap-1">
                        <button
                          onClick={() => handleEditChunk(chunkData)}
                          className="p-1 text-neutral-400 hover:text-blue-400 transition-colors"
                          title="Edit"
                        >
                          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                          </svg>
                        </button>
                        {!chunkData.has_embedding && (
                          <button
                            onClick={() => handleReembed(chunkData.chunk.id)}
                            className="p-1 text-neutral-400 hover:text-green-400 transition-colors"
                            title="Generate Embedding"
                          >
                            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                          </button>
                        )}
                        <button
                          onClick={() => handleDeleteChunk(chunkData.chunk.id)}
                          className="p-1 text-neutral-400 hover:text-red-400 transition-colors"
                          title="Delete"
                        >
                          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                          </svg>
                        </button>
                      </div>
                    </div>

                    {/* Chunk Content */}
                    {editingChunkId === chunkData.chunk.id ? (
                      <div className="space-y-2">
                        <textarea
                          value={editContent}
                          onChange={(e) => setEditContent(e.target.value)}
                          className="w-full h-32 bg-neutral-900 text-neutral-200 text-sm p-2 rounded border border-neutral-600 resize-y"
                        />
                        <div className="flex gap-2">
                          <button
                            onClick={handleSaveEdit}
                            className="px-3 py-1 bg-green-600 hover:bg-green-500 text-white rounded text-sm transition-colors"
                          >
                            Save
                          </button>
                          <button
                            onClick={() => {
                              setEditingChunkId(null);
                              setEditContent("");
                            }}
                            className="px-3 py-1 bg-neutral-700 hover:bg-neutral-600 text-white rounded text-sm transition-colors"
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <p className="text-neutral-300 text-sm whitespace-pre-wrap line-clamp-4">
                        {chunkData.chunk.content}
                      </p>
                    )}

                    {/* Chunk Footer */}
                    <div className="mt-2 text-xs text-neutral-500 flex gap-4">
                      <span>{chunkData.chunk.content.length} chars</span>
                      <span>~{chunkData.token_estimate} tokens</span>
                    </div>
                  </motion.div>
                ))}
              </AnimatePresence>

              {filteredChunks.length === 0 && !loading && (
                <div className="text-center text-neutral-500 py-8">
                  No chunks found matching the filter criteria.
                </div>
              )}
            </div>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
};

export default ChunkViewer;
