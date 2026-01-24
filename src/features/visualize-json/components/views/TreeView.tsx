import JsonListView from "../JsonListView";
import type { JsonNode } from "../../types";

interface TreeViewProps {
  json: JsonNode;
  onToggle: (path: string) => void;
  mode: "dark" | "light" | "system";
}

export default function TreeView({ json, onToggle, mode }: TreeViewProps) {
  return (
    <div className="flex-1 overflow-auto p-6 bg-app-bg/50">
      <JsonListView json={json} onToggle={onToggle} mode={mode} />
    </div>
  );
}
