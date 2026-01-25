import { Database, ChevronRight } from "lucide-react";

interface EmptyStateProps {
  onInitialize: () => void;
}

export function EmptyState({ onInitialize }: EmptyStateProps) {
  return (
    <div className="bg-app-panel border border-app-border rounded-xl p-12 flex flex-col items-center justify-center text-center shadow-xl">
      <div className="w-16 h-16 rounded-2xl bg-app-card flex items-center justify-center text-app-subtext mb-6">
        <Database className="w-8 h-8 opacity-40" />
      </div>
      <h3 className="text-xl font-bold text-app-text mb-2">No connectors established</h3>
      <p className="text-app-subtext max-w-sm mb-8">
        Start by defining your first flat-schemed database source to enable hybrid RAG operations.
      </p>
      <button
        onClick={onInitialize}
        className="flex items-center gap-2 text-app-accent font-bold text-sm group"
      >
        Initialize Connection
        <ChevronRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
      </button>
    </div>
  );
}
