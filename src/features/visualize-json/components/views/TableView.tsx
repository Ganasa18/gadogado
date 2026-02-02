import { useEffect, useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from "lucide-react";

import type { JsonNode } from "../../types";

type TableData =
  | { type: "none"; headers: string[]; rows: Array<{ path: string; data: Record<string, JsonNode> }> }
  | {
      type: "array-of-objects";
      headers: string[];
      rows: Array<{ path: string; data: Record<string, JsonNode> }>;
    }
  | { type: "array-plain"; headers: string[]; rows: Array<{ path: string; data: Record<string, JsonNode> }> }
  | { type: "object"; headers: string[]; rows: Array<{ path: string; data: Record<string, JsonNode> }> }
  | {
      type: "primitive";
      headers: string[];
      rows: Array<{ path: string; data: Record<string, JsonNode> }>;
    };

interface TableViewProps {
  root: JsonNode;
  focusedPath: string;
  onChangeFocusedPath: (path: string) => void;
}

export default function TableView({ root, focusedPath, onChangeFocusedPath }: TableViewProps) {
  const focusedNode = useMemo(() => {
    return findNodeByPath(root, focusedPath) || root;
  }, [root, focusedPath]);

  const tableData = useMemo<TableData>(() => {
    if (!focusedNode) return { type: "none", headers: [], rows: [] };

    if (focusedNode.type === "array") {
      const children = focusedNode.children || [];
      const allObjects = children.every((c) => c.type === "object");

      if (allObjects && children.length > 0) {
        const keysSet = new Set<string>();
        children.forEach((c) => {
          c.children?.forEach((cc) => keysSet.add(cc.key));
        });
        const headers = Array.from(keysSet);
        return {
          type: "array-of-objects",
          headers,
          rows: children.map((c) => {
            const rowData: Record<string, JsonNode> = {};
            c.children?.forEach((cc) => {
              rowData[cc.key] = cc;
            });
            return { path: c.path, data: rowData };
          }),
        };
      }

      return {
        type: "array-plain",
        headers: ["Value"],
        rows: children.map((c) => ({ path: c.path, data: { Value: c } })),
      };
    }

    if (focusedNode.type === "object") {
      const children = focusedNode.children || [];
      return {
        type: "object",
        headers: ["Key", "Value", "Type"],
        rows: children.map((c) => ({
          path: c.path,
          data: {
            Key: { ...c, value: c.key, type: "string" } as JsonNode,
            Value: c,
            Type: { ...c, value: c.type, type: "string" } as JsonNode,
          },
        })),
      };
    }

    return {
      type: "primitive",
      headers: ["Value"],
      rows: [{ path: focusedNode.path, data: { Value: focusedNode } }],
    };
  }, [focusedNode]);

  const [currentPage, setCurrentPage] = useState(1);
  const [rowsPerPage, setRowsPerPage] = useState(15);

  useEffect(() => {
    setCurrentPage(1);
  }, [focusedPath]);

  const totalPages = Math.max(1, Math.ceil(tableData.rows.length / rowsPerPage));

  const paginatedRows = useMemo(() => {
    const startIndex = (currentPage - 1) * rowsPerPage;
    const endIndex = startIndex + rowsPerPage;
    return tableData.rows.slice(startIndex, endIndex);
  }, [tableData.rows, currentPage, rowsPerPage]);

  return (
    <div className="flex flex-col h-full bg-app-bg/50">
      <div className="flex-none px-4 py-3 border-b border-app-border bg-app-panel/50 flex items-center gap-2 overflow-x-auto no-scrollbar">
        <Breadcrumbs focusedPath={focusedPath} onChangeFocusedPath={onChangeFocusedPath} />
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
                  <div className="flex items-center gap-2">{header}</div>
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
                    const cellData = row.data[header];
                    return (
                      <td key={header} className="p-3 px-4 font-mono text-[12px] min-w-[120px] max-w-[400px]">
                        {cellData ? (
                          <div className="flex items-start justify-between gap-2 overflow-hidden">
                            <span className={`break-words line-clamp-3 ${getCellColor(cellData.type)}`}>
                              {getValuePreview(cellData)}
                            </span>
                            {(cellData.type === "object" || cellData.type === "array") && (
                              <button
                                onClick={() => onChangeFocusedPath(cellData.path)}
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

      <div className="flex-none px-4 py-3 border-t border-app-border bg-app-panel/80 backdrop-blur-md z-30">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <div className="text-[11px] text-app-subtext font-medium bg-app-bg/50 px-2 py-1 rounded-md border border-app-border">
              Showing <span className="text-app-text">{(currentPage - 1) * rowsPerPage + 1}</span> to{" "}
              <span className="text-app-text">{Math.min(currentPage * rowsPerPage, tableData.rows.length)}</span> of{" "}
              <span className="text-app-text">{tableData.rows.length.toLocaleString()}</span> entries
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
  );
}

function Breadcrumbs({
  focusedPath,
  onChangeFocusedPath,
}: {
  focusedPath: string;
  onChangeFocusedPath: (path: string) => void;
}) {
  return (
    <div className="flex items-center gap-1.5 text-[11px] font-medium py-1">
      <button
        onClick={() => onChangeFocusedPath("$")}
        className={`px-2 py-1 rounded transition-colors ${
          focusedPath === "$"
            ? "bg-app-accent/20 text-app-accent font-bold"
            : "text-app-subtext hover:bg-app-panel hover:text-app-text"
        }`}
      >
        root
      </button>

      {focusedPath !== "$" &&
        focusedPath
          .split(".")
          .filter((p) => p !== "$")
          .map((part, i, arr) => (
            <div key={i} className="flex items-center gap-1">
              <span className="text-app-subtext/30 px-1">/</span>
              <button
                onClick={() => {
                  const newPath = "$." + arr.slice(0, i + 1).join(".");
                  onChangeFocusedPath(newPath);
                }}
                className={`px-2 py-1 rounded transition-colors ${
                  i === arr.length - 1
                    ? "bg-app-accent/20 text-app-accent font-bold"
                    : "text-app-subtext hover:bg-app-panel hover:text-app-text"
                }`}
              >
                {part.replace(/[\[\]]/g, "")}
              </button>
            </div>
          ))}
    </div>
  );
}

function findNodeByPath(root: JsonNode, path: string): JsonNode | null {
  if (root.path === path) return root;
  if (root.children) {
    for (const child of root.children) {
      const found = findNodeByPath(child, path);
      if (found) return found;
    }
  }
  return null;
}

function getValuePreview(node: JsonNode) {
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
}

function getCellColor(type: JsonNode["type"]) {
  if (type === "string") return "text-orange-400";
  if (type === "number") return "text-purple-400";
  if (type === "boolean") return "text-pink-400";
  if (type === "object") return "text-blue-400";
  if (type === "array") return "text-green-400";
  return "text-app-subtext";
}
