import { useMemo, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  Panel,
  BackgroundVariant,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useJsonVisualization } from "./hooks/useJsonVisualization";
import { useGraphLayout } from "./hooks/useGraphLayout";
import JsonListView from "./components/JsonListView";
import CustomGraphNode from "./components/CustomGraphNode";
import { Button } from "../../shared/components/Button";
import { useThemeStore } from "../../features/theme/themeStore";
import { JsonNode, HistoryItem } from "./types";
import {
  Minimize2,
  Search,
  Layout,
  Table as TableIcon,
  FileJson,
  Trash2,
  Play,
  Share2,
  Download,
  Clock,
  X,
  FileText,
} from "lucide-react";

const VisualizeJsonPage = () => {
  const {
    state,
    saveToLocalStorage,
    clearLocalStorage,
    parseJson,
    parseData,
    createHistoryItem,
    loadFromHistory,
    removeFromHistory,
    toggleNode,
  } = useJsonVisualization();

  const [inputValue, setInputValue] = useState("");
  const { mode } = useThemeStore();

  const { nodes: layoutedNodes, edges: layoutedEdges } = useGraphLayout(
    state.json
  );

  const nodeTypes = useMemo(() => ({ custom: CustomGraphNode }), []);

  const handleParseJson = () => {
    const jsonNode = parseJson(inputValue);
    if (!jsonNode) {
      alert("Invalid JSON format. Please check your input.");
      return;
    }
    const parsedData = jsonNode.value;
    const newItem = createHistoryItem(parsedData, 'json', 'Pasted JSON');
    const updatedHistory = [newItem, ...state.history.slice(0, 9)];
    saveToLocalStorage({ ...state, json: jsonNode, history: updatedHistory });
    setInputValue("");
  };

  const handleFileImport = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const content = typeof reader.result === "string" ? reader.result : "";
        const extension = file.name.split(".").pop()?.toLowerCase() || "json";

        let format: HistoryItem["format"] = "json";
        if (extension === "yaml" || extension === "yml") format = "yaml";
        else if (extension === "toml") format = "toml";
        else if (extension === "xml") format = "xml";
        else if (extension === "csv") format = "csv";

        const jsonNode = parseData(content, format);
        if (!jsonNode) {
          alert(
            `Invalid ${extension.toUpperCase()} file. Please check the file content.`
          );
          return;
        }

        const parsedData = jsonNode.value;
        const newItem = createHistoryItem(parsedData, format, file.name, content);
        const updatedHistory = [newItem, ...state.history.slice(0, 9)];
        saveToLocalStorage({ ...state, json: jsonNode, history: updatedHistory });
        setInputValue("");
      } catch (error) {
        console.error("Error importing file:", error);
        alert(
          `Error importing file: ${
            error instanceof Error ? error.message : "Unknown error"
          }`
        );
      }
    };
    reader.onerror = () => {
      alert("Unable to read the selected file.");
    };
    reader.readAsText(file);
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      handleFileImport(file);
    }
    event.target.value = "";
  };

  const handleClear = () => {
    clearLocalStorage();
    setInputValue("");
  };

  const [viewMode, setViewMode] = useState<"list" | "graph" | "table">("graph");

  const getValuePreview = (node: JsonNode) => {
    if (node.type === "object") {
      return `{${node.children?.length ?? 0} keys}`;
    }
    if (node.type === "array") {
      return `[${node.children?.length ?? 0} items]`;
    }
    if (node.type === "string") {
      return `"${node.value}"`;
    }
    if (node.type === "boolean") {
      return String(node.value);
    }
    if (node.type === "null") {
      return "null";
    }
    return String(node.value ?? "");
  };

  const tableRows = useMemo(() => {
    if (!state.json) return [] as JsonNode[];
    const rows: JsonNode[] = [];
    const walk = (node: JsonNode) => {
      rows.push(node);
      node.children?.forEach(walk);
    };
    walk(state.json);
    return rows;
  }, [state.json]);

  return (
    <div className={`min-h-screen bg-app-bg text-app-text`}>
      <div className="container mx-auto p-4 md:p-6 max-w-[1920px]">
        {/* Header Section */}
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
              onClick={() => setViewMode("list")}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                viewMode === "list"
                  ? "bg-app-accent text-white"
                  : "text-app-subtext hover:text-app-text"
              }`}>
              <Layout size={14} /> Tree
            </button>
            <button
              onClick={() => setViewMode("graph")}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                viewMode === "graph"
                  ? "bg-app-accent text-white"
                  : "text-app-subtext hover:text-app-text"
              }`}>
              <Share2 size={14} /> Graph
            </button>
            <button
              onClick={() => setViewMode("table")}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                viewMode === "table"
                  ? "bg-app-accent text-white"
                  : "text-app-subtext hover:text-app-text"
              }`}>
              <TableIcon size={14} /> Table
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-[380px_1fr] h-[calc(100vh-160px)]">
          {/* Sidebar / Input Panel */}
          <div className="flex flex-col gap-6 overflow-hidden">
            <div className="flex-none bg-app-card rounded-lg border border-app-border shadow-sm p-4">
              <div className="flex items-center gap-2 mb-4">
                <FileJson className="text-app-accent" size={18} />
                <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">
                  Input Source
                </h3>
              </div>
              <div className="space-y-4">
                <div className="relative group">
                  <textarea
                    value={inputValue}
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                      setInputValue(e.target.value)
                    }
                    placeholder='{"event_id": "evt_001", "action": "checkout"}'
                    className="w-full h-48 rounded-lg border border-app-border bg-app-bg px-4 py-3 text-[13px] font-mono text-app-text focus:border-app-accent/50 focus:outline-none transition-all resize-none"
                  />
                  {/* <div className="absolute top-2 right-2 flex gap-1">
                    <button
                      onClick={() =>
                        setInputValue(
                          '{\n  "event_id": "evt_89402_x",\n  "timestamp": 169824921104,\n  "type": "interaction",\n  "trigger": "click",\n  "meta": {\n    "target_element": {\n      "tag": "button",\n      "id": "submit-login",\n      "class": "btn-primary large"\n    }\n  }\n}'
                        )
                      }
                      className="p-1.5 rounded-md bg-app-panel text-app-subtext hover:text-app-accent transition"
                      title="Insert Example">
                      <Play size={14} />
                    </button>
                  </div> */}
                </div>

                <div className="flex gap-2">
                  <Button
                    onClick={handleParseJson}
                    className="flex-1 bg-app-accent hover:bg-app-accent/90 text-white font-bold h-10">
                    Generate View
                  </Button>
                  <Button
                    variant="secondary"
                    onClick={handleClear}
                    className="px-3 border-app-border hover:bg-red-500/10 hover:text-red-500 hover:border-red-500/20">
                    <Trash2 size={18} />
                  </Button>
                </div>

                <div className="relative overflow-hidden rounded-lg border border-dashed border-app-border p-4 hover:border-app-subtext transition cursor-pointer group">
                  <input
                    type="file"
                    accept=".json,.yaml,.yml,.toml,.xml,.csv"
                    onChange={handleFileChange}
                    className="absolute inset-0 opacity-0 cursor-pointer z-10"
                  />
                  <div className="text-center">
                    <div className="inline-flex p-2 rounded-full bg-app-panel mb-2 group-hover:text-app-accent transition">
                      <Download size={16} />
                    </div>
                    <span className="text-xs font-semibold text-app-subtext block group-hover:text-app-text transition">
                      Import File (JSON/YAML/TOML/XML/CSV)
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Tree Explorer in Sidebar */}
            <div className="flex-1 overflow-hidden bg-app-card rounded-lg border border-app-border shadow-sm flex flex-col">
              <div className="p-4 py-3 border-b border-app-border flex-none">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Minimize2 className="text-purple-400" size={18} />
                    <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">
                      Hierarchy
                    </h3>
                  </div>
                  {state.json && (
                    <span className="text-[10px] px-2 py-0.5 rounded-full bg-app-panel text-app-subtext border border-app-border">
                      Sync Enabled
                    </span>
                  )}
                </div>
              </div>
              <div className="flex-1 overflow-auto p-2 custom-scrollbar">
                {!state.json ? (
                  <div className="h-full flex flex-col items-center justify-center text-app-subtext opacity-50">
                    <Search className="mb-2" size={32} />
                    <p className="text-xs font-medium uppercase tracking-widest">
                      No data mapped
                    </p>
                  </div>
                ) : (
                  <div className="tree-container">
                    <JsonListView json={state.json} onToggle={toggleNode} />
                  </div>
                )}
              </div>
            </div>

            {/* History List */}
            {state.history.length > 0 && (
              <div className="bg-app-card rounded-lg border border-app-border shadow-sm overflow-hidden">
                <div className="p-3 border-b border-app-border flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Clock className="text-app-accent" size={16} />
                    <h3 className="text-xs font-bold uppercase tracking-wider text-app-text">
                      History
                    </h3>
                  </div>
                  <span className="text-[10px] text-app-subtext">
                    {state.history.length} items
                  </span>
                </div>
                <div className="max-h-48 overflow-y-auto custom-scrollbar">
                  {state.history.map((item) => (
                    <div
                      key={item.id}
                      className="group p-3 border-b border-app-border/50 hover:bg-app-panel/50 transition cursor-pointer">
                      <div className="flex items-start gap-2">
                        <FileText
                          size={14}
                          className="text-app-subtext mt-0.5 flex-shrink-0"
                        />
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
                              }`}>
                              {item.format}
                            </span>
                          </div>
                          <div className="text-[10px] text-app-subtext">
                            {new Date(item.timestamp).toLocaleString()}
                          </div>
                        </div>
                        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition">
                          <button
                            onClick={() => loadFromHistory(item.id)}
                            className="p-1 rounded hover:bg-app-accent/20 hover:text-app-accent transition"
                            title="Load this item">
                            <Play size={12} />
                          </button>
                          <button
                            onClick={() => removeFromHistory(item.id)}
                            className="p-1 rounded hover:bg-red-500/20 hover:text-red-500 transition"
                            title="Remove from history">
                            <X size={12} />
                          </button>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Main Visualization Area */}
          <div className="flex flex-col h-full overflow-hidden rounded-lg border border-app-border bg-app-card/30 relative">
            {!state.json ? (
              <div className="absolute inset-0 flex flex-col items-center justify-center text-app-subtext bg-app-bg/50 z-10">
                <div className="w-20 h-20 bg-app-card rounded-3xl flex items-center justify-center mb-6 border border-app-border shadow-lg">
                  <Share2 className="text-app-accent opacity-50" size={40} />
                </div>
                <p className="text-xl font-bold tracking-tight text-app-text">
                  Ready to Visualize
                </p>
                <p className="text-sm text-app-subtext mt-2">
                  Paste your JSON data on the left to start exploration.
                </p>
              </div>
            ) : viewMode === "graph" ? (
              <ReactFlow
                nodes={layoutedNodes}
                edges={layoutedEdges}
                nodeTypes={nodeTypes}
                fitView
                fitViewOptions={{ padding: 0.2 }}
                minZoom={0.1}
                maxZoom={4}
                defaultEdgeOptions={{
                  type: "smoothstep",
                  animated: true,
                  style: { 
                    stroke: mode === 'dark' ? '#6b7280' : '#d1d5db',
                    strokeWidth: 1.5,
                    opacity: 0.6 
                  },
                }}
                className="bg-app-bg">
                <Background
                  color={mode === 'dark' ? '#6b7280' : '#d1d5db'}
                  variant={BackgroundVariant.Dots}
                  gap={20}
                  size={1}
                  className="opacity-20"
                />
                <Controls 
                  className="!bg-app-panel/95 !border-app-border !text-app-text rounded-lg !backdrop-blur-sm"
                  style={{ 
                    '--xy-controls-button-background': 'var(--color-app-bg)',
                    '--xy-controls-button-hover-background': 'var(--color-app-accent)',
                    '--xy-controls-button-text': 'var(--color-app-text)',
                    '--xy-controls-button-hover-text': 'white',
                    '--xy-controls-border': 'var(--color-app-border)',
                  } as React.CSSProperties}
                />
                {/* <MiniMap
                  nodeColor={(node: any) => {
                    const type = node.data?.type;
                    if (type === "object") return "#3b82f6";
                    if (type === "array") return "#10b981";
                    return "#3f3f46";
                  }}
                  maskColor="rgba(24, 24, 24, 0.7)"
                  className="!bg-app-panel !border-app-border !bottom-6 !right-6 rounded-xl overflow-hidden border shadow-lg"
                /> */}

                <Panel
                  position="top-left"
                  className="bg-app-panel/90 border border-app-border p-3 rounded-xl text-[11px] text-app-subtext shadow-lg flex flex-col gap-2">
                  <div className="font-bold text-app-text uppercase tracking-widest text-[10px] mb-1">
                    Key Types
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-blue-400"></div>
                    <span className="flex-1">Object</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-green-400"></div>
                    <span className="flex-1">Array</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-orange-400"></div>
                    <span className="flex-1">Value</span>
                  </div>
                </Panel>
              </ReactFlow>
            ) : viewMode === "table" ? (
              <div className="flex-1 overflow-auto p-0 custom-scrollbar bg-app-bg/50">
                <table className="w-full text-sm border-collapse">
                  <thead className="sticky top-0 bg-app-panel z-10 border-b border-app-border">
                    <tr>
                      <th className="text-left p-4 pl-6 font-bold text-app-subtext uppercase tracking-widest text-[11px]">
                        Path
                      </th>
                      <th className="text-left p-4 font-bold text-app-subtext uppercase tracking-widest text-[11px]">
                        Type
                      </th>
                      <th className="text-left p-4 pr-6 font-bold text-app-subtext uppercase tracking-widest text-[11px]">
                        Preview
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-app-border">
                    {tableRows.map((row) => (
                      <tr
                        key={row.path}
                        className="group hover:bg-app-panel/50 transition-colors">
                        <td className="p-3 pl-6 font-mono text-xs text-app-accent/70 transition-colors group-hover:text-app-accent">
                          {row.path}
                        </td>
                        <td className="p-3">
                          <span
                            className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wide border ${
                              row.type === "object"
                                ? "bg-blue-500/10 text-blue-400 border-blue-500/20"
                                : row.type === "array"
                                ? "bg-green-500/10 text-green-500 border-green-500/20"
                                : "bg-app-panel text-app-subtext border-app-border"
                            }`}>
                            {row.type}
                          </span>
                        </td>
                        <td className="p-3 pr-6 font-mono text-xs text-app-subtext group-hover:text-app-text truncate max-w-md">
                          {getValuePreview(row)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="flex-1 overflow-auto p-6 bg-app-bg/50">
                <JsonListView
                  json={state.json}
                  onToggle={toggleNode}
                  mode={mode}
                />
              </div>
            )}
          </div>
        </div>
      </div>

      <style>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 8px;
          height: 8px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: transparent;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: var(--color-app-border);
          border-radius: 4px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: var(--color-app-subtext);
        }
      `}</style>
    </div>
  );
};

export default VisualizeJsonPage;
