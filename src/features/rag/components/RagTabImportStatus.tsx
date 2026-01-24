import { X } from "lucide-react";
import { getStatusIcon, type ImportProgress } from "../ragTabUtils";

type Props = {
  progress: ImportProgress;
  onClear: () => void;
};

export function RagTabImportStatus({ progress, onClear }: Props) {
  if (progress.status === "idle") return null;

  const className =
    progress.status === "complete"
      ? "bg-green-500/10 border-green-500/30"
      : progress.status === "error"
        ? "bg-red-500/10 border-red-500/30"
        : "bg-app-accent/10 border-app-accent/30";

  return (
    <div className={`mx-6 mt-4 p-4 rounded-lg border flex items-center gap-3 ${className}`}>
      {getStatusIcon(progress.status)}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-app-text">{progress.message}</div>
        {progress.error && <div className="text-xs text-red-500 mt-1">{progress.error}</div>}
      </div>
      {(progress.status === "complete" || progress.status === "error") && (
        <button onClick={onClear} className="p-1 text-app-text-muted hover:text-app-text transition-colors">
          <X className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}
