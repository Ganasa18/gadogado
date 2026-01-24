import {
  AlertCircle,
  CheckCircle2,
  FileCheck,
  FileCode,
  FileSpreadsheet,
  FileText,
  Loader2,
} from "lucide-react";

export const SUPPORTED_EXTENSIONS = ["pdf", "docx", "xlsx", "csv", "txt", "web"];

export type ImportStatus =
  | "idle"
  | "validating"
  | "preprocessing"
  | "processing"
  | "chunking"
  | "embedding"
  | "complete"
  | "error";

export interface ImportProgress {
  status: ImportStatus;
  message: string;
  fileName?: string;
  error?: string;
}

export function getFileExtension(filePath: string): string {
  const parts = filePath.split(".");
  return (parts[parts.length - 1] ?? "").toLowerCase();
}

export function isSupportedFile(filePath: string): boolean {
  const ext = getFileExtension(filePath);
  return SUPPORTED_EXTENSIONS.includes(ext);
}

export function getFileIcon(fileType: string) {
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

export function getStatusIcon(status: ImportStatus) {
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

export function getStatusMessage(status: ImportStatus, fileName?: string): string {
  const name = fileName ? `"${fileName}"` : "file";
  switch (status) {
    case "validating":
      return `Validating ${name}...`;
    case "preprocessing":
      return "Preprocessing CSV (detecting content type)...";
    case "processing":
      return `Parsing ${name}...`;
    case "chunking":
      return "Creating smart chunks...";
    case "embedding":
      return "Generating embeddings...";
    case "complete":
      return `${name} imported successfully. Ready for chat.`;
    case "error":
      return `Failed to import ${name}`;
    default:
      return "";
  }
}
