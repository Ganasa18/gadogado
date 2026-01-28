// =============================================================================
// Mock Server Tab - Main Page Component
// Orchestration layer for the mock server UI
// =============================================================================

import { useCallback, useEffect, useMemo, useState } from "react";
import { LayoutGrid, Plus, Settings2, FileCheck, Reply, Terminal } from "lucide-react";
import { Button } from "../../../shared/components/Button";

// Feature imports
import {
  useMockServerConfig,
  useRouteManagement,
  useTrafficLogs,
  useCurlGenerator,
  useCopyToClipboard,
} from "../hooks";

// Component imports
import {
  EndpointSidebar,
  QuickSwitcherSidebar,
  TrafficLogsView,
  RouteEditorHeader,
  EndpointDefinitionSection,
  IncomingRequestSection,
  MockResponseSection,
  CurlCommandSection,
  MockServerLayout,
} from "../components";

import { createRoute, type MockRoute } from "../types";

type ViewMode = "endpoints" | "logs";
type EditorTab = "definition" | "validation" | "response" | "curl";

export default function MockServerTab() {
  // View state
  const [viewMode, setViewMode] = useState<ViewMode>("endpoints");
  const [activeTab, setActiveTab] = useState<EditorTab>("definition");

  // Server state
  const {
    config,
    savedConfig,
    status,
    loading,
    saving,
    starting,
    error,
    saveConfig,
    startServer,
    stopServer,
    setConfig,
  } = useMockServerConfig();

  // Route management
  const {
    selectedRouteId,
    selectedRoute,
    hasUnsavedChanges,
    actions: {
      setSelectedRouteId,
      removeRoute,
      updateRoute,
      discardRouteChanges,
    },
  } = useRouteManagement({
    config,
    savedConfig,
    setConfig,
    onPersistRoute: async () => {
      if (status?.running) {
        await saveConfig();
      }
    },
  });

  // Logs
  const {
    logs,
    loading: logsLoading,
    error: logsError,
    refresh: refreshLogs,
  } = useTrafficLogs({
    enabled: viewMode === "logs",
    refreshInterval: 2000,
  });

  // Utilities
  const { generateCurlCommand } = useCurlGenerator();
  const { lastCopied, copyToClipboard } = useCopyToClipboard();

  // Base URL for cURL commands
  const baseUrl = useMemo(() => {
    const port = config?.port ?? status?.port ?? 4010;
    return `http://127.0.0.1:${port}`;
  }, [config?.port, status?.port]);

  // Initialize selected route on load
  useEffect(() => {
    if (config && config.routes.length > 0 && !selectedRouteId) {
      setSelectedRouteId(config.routes[0].id);
    }
  }, [config, selectedRouteId, setSelectedRouteId]);

  // Handle port change
  const handlePortChange = useCallback(
    (port: number) => {
      setConfig((prev) => (prev ? { ...prev, port } : prev));
    },
    [setConfig],
  );

  // Handle add route
  const handleAddRoute = useCallback(() => {
    const newRoute = createRoute();
    setConfig((prev) =>
      prev ? { ...prev, routes: [newRoute, ...prev.routes] } : prev,
    );
    setSelectedRouteId(newRoute.id);
    setActiveTab("definition");
  }, [setConfig, setSelectedRouteId]);

  // Loading state
  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-app-bg text-app-subtext">
        <div className="flex flex-col items-center gap-4">
          <div className="w-10 h-10 border-2 border-app-accent border-t-transparent rounded-full animate-spin" />
          <p className="text-sm font-medium tracking-wide">Loading Mock Engine...</p>
        </div>
      </div>
    );
  }

  // Error state (no config)
  if (!config) {
    return (
      <div className="flex h-full items-center justify-center bg-app-bg">
        <div className="text-red-400 font-medium">Unable to load mock server config.</div>
      </div>
    );
  }

  return (
    <MockServerLayout
      leftSidebar={
        <EndpointSidebar
          viewMode={viewMode}
          status={status}
          config={config}
          starting={starting}
          onSetViewMode={setViewMode}
          onStartServer={startServer}
          onStopServer={stopServer}
          onPortChange={handlePortChange}
        />
      }
      rightSidebar={
        <QuickSwitcherSidebar
          config={config}
          status={status}
          selectedRouteId={selectedRouteId}
          onRouteSelect={setSelectedRouteId}
          onAddRoute={handleAddRoute}
          onRemoveRoute={removeRoute}
        />
      }
    >
      <div className="flex-1 flex flex-col min-w-0 h-full relative">
        {viewMode === "logs" ? (
          <TrafficLogsView
            logs={logs}
            loading={logsLoading}
            error={logsError}
            onRefresh={refreshLogs}
          />
        ) : selectedRoute ? (
          <div className="flex-1 flex flex-col h-full bg-app-panel rounded-tl-[32px] overflow-hidden border-t border-l border-app-border mx-2 my-2 shadow-xl">
            <RouteEditorHeader
              route={selectedRoute}
              status={status}
              error={error}
              hasUnsavedChanges={hasUnsavedChanges}
              saving={saving}
              starting={starting}
              onUpdateRoute={(updater: (r: MockRoute) => MockRoute) =>
                updateRoute(selectedRoute.id, updater, {
                  persist: status?.running ?? false,
                })
              }
              onDiscardChanges={() => discardRouteChanges(selectedRoute.id)}
              onSave={saveConfig}
            />

            {/* Editor Tabs Navigation */}
            <div className="px-8 flex items-center gap-1 border-b border-app-border bg-app-bg/30">
              {[
                { id: "definition", label: "Definition", icon: Settings2 },
                { id: "validation", label: "Body Validation", icon: FileCheck },
                { id: "response", label: "Mock Response", icon: Reply },
                { id: "curl", label: "CURL Generator", icon: Terminal },
              ].map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id as EditorTab)}
                  className={`flex items-center gap-2 px-6 py-4 text-xs font-bold transition-all relative ${
                    activeTab === tab.id 
                      ? "text-app-accent" 
                      : "text-app-subtext hover:text-app-text"
                  }`}>
                  <tab.icon className="w-3.5 h-3.5" />
                  {tab.label}
                  {activeTab === tab.id && (
                    <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-app-accent" />
                  )}
                </button>
              ))}
            </div>

            {/* Tab Content */}
            <div className="flex-1 overflow-y-auto p-8 space-y-12 pb-20 custom-scrollbar">
              {activeTab === "definition" && (
                <EndpointDefinitionSection
                  route={selectedRoute}
                  onUpdateRoute={(updater: (r: MockRoute) => MockRoute) => updateRoute(selectedRoute.id, updater)}
                />
              )}

              {activeTab === "validation" && (
                <IncomingRequestSection
                  route={selectedRoute}
                  showOnly="body"
                  onUpdateRoute={(updater: (r: MockRoute) => MockRoute) => updateRoute(selectedRoute.id, updater)}
                />
              )}

              {activeTab === "response" && (
                <MockResponseSection
                  route={selectedRoute}
                  onUpdateRoute={(updater: (r: MockRoute) => MockRoute) => updateRoute(selectedRoute.id, updater)}
                />
              )}

              {activeTab === "curl" && (
                <CurlCommandSection
                  route={selectedRoute}
                  baseUrl={baseUrl}
                  generateCurlCommand={generateCurlCommand}
                  lastCopied={lastCopied}
                  onCopyToClipboard={copyToClipboard}
                />
              )}
            </div>

            {/* Bottom Footer Action */}
            <div className="absolute bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-3">
              {activeTab !== 'curl' && (
                <Button
                  onClick={() => {
                    const tabs: EditorTab[] = ['definition', 'validation', 'response', 'curl'];
                    const nextIndex = (tabs.indexOf(activeTab) + 1) % tabs.length;
                    setActiveTab(tabs[nextIndex]);
                  }}
                  className="bg-app-card text-app-subtext hover:text-app-text border border-app-border h-11 px-8 rounded-xl font-bold shadow-lg">
                  Next Step
                </Button>
              )}
            </div>
          </div>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center text-app-subtext space-y-6">
            <div className="w-20 h-20 rounded-[32px] bg-app-card border border-app-border flex items-center justify-center mb-2 shadow-xl">
              <LayoutGrid className="w-10 h-10 opacity-20" />
            </div>
            <div className="space-y-2 text-center">
              <h3 className="text-xl font-bold text-app-text tracking-tight">No Endpoint Selected</h3>
              <p className="text-sm max-w-xs text-app-subtext font-medium">
                Select an existing endpoint from the quick switcher or create a new one to get started.
              </p>
            </div>
            {status?.running && (
              <div className="flex items-center gap-2 px-4 py-2 bg-orange-500/10 border border-orange-500/20 rounded-xl animate-in fade-in slide-in-from-bottom-2 duration-300">
                <div className="w-1.5 h-1.5 rounded-full bg-orange-500 animate-pulse" />
                <span className="text-[10px] font-bold text-orange-500 uppercase tracking-widest leading-none">
                  Stop Engine to Add New Endpoints
                </span>
              </div>
            )}
            <Button
              onClick={handleAddRoute}
              disabled={status?.running}
              className={`mt-4 h-12 px-8 rounded-2xl font-bold shadow-lg transition-all flex items-center gap-3 ${
                status?.running
                  ? "bg-app-card text-app-subtext border border-app-border opacity-50 cursor-not-allowed"
                  : "bg-app-accent hover:bg-blue-600 text-white shadow-app-accent/10"
              }`}>
              <Plus className="w-5 h-5" /> Create New Endpoint
            </Button>
          </div>
        )}
      </div>
    </MockServerLayout>
  );
}
