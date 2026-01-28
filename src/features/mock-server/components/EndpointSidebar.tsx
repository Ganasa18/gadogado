// =============================================================================
// Endpoint Sidebar Component
// Left navigation sidebar with server controls
// =============================================================================

import { Square, Play, Server, Activity, Box } from "lucide-react";
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
    <div className="h-full bg-app-bg flex flex-col border-r border-app-border">
      {/* Top Section: Brand & Navigation */}
      <div className="flex-shrink-0">
        {/* Brand Header */}
        <div className="p-6 flex items-center gap-3">
          <div className="w-8 h-8 bg-app-accent rounded-lg flex items-center justify-center">
            <Box className="w-5 h-5 text-white" />
          </div>
          <span className="font-bold text-lg tracking-tight text-app-text">
            MockFlow
          </span>
        </div>
      </div>

      {/* Scrollable Content Area */}
      <div className="flex-1 overflow-y-auto min-h-0 py-4 custom-scrollbar">
        <nav className="px-4 space-y-2">
          <Button
            variant="ghost"
            className={`w-full justify-start gap-3 h-11 px-4 rounded-xl transition-all duration-200 ${
              viewMode === "endpoints"
                ? "bg-app-accent text-white opacity-100"
                : "text-app-subtext hover:text-app-text hover:bg-app-card/10"
            }`}
            onClick={() => onSetViewMode("endpoints")}>
            <Server className="w-4 h-4" />
            <span className="font-medium">Endpoints</span>
          </Button>

          <Button
            variant="ghost"
            className={`w-full justify-start gap-3 h-11 px-4 rounded-xl transition-all duration-200 ${
              viewMode === "logs"
                ? "bg-app-accent text-white opacity-100"
                : "text-app-subtext hover:text-app-text hover:bg-app-card/10"
            }`}
            onClick={() => onSetViewMode("logs")}>
            <Activity className="w-4 h-4" />
            <span className="font-medium">Traffic Logs</span>
          </Button>
        </nav>
      </div>

      {/* Footer Status Section */}
      <div className="p-4 flex-shrink-0 border-t border-app-border/30">
        <div className=" p-4 border-none  space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
              Server Status
            </span>
            <div
              className={`w-1.5 h-1.5 rounded-full ${status?.running ? "bg-app-success" : "bg-red-500"}`}
            />
          </div>

          {status?.running ? (
            <Button
              onClick={onStopServer}
              disabled={starting}
              className="w-full h-11 rounded-xl bg-red-500/10 text-red-500 border border-red-500/20 hover:bg-red-500/20 transition-all font-bold">
              {starting ? (
                <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2" />
              ) : (
                <Square className="w-4 h-4 mr-2 fill-current" />
              )}
              Stop Engine
            </Button>
          ) : (
            <Button
              onClick={onStartServer}
              disabled={starting}
              className="w-full h-11 rounded-xl bg-app-accent hover:bg-blue-600 text-white font-bold transition-all">
              {starting ? (
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" />
              ) : (
                <Play className="w-4 h-4 mr-2 fill-current" />
              )}
              Start Engine
            </Button>
          )}

          <div className="space-y-1.5">
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest block px-1">
              Global Port
            </span>
            <Input
              type="number"
              value={config?.port ?? 4010}
              onChange={(e: any) =>
                onPortChange(parseInt(e.target.value) || 4010)
              }
              className="h-10 text-sm bg-app-bg border-app-border rounded-xl text-app-text focus:ring-app-accent focus:border-app-accent"
            />
          </div>
        </div>
      </div>
    </div>
  );
}
