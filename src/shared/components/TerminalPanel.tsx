import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, ChevronUp, Copy, Eraser, Search } from "lucide-react";
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
  const [collapsed, setCollapsed] = useState(false);
  const [live, setLive] = useState(true);
  const logContainerRef = useRef<HTMLDivElement>(null);
  const { data: logs = [], clear } = useLogsQuery(live);

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

  const terminalHeight = collapsed ? "h-10" : "h-56";

  return (
    <section
      className={`flex-none border-t border-app-border bg-app-bg ${terminalHeight}`}
      data-purpose="terminal-panel">
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
            <button
              className="hover:text-white transition"
              onClick={() => {
                const text = filteredLogs
                  .map(
                    (l) => `[${l.time}] [${l.level}] [${l.source}] ${l.message}`
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
          </div>
        </div>
        {!collapsed && (
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
      </div>
    </section>
  );
}
