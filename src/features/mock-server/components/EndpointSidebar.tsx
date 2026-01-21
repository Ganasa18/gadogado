// =============================================================================
// Endpoint Sidebar Component
// Left navigation sidebar with server controls
// =============================================================================

import { Square, Play, Settings, Server, Activity } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { Input } from "../../../shared/components/Input";
import type { MockServerStatus, MockServerConfig } from "../types";

export interface EndpointSidebarProps {
  viewMode: "endpoints" | "logs";
  status: MockServerStatus | null;
  config: MockServerConfig | null;
  starting: boolean;
  onSetViewMode: (mode: "endpoints" | "logs") => void;
  onStartServer: () => void;
  onStopServer: () => void;
  onPortChange: (port: number) => void;
}

export function EndpointSidebar({
  viewMode,
  status,
  config,
  starting,
  onSetViewMode,
  onStartServer,
  onStopServer,
  onPortChange,
}: EndpointSidebarProps) {
  return (
    <div className="w-64 flex-shrink-0 border-r border-app-border bg-app-panel flex flex-col">
      <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
        <Button
          variant={viewMode === "endpoints" ? "primary" : "ghost"}
          className={`w-full justify-start gap-3 ${
            viewMode === "endpoints"
              ? "bg-app-accent/10 text-app-accent"
              : "text-app-subtext"
          }`}
          onClick={() => onSetViewMode("endpoints")}
        >
          <Server className="w-4 h-4" />
          Endpoints
        </Button>
        <Button
          variant={viewMode === "logs" ? "primary" : "ghost"}
          className={`w-full justify-start gap-3 ${
            viewMode === "logs"
              ? "bg-app-accent/10 text-app-accent"
              : "text-app-subtext"
          }`}
          onClick={() => onSetViewMode("logs")}
        >
          <Activity className="w-4 h-4" />
          Traffic Logs
        </Button>
      </nav>

      <div className="p-3 border-t border-app-border space-y-3">
        <div className="flex items-center justify-between px-2">
          <span className="text-xs font-semibold text-app-subtext uppercase tracking-wider">
            Status
          </span>
          <span
            className={`flex items-center gap-1.5 text-[10px] font-bold px-2 py-0.5 rounded-full ${
              status?.running
                ? "bg-emerald-500/10 text-emerald-500"
                : "bg-app-subtext/10 text-app-subtext"
            }`}
          >
            <div
              className={`w-1.5 h-1.5 rounded-full ${
                status?.running ? "bg-emerald-500" : "bg-app-subtext"
              }`}
            />
            {status?.running ? "LIVE" : "STOPPED"}
          </span>
        </div>

        {status?.running ? (
          <Button
            onClick={onStopServer}
            disabled={starting}
            className="w-full justify-center bg-app-card border border-app-border hover:bg-red-500/10 hover:text-red-400 hover:border-red-500/20 transition-colors"
          >
            {starting ? (
              <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2" />
            ) : (
              <Square className="w-4 h-4 mr-2" />
            )}
            Stop Server
          </Button>
        ) : (
          <Button
            onClick={onStartServer}
            disabled={starting}
            className="w-full justify-center bg-emerald-500 hover:bg-emerald-600 text-white border-0 shadow-lg shadow-emerald-500/20"
          >
            {starting ? (
              <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" />
            ) : (
              <Play className="w-4 h-4 mr-2" />
            )}
            Start Engine
          </Button>
        )}

        <div className="pt-2">
          <div className="flex items-center gap-2 mb-1 px-2">
            <Settings className="w-3 h-3 text-app-subtext" />
            <span className="text-[10px] text-app-subtext uppercase font-semibold">
              Global Port
            </span>
          </div>
          <Input
            type="number"
            value={config?.port ?? 4010}
            onChange={(e: any) =>
              onPortChange(parseInt(e.target.value) || 4010)
            }
            className="h-8 text-xs bg-app-bg border-app-border"
          />
        </div>
      </div>
    </div>
  );
}
