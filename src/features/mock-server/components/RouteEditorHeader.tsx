// =============================================================================
// Route Editor Header Component
// Header for route editor with method badge, status, and actions
// =============================================================================

import { Save } from "lucide-react";
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
  error,
  hasUnsavedChanges,
  saving,
  starting,
  onUpdateRoute,
  onDiscardChanges,
  onSave,
}: RouteEditorHeaderProps) {
  return (
    <div className="h-16 border-b border-app-border flex items-center justify-between px-6 bg-app-bg/50 backdrop-blur-sm sticky top-0 z-10 w-full">
      <div className="flex items-center gap-4 min-w-0 flex-1 mr-4">
        <MethodBadge method={route.method} />
        <span className="font-mono text-sm text-app-text/70 truncate">
          {route.path}
        </span>
      </div>
      <div className="flex items-center gap-2 flex-shrink-0">
        {error && (
          <div className="text-xs text-red-400 bg-red-400/10 px-2 py-1 rounded border border-red-400/20 mr-2 animate-in fade-in">
            {error}
          </div>
        )}
        <div className="flex items-center gap-2 mr-4">
          <span className="text-xs text-app-subtext">Is Active</span>
          <Switch
            checked={route.enabled}
            onCheckedChange={(c) =>
              onUpdateRoute((r) => ({ ...r, enabled: c }))
            }
          />
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={onDiscardChanges}
          className="text-app-subtext hover:text-red-400">
          Discard
        </Button>
        {hasUnsavedChanges && (
          <span className="text-[10px] text-amber-400 bg-amber-400/10 px-2 py-1 rounded border border-amber-400/20">
            Unsaved
          </span>
        )}
        <Button
          size="sm"
          onClick={onSave}
          disabled={saving || starting}
          className="bg-app-accent hover:bg-blue-600 text-white gap-2 shadow-lg shadow-blue-500/20">
          {saving || starting ? (
            "Deploying..."
          ) : (
            <>
              <Save className="w-3.5 h-3.5" /> Deploy Changes
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
