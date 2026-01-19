import React, { useState, useCallback } from "react";
import { motion } from "framer-motion";
import {
  preprocessCsvFile,
  previewCsvRows,
  analyzeCsv,
  type CsvPreprocessingResponse,
  type CsvPreviewRow,
} from "../api";

interface CsvUploadPreviewProps {
  onProceed?: (result: CsvPreprocessingResponse) => void;
  onCancel?: () => void;
}

export const CsvUploadPreview: React.FC<CsvUploadPreviewProps> = ({
  onProceed,
  onCancel,
}) => {
  const [filePath, setFilePath] = useState<string>("");
  const [preprocessingResult, setPreprocessingResult] =
    useState<CsvPreprocessingResponse | null>(null);
  const [previewRows, setPreviewRows] = useState<CsvPreviewRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showFullPreview, setShowFullPreview] = useState(false);

  const handleFileSelect = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      // Use Tauri file dialog
      const selected = await (window as any).__TAURI__.dialog.open({
        multiple: false,
        filters: [
          {
            name: "CSV",
            extensions: ["csv"],
          },
        ],
      });

      if (!selected) return;

      setFilePath(selected as string);

      // Analyze the CSV first
      const analysis = await analyzeCsv(selected as string);
      console.log("CSV Analysis:", analysis);

      // Preprocess the CSV
      const result = await preprocessCsvFile({ filePath: selected as string });
      setPreprocessingResult(result);

      // Get preview rows (first 5)
      const preview = await previewCsvRows(selected as string, 5);
      setPreviewRows(preview);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to process CSV");
      console.error("CSV processing error:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleProceed = useCallback(() => {
    if (preprocessingResult && onProceed) {
      onProceed(preprocessingResult);
    }
  }, [preprocessingResult, onProceed]);

  const getContentTypeBadge = () => {
    if (!preprocessingResult) return null;
    const isNarrative = preprocessingResult.contentType.includes("Narrative");
    return (
      <span
        className={`px-2 py-1 rounded text-xs font-medium ${
          isNarrative
            ? "bg-blue-100 text-blue-700"
            : "bg-green-100 text-green-700"
        }`}
      >
        {preprocessingResult.contentType}
      </span>
    );
  };

  const getConfidenceBadge = () => {
    if (!preprocessingResult) return null;
    const confidence = preprocessingResult.analysis.confidenceScore;
    const color =
      confidence > 0.8
        ? "bg-green-100 text-green-700"
        : confidence > 0.5
        ? "bg-yellow-100 text-yellow-700"
        : "bg-red-100 text-red-700";

    return (
      <span className={`px-2 py-1 rounded text-xs font-medium ${color}`}>
        {Math.round(confidence * 100)}% confidence
      </span>
    );
  };

  return (
    <div className="p-6 bg-white rounded-lg shadow-lg">
      <h2 className="text-2xl font-bold mb-4">CSV Upload & Preview</h2>

      {!filePath ? (
        <div>
          <button
            onClick={handleFileSelect}
            disabled={loading}
            className="px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-400 transition-colors"
          >
            {loading ? "Loading..." : "Select CSV File"}
          </button>
          {error && <p className="mt-4 text-red-500">{error}</p>}
        </div>
      ) : (
        <div className="space-y-6">
          {/* File Info */}
          <div className="p-4 bg-gray-50 rounded-lg">
            <h3 className="font-semibold mb-2">File: {filePath.split(/[/\\]/).pop()}</h3>
            {preprocessingResult && (
              <div className="flex flex-wrap gap-2">
                {getContentTypeBadge()}
                {getConfidenceBadge()}
                <span className="px-2 py-1 rounded text-xs font-medium bg-gray-100 text-gray-700">
                  {preprocessingResult.rowCount} rows
                </span>
                <span className="px-2 py-1 rounded text-xs font-medium bg-gray-100 text-gray-700">
                  {preprocessingResult.headers.length} columns
                </span>
              </div>
            )}
          </div>

          {/* Analysis Results */}
          {preprocessingResult && (
            <div className="grid grid-cols-2 gap-4">
              <div className="p-4 bg-blue-50 rounded-lg">
                <p className="text-sm text-blue-600 font-medium">
                  Average Field Length
                </p>
                <p className="text-2xl font-bold text-blue-700">
                  {preprocessingResult.analysis.avgValueLength.toFixed(1)} chars
                </p>
              </div>
              <div className="p-4 bg-purple-50 rounded-lg">
                <p className="text-sm text-purple-600 font-medium">
                  Lexical Diversity
                </p>
                <p className="text-2xl font-bold text-purple-700">
                  {(preprocessingResult.analysis.lexicalDiversity * 100).toFixed(1)}%
                </p>
              </div>
              <div className="p-4 bg-green-50 rounded-lg">
                <p className="text-sm text-green-600 font-medium">Numeric Ratio</p>
                <p className="text-2xl font-bold text-green-700">
                  {(preprocessingResult.analysis.numericRatio * 100).toFixed(1)}%
                </p>
              </div>
              <div className="p-4 bg-orange-50 rounded-lg">
                <p className="text-sm text-orange-600 font-medium">
                  Processing Time
                </p>
                <p className="text-2xl font-bold text-orange-700">
                  {preprocessingResult.processingTimeMs}ms
                </p>
              </div>
            </div>
          )}

          {/* Preview */}
          {previewRows.length > 0 && (
            <div>
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-semibold">Preview (first 5 rows)</h3>
                <button
                  onClick={() => setShowFullPreview(!showFullPreview)}
                  className="text-sm text-blue-500 hover:text-blue-700"
                >
                  {showFullPreview ? "Show less" : "Show more"}
                </button>
              </div>
              <div className="bg-gray-50 rounded-lg p-4 max-h-96 overflow-y-auto">
                <pre className="text-sm whitespace-pre-wrap font-mono">
                  {showFullPreview
                    ? preprocessingResult?.processedText
                    : previewRows.map((row) => row.content).join("\n\n---\n\n")}
                </pre>
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex gap-4 justify-end">
            {onCancel && (
              <button
                onClick={onCancel}
                className="px-6 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
              >
                Cancel
              </button>
            )}
            <button
              onClick={handleFileSelect}
              className="px-6 py-2 border border-blue-500 text-blue-500 rounded-lg hover:bg-blue-50 transition-colors"
            >
              Change File
            </button>
            {onProceed && (
              <button
                onClick={handleProceed}
                disabled={!preprocessingResult}
                className="px-6 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                Proceed to Import
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
