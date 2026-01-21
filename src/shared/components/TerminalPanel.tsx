import { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emit } from "@tauri-apps/api/event";
import {
  ChevronDown,
  ChevronUp,
  Copy,
  Eraser,
  ExternalLink,
  Search,
  Minimize,
} from "lucide-react";
import { useLogsQuery } from "../../hooks/useLlmApi";

type LogLevelFilter = "all" | "error" | "warn" | "info" | "debug";

const LEVEL_OPTIONS: { value: LogLevelFilter; label: string }[] = [
  { value: "all", label: "All levels" },
  { value: "error", label: "Error" },
  { value: "warn", label: "Warn" },
  { value: "info", label: "Info" },
  { value: "debug", label: "Debug" },
];

export default function TerminalPanel() {
  const [searchQuery, setSearchQuery] = useState("");
  const [levelFilter, setLevelFilter] = useState<LogLevelFilter>("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const [collapsed, setCollapsed] = useState(true);
  const [live, setLive] = useState(true);
  const [height, setHeight] = useState(224);
  const [isResizing, setIsResizing] = useState(false);
  const [isDetached, setIsDetached] = useState(false);
  const logContainerRef = useRef<HTMLDivElement>(null);
  const resizeHandleRef = useRef<HTMLDivElement>(null);
  const { data: logs = [], clear } = useLogsQuery(live);

  useEffect(() => {
    emit("terminal-logs", logs);
  }, [logs]);

  const handleDetach = async () => {
    console.log("handleDetach called");
    try {
      const terminalWindow = await WebviewWindow.getByLabel("terminal");
      console.log("Existing terminal window found:", terminalWindow);
      if (terminalWindow) {
        await terminalWindow.show();
        await terminalWindow.setFocus();
        await emit("terminal-logs", logs);
        setIsDetached(true);
      }
    } catch (e) {
      console.log("Creating new terminal window, error:", e);
      const terminalWindow = new WebviewWindow("terminal", {
        url: "index.html?label=terminal",
        title: "Terminal",
        width: 1000,
        height: 600,
        resizable: true,
        decorations: true,
        center: true,
        visible: true,
      });
      console.log("Terminal window creation requested");
      await terminalWindow.once("tauri://created", async () => {
        console.log("Terminal window created event");
        await emit("terminal-logs", logs);
        setIsDetached(true);
      });
      await terminalWindow.once("tauri://error", (e) => {
        console.error("Terminal window error:", e);
      });
      await terminalWindow.once("tauri://close-requested", () => {
        console.log("Terminal window close requested");
        setIsDetached(false);
      });
    }
  };

  const handleReattach = async () => {
    try {
      const terminalWindow = await WebviewWindow.getByLabel("terminal");
      if (terminalWindow) {
        await terminalWindow.hide();
        setIsDetached(false);
      }
    } catch {
      setIsDetached(false);
    }
  };

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newHeight = window.innerHeight - e.clientY;
      setHeight(Math.max(100, Math.min(newHeight, window.innerHeight * 0.8)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  const filteredLogs = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    return logs.filter((log) => {
      const level = log.level.toLowerCase();
      if (levelFilter !== "all" && level !== levelFilter) {
        return false;
      }
      if (!query) return true;
      return (
        log.message.toLowerCase().includes(query) ||
        log.source.toLowerCase().includes(query)
      );
    });
  }, [logs, searchQuery, levelFilter]);

  useEffect(() => {
    if (!autoScroll || collapsed) return;
    const container = logContainerRef.current;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [filteredLogs, autoScroll, collapsed]);

  const terminalContent = (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between px-3 py-2 border-b border-app-border">
        <div className="flex items-center gap-3">
          <button
            className="text-gray-400 hover:text-white transition"
            onClick={() => setCollapsed((prev) => !prev)}
            aria-label={collapsed ? "Expand terminal" : "Collapse terminal"}>
            {collapsed ? (
              <ChevronUp className="w-4 h-4" />
            ) : (
              <ChevronDown className="w-4 h-4" />
            )}
          </button>
          <div className="text-xs font-medium text-app-text">Terminal</div>
          <div className="flex items-center gap-2 text-[10px] text-app-subtext">
            <span
              className={`h-1.5 w-1.5 rounded-full ${
                live ? "bg-app-success" : "bg-gray-500"
              }`}
            />
            <span>{live ? "Live" : "Paused"}</span>
            <span className="opacity-60">{filteredLogs.length} lines</span>
          </div>
        </div>
        <div className="flex items-center gap-2 text-[10px] text-app-subtext">
          <button
            className={`rounded border px-2 py-1 transition ${
              live
                ? "border-app-border hover:text-white"
                : "border-app-accent text-app-accent"
            }`}
            onClick={() => setLive((prev) => !prev)}>
            {live ? "Pause" : "Resume"}
          </button>
          <button
            className={`rounded border px-2 py-1 transition ${
              autoScroll
                ? "border-app-accent text-app-accent"
                : "border-app-border hover:text-white"
            }`}
            onClick={() => setAutoScroll((prev) => !prev)}>
            Auto-scroll
          </button>
          {!collapsed && !isDetached && (
            <>
              <button
                className="hover:text-white transition"
                onClick={() => {
                  const text = filteredLogs
                    .map(
                      (l) =>
                        `[${l.time}] [${l.level}] [${l.source}] ${l.message}`,
                    )
                    .join("\n");
                  navigator.clipboard.writeText(text);
                }}
                aria-label="Copy logs">
                <Copy className="w-3.5 h-3.5" />
              </button>
              <button
                className="hover:text-white flex items-center gap-1 transition"
                onClick={clear}>
                <Eraser className="w-3.5 h-3.5" /> Clear
              </button>
              <button
                className="hover:text-white transition"
                onClick={handleDetach}
                aria-label="Detach terminal">
                <ExternalLink className="w-3.5 h-3.5" />
              </button>
            </>
          )}
          {isDetached && (
            <button
              className="text-app-accent hover:text-white flex items-center gap-1 transition border border-app-accent rounded px-2 py-1"
              onClick={handleReattach}
              aria-label="Reattach terminal">
              <Minimize className="w-3.5 h-3.5" /> Reattach
            </button>
          )}
        </div>
      </div>
      {!collapsed && !isDetached && (
        <>
          <div className="p-2 border-b border-app-border flex gap-2 items-center">
            <div className="relative flex-1">
              <Search className="w-3.5 h-3.5 absolute left-3 top-2.5 text-gray-500" />
              <input
                className="w-full bg-transparent border border-app-border rounded py-1.5 pl-9 pr-3 text-xs focus:border-gray-500 transition placeholder-gray-500 text-gray-300 outline-none"
                placeholder="Search logs..."
                type="text"
                value={searchQuery}
                onInput={(e: any) => setSearchQuery(e.target.value)}
              />
            </div>
            <select
              className="bg-[#18181b] border border-app-border rounded px-2 py-1 text-xs text-gray-300 outline-none hover:border-gray-500 transition"
              value={levelFilter}
              onChange={(e: any) =>
                setLevelFilter(e.target.value as LogLevelFilter)
              }>
              {LEVEL_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div
            ref={logContainerRef}
            className="flex-1 bg-[#0c0c0e] p-3 overflow-y-auto font-mono text-[10px] leading-relaxed text-gray-200 select-text">
            {filteredLogs.map((log, i) => {
              const level = log.level.toUpperCase();
              const levelClass =
                level === "ERROR"
                  ? "text-red-400"
                  : level === "WARN"
                    ? "text-yellow-400"
                    : "text-blue-400";
              return (
                <div key={i} className="mb-1 animate-in fade-in duration-300">
                  <span className="text-gray-500">{log.time}</span>{" "}
                  <span className={levelClass}>[{level}]</span>{" "}
                  <span className="text-purple-400">[{log.source}]</span>{" "}
                  {log.message}
                </div>
              );
            })}
            {filteredLogs.length === 0 && (
              <div className="text-gray-600 italic">No logs found...</div>
            )}
          </div>
        </>
      )}
      {!collapsed && isDetached && (
        <div className="flex-1 bg-[#0c0c0e] flex items-center justify-center">
          <div className="text-center text-gray-400 text-sm">
            <p className="mb-2">Terminal is detached to a separate window</p>
            <p className="text-xs">
              Click the "Reattach" button above to bring it back
            </p>
          </div>
        </div>
      )}
    </div>
  );

  const terminalHeight = `${height}px`;

  if (collapsed) {
    return ReactDOM.createPortal(
      <div className="fixed bottom-0 right-1 z-[9999] w-full bg-[#18181b]  border-app-border rounded-md">
        <div className="flex items-center justify-between px-3 py-2 border-b border-app-border bg-[#0c0c0e] rounded-t-lg">
          <div className="flex items-center gap-2">
            <button
              className="text-gray-400 hover:text-white transition"
              onClick={() => setCollapsed(false)}
              aria-label="Expand terminal">
              <ChevronUp className="w-4 h-4" />
            </button>
            <div className="text-xs font-medium text-app-text">Terminal</div>
            <div className="flex items-center gap-1 text-[10px] text-app-subtext">
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  live ? "bg-app-success" : "bg-gray-500"
                }`}
              />
              <span>{filteredLogs.length} lines</span>
              {isDetached && (
                <span className="text-app-accent">(Detached)</span>
              )}
            </div>
          </div>
          {!isDetached && (
            <button
              className="text-gray-400 hover:text-white transition"
              onClick={handleDetach}
              aria-label="Detach terminal">
              <ExternalLink className="w-3.5 h-3.5" />
            </button>
          )}
          {isDetached && (
            <button
              className="text-app-accent hover:text-white transition"
              onClick={handleReattach}
              aria-label="Reattach terminal">
              <Minimize className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>,
      document.body,
    );
  }

  return (
    <section
      className="flex-none border-t border-app-border bg-app-bg relative"
      style={{ height: terminalHeight }}
      data-purpose="terminal-panel">
      <div
        ref={resizeHandleRef}
        onMouseDown={(e) => {
          e.preventDefault();
          setIsResizing(true);
        }}
        className="absolute top-0 left-0 right-0 h-1 cursor-ns-resize hover:bg-app-accent transition-colors z-10"
        data-purpose="resize-handle"
      />
      {terminalContent}
    </section>
  );
}
