import { invoke } from "@tauri-apps/api/core";

export interface CsvPreprocessingRequest {
  filePath: string;
  config?: CsvPreprocessingRequestConfig;
}

export interface CsvPreprocessingRequestConfig {
  minValueLengthThreshold?: number;
  minLexicalDiversity?: number;
  maxNumericRatio?: number;
  minSampleRows?: number;
  maxSampleRows?: number;
}

export interface CsvFieldAnalysis {
  avgValueLength: number;
  lexicalDiversity: number;
  totalFields: number;
  numericRatio: number;
  rowCount: number;
  emptyFieldCount: number;
  maxValueLength: number;
  minValueLength: number;
  confidenceScore: number;
}

export interface CsvPreprocessingResponse {
  contentType: string;
  processedText: string;
  rowCount: number;
  analysis: CsvFieldAnalysis;
  headers: string[];
  processingTimeMs: number;
}

export interface CsvPreviewRow {
  index: number;
  content: string;
}

export async function preprocessCsvFile(
  request: CsvPreprocessingRequest,
): Promise<CsvPreprocessingResponse> {
  return await invoke<CsvPreprocessingResponse>("csv_preprocess_file", { request });
}

export async function previewCsvRows(
  filePath: string,
  previewCount: number,
): Promise<CsvPreviewRow[]> {
  return await invoke<CsvPreviewRow[]>("csv_preview_rows", {
    filePath,
    previewCount,
  });
}

export async function analyzeCsv(filePath: string): Promise<string> {
  return await invoke<string>("csv_analyze", { filePath });
}
