import { useEffect, useMemo, useState } from "react";

import { useThemeStore } from "../../features/theme/themeStore";

import { useFixJsonAI } from "./hooks/useFixJsonAI";
import { useJsonVisualization } from "./hooks/useJsonVisualization";
import { useNodeDetailModal } from "./hooks/useNodeDetailModal";
import { useToggleNodeEvent } from "./hooks/useToggleNodeEvent";

import PageHeader from "./components/PageHeader";
import HistoryPanel from "./components/HistoryPanel";
import HierarchyPanel from "./components/HierarchyPanel";
import InputSourcePanel from "./components/InputSourcePanel";
import NodeDetailModal from "./components/NodeDetailModal";
import VisualizeJsonGlobalStyles from "./components/VisualizeJsonGlobalStyles";
import EmptyState from "./components/views/EmptyState";
import GraphView from "./components/views/GraphView";
import JsonView from "./components/views/JsonView";
import TableView from "./components/views/TableView";
import TreeView from "./components/views/TreeView";

import type { HistoryItem } from "./types";

export type ViewMode = "list" | "graph" | "table" | "json";

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

  const { mode } = useThemeStore();
  const { fixJson, isFixing, error: aiError, clearError: clearAiError } = useFixJsonAI();
  const { nodeDetail, closeNodeDetail } = useNodeDetailModal();
  useToggleNodeEvent(toggleNode);

  const [viewMode, setViewMode] = useState<ViewMode>("graph");
  const [focusedPath, setFocusedPath] = useState<string>("$");

  const [inputValue, setInputValue] = useState("");
  const [originalInputValue, setOriginalInputValue] = useState("");
  const [hasAiFixed, setHasAiFixed] = useState(false);

  useEffect(() => {
    if (hasAiFixed && inputValue.trim() !== originalInputValue.trim()) {
      return;
    }
    if (!hasAiFixed && originalInputValue) {
      setOriginalInputValue("");
    }
  }, [hasAiFixed, inputValue, originalInputValue]);

  const history = useMemo(() => state.history as HistoryItem[], [state.history]);

  const handleParseJson = () => {
    const jsonNode = parseJson(inputValue);
    if (!jsonNode) {
      alert("Invalid JSON format. Please check your input.");
      return;
    }

    const newItem = createHistoryItem(jsonNode.value, "json", "Pasted JSON");
    saveToLocalStorage({
      ...state,
      json: jsonNode,
      history: [newItem, ...history.slice(0, 9)],
    });
    setInputValue("");
    setHasAiFixed(false);
    clearAiError();
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
          alert(`Invalid ${extension.toUpperCase()} file. Please check the file content.`);
          return;
        }

        const newItem = createHistoryItem(jsonNode.value, format, file.name, content);
        saveToLocalStorage({
          ...state,
          json: jsonNode,
          history: [newItem, ...history.slice(0, 9)],
        });
        setInputValue("");
        setHasAiFixed(false);
        clearAiError();
      } catch (error) {
        console.error("Error importing file:", error);
        alert(`Error importing file: ${error instanceof Error ? error.message : "Unknown error"}`);
      }
    };

    reader.onerror = () => {
      alert("Unable to read the selected file.");
    };

    reader.readAsText(file);
  };

  const handleClear = () => {
    clearLocalStorage();
    setInputValue("");
    setOriginalInputValue("");
    setHasAiFixed(false);
    clearAiError();
    setFocusedPath("$");
  };

  const handleFixWithAI = async () => {
    if (!inputValue.trim() || isFixing) return;

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
    if (!originalInputValue) return;
    setInputValue(originalInputValue);
    setHasAiFixed(false);
    clearAiError();
  };

  const handleRegenerateAiFix = async () => {
    if (!originalInputValue.trim() || isFixing) return;

    const fixedJson = await fixJson(originalInputValue);
    if (fixedJson) {
      setInputValue(fixedJson);
      setHasAiFixed(true);
    }
  };

  const handleSelectPath = (path: string) => {
    setFocusedPath(path);
    setViewMode("table");
  };

  return (
    <div className="min-h-screen bg-app-bg text-app-text">
      <div className="container mx-auto p-4 md:p-6">
        <PageHeader viewMode={viewMode} onChangeViewMode={setViewMode} />

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-[380px_1fr] h-[calc(100vh-160px)]">
          <div className="flex flex-col gap-6 overflow-hidden">
            <InputSourcePanel
              inputValue={inputValue}
              onChangeInputValue={(value) => {
                setInputValue(value);
                if (hasAiFixed) setHasAiFixed(false);
                clearAiError();
              }}
              onParseJson={handleParseJson}
              onClear={handleClear}
              onFileImport={handleFileImport}
              hasAiFixed={hasAiFixed}
              isFixing={isFixing}
              aiError={aiError}
              onFixWithAi={handleFixWithAI}
              onRegenerateAiFix={handleRegenerateAiFix}
              onRevertToOriginal={handleRevertToOriginal}
              onClearAiError={clearAiError}
            />

            <HierarchyPanel
              json={state.json}
              focusedPath={focusedPath}
              onToggleNode={toggleNode}
              onSelectPath={handleSelectPath}
            />

            <HistoryPanel
              history={history}
              onLoad={loadFromHistory}
              onRemove={removeFromHistory}
            />
          </div>

          <div className="flex flex-col h-full overflow-hidden rounded-lg border border-app-border bg-app-card/30 relative">
            {!state.json ? (
              <EmptyState />
            ) : viewMode === "graph" ? (
              <GraphView json={state.json} mode={mode} onSwitchToTable={() => setViewMode("table")} />
            ) : viewMode === "table" ? (
              <TableView
                root={state.json}
                focusedPath={focusedPath}
                onChangeFocusedPath={setFocusedPath}
              />
            ) : viewMode === "json" ? (
              <JsonView json={state.json} />
            ) : (
              <TreeView json={state.json} onToggle={toggleNode} mode={mode} />
            )}
          </div>
        </div>
      </div>

      <VisualizeJsonGlobalStyles />
      <NodeDetailModal node={nodeDetail} onClose={closeNodeDetail} />
    </div>
  );
};

export default VisualizeJsonPage;
