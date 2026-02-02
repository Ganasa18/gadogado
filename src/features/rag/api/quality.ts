import { invoke } from "@tauri-apps/api/core";
import type { RagDocument } from "../types";

export interface CollectionQualityMetrics {
  id: number;
  collection_id: number;
  computed_at: string;
  avg_quality_score: number | null;
  avg_ocr_confidence: number | null;
  total_documents: number;
  documents_with_warnings: number;
  total_chunks: number;
  avg_chunk_quality: number | null;
  best_reranker: string | null;
  reranker_score: number | null;
}

export interface DocumentWarning {
  id: number;
  doc_id: number;
  warning_type: string;
  page_number: number | null;
  chunk_index: number | null;
  severity: string;
  message: string;
  suggestion: string | null;
  created_at: string;
}

export interface DocumentWarningInput {
  doc_id: number;
  warning_type: string;
  page_number?: number;
  chunk_index?: number;
  severity: string;
  message: string;
  suggestion?: string;
}

export interface RetrievalGap {
  id: number;
  collection_id: number;
  query_hash: string;
  query_length: number | null;
  result_count: number | null;
  max_confidence: number | null;
  avg_confidence: number | null;
  gap_type: string | null;
  created_at: string;
}

export interface RetrievalGapInput {
  collection_id: number;
  query_hash: string;
  query_length?: number;
  result_count?: number;
  max_confidence?: number;
  avg_confidence?: number;
  gap_type?: string;
}

export async function getCollectionQuality(
  collectionId: number,
): Promise<CollectionQualityMetrics | null> {
  return await invoke<CollectionQualityMetrics | null>(
    "rag_get_collection_quality",
    {
      collectionId,
    },
  );
}

export async function computeCollectionQuality(
  collectionId: number,
): Promise<CollectionQualityMetrics> {
  return await invoke<CollectionQualityMetrics>(
    "rag_compute_collection_quality",
    {
      collectionId,
    },
  );
}

export async function getDocumentWarnings(docId: number): Promise<DocumentWarning[]> {
  return await invoke<DocumentWarning[]>("rag_get_document_warnings", {
    doc_id: docId,
  });
}

export async function createDocumentWarning(
  input: DocumentWarningInput,
): Promise<DocumentWarning> {
  return await invoke<DocumentWarning>("rag_create_document_warning", { input });
}

export async function getLowQualityDocuments(
  collectionId: number,
  threshold?: number,
  limit?: number,
): Promise<RagDocument[]> {
  return await invoke<RagDocument[]>("rag_get_low_quality_documents", {
    collectionId,
    threshold,
    limit,
  });
}

export async function recordRetrievalGap(input: RetrievalGapInput): Promise<RetrievalGap> {
  return await invoke<RetrievalGap>("rag_record_retrieval_gap", { input });
}

export async function getRetrievalGaps(
  collectionId: number,
  limit?: number,
): Promise<RetrievalGap[]> {
  return await invoke<RetrievalGap[]>("rag_get_retrieval_gaps", {
    collectionId,
    limit,
  });
}
