// =============================================================================
// Mock Server Tab - Main Page Component
// Orchestration layer for the mock server UI
// =============================================================================

import { useCallback, useEffect, useMemo, useState } from "react";
import { LayoutGrid, Plus } from "lucide-react";
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
} from "../components";

import { createRoute } from "../types";

type ViewMode = "endpoints" | "logs";

export default function MockServerTab() {
  // View state
  const [viewMode, setViewMode] = useState<ViewMode>("endpoints");

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
  }, [setConfig, setSelectedRouteId]);

  // Loading state
  if (loading) {
    return (
      <div className="flex h-full items-center justify-center text-app-subtext">
        <div className="flex flex-col items-center gap-4">
          <div className="w-8 h-8 border-2 border-app-accent border-t-transparent rounded-full animate-spin" />
          <p>Loading Mock Server...</p>
        </div>
      </div>
    );
  }

  // Error state (no config)
  if (!config) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-red-400">Unable to load mock server config.</div>
      </div>
    );
  }

  return (
    <div className="flex h-full bg-app-bg text-app-text overflow-hidden font-sans">
      {/* Left Sidebar */}
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

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0 bg-app-bg relative">
        {viewMode === "logs" ? (
          <TrafficLogsView
            logs={logs}
            loading={logsLoading}
            error={logsError}
            onRefresh={refreshLogs}
          />
        ) : selectedRoute ? (
          <div className="flex-1 flex flex-col h-full">
            <RouteEditorHeader
              route={selectedRoute}
              status={status}
              error={error}
              hasUnsavedChanges={hasUnsavedChanges}
              saving={saving}
              starting={starting}
              onUpdateRoute={(updater) =>
                updateRoute(selectedRoute.id, updater, {
                  persist: status?.running ?? false,
                })
              }
              onDiscardChanges={() => discardRouteChanges(selectedRoute.id)}
              onSave={saveConfig}
            />

            <div className="flex-1 overflow-y-auto p-6 space-y-8">
              <EndpointDefinitionSection
                route={selectedRoute}
                onUpdateRoute={(updater) =>
                  updateRoute(selectedRoute.id, updater)
                }
              />

              <IncomingRequestSection
                route={selectedRoute}
                onUpdateRoute={(updater) =>
                  updateRoute(selectedRoute.id, updater)
                }
              />

              <MockResponseSection
                route={selectedRoute}
                onUpdateRoute={(updater) =>
                  updateRoute(selectedRoute.id, updater)
                }
              />

              <CurlCommandSection
                route={selectedRoute}
                baseUrl={baseUrl}
                generateCurlCommand={generateCurlCommand}
                lastCopied={lastCopied}
                onCopyToClipboard={copyToClipboard}
              />
            </div>
          </div>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center text-app-subtext space-y-4">
            <div className="w-16 h-16 rounded-2xl bg-app-panel flex items-center justify-center mb-2">
              <LayoutGrid className="w-8 h-8 opacity-50" />
            </div>
            <h3 className="text-lg font-medium text-app-text">
              No Endpoint Selected
            </h3>
            <p className="text-sm max-w-xs text-center">
              Select an existing endpoint from the quick switcher or create a
              new one to get started.
            </p>
            <Button
              onClick={handleAddRoute}
              className="mt-4 bg-app-accent text-white">
              <Plus className="w-4 h-4 mr-2" /> Create New Endpoint
            </Button>
          </div>
        )}
      </div>

      {/* Right Sidebar */}
      <QuickSwitcherSidebar
        config={config}
        selectedRouteId={selectedRouteId}
        onRouteSelect={setSelectedRouteId}
        onAddRoute={handleAddRoute}
        onRemoveRoute={removeRoute}
      />
    </div>
  );
}
