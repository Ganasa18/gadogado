// =============================================================================
// Route Management Hook
// Manages CRUD operations for mock routes
// =============================================================================

import { useCallback, useMemo, useState } from "react";
import type { MockRoute, MockServerConfig } from "../types";
import { createRoute, cloneRoute, serializeConfig } from "../types";

export interface UseRouteManagementReturn {
  selectedRouteId: string | null;
  selectedRoute: MockRoute | undefined;
  hasUnsavedChanges: boolean;

  actions: {
    setSelectedRouteId: (id: string | null) => void;
    addRoute: () => void;
    removeRoute: (id: string) => void;
    updateRoute: (
      id: string,
      updater: (route: MockRoute) => MockRoute,
      options?: { persist?: boolean }
    ) => void;
    discardRouteChanges: (id: string) => void;
  };
}

export interface UseRouteManagementProps {
  config: MockServerConfig | null;
  savedConfig: MockServerConfig | null;
  setConfig: (config: MockServerConfig | ((prev: MockServerConfig | null) => MockServerConfig | null)) => void;
  onPersistRoute?: (route: MockRoute) => void;
}

/**
 * Hook for managing mock server routes
 * Handles CRUD operations and unsaved changes tracking
 */
export function useRouteManagement({
  config,
  savedConfig,
  setConfig,
  onPersistRoute,
}: UseRouteManagementProps): UseRouteManagementReturn {
  const [selectedRouteId, setSelectedRouteId] = useState<string | null>(null);

  const selectedRoute = useMemo(
    () => config?.routes.find((r) => r.id === selectedRouteId),
    [config?.routes, selectedRouteId]
  );

  const hasUnsavedChanges = useMemo(() => {
    if (!config || !savedConfig) return false;
    return serializeConfig(config) !== serializeConfig(savedConfig);
  }, [config, savedConfig]);

  const addRoute = useCallback(() => {
    const newRoute = createRoute();
    setConfig((prev) =>
      prev ? { ...prev, routes: [newRoute, ...prev.routes] } : prev
    );
    setSelectedRouteId(newRoute.id);
  }, [setConfig]);

  const removeRoute = useCallback((id: string) => {
    setConfig((prev) => {
      if (!prev) return prev;
      const newRoutes = prev.routes.filter((route) => route.id !== id);
      if (selectedRouteId === id) {
        setSelectedRouteId(newRoutes[0]?.id || null);
      }
      return { ...prev, routes: newRoutes };
    });
  }, [setConfig, selectedRouteId]);

  const updateRoute = useCallback(
    (
      id: string,
      updater: (route: MockRoute) => MockRoute,
      options?: { persist?: boolean }
    ) => {
      setConfig((prev) => {
        if (!prev) return prev;
        const nextConfig = {
          ...prev,
          routes: prev.routes.map((route) =>
            route.id === id ? updater(route) : route
          ),
        };
        if (options?.persist && onPersistRoute) {
          const updatedRoute = nextConfig.routes.find(r => r.id === id);
          if (updatedRoute) {
            void onPersistRoute(updatedRoute);
          }
        }
        return nextConfig;
      });
    },
    [setConfig, onPersistRoute]
  );

  const discardRouteChanges = useCallback(
    (id: string) => {
      if (!savedConfig) return;
      const savedRoute = savedConfig.routes.find((route) => route.id === id);
      if (!savedRoute) {
        removeRoute(id);
        return;
      }
      updateRoute(id, () => cloneRoute(savedRoute));
    },
    [removeRoute, savedConfig, updateRoute]
  );

  return {
    selectedRouteId,
    selectedRoute,
    hasUnsavedChanges,
    actions: {
      setSelectedRouteId,
      addRoute,
      removeRoute,
      updateRoute,
      discardRouteChanges,
    },
  };
}
