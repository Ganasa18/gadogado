// =============================================================================
// Quick Switcher Sidebar Component
// Right sidebar for quick route navigation
// =============================================================================

import { AnimatePresence, motion } from "framer-motion";
import { Search, Plus, Trash2, ChevronDown } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { MethodBadge } from "./MethodBadge";
import type { MockServerConfig, MockServerStatus } from "../types";

export interface QuickSwitcherSidebarProps {
  config: MockServerConfig;
  status: MockServerStatus | null;
  selectedRouteId: string | null;
  onRouteSelect: (id: string) => void;
  onAddRoute: () => void;
  onRemoveRoute: (id: string) => void;
}

export function QuickSwitcherSidebar({
  config,
  status,
  selectedRouteId,
  onRouteSelect,
  onAddRoute,
  onRemoveRoute,
}: QuickSwitcherSidebarProps) {
  return (
    <div className="w-80 flex-shrink-0 bg-app-bg flex flex-col border-l border-app-border">
      <div className="p-6 flex items-center justify-between border-b border-app-border">
        <div className="flex items-center gap-2">
          <span className="text-xs font-bold uppercase tracking-widest text-app-subtext">
            Quick Switcher
          </span>
          <ChevronDown className="w-3.5 h-3.5 text-app-subtext" />
        </div>
      </div>

      <div className="p-4">
        <div className="relative group">
          <Search className="w-4 h-4 absolute left-3.5 top-3 text-app-subtext/40 group-focus-within:text-app-accent transition-colors" />
          <input
            className="w-full bg-app-card border border-app-border rounded-xl py-2.5 pl-10 pr-4 text-xs text-app-text outline-none focus:border-app-accent transition-all placeholder:text-app-subtext/30"
            placeholder="Search endpoints..."
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 pb-6 space-y-4 custom-scrollbar">
        <div className="px-1 text-[10px] font-bold text-app-subtext uppercase tracking-widest">
          Recent Endpoints
        </div>

        <div className="space-y-3">
          <AnimatePresence>
            {config.routes.map((route) => {
              const isActive = selectedRouteId === route.id;
              return (
                <motion.div
                  key={route.id}
                  layout
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  onClick={() => onRouteSelect(route.id)}
                  className={`group relative flex flex-col gap-3 p-4 rounded-2xl border cursor-pointer transition-all duration-300 ${
                    isActive
                      ? "bg-app-card border-app-accent shadow-lg"
                      : "bg-app-bg border-app-border hover:border-app-subtext/20 hover:bg-app-card/50"
                  }`}>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <MethodBadge method={route.method} />
                      <div className={`w-1 h-1 rounded-full ${route.enabled ? "bg-app-success" : "bg-app-subtext/20"}`} />
                      <span className={`text-[10px] uppercase font-bold tracking-tight ${route.enabled ? "text-app-success/80" : "text-app-subtext/40"}`}>
                        {route.enabled ? "Active" : "Static"}
                      </span>
                    </div>
                  </div>

                  <div className="space-y-1">
                    <div className={`font-mono text-xs truncate transition-colors ${isActive ? "text-app-text" : "text-app-subtext group-hover:text-app-text"}`}>
                      {route.path}
                    </div>
                    <div className="text-[10px] text-app-subtext/60 italic truncate">
                      {route.name || "No description provided"}
                    </div>
                  </div>

                  <Button
                    size="icon"
                    variant="ghost"
                    className="absolute top-4 right-4 h-6 w-6 opacity-0 group-hover:opacity-100 text-app-subtext/30 hover:text-red-500 hover:bg-red-500/10 rounded-lg transition-all"
                    onClick={(e) => {
                      e.stopPropagation();
                      onRemoveRoute(route.id);
                    }}>
                    <Trash2 className="w-3.5 h-3.5" />
                  </Button>
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>

        <Button
          variant="ghost"
          disabled={status?.running}
          className={`w-full h-12 rounded-2xl border border-dashed border-app-border text-app-subtext/60 hover:text-app-text transition-all text-xs font-bold gap-2 ${
            status?.running ? "opacity-30 cursor-not-allowed" : "hover:border-app-subtext/20 hover:bg-app-card/30"
          }`}
          onClick={onAddRoute}>
          <div className="w-5 h-5 rounded-full bg-app-card flex items-center justify-center">
            <Plus className="w-3.5 h-3.5" />
          </div>
          New Mock Endpoint
        </Button>
      </div>
    </div>
  );
}
