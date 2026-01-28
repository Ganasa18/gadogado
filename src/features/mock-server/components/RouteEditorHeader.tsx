// =============================================================================
// Route Editor Header Component
// Header for route editor with method badge, status, and actions
// =============================================================================

import { Rocket } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { Switch } from "../../../shared/components/Switch";
import { MethodBadge } from "./MethodBadge";
import type { MockRoute, MockServerStatus } from "../types";

export interface RouteEditorHeaderProps {
  route: MockRoute;
  status: MockServerStatus | null;
  error: string | null;
  hasUnsavedChanges: boolean;
  saving: boolean;
  starting: boolean;
  onUpdateRoute: (updater: (route: MockRoute) => MockRoute) => void;
  onDiscardChanges: () => void;
  onSave: () => void;
}

export function RouteEditorHeader({
  route,
  status,
  error,
  hasUnsavedChanges,
  saving,
  starting,
  onUpdateRoute,
  onDiscardChanges,
  onSave,
}: RouteEditorHeaderProps) {
  return (
    <div className="h-20 bg-app-bg flex items-center justify-between px-10 border-b border-app-border sticky top-0 z-10">
      <div className="flex items-center pl-4 gap-6 min-w-0 flex-1">
        <div className="flex items-center gap-6">
          <MethodBadge method={route.method} />
          <span className="font-mono text-sm text-app-subtext transition-colors truncate max-w-md">
            {route.path}
          </span>
        </div>
      </div>

      <div className="flex items-center gap-6">
        {error && (
          <div className="text-[10px] uppercase font-bold text-red-500 bg-red-500/5 px-3 py-1.5 rounded-lg border border-red-500/10">
            {error}
          </div>
        )}

        <div className="flex items-center gap-4 bg-app-card px-4 py-2 rounded-xl border border-app-border">
          <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
            Status
          </span>
          <Switch
            checked={route.enabled}
            onCheckedChange={(c) =>
              onUpdateRoute((r) => ({ ...r, enabled: c }))
            }
          />
        </div>

        <div className="h-6 w-px bg-app-border" />

        <div className="flex items-center gap-3">
          {status?.running && hasUnsavedChanges && (
            <span className="text-[10px] font-bold text-orange-500/80 uppercase tracking-tight mr-2 animate-pulse">
              Stop engine to deploy changes
            </span>
          )}
          <Button
            variant="ghost"
            onClick={onDiscardChanges}
            disabled={!hasUnsavedChanges || status?.running}
            className="text-app-subtext hover:text-app-text transition-colors text-xs font-bold font-sans">
            Discard
          </Button>

          <Button
            onClick={onSave}
            disabled={saving || starting || status?.running}
            className={`h-11 px-6 rounded-xl font-bold flex items-center gap-2 transition-all ${
              hasUnsavedChanges && !status?.running
                ? "bg-app-accent hover:bg-blue-600 text-white shadow-md"
                : "bg-app-card text-app-subtext border border-app-border opacity-50 cursor-not-allowed"
            }`}>
            {saving || starting ? (
              <div className="w-4 h-4 border-2 border-white/20 border-t-white rounded-full animate-spin" />
            ) : (
              <Rocket
                className={`w-4 h-4 ${hasUnsavedChanges && !status?.running ? "fill-current" : ""}`}
              />
            )}
            {saving || starting ? "Deploying..." : "Deploy"}
          </Button>
        </div>
      </div>
    </div>
  );
}
