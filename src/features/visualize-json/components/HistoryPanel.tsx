import { Clock, FileText, Play, X } from "lucide-react";

import type { HistoryItem } from "../types";

interface HistoryPanelProps {
  history: HistoryItem[];
  onLoad: (itemId: string) => void;
  onRemove: (itemId: string) => void;
}

export default function HistoryPanel({ history, onLoad, onRemove }: HistoryPanelProps) {
  if (!history.length) return null;

  return (
    <div className="bg-app-card rounded-lg border border-app-border shadow-sm overflow-hidden">
      <div className="p-3 border-b border-app-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Clock className="text-app-accent" size={16} />
          <h3 className="text-xs font-bold uppercase tracking-wider text-app-text">History</h3>
        </div>
        <span className="text-[10px] text-app-subtext">{history.length} items</span>
      </div>

      <div className="max-h-48 overflow-y-auto custom-scrollbar">
        {history.map((item) => (
          <div
            key={item.id}
            className="group p-3 border-b border-app-border/50 hover:bg-app-panel/50 transition cursor-pointer"
          >
            <div className="flex items-start gap-2">
              <FileText size={14} className="text-app-subtext mt-0.5" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs font-semibold text-app-text truncate">
                    {item.filename || "Untitled"}
                  </span>
                  <span
                    className={`text-[9px] px-1.5 py-0.5 rounded uppercase font-bold ${
                      item.format === "json"
                        ? "bg-blue-500/10 text-blue-400"
                        : item.format === "yaml"
                          ? "bg-purple-500/10 text-purple-400"
                          : item.format === "toml"
                            ? "bg-orange-500/10 text-orange-400"
                            : item.format === "xml"
                              ? "bg-green-500/10 text-green-400"
                              : "bg-app-panel text-app-subtext"
                    }`}
                  >
                    {item.format}
                  </span>
                </div>
                <div className="text-[10px] text-app-subtext">
                  {new Date(item.timestamp).toLocaleString()}
                </div>
              </div>

              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition">
                <button
                  onClick={() => onLoad(item.id)}
                  className="p-1 rounded hover:bg-app-accent/20 hover:text-app-accent transition"
                  title="Load this item"
                >
                  <Play size={12} />
                </button>
                <button
                  onClick={() => onRemove(item.id)}
                  className="p-1 rounded hover:bg-red-500/20 hover:text-red-500 transition"
                  title="Remove from history"
                >
                  <X size={12} />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
