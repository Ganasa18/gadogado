import { Minimize2, Search } from "lucide-react";

import JsonListView from "./JsonListView";
import type { JsonNode } from "../types";

interface HierarchyPanelProps {
  json: JsonNode | null;
  focusedPath: string;
  onToggleNode: (path: string) => void;
  onSelectPath: (path: string) => void;
}

export default function HierarchyPanel({
  json,
  focusedPath,
  onToggleNode,
  onSelectPath,
}: HierarchyPanelProps) {
  return (
    <div className="flex-1 overflow-hidden bg-app-card rounded-lg border border-app-border shadow-sm flex flex-col">
      <div className="p-4 py-3 border-b border-app-border flex-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Minimize2 className="text-purple-400" size={18} />
            <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">Hierarchy</h3>
          </div>
          {json && (
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-app-panel text-app-subtext border border-app-border">
              Sync Enabled
            </span>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-2 custom-scrollbar">
        {!json ? (
          <div className="h-full flex flex-col items-center justify-center text-app-subtext opacity-50">
            <Search className="mb-2" size={32} />
            <p className="text-xs font-medium uppercase tracking-widest">No data mapped</p>
          </div>
        ) : (
          <div className="tree-container">
            <JsonListView
              json={json}
              onToggle={onToggleNode}
              onSelect={onSelectPath}
              activePath={focusedPath}
            />
          </div>
        )}
      </div>
    </div>
  );
}
