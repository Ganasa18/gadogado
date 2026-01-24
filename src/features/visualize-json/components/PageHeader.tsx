import { Code, Layout, Share2, Table as TableIcon } from "lucide-react";

import type { ViewMode } from "../VisualizeJsonPage";

interface PageHeaderProps {
  viewMode: ViewMode;
  onChangeViewMode: (mode: ViewMode) => void;
}

export default function PageHeader({ viewMode, onChangeViewMode }: PageHeaderProps) {
  return (
    <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-6">
      <div>
        <h1 className="text-3xl font-extrabold tracking-tight text-app-text">
          Visual Event Inspector
        </h1>
        <p className="text-app-subtext text-sm mt-1 flex items-center gap-2">
          <span className="flex h-2 w-2 rounded-full bg-app-success animate-pulse"></span>
          Modern Node-Based JSON Analysis Engine
        </p>
      </div>

      <div className="flex items-center gap-2 bg-app-panel p-1 rounded-xl border border-app-border">
        <button
          onClick={() => onChangeViewMode("list")}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
            viewMode === "list"
              ? "bg-app-accent text-white"
              : "text-app-subtext hover:text-app-text"
          }`}
        >
          <Layout size={14} /> Tree
        </button>
        <button
          onClick={() => onChangeViewMode("graph")}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
            viewMode === "graph"
              ? "bg-app-accent text-white"
              : "text-app-subtext hover:text-app-text"
          }`}
        >
          <Share2 size={14} /> Graph
        </button>
        <button
          onClick={() => onChangeViewMode("table")}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
            viewMode === "table"
              ? "bg-app-accent text-white"
              : "text-app-subtext hover:text-app-text"
          }`}
        >
          <TableIcon size={14} /> Table
        </button>
        <button
          onClick={() => onChangeViewMode("json")}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
            viewMode === "json"
              ? "bg-app-accent text-white"
              : "text-app-subtext hover:text-app-text"
          }`}
        >
          <Code size={14} /> JSON
        </button>
      </div>
    </div>
  );
}
