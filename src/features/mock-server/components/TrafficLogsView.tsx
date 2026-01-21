// =============================================================================
// Traffic Logs View Component
// Displays mock server traffic logs
// =============================================================================

import { Terminal } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import type { LogEntry } from "../types";

export interface TrafficLogsViewProps {
  logs: LogEntry[];
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
}

export function TrafficLogsView({
  logs,
  loading,
  error,
  onRefresh,
}: TrafficLogsViewProps) {
  return (
    <div className="flex-1 flex flex-col h-full">
      <div className="h-16 border-b border-app-border flex items-center justify-between px-6 bg-app-bg/50 backdrop-blur-sm sticky top-0 z-10 w-full">
        <div className="flex items-center gap-3 min-w-0 flex-1 mr-4">
          <Terminal className="w-4 h-4 text-app-subtext" />
          <span className="text-xs font-bold uppercase tracking-widest text-app-subtext">
            Traffic Logs
          </span>
          <span className="text-[10px] text-app-subtext/70">
            Auto refresh every 2s
          </span>
        </div>
        <div className="flex items-center gap-2 flex-shrink-0">
          {error && (
            <div className="text-xs text-red-400 bg-red-400/10 px-2 py-1 rounded border border-red-400/20 animate-in fade-in">
              {error}
            </div>
          )}
          <Button
            size="sm"
            variant="ghost"
            onClick={onRefresh}
            className="text-app-subtext hover:text-app-text"
          >
            Refresh
          </Button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-6">
        {loading ? (
          <div className="text-xs text-app-subtext">Loading logs...</div>
        ) : logs.length === 0 ? (
          <div className="text-xs text-app-subtext">
            No logs yet. Start the engine and send a request from curl or
            Postman.
          </div>
        ) : (
          <div className="space-y-2">
            {logs.map((entry, idx) => (
              <div
                key={`${entry.time}-${idx}`}
                className="p-3 rounded-lg border border-app-border bg-app-card/40"
              >
                <div className="flex items-center justify-between gap-3 mb-1">
                  <div className="text-[10px] uppercase tracking-wider text-app-subtext">
                    {entry.time} - {entry.level}
                  </div>
                  <span className="text-[10px] text-app-subtext/60">
                    {entry.source}
                  </span>
                </div>
                <div className="text-xs text-app-text">{entry.message}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
