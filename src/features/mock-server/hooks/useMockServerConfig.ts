// =============================================================================
// Mock Server Config Hook
// Manages configuration loading, saving, and server control
// =============================================================================

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { isTauri } from "../../../utils/tauri";
import { mockServerApi } from "../api/client";
import type { MockServerConfig, MockServerStatus } from "../types";
import { createRoute, cloneConfig } from "../types";

export interface UseMockServerConfigReturn {
  // State
  config: MockServerConfig | null;
  savedConfig: MockServerConfig | null;
  status: MockServerStatus | null;
  loading: boolean;
  saving: boolean;
  starting: boolean;
  error: string | null;

  // Actions
  saveConfig: () => Promise<void>;
  startServer: () => Promise<void>;
  stopServer: () => Promise<void>;
  updateConfig: (
    updater: (config: MockServerConfig) => MockServerConfig,
  ) => void;
  setConfig: Dispatch<SetStateAction<MockServerConfig | null>>;
}

/**
 * Hook for managing mock server configuration and server state
 */
export function useMockServerConfig(): UseMockServerConfigReturn {
  const [config, setConfig] = useState<MockServerConfig | null>(null);
  const [savedConfig, setSavedConfig] = useState<MockServerConfig | null>(null);
  const [status, setStatus] = useState<MockServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeConfigRequest = useRef(0);
  const saveInFlight = useRef<Promise<void> | null>(null);
  const serverInFlight = useRef<Promise<void> | null>(null);

  // Load configuration on mount
  const loadConfig = useCallback(async () => {
    if (!isTauri()) return;
    const requestId = ++activeConfigRequest.current;
    setLoading(true);
    setError(null);
    try {
      const [configResponse, statusResponse] = await Promise.all([
        mockServerApi.getConfig(),
        mockServerApi.getStatus(),
      ]);
      if (activeConfigRequest.current !== requestId) return;
      const nextConfig =
        configResponse.routes.length === 0
          ? { ...configResponse, routes: [createRoute()] }
          : configResponse;
      setConfig(nextConfig);
      setSavedConfig(cloneConfig(configResponse));
      setStatus(statusResponse);
    } catch (err: any) {
      if (activeConfigRequest.current !== requestId) return;
      setError(err?.message || "Failed to load mock server config.");
    } finally {
      if (activeConfigRequest.current === requestId) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Save configuration
  const saveConfigWithConfig = useCallback(
    async (nextConfig: MockServerConfig) => {
      if (!isTauri()) return;
      if (saveInFlight.current) {
        await saveInFlight.current;
        return;
      }
      const run = (async () => {
        setSaving(true);
        setError(null);
        try {
          const validRoutes = nextConfig.routes.map((r) => ({
            ...r,
            id:
              r.id ||
              `mock_${Date.now()}_${Math.random().toString(36).slice(2)}`,
            matchers: {
              queryParams: r.matchers.queryParams ?? [],
              headers: r.matchers.headers ?? [],
              body: r.matchers.body
                ? {
                    ...r.matchers.body,
                    bodyType: r.matchers.body.bodyType ?? "raw_json",
                    formData: r.matchers.body.formData ?? [],
                    formUrlencode: r.matchers.body.formUrlencode ?? [],
                  }
                : null,
            },
            response: {
              ...r.response,
              headers: r.response.headers ?? [],
              bodyType: r.response.bodyType ?? "raw",
              rawSubType: r.response.rawSubType ?? "json",
              formData: r.response.formData ?? [],
              formUrlencode: r.response.formUrlencode ?? [],
              delayMs: r.response.delayMs ?? null,
            },
          }));

          const configToSend = { ...nextConfig, routes: validRoutes };

          const updated = await mockServerApi.updateConfig(configToSend);
          setConfig(updated);
          setSavedConfig(cloneConfig(updated));

          const nextStatus = await mockServerApi.getStatus();
          setStatus(nextStatus);
        } catch (err: any) {
          console.error("Failed to save config:", err);
          setError(
            err?.message ||
              err?.toString() ||
              "Failed to save mock server config.",
          );
          throw err;
        } finally {
          setSaving(false);
        }
      })();
      saveInFlight.current = run;
      try {
        await run;
      } finally {
        saveInFlight.current = null;
      }
    },
    [],
  );

  // Public save method
  const saveConfig = useCallback(async () => {
    if (!isTauri() || !config) return;
    try {
      const wasRunning = status?.running ?? false;
      await saveConfigWithConfig(config);

      if (wasRunning) {
        setStarting(true);
        setError(null);
        try {
          await mockServerApi.stop();
          const startResponse = await mockServerApi.start();
          setStatus(startResponse);
        } catch (restartErr: any) {
          console.error("Failed to restart server:", restartErr);
          setError(
            restartErr?.message || "Failed to restart server after deployment.",
          );
        } finally {
          setStarting(false);
        }
      }
    } catch (err) {
      console.error("Save config error:", err);
    }
  }, [config, saveConfigWithConfig, status?.running]);

  // Start server
  const startServer = async () => {
    if (!isTauri()) return;
    if (serverInFlight.current) return serverInFlight.current;
    const run = (async () => {
      setStarting(true);
      setError(null);
      try {
        if (config) {
          try {
            await saveConfigWithConfig(config);
          } catch (saveErr: any) {
            console.error("Failed to save config before starting:", saveErr);
            setError("Failed to save config before starting server.");
            return;
          }
        }
        const startResponse = await mockServerApi.start();
        setStatus(startResponse);
      } catch (err: any) {
        console.error("Failed to start server:", err);
        const errMsg = err?.message || err?.toString() || "";
        if (errMsg.includes("already running")) {
          try {
            const updated = await mockServerApi.getStatus();
            setStatus(updated);
          } catch {
            setError("Server may be running but status check failed.");
          }
        } else {
          setError(errMsg || "Failed to start mock server.");
        }
      } finally {
        setStarting(false);
      }
    })();
    serverInFlight.current = run;
    try {
      await run;
    } finally {
      serverInFlight.current = null;
    }
  };

  // Stop server
  const stopServer = async () => {
    if (!isTauri()) return;
    if (serverInFlight.current) return serverInFlight.current;
    const run = (async () => {
      setStarting(true);
      setError(null);
      try {
        const stopResponse = await mockServerApi.stop();
        setStatus(stopResponse);
      } catch (err: any) {
        console.error("Failed to stop server:", err);
        const errMsg = err?.message || err?.toString() || "";
        setError(errMsg || "Failed to stop mock server.");
      } finally {
        setStarting(false);
      }
    })();
    serverInFlight.current = run;
    try {
      await run;
    } finally {
      serverInFlight.current = null;
    }
  };

  // Update config helper
  const updateConfig = useCallback(
    (updater: (config: MockServerConfig) => MockServerConfig) => {
      setConfig((prev) => {
        if (!prev) return prev;
        return updater(prev);
      });
    },
    [],
  );

  return {
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
    updateConfig,
    setConfig,
  };
}
