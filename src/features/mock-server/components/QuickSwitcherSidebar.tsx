// =============================================================================
// Quick Switcher Sidebar Component
// Right sidebar for quick route navigation
// =============================================================================

import { AnimatePresence, motion } from "framer-motion";
import { Search, Plus, Trash2 } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { MethodBadge } from "./MethodBadge";
import type { MockServerConfig } from "../types";

export interface QuickSwitcherSidebarProps {
  config: MockServerConfig;
  selectedRouteId: string | null;
  onRouteSelect: (id: string) => void;
  onAddRoute: () => void;
  onRemoveRoute: (id: string) => void;
}

export function QuickSwitcherSidebar({
  config,
  selectedRouteId,
  onRouteSelect,
  onAddRoute,
  onRemoveRoute,
}: QuickSwitcherSidebarProps) {
  return (
    <div className="w-72 flex-shrink-0 border-l border-app-border bg-app-card/30 flex flex-col">
      <div className="p-4 border-b border-app-border flex items-center justify-between">
        <h3 className="text-xs font-bold uppercase tracking-widest text-app-subtext">
          Quick Switcher
        </h3>
        <div className="w-5 h-5 rounded border border-app-border flex items-center justify-center">
          <span className="text-[10px] text-app-subtext">⌘</span>
        </div>
      </div>

      <div className="p-3 border-b border-app-border">
        <div className="relative">
          <Search className="w-3.5 h-3.5 absolute left-3 top-2.5 text-app-subtext" />
          <input
            className="w-full bg-app-bg border border-app-border rounded-md py-1.5 pl-9 pr-3 text-xs text-app-text outline-none focus:border-app-accent transition-colors placeholder:text-app-subtext/50"
            placeholder="Filter endpoints..."
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        <div className="px-2 py-2 text-[10px] font-bold text-app-subtext uppercase tracking-wider">
          Recent Endpoints
        </div>

        <AnimatePresence>
          {config.routes.map((route) => {
            const isActive = selectedRouteId === route.id;
            return (
              <motion.div
                key={route.id}
                layout
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                onClick={() => onRouteSelect(route.id)}
                className={`group flex flex-col gap-1 p-3 rounded-lg border cursor-pointer transition-all duration-200 ${
                  isActive
                    ? "bg-app-accent/5 border-app-accent/20 shadow-sm"
                    : "bg-transparent border-transparent hover:bg-app-panel hover:border-app-border"
                }`}>
                <div className="flex items-center justify-between">
                  <MethodBadge method={route.method} />
                  {route.enabled ? (
                    <span className="text-[10px] text-emerald-500 font-medium opacity-80">
                      Active
                    </span>
                  ) : (
                    <span className="text-[10px] text-app-subtext font-medium opacity-50">
                      Inactive
                    </span>
                  )}
                </div>
                <div
                  className={`font-mono text-xs truncate transition-colors ${
                    isActive
                      ? "text-app-text font-medium"
                      : "text-app-subtext group-hover:text-app-text"
                  }`}>
                  {route.path}
                </div>
                <div className="flex items-center justify-between pt-1">
                  <span className="text-[10px] text-app-subtext">
                    {route.name}
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-5 w-5 opacity-0 group-hover:opacity-100 text-app-subtext hover:text-red-400"
                    onClick={(e) => {
                      e.stopPropagation();
                      onRemoveRoute(route.id);
                    }}>
                    <Trash2 className="w-3 h-3" />
                  </Button>
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>

        <Button
          variant="ghost"
          className="w-full mt-2 text-xs text-app-subtext hover:text-app-accent border border-dashed border-app-border hover:border-app-accent/50"
          onClick={onAddRoute}>
          <Plus className="w-3.5 h-3.5 mr-2" /> Add Mock
        </Button>
      </div>
    </div>
  );
}
