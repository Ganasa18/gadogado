import { useMemo, useState, useEffect } from "react";
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
  Code,
  Copy,
  Check,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Wand2,
  RefreshCw,
  Undo2,
  Loader2,
  Braces,
  Brackets,
  Type,
  Hash,
  ToggleLeft,
  Ban,
} from "lucide-react";
import { useFixJsonAI } from "./hooks/useFixJsonAI";

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
  const [originalInputValue, setOriginalInputValue] = useState("");
  const [hasAiFixed, setHasAiFixed] = useState(false);
  const { mode } = useThemeStore();
  const { fixJson, isFixing, error: aiError, clearError: clearAiError } = useFixJsonAI();

  const { nodes: layoutedNodes, edges: layoutedEdges, isTooLarge: isGraphTooLarge } = useGraphLayout(
    state.json
  );

  const nodeTypes = useMemo(() => ({ custom: CustomGraphNode }), []);

  const handleParseJson = () => {
    const jsonNode = parseJson(inputValue);
    if (!jsonNode) {
      alert("Invalid JSON format. Please check your input.");
      return;
    }
    const newItem = createHistoryItem(jsonNode.value, "json", "Pasted JSON");
    saveToLocalStorage({ ...state, json: jsonNode, history: [newItem, ...state.history.slice(0, 9)] });
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

        const newItem = createHistoryItem(jsonNode.value, format, file.name, content);
        saveToLocalStorage({ ...state, json: jsonNode, history: [newItem, ...state.history.slice(0, 9)] });
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
    setOriginalInputValue("");
    setHasAiFixed(false);
    clearAiError();
  };

  const handleFixWithAI = async () => {
    if (!inputValue.trim() || isFixing) return;

    // Store original value for revert
    if (!hasAiFixed) {
      setOriginalInputValue(inputValue);
    }

    const fixedJson = await fixJson(inputValue);
    if (fixedJson) {
      setInputValue(fixedJson);
      setHasAiFixed(true);
    }
  };

  const handleRevertToOriginal = () => {
    if (originalInputValue) {
      setInputValue(originalInputValue);
      setHasAiFixed(false);
      clearAiError();
    }
  };

  const handleRegenerateAiFix = async () => {
    if (!originalInputValue.trim() || isFixing) return;

    const fixedJson = await fixJson(originalInputValue);
    if (fixedJson) {
      setInputValue(fixedJson);
    }
  };

  useEffect(() => {
    const handleToggleNode = (event: Event) => {
      const customEvent = event as CustomEvent<string>;
      toggleNode(customEvent.detail);
    };
    window.addEventListener("toggleNode", handleToggleNode);
    return () => {
      window.removeEventListener("toggleNode", handleToggleNode);
    };
  }, [toggleNode]);

  const [viewMode, setViewMode] = useState<"list" | "graph" | "table" | "json">(
    "graph"
  );
  const [copiedToClipboard, setCopiedToClipboard] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [rowsPerPage, setRowsPerPage] = useState(15);
  const [focusedPath, setFocusedPath] = useState<string>("$");

  // Node detail modal state
  const [nodeDetailModal, setNodeDetailModal] = useState<{
    show: boolean;
    label: string;
    type: string;
    value: unknown;
    path: string;
  } | null>(null);
  const [copiedContent, setCopiedContent] = useState(false);
  const [copiedPath, setCopiedPath] = useState(false);

  // Listen for node click events from CustomGraphNode
  useEffect(() => {
    const handleShowNodeDetail = (event: Event) => {
      const customEvent = event as CustomEvent<{
        label: string;
        type: string;
        value: unknown;
        path: string;
      }>;
      setNodeDetailModal({
        show: true,
        ...customEvent.detail,
      });
    };
    window.addEventListener("showNodeDetail", handleShowNodeDetail);
    return () => {
      window.removeEventListener("showNodeDetail", handleShowNodeDetail);
    };
  }, []);

  const closeNodeDetailModal = () => {
    setNodeDetailModal(null);
    setCopiedContent(false);
    setCopiedPath(false);
  };

  const getFormattedValue = (value: unknown): string => {
    if (value === null) return "null";
    if (typeof value === "string") return value;
    return JSON.stringify(value, null, 2);
  };

  const handleCopyContent = async () => {
    if (!nodeDetailModal) return;
    const content = getFormattedValue(nodeDetailModal.value);
    await navigator.clipboard.writeText(content);
    setCopiedContent(true);
    setTimeout(() => setCopiedContent(false), 1500);
  };

  const handleCopyPath = async () => {
    if (!nodeDetailModal) return;
    await navigator.clipboard.writeText(nodeDetailModal.path);
    setCopiedPath(true);
    setTimeout(() => setCopiedPath(false), 1500);
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "object":
        return <Braces size={16} />;
      case "array":
        return <Brackets size={16} />;
      case "string":
        return <Type size={16} />;
      case "number":
        return <Hash size={16} />;
      case "boolean":
        return <ToggleLeft size={16} />;
      case "null":
        return <Ban size={16} />;
      default:
        return null;
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case "object":
        return "text-blue-400 bg-blue-500/10 border-blue-500/20";
      case "array":
        return "text-green-400 bg-green-500/10 border-green-500/20";
      case "string":
        return "text-orange-400 bg-orange-500/10 border-orange-500/20";
      case "number":
        return "text-purple-400 bg-purple-500/10 border-purple-500/20";
      case "boolean":
        return "text-pink-400 bg-pink-500/10 border-pink-500/20";
      case "null":
        return "text-gray-400 bg-gray-500/10 border-gray-500/20";
      default:
        return "text-app-subtext bg-app-panel border-app-border";
    }
  };

  const findNodeByPath = (root: JsonNode | null, path: string): JsonNode | null => {
    if (!root) return null;
    if (root.path === path) return root;
    if (root.children) {
      for (const child of root.children) {
        const found = findNodeByPath(child, path);
        if (found) return found;
      }
    }
    return null;
  };

  const focusedNode = useMemo(() => {
    return findNodeByPath(state.json, focusedPath) || state.json;
  }, [state.json, focusedPath]);

  const getValuePreview = (node: JsonNode) => {
    if (node.type === "object") {
      return `{${node.children?.length ?? 0} keys}`;
    }
    if (node.type === "array") {
      return `[${node.children?.length ?? 0} items]`;
    }
    if (node.type === "string") {
      const val = String(node.value);
      return val.length > 150 ? `"${val.substring(0, 150)}..."` : `"${val}"`;
    }
    if (node.type === "boolean") {
      return String(node.value);
    }
    if (node.type === "null") {
      return "null";
    }
    return String(node.value ?? "");
  };

  const controlsStyle = useMemo(
    () => ({
      controlsBg:
        mode === "dark"
          ? "rgba(30, 30, 30, 0.95)"
          : "rgba(255, 255, 255, 0.95)",
      buttonBg: mode === "dark" ? "#1e1e1e" : "#ffffff",
      textColor: mode === "dark" ? "#ffffff" : "#000000",
    }),
    [mode]
  );

  const tableData = useMemo(() => {
    if (!focusedNode) return { type: 'none', headers: [], rows: [] };

    if (focusedNode.type === 'array') {
      // Check if all children are objects
      const children = focusedNode.children || [];
      const allObjects = children.every(c => c.type === 'object');
      
      if (allObjects && children.length > 0) {
        const keysSet = new Set<string>();
        children.forEach(c => {
          c.children?.forEach(cc => keysSet.add(cc.key));
        });
        const headers = Array.from(keysSet);
        return { 
          type: 'array-of-objects', 
          headers, 
          rows: children.map(c => {
            const rowData: Record<string, JsonNode> = {};
            c.children?.forEach(cc => { rowData[cc.key] = cc; });
            return { path: c.path, data: rowData };
          })
        };
      } else {
        return { 
          type: 'array-plain', 
          headers: ['Value'], 
          rows: children.map(c => ({ path: c.path, data: { 'Value': c } as Record<string, JsonNode> })) 
        };
      }
    } else if (focusedNode.type === 'object') {
      const children = focusedNode.children || [];
      return {
        type: 'object',
        headers: ['Key', 'Value', 'Type'],
        rows: children.map(c => ({ 
          path: c.path, 
          data: { 
            'Key': { ...c, value: c.key, type: 'string' } as JsonNode, 
            'Value': c, 
            'Type': { ...c, value: c.type, type: 'string' } as JsonNode 
          } as Record<string, JsonNode>
        }))
      };
    }

    return { 
      type: 'primitive', 
      headers: ['Value'], 
      rows: [{ path: focusedNode.path, data: { 'Value': focusedNode } as Record<string, JsonNode> }] 
    };
  }, [focusedNode]);

  const totalPages = Math.ceil(tableData.rows.length / rowsPerPage);

  useEffect(() => {
    setCurrentPage(1);
  }, [viewMode]);

  const paginatedRows = useMemo(() => {
    const startIndex = (currentPage - 1) * rowsPerPage;
    const endIndex = startIndex + rowsPerPage;
    return tableData.rows.slice(startIndex, endIndex);
  }, [tableData.rows, currentPage, rowsPerPage]);

  const handleCopyJson = () => {
    if (state.json) {
      const jsonString = JSON.stringify(state.json.value, null, 2);
      navigator.clipboard.writeText(jsonString);
      setCopiedToClipboard(true);
      setTimeout(() => setCopiedToClipboard(false), 2000);
    }
  };

  return (
    <div className={`min-h-screen bg-app-bg text-app-text`}>
      <div className="container mx-auto p-4 md:p-6 ">
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
            <button
              onClick={() => setViewMode("json")}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                viewMode === "json"
                  ? "bg-app-accent text-white"
                  : "text-app-subtext hover:text-app-text"
              }`}>
              <Code size={14} /> JSON
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
                    onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => {
                      setInputValue(e.target.value);
                      if (hasAiFixed) {
                        setHasAiFixed(false);
                      }
                      clearAiError();
                    }}
                    placeholder='{"event_id": "evt_001", "action": "checkout"}'
                    className={`w-full h-48 rounded-lg border bg-app-bg px-4 py-3 text-[13px] font-mono text-app-text focus:border-app-accent/50 focus:outline-none transition-all resize-none ${
                      aiError ? "border-red-500/50" : "border-app-border"
                    }`}
                  />
                  <div className="absolute top-2 right-2 flex gap-1">
                    <button
                      onClick={handleFixWithAI}
                      disabled={!inputValue.trim() || isFixing}
                      className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-purple-500/10 text-purple-400 hover:bg-purple-500/20 hover:text-purple-300 disabled:opacity-40 disabled:cursor-not-allowed transition text-[11px] font-semibold border border-purple-500/20"
                      title="Fix JSON with AI">
                      {isFixing ? (
                        <Loader2 size={12} className="animate-spin" />
                      ) : (
                        <Wand2 size={12} />
                      )}
                      FIX AI
                    </button>
                  </div>
                </div>

                {aiError && (
                  <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-xs">
                    <X size={14} />
                    <span className="flex-1">{aiError}</span>
                    <button onClick={clearAiError} className="hover:text-red-300 transition">
                      <X size={12} />
                    </button>
                  </div>
                )}

                <div className="flex gap-2">
                  <Button
                    onClick={handleParseJson}
                    className="flex-1 bg-app-accent hover:bg-app-accent/90 text-white font-bold h-10">
                    Generate View
                  </Button>
                  {hasAiFixed && (
                    <>
                      <Button
                        variant="secondary"
                        onClick={handleRegenerateAiFix}
                        disabled={isFixing}
                        className="px-3 border-app-border hover:bg-purple-500/10 hover:text-purple-400 hover:border-purple-500/20"
                        title="Regenerate AI fix">
                        {isFixing ? (
                          <Loader2 size={18} className="animate-spin" />
                        ) : (
                          <RefreshCw size={18} />
                        )}
                      </Button>
                      <Button
                        variant="secondary"
                        onClick={handleRevertToOriginal}
                        className="px-3 border-app-border hover:bg-orange-500/10 hover:text-orange-400 hover:border-orange-500/20"
                        title="Revert to original">
                        <Undo2 size={18} />
                      </Button>
                    </>
                  )}
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
                    <JsonListView 
                      json={state.json} 
                      onToggle={toggleNode} 
                      onSelect={(path) => {
                        setFocusedPath(path);
                        setViewMode("table");
                      }}
                      activePath={focusedPath}
                    />
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
                          className="text-app-subtext mt-0.5"
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
                    stroke: mode === "dark" ? "#6b7280" : "#d1d5db",
                    strokeWidth: 1.5,
                    opacity: 0.6,
                  },
                }}
                className="bg-app-bg">
                <Background
                  color={mode === "dark" ? "#6b7280" : "#d1d5db"}
                  variant={BackgroundVariant.Dots}
                  gap={20}
                  size={1}
                  className="opacity-20"
                />
                <Controls />
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
                
                {isGraphTooLarge && (
                  <div className="absolute inset-0 flex items-center justify-center bg-app-bg/80 z-50 backdrop-blur-sm p-6 text-center">
                    <div className="max-w-md bg-app-card border border-app-border p-6 rounded-2xl shadow-2xl">
                      <Layout className="w-12 h-12 text-orange-400 mx-auto mb-4" />
                      <h3 className="text-lg font-bold text-app-text mb-2">Graph Too Large</h3>
                      <p className="text-sm text-app-subtext mb-6">
                        Data ini memiliki lebih dari 150 node yang terlihat. Merendernya sebagai graph akan membuat aplikasi sangat lambat. 
                        Silakan gunakan <b>Table</b> atau <b>Tree view</b> untuk performa lebih baik, atau tutup beberapa node folder.
                      </p>
                      <div className="flex gap-3 justify-center">
                        <Button 
                          onClick={() => setViewMode("table")}
                          className="bg-app-accent text-white"
                        >
                          Switch to Table View
                        </Button>
                      </div>
                    </div>
                  </div>
                )}
              </ReactFlow>
            ) : viewMode === "table" ? (
              <div className="flex flex-col h-full bg-app-bg/50">
                {/* Table Header / Breadcrumbs */}
                <div className="flex-none px-4 py-3 border-b border-app-border bg-app-panel/50 flex items-center gap-2 overflow-x-auto no-scrollbar">
                  <div className="flex items-center gap-1.5 text-[11px] font-medium py-1">
                    <button 
                      onClick={() => setFocusedPath("$")}
                      className={`px-2 py-1 rounded transition-colors ${focusedPath === "$" ? "bg-app-accent/20 text-app-accent font-bold" : "text-app-subtext hover:bg-app-panel hover:text-app-text"}`}
                    >
                      root
                    </button>
                    {focusedPath !== "$" && focusedPath.split('.').filter(p => p !== '$').map((part, i, arr) => (
                      <div key={i} className="flex items-center gap-1">
                        <span className="text-app-subtext/30 px-1">/</span>
                        <button
                          onClick={() => {
                            const newPath = "$." + arr.slice(0, i + 1).join('.');
                            setFocusedPath(newPath);
                          }}
                          className={`px-2 py-1 rounded transition-colors ${i === arr.length - 1 ? "bg-app-accent/20 text-app-accent font-bold" : "text-app-subtext hover:bg-app-panel hover:text-app-text"}`}
                        >
                          {part.replace(/[\[\]]/g, '')}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="flex-1 overflow-auto custom-scrollbar relative">
                  <table className="w-full text-sm border-collapse min-w-max">
                    <thead className="sticky top-0 bg-app-panel z-20 shadow-sm">
                      <tr>
                        {tableData.headers.map((header) => (
                          <th 
                            key={header} 
                            className="text-left p-3 px-4 font-bold text-app-subtext uppercase tracking-widest text-[10px] whitespace-nowrap border-b border-app-border bg-app-panel"
                          >
                            <div className="flex items-center gap-2">
                              {header}
                            </div>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-app-border/40">
                      {paginatedRows.length === 0 ? (
                        <tr>
                          <td colSpan={tableData.headers.length} className="p-10 text-center text-app-subtext italic">
                            No data available in this node
                          </td>
                        </tr>
                      ) : (
                        paginatedRows.map((row, index) => (
                          <tr
                            key={row.path + index}
                            className={`group hover:bg-app-panel/60 transition-colors ${
                              index % 2 === 0 ? "bg-app-bg/30" : "bg-transparent"
                            }`}
                          >
                            {tableData.headers.map((header) => {
                              const cellData = (row.data as Record<string, JsonNode>)[header];
                              return (
                                <td 
                                  key={header} 
                                  className="p-3 px-4 font-mono text-[12px] min-w-[120px] max-w-[400px]"
                                >
                                  {cellData ? (
                                    <div className="flex items-start justify-between gap-2 overflow-hidden">
                                      <span className={`break-words line-clamp-3 ${
                                        cellData.type === 'string' ? 'text-orange-400' :
                                        cellData.type === 'number' ? 'text-purple-400' :
                                        cellData.type === 'boolean' ? 'text-pink-400' :
                                        cellData.type === 'object' ? 'text-blue-400' :
                                        cellData.type === 'array' ? 'text-green-400' :
                                        'text-app-subtext'
                                      }`}>
                                        {getValuePreview(cellData)}
                                      </span>
                                      {(cellData.type === 'object' || cellData.type === 'array') && (
                                        <button 
                                          onClick={() => setFocusedPath(cellData.path)}
                                          className="flex-none p-1 rounded bg-app-panel/80 text-[9px] text-app-subtext hover:text-app-accent hover:bg-app-accent/10 transition-all opacity-0 group-hover:opacity-100 border border-app-border"
                                        >
                                          Enter
                                        </button>
                                      )}
                                    </div>
                                  ) : (
                                    <span className="text-app-subtext/20">-</span>
                                  )}
                                </td>
                              );
                            })}
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>

                {/* Enhanced Pagination Controls */}
                <div className="flex-none px-4 py-3 border-t border-app-border bg-app-panel/80 backdrop-blur-md z-30">
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                    <div className="flex items-center gap-4">
                      <div className="text-[11px] text-app-subtext font-medium bg-app-bg/50 px-2 py-1 rounded-md border border-app-border">
                        Showing <span className="text-app-text">{(currentPage - 1) * rowsPerPage + 1}</span> to{" "}
                        <span className="text-app-text">{Math.min(currentPage * rowsPerPage, tableData.rows.length)}</span>{" "}
                        of <span className="text-app-text">{tableData.rows.length.toLocaleString()}</span> entries
                      </div>
                      
                      <div className="flex items-center gap-2">
                        <span className="text-[10px] text-app-subtext uppercase font-bold tracking-tighter">Per Page:</span>
                        <select 
                          value={rowsPerPage} 
                          onChange={(e) => {
                            setRowsPerPage(Number(e.target.value));
                            setCurrentPage(1);
                          }}
                          className="bg-app-bg text-app-text text-[11px] border border-app-border rounded-md px-1 py-0.5 focus:outline-none focus:border-app-accent"
                        >
                          <option value={10}>10</option>
                          <option value={15}>15</option>
                          <option value={25}>25</option>
                          <option value={50}>50</option>
                          <option value={100}>100</option>
                        </select>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => setCurrentPage(1)}
                        disabled={currentPage === 1}
                        className="p-2 rounded-md border border-app-border bg-app-bg text-app-text hover:bg-app-accent hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-all shadow-sm"
                        title="First Page"
                      >
                        <ChevronsLeft size={16} />
                      </button>
                      <button
                        onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                        disabled={currentPage === 1}
                        className="flex items-center gap-1.5 px-3 py-2 rounded-md border border-app-border bg-app-bg text-app-text hover:bg-app-accent hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-all text-xs font-bold shadow-sm"
                      >
                        <ChevronLeft size={16} /> Prev
                      </button>
                      
                      <div className="flex items-center px-4 h-9 rounded-md bg-app-accent/10 border border-app-accent/20 text-app-accent text-xs font-bold min-w-[80px] justify-center">
                        {currentPage} <span className="mx-2 opacity-30 text-app-text">/</span> {totalPages}
                      </div>

                      <button
                        onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                        disabled={currentPage === totalPages}
                        className="flex items-center gap-1.5 px-3 py-2 rounded-md border border-app-border bg-app-bg text-app-text hover:bg-app-accent hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-all text-xs font-bold shadow-sm"
                      >
                        Next <ChevronRight size={16} />
                      </button>
                      <button
                        onClick={() => setCurrentPage(totalPages)}
                        disabled={currentPage === totalPages}
                        className="p-2 rounded-md border border-app-border bg-app-bg text-app-text hover:bg-app-accent hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-all shadow-sm"
                        title="Last Page"
                      >
                        <ChevronsRight size={16} />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ) : viewMode === "json" ? (
              <div className="flex flex-col h-full">
                <div className="flex-none px-4 py-3 border-b border-app-border flex items-center justify-between bg-app-panel">
                  <div className="flex items-center gap-2">
                    <Code className="text-app-accent" size={18} />
                    <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">
                      Formatted JSON
                    </h3>
                  </div>
                  <button
                    onClick={handleCopyJson}
                    disabled={!state.json}
                    className="flex items-center gap-2 px-4 py-2 rounded-lg bg-app-accent hover:bg-app-accent/90 text-white text-sm font-semibold disabled:opacity-50 disabled:cursor-not-allowed transition">
                    {copiedToClipboard ? (
                      <>
                        <Check size={16} />
                        Copied!
                      </>
                    ) : (
                      <>
                        <Copy size={16} />
                        Copy JSON
                      </>
                    )}
                  </button>
                </div>
                <div className="flex-1 overflow-auto p-0 bg-app-bg/50">
                  <pre className="json-formatter p-6 text-sm font-mono text-app-text whitespace-pre-wrap break-all leading-relaxed select-text">
                    {state.json
                      ? JSON.stringify(state.json.value, null, 2)
                      : "No data loaded"}
                  </pre>
                </div>
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
        .no-scrollbar::-webkit-scrollbar {
          display: none;
        }
        .no-scrollbar {
          -ms-overflow-style: none;
          scrollbar-width: none;
        }

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
        
        .react-flow__controls {
          background: ${controlsStyle.controlsBg} !important;
          border: 1px solid ${
            mode === "dark" ? "#374151" : "#e5e7eb"
          } !important;
          backdrop-filter: blur(8px) !important;
          border-radius: 8px !important;
        }
        
        .react-flow__controls-button {
          background: ${controlsStyle.buttonBg} !important;
          border-bottom: 1px solid ${
            mode === "dark" ? "#374151" : "#e5e7eb"
          } !important;
          color: ${controlsStyle.textColor} !important;
          fill: ${controlsStyle.textColor} !important;
        }
        
        .react-flow__controls-button:hover {
          background: var(--color-app-accent, #3b82f6) !important;
          color: white !important;
          fill: white !important;
        }
        
        .react-flow__controls-button:last-child {
          border-bottom: none !important;
        }
        
        .react-flow__controls-button svg {
          width: 16px !important;
          height: 16px !important;
        }

        .json-formatter {
          line-height: 1.6;
        }
        
        .json-formatter::-webkit-scrollbar {
          width: 8px;
          height: 8px;
        }
        
        .json-formatter::-webkit-scrollbar-track {
          background: transparent;
        }
        
        .json-formatter::-webkit-scrollbar-thumb {
          background: var(--color-app-border);
          border-radius: 4px;
        }
        
        .json-formatter::-webkit-scrollbar-thumb:hover {
          background: var(--color-app-subtext);
        }
      `}</style>

      {/* Node Detail Modal - Single Overlay */}
      {nodeDetailModal && (
        <div
          className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={closeNodeDetailModal}
        >
          <div
            className="bg-app-card border border-app-border rounded-xl shadow-2xl w-[90vw] max-w-lg max-h-[80vh] overflow-hidden animate-in fade-in zoom-in-95 duration-200"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div className={`flex items-center justify-between px-4 py-3 border-b border-app-border ${getTypeColor(nodeDetailModal.type)}`}>
              <div className="flex items-center gap-3">
                <span className={`p-1.5 rounded-md ${getTypeColor(nodeDetailModal.type)}`}>
                  {getTypeIcon(nodeDetailModal.type)}
                </span>
                <div>
                  <h3 className="font-bold text-app-text">{nodeDetailModal.label}</h3>
                  <span className="text-[10px] text-app-subtext uppercase tracking-wider">
                    {nodeDetailModal.type}
                  </span>
                </div>
              </div>
              <button
                onClick={closeNodeDetailModal}
                className="p-1.5 rounded-lg hover:bg-app-bg/50 text-app-subtext hover:text-app-text transition"
              >
                <X size={18} />
              </button>
            </div>

            {/* Modal Content */}
            <div className="p-4 space-y-4">
              {/* Content Section */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-app-subtext uppercase font-semibold tracking-wide">
                    Content
                  </span>
                  <button
                    onClick={handleCopyContent}
                    className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-semibold transition ${
                      copiedContent
                        ? "bg-green-500/20 text-green-400 border border-green-500/30"
                        : "bg-app-panel hover:bg-app-accent/20 text-app-subtext hover:text-app-accent border border-app-border"
                    }`}
                  >
                    {copiedContent ? <Check size={12} /> : <Copy size={12} />}
                    {copiedContent ? "Copied!" : "Copy"}
                  </button>
                </div>
                <div className="bg-app-bg rounded-lg border border-app-border p-3 max-h-[40vh] overflow-auto custom-scrollbar">
                  <pre className="font-mono text-sm text-app-text whitespace-pre-wrap break-all select-text">
                    {getFormattedValue(nodeDetailModal.value)}
                  </pre>
                </div>
              </div>

              {/* JSON Path Section */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-app-subtext uppercase font-semibold tracking-wide">
                    JSON Path
                  </span>
                  <button
                    onClick={handleCopyPath}
                    className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-semibold transition ${
                      copiedPath
                        ? "bg-green-500/20 text-green-400 border border-green-500/30"
                        : "bg-app-panel hover:bg-app-accent/20 text-app-subtext hover:text-app-accent border border-app-border"
                    }`}
                  >
                    {copiedPath ? <Check size={12} /> : <Copy size={12} />}
                    {copiedPath ? "Copied!" : "Copy"}
                  </button>
                </div>
                <div className="bg-app-bg rounded-lg border border-app-border px-3 py-2">
                  <code className="font-mono text-sm text-app-accent select-text">
                    {nodeDetailModal.path}
                  </code>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default VisualizeJsonPage;
