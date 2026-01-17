import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "framer-motion";
import {
  Activity,
  ChevronRight,
  Copy,
  LayoutGrid,
  Play,
  Plus,
  Save,
  Search,
  Server,
  Settings,
  Square,
  Terminal,
  Trash2,
} from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { Input } from "../../../shared/components/Input";
import { Select } from "../../../shared/components/Select";
import { Switch } from "../../../shared/components/Switch";
import { TextArea } from "../../../shared/components/TextArea";
import { isTauri } from "../../../utils/tauri";

const METHODS = ["GET", "POST", "PATCH", "DELETE", "OPTIONS", "PUT"] as const;
const MATCH_MODES = ["exact", "contains", "regex"] as const;

type HttpMethod = (typeof METHODS)[number];
type MatchMode = (typeof MATCH_MODES)[number];

interface MockKeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

interface MockBodyMatch {
  mode: MatchMode;
  value: string;
}

interface MockRouteMatchers {
  queryParams: MockKeyValue[];
  headers: MockKeyValue[];
  body?: MockBodyMatch | null;
}

interface MockResponse {
  status: number;
  headers: MockKeyValue[];
  body: string;
  delayMs?: number | null;
}

interface MockRoute {
  id: string;
  name: string;
  enabled: boolean;
  method: HttpMethod;
  path: string;
  matchers: MockRouteMatchers;
  response: MockResponse;
}

interface MockServerConfig {
  port: number;
  routes: MockRoute[];
}

interface MockServerStatus {
  running: boolean;
  port: number;
  url: string;
  routeCount: number;
}

interface LogEntry {
  time: string;
  level: string;
  source: string;
  message: string;
}

const createKeyValue = (): MockKeyValue => ({
  key: "",
  value: "",
  enabled: true,
});

const createRoute = (): MockRoute => ({
  id:
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `mock_${Date.now()}_${Math.random().toString(36).slice(2)}`,
  name: "New mock",
  enabled: true,
  method: "GET",
  path: "/api/v1/resource",
  matchers: {
    queryParams: [],
    headers: [],
    body: null,
  },
  response: {
    status: 200,
    headers: [
      { key: "Content-Type", value: "application/json", enabled: true }
    ],
    body: "{\n  \"status\": \"success\",\n  \"data\": {\n    \"id\": \"123\",\n    \"message\": \"Hello World\"\n  }\n}",
    delayMs: null,
  },
});

const MethodBadge = ({ method }: { method: string }) => {
  const colors: Record<string, string> = {
    GET: "text-green-400 bg-green-400/10 border-green-400/20",
    POST: "text-blue-400 bg-blue-400/10 border-blue-400/20",
    PUT: "text-orange-400 bg-orange-400/10 border-orange-400/20",
    DELETE: "text-red-400 bg-red-400/10 border-red-400/20",
    PATCH: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  };
  const style = colors[method] || "text-gray-400 bg-gray-400/10 border-gray-400/20";
  
  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-bold border ${style}`}>
      {method}
    </span>
  );
};

const cloneRoute = (route: MockRoute): MockRoute =>
  JSON.parse(JSON.stringify(route)) as MockRoute;

const cloneConfig = (config: MockServerConfig): MockServerConfig =>
  JSON.parse(JSON.stringify(config)) as MockServerConfig;

const normalizeConfig = (config: MockServerConfig): MockServerConfig => ({
  ...config,
  routes: config.routes.map((route) => ({
    ...route,
    matchers: {
      queryParams: route.matchers.queryParams ?? [],
      headers: route.matchers.headers ?? [],
      body: route.matchers.body ? { ...route.matchers.body } : null,
    },
    response: {
      ...route.response,
      headers: route.response.headers ?? [],
      delayMs: route.response.delayMs ?? null,
    },
  })),
});

const serializeConfig = (config: MockServerConfig | null) =>
  config ? JSON.stringify(normalizeConfig(config)) : "";

export default function MockServerTab() {
  const [config, setConfig] = useState<MockServerConfig | null>(null);
  const [savedConfig, setSavedConfig] = useState<MockServerConfig | null>(null);
  const [status, setStatus] = useState<MockServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logsError, setLogsError] = useState<string | null>(null);
  const [logsLoading, setLogsLoading] = useState(false);
  const [selectedRouteId, setSelectedRouteId] = useState<string | null>(null);
  const [lastCopied, setLastCopied] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<"endpoints" | "logs">("endpoints");
  const activeConfigRequest = useRef(0);
  const saveInFlight = useRef<Promise<void> | null>(null);
  const serverInFlight = useRef<Promise<void> | null>(null);
  const activeLogRequest = useRef(0);

  const copyToClipboard = useCallback((label: string, value: string) => {
    if (!value) return;
    navigator.clipboard.writeText(value);
    setLastCopied(label);
    window.setTimeout(() => setLastCopied(null), 1500);
  }, []);

  const baseUrl = useMemo(() => {
    const port = config?.port ?? status?.port ?? 4010;
    return `http://127.0.0.1:${port}`;
  }, [config?.port, status?.port]);

  const loadConfig = useCallback(async () => {
    if (!isTauri()) return;
    const requestId = ++activeConfigRequest.current;
    setLoading(true);
    setError(null);
    try {
      const [configResponse, statusResponse] = await Promise.all([
        invoke<MockServerConfig>("mock_server_get_config"),
        invoke<MockServerStatus>("mock_server_status"),
      ]);
      if (activeConfigRequest.current !== requestId) return;
      const nextConfig =
        configResponse.routes.length === 0
          ? { ...configResponse, routes: [createRoute()] }
          : configResponse;
      setConfig(nextConfig);
      setSavedConfig(cloneConfig(configResponse));
      setStatus(statusResponse);
      setSelectedRouteId(
        (prev) => prev ?? nextConfig.routes[0]?.id ?? null
      );
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
            id: r.id || `mock_${Date.now()}_${Math.random().toString(36).slice(2)}`,
            response: {
              ...r.response,
              delayMs: r.response.delayMs ?? null,
            },
          }));

          const configToSend = { ...nextConfig, routes: validRoutes };

          const updated = await invoke<MockServerConfig>(
            "mock_server_update_config",
            { config: configToSend }
          );
          setConfig(updated);
          setSavedConfig(cloneConfig(updated));

          const nextStatus = await invoke<MockServerStatus>("mock_server_status");
          setStatus(nextStatus);
        } catch (err: any) {
          console.error("Failed to save config:", err);
          setError(err?.message || err?.toString() || "Failed to save mock server config.");
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
    []
  );

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
        if (options?.persist) {
          void saveConfigWithConfig(nextConfig).catch((err) => {
            console.error("Failed to persist route update:", err);
          });
        }
        return nextConfig;
      });
    },
    [saveConfigWithConfig]
  );

  const addRoute = () => {
    const newRoute = createRoute();
    setConfig((prev) =>
      prev ? { ...prev, routes: [newRoute, ...prev.routes] } : prev
    );
    setSelectedRouteId(newRoute.id);
  };

  const removeRoute = (id: string) => {
    setConfig((prev) => {
      if (!prev) return prev;
      const newRoutes = prev.routes.filter((route) => route.id !== id);
      if (selectedRouteId === id) {
        setSelectedRouteId(newRoutes[0]?.id || null);
      }
      return { ...prev, routes: newRoutes };
    });
  };

  const saveConfig = useCallback(async () => {
    if (!isTauri() || !config) return;
    try {
      await saveConfigWithConfig(config);
    } catch (err) {
      console.error("Save config error:", err);
    }
  }, [config, saveConfigWithConfig]);

  const startServer = async () => {
    if (!isTauri()) return;
    if (serverInFlight.current) return serverInFlight.current;
    const run = (async () => {
      setStarting(true);
      setError(null);
      try {
        if (config && hasUnsavedChanges) {
          try {
            await saveConfigWithConfig(config);
          } catch (saveErr: any) {
            console.error("Failed to save config before starting:", saveErr);
            setError("Failed to save config before starting server.");
            return;
          }
        }
        const startResponse = await invoke<MockServerStatus>("mock_server_start");
        setStatus(startResponse);
      } catch (err: any) {
        console.error("Failed to start server:", err);
        const errMsg = err?.message || err?.toString() || "";
        if (errMsg.includes("already running")) {
          try {
            const updated = await invoke<MockServerStatus>("mock_server_status");
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

  const stopServer = async () => {
    if (!isTauri()) return;
    if (serverInFlight.current) return serverInFlight.current;
    const run = (async () => {
      setStarting(true);
      setError(null);
      try {
        const stopResponse = await invoke<MockServerStatus>("mock_server_stop");
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

  const selectedRoute = useMemo(
    () => config?.routes.find((r) => r.id === selectedRouteId),
    [config?.routes, selectedRouteId]
  );

  const hasUnsavedChanges = useMemo(() => {
    if (!config || !savedConfig) return false;
    return serializeConfig(config) !== serializeConfig(savedConfig);
  }, [config, savedConfig]);

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

  const loadLogs = useCallback(async () => {
    if (!isTauri()) return;
    const requestId = Date.now();
    activeLogRequest.current = requestId;
    setLogsLoading(true);
    setLogsError(null);
    try {
      const entries = await invoke<LogEntry[]>("get_logs");
      if (activeLogRequest.current !== requestId) return;
      setLogs(entries.filter((entry) => entry.source === "MockServer"));
    } catch (err: any) {
      if (activeLogRequest.current !== requestId) return;
      console.error("Failed to load logs:", err);
      setLogsError(err?.message || "Failed to load logs.");
    } finally {
      if (activeLogRequest.current === requestId) {
        setLogsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (viewMode !== "logs") return;
    void loadLogs();
    const intervalId = window.setInterval(loadLogs, 2000);
    return () => window.clearInterval(intervalId);
  }, [loadLogs, viewMode]);

  const generateCurlCommand = (route: MockRoute) => {
    let cmd = `curl -X ${route.method} "${baseUrl}${route.path}"`;
    route.matchers.headers.forEach(h => {
        if (h.enabled && h.key) cmd += ` \\\n  -H "${h.key}: ${h.value}"`;
    });
    if (route.method !== 'GET' && route.matchers.body?.value) {
        cmd += ` \\\n  -d '${route.matchers.body.value.replace(/'/g, "'\\''")}'`;
    }
    return cmd;
  };

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

  if (!config) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-red-400">Unable to load mock server config.</div>
      </div>
    );
  }

  return (
    <div className="flex h-full bg-app-bg text-app-text overflow-hidden font-sans">
      {/* 1. Left Sidebar: Navigation */}
      <div className="w-64 flex-shrink-0 border-r border-app-border bg-app-panel flex flex-col">
        <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
            <Button
                variant={viewMode === "endpoints" ? "primary" : "ghost"}
                className={`w-full justify-start gap-3 ${viewMode === "endpoints" ? "bg-app-accent/10 text-app-accent" : "text-app-subtext"}`}
                onClick={() => setViewMode("endpoints")}
            >
                <Server className="w-4 h-4" />
                Endpoints
            </Button>
            <Button
                variant={viewMode === "logs" ? "primary" : "ghost"}
                className={`w-full justify-start gap-3 ${viewMode === "logs" ? "bg-app-accent/10 text-app-accent" : "text-app-subtext"}`}
                onClick={() => setViewMode("logs")}
            >
                <Activity className="w-4 h-4" />
                Traffic Logs
            </Button>
            {/* <Button
                 variant="ghost"
                 className="w-full justify-start gap-3 text-app-subtext opacity-50 cursor-not-allowed"
                 disabled
            >
                <Layers className="w-4 h-4" />
                Environments
            </Button> */}
        </nav>

        <div className="p-3 border-t border-app-border space-y-3">
             <div className="flex items-center justify-between px-2">
                 <span className="text-xs font-semibold text-app-subtext uppercase tracking-wider">Status</span>
                 <span className={`flex items-center gap-1.5 text-[10px] font-bold px-2 py-0.5 rounded-full ${status?.running ? "bg-emerald-500/10 text-emerald-500" : "bg-app-subtext/10 text-app-subtext"}`}>
                     <div className={`w-1.5 h-1.5 rounded-full ${status?.running ? "bg-emerald-500" : "bg-app-subtext"}`} />
                     {status?.running ? "LIVE" : "STOPPED"}
                 </span>
             </div>
             
             {status?.running ? (
                 <Button onClick={stopServer} disabled={starting} className="w-full justify-center bg-app-card border border-app-border hover:bg-red-500/10 hover:text-red-400 hover:border-red-500/20 transition-colors">
                     {starting ? (
                        <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2" />
                     ) : (
                        <Square className="w-4 h-4 mr-2" />
                     )}
                     Stop Server
                 </Button>
             ) : (
                 <Button onClick={startServer} disabled={starting} className="w-full justify-center bg-emerald-500 hover:bg-emerald-600 text-white border-0 shadow-lg shadow-emerald-500/20">
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
                    <span className="text-[10px] text-app-subtext uppercase font-semibold">Global Port</span>
                 </div>
                 <Input 
                    type="number" 
                    value={config.port} 
                    onChange={(e: any) => setConfig({...config, port: parseInt(e.target.value) || 4010})}
                    className="h-8 text-xs bg-app-bg border-app-border"
                 />
             </div>
        </div>
      </div>

      {/* 2. Main Content: Route Editor */}
      <div className="flex-1 flex flex-col min-w-0 bg-app-bg relative">
        {viewMode === "logs" ? (
            <div className="flex-1 flex flex-col h-full">
                <div className="h-16 border-b border-app-border flex items-center justify-between px-6 bg-app-bg/50 backdrop-blur-sm sticky top-0 z-10 w-full">
                    <div className="flex items-center gap-3 min-w-0 flex-1 mr-4">
                        <Terminal className="w-4 h-4 text-app-subtext" />
                        <span className="text-xs font-bold uppercase tracking-widest text-app-subtext">Traffic Logs</span>
                        <span className="text-[10px] text-app-subtext/70">Auto refresh every 2s</span>
                    </div>
                    <div className="flex items-center gap-2 flex-shrink-0">
                        {logsError && (
                            <div className="text-xs text-red-400 bg-red-400/10 px-2 py-1 rounded border border-red-400/20 animate-in fade-in">
                                {logsError}
                            </div>
                        )}
                        <Button size="sm" variant="ghost" onClick={loadLogs} className="text-app-subtext hover:text-app-text">
                            Refresh
                        </Button>
                    </div>
                </div>
                <div className="flex-1 overflow-y-auto p-6">
                    {logsLoading ? (
                        <div className="text-xs text-app-subtext">Loading logs...</div>
                    ) : logs.length === 0 ? (
                        <div className="text-xs text-app-subtext">
                            No logs yet. Start the engine and send a request from curl or Postman.
                        </div>
                    ) : (
                        <div className="space-y-2">
                            {logs.map((entry, idx) => (
                                <div key={`${entry.time}-${idx}`} className="p-3 rounded-lg border border-app-border bg-app-card/40">
                                    <div className="flex items-center justify-between gap-3 mb-1">
                                        <div className="text-[10px] uppercase tracking-wider text-app-subtext">
                                            {entry.time} - {entry.level}
                                        </div>
                                        <span className="text-[10px] text-app-subtext/60">{entry.source}</span>
                                    </div>
                                    <div className="text-xs text-app-text">{entry.message}</div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        ) : selectedRoute ? (
            <div className="flex-1 flex flex-col h-full">
                {/* Header */}
                <div className="h-16 border-b border-app-border flex items-center justify-between px-6 bg-app-bg/50 backdrop-blur-sm sticky top-0 z-10 w-full">
                    <div className="flex items-center gap-4 min-w-0 flex-1 mr-4">
                        <MethodBadge method={selectedRoute.method} />
                        <span className="font-mono text-sm text-app-text/70 truncate">{selectedRoute.path}</span>
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
                              checked={selectedRoute.enabled}
                              onCheckedChange={(c) =>
                                updateRoute(selectedRoute.id, r => ({...r, enabled: c}), {
                                  persist: status?.running ?? false,
                                })
                              }
                            />
                         </div>
                         <Button variant="ghost" size="sm" onClick={() => discardRouteChanges(selectedRoute.id)} className="text-app-subtext hover:text-red-400">
                             Discard
                         </Button>
                         {hasUnsavedChanges && (
                            <span className="text-[10px] text-amber-400 bg-amber-400/10 px-2 py-1 rounded border border-amber-400/20">
                                Unsaved
                            </span>
                         )}
                         <Button size="sm" onClick={saveConfig} disabled={saving} className="bg-app-accent hover:bg-blue-600 text-white gap-2 shadow-lg shadow-blue-500/20">
                             {saving ? "Deploying..." : (
                                 <>
                                    <Save className="w-3.5 h-3.5" /> Deploy Changes
                                 </>
                             )}
                         </Button>
                    </div>
                </div>

                {/* Scrollable Form Area */}
                <div className="flex-1 overflow-y-auto p-6 space-y-8">
                    
                    {/* Endpoint Header */}
                    <section className="space-y-4">
                        <div className="flex items-center gap-2 text-app-subtext">
                             <LayoutGrid className="w-4 h-4" />
                             <h3 className="text-xs font-bold uppercase tracking-widest">Endpoint Definition</h3>
                        </div>
                        <div className="grid grid-cols-[120px_1fr] gap-4">
                             <div className="space-y-1">
                                <label className="text-[10px] uppercase text-app-subtext font-semibold pl-1">Method</label>
                                <Select 
                                    options={METHODS.map(m => ({label: m, value: m}))}
                                    value={selectedRoute.method}
                                    onChange={(v) => updateRoute(selectedRoute.id, r => ({...r, method: v}))}
                                    className="h-10 bg-app-card border-app-border"
                                />
                             </div>
                             <div className="space-y-1">
                                <label className="text-[10px] uppercase text-app-subtext font-semibold pl-1">Request URL Path</label>
                                <Input 
                                    value={selectedRoute.path}
                                    onChange={(e) => updateRoute(selectedRoute.id, r => ({...r, path: e.target.value}))}
                                    className="h-10 bg-app-card border-app-border font-mono text-sm"
                                    placeholder="/api/v1/resource"
                                />
                             </div>
                        </div>
                    </section>

                    {/* Incoming Request */}
                    <section className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-500 delay-100">
                         <div className="flex items-center justify-between border-b border-app-border pb-2">
                            <div className="flex items-center gap-2 text-app-subtext">
                                <ChevronRight className="w-4 h-4" />
                                <h3 className="text-xs font-bold uppercase tracking-widest">Incoming Request</h3>
                            </div>
                         </div>
                         
                         <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <label className="text-[10px] uppercase text-app-subtext font-semibold">Headers Matcher</label>
                                <Button size="sm" variant="ghost" className="h-6 text-[10px] text-app-accent" onClick={() => updateRoute(selectedRoute.id, r => ({...r, matchers: {...r.matchers, headers: [...r.matchers.headers, createKeyValue()]}}))}>
                                    ADD HEADER
                                </Button>
                            </div>
                            <div className="space-y-2">
                                {selectedRoute.matchers.headers.map((h, idx) => (
                                    <div key={idx} className="flex gap-2 animate-in fade-in slide-in-from-left-2">
                                        <Input placeholder="Header Name" value={h.key} onChange={(e) => {
                                             const newHeaders = [...selectedRoute.matchers.headers];
                                             newHeaders[idx].key = e.target.value;
                                             updateRoute(selectedRoute.id, r => ({...r, matchers: {...r.matchers, headers: newHeaders}}));
                                        }} className="bg-app-card border-app-border text-xs" />
                                        <Input placeholder="Value" value={h.value} onChange={(e) => {
                                             const newHeaders = [...selectedRoute.matchers.headers];
                                             newHeaders[idx].value = e.target.value;
                                             updateRoute(selectedRoute.id, r => ({...r, matchers: {...r.matchers, headers: newHeaders}}));
                                        }} className="bg-app-card border-app-border text-xs" />
                                        <Button size="icon" variant="ghost" onClick={() => {
                                             const newHeaders = selectedRoute.matchers.headers.filter((_, i) => i !== idx);
                                             updateRoute(selectedRoute.id, r => ({...r, matchers: {...r.matchers, headers: newHeaders}}));
                                        }} className="text-app-subtext hover:text-red-400">
                                            <Trash2 className="w-3.5 h-3.5" />
                                        </Button>
                                    </div>
                                ))}
                                {selectedRoute.matchers.headers.length === 0 && (
                                    <div className="text-center py-4 border border-dashed border-app-border rounded-lg text-xs text-app-subtext">
                                        No header requirements defined
                                    </div>
                                )}
                            </div>
                         </div>

                         <div className="space-y-4 pt-4">
                            <label className="text-[10px] uppercase text-app-subtext font-semibold">Body Validation Schema (JSON)</label>
                            <div className="relative group">
                                <TextArea 
                                    className="font-mono text-xs min-h-[120px] bg-app-card/50 border-app-border leading-relaxed"
                                    value={selectedRoute.matchers.body?.value || ""}
                                    placeholder='{ "key": "value"... }'
                                    onChange={(e) => updateRoute(selectedRoute.id, r => ({
                                        ...r,
                                        matchers: {
                                            ...r.matchers,
                                            body: e.target.value ? { mode: r.matchers.body?.mode || "contains", value: e.target.value } : null
                                        }
                                    }))}
                                />
                                <div className="absolute bottom-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                     <span className="text-[10px] text-app-subtext bg-app-bg px-2 py-1 rounded border border-app-border">VALIDATOR ACTIVE</span>
                                </div>
                            </div>
                         </div>
                    </section>

                    {/* Mock Response */}
                    <section className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-500 delay-200">
                         <div className="flex items-center justify-between border-b border-app-border pb-2">
                            <div className="flex items-center gap-2 text-app-subtext">
                                <Copy className="w-4 h-4 -scale-x-100" />
                                <h3 className="text-xs font-bold uppercase tracking-widest">Mock Response</h3>
                            </div>
                            <div className="flex items-center gap-2">
                                <span className="text-[10px] uppercase text-app-subtext font-semibold">Status</span>
                                <Input 
                                    type="number" 
                                    className="w-20 h-7 text-xs bg-app-card border-app-border text-center font-mono"
                                    value={selectedRoute.response.status}
                                    onChange={(e) => updateRoute(selectedRoute.id, r => ({...r, response: {...r.response, status: parseInt(e.target.value)||200}}))}
                                />
                            </div>
                         </div>

                        <div className="relative group">
                             <TextArea 
                                className="font-mono text-xs min-h-[200px] bg-[#1e1e1e] border-app-border text-emerald-100/80 leading-relaxed"
                                value={selectedRoute.response.body}
                                onChange={(e) => updateRoute(selectedRoute.id, r => ({...r, response: {...r.response, body: e.target.value}}))}
                             />
                             <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                <Button size="sm" variant="ghost" className="h-6 px-2 bg-app-panel border border-app-border text-app-subtext shadow-sm text-[10px]" onClick={() => {
                                    try {
                                        const fmt = JSON.stringify(JSON.parse(selectedRoute.response.body), null, 2);
                                        updateRoute(selectedRoute.id, r => ({...r, response: {...r.response, body: fmt}}));
                                    } catch(e) {}
                                }}>
                                    Prettify
                                </Button>
                             </div>
                        </div>
                    </section>

                    {/* cURL Display */}
                    <section className="pt-8 pb-10">
                        <div className="rounded-lg bg-app-panel border border-app-border p-4 space-y-3">
                            <div className="flex items-center justify-between">
                                <h4 className="text-xs font-bold uppercase tracking-widest text-app-subtext">Generate Request cURL</h4>
                                <Button size="sm" variant="ghost" onClick={() => copyToClipboard("cURL", generateCurlCommand(selectedRoute))} className="text-app-accent hover:bg-app-accent/10">
                                    <Copy className="w-3.5 h-3.5 mr-2" />
                                    {lastCopied === "cURL" ? "Copied!" : "Copy cURL"}
                                </Button>
                            </div>
                            <div className="bg-[#101010] p-3 rounded border border-app-border overflow-x-auto">
                                <pre className="text-[10px] font-mono text-app-text/70 whitespace-pre-wrap break-all">
                                    {generateCurlCommand(selectedRoute)}
                                </pre>
                            </div>
                        </div>
                    </section>
                </div>
            </div>
        ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-app-subtext space-y-4">
                <div className="w-16 h-16 rounded-2xl bg-app-panel flex items-center justify-center mb-2">
                    <LayoutGrid className="w-8 h-8 opacity-50" />
                </div>
                <h3 className="text-lg font-medium text-app-text">No Endpoint Selected</h3>
                <p className="text-sm max-w-xs text-center">Select an existing endpoint from the quick switcher or create a new one to get started.</p>
                <Button onClick={addRoute} className="mt-4 bg-app-accent text-white">
                    <Plus className="w-4 h-4 mr-2" /> Create New Endpoint
                </Button>
            </div>
        )}
      </div>

      {/* 3. Right Sidebar: Quick Switcher */}
      <div className="w-72 flex-shrink-0 border-l border-app-border bg-app-card/30 flex flex-col">
          <div className="p-4 border-b border-app-border flex items-center justify-between">
               <h3 className="text-xs font-bold uppercase tracking-widest text-app-subtext">Quick Switcher</h3>
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
                   {config.routes.map(route => {
                       const isActive = selectedRouteId === route.id;
                       return (
                           <motion.div
                                key={route.id}
                                layout
                                initial={{ opacity: 0, x: 20 }}
                                animate={{ opacity: 1, x: 0 }}
                                exit={{ opacity: 0, x: -20 }}
                                onClick={() => setSelectedRouteId(route.id)}
                                className={`
                                    group flex flex-col gap-1 p-3 rounded-lg border cursor-pointer transition-all duration-200
                                    ${isActive 
                                        ? "bg-app-accent/5 border-app-accent/20 shadow-sm" 
                                        : "bg-transparent border-transparent hover:bg-app-panel hover:border-app-border"}
                                `}
                           >
                                <div className="flex items-center justify-between">
                                    <MethodBadge method={route.method} />
                                    {route.enabled ? (
                                        <span className="text-[10px] text-emerald-500 font-medium opacity-80">Active</span>
                                    ) : (
                                        <span className="text-[10px] text-app-subtext font-medium opacity-50">Inactive</span>
                                    )}
                                </div>
                                <div className={`font-mono text-xs truncate transition-colors ${isActive ? "text-app-text font-medium" : "text-app-subtext group-hover:text-app-text"}`}>
                                    {route.path}
                                </div>
                                <div className="flex items-center justify-between pt-1">
                                    <span className="text-[10px] text-app-subtext">{route.name}</span>
                                    <Button size="icon" variant="ghost" className="h-5 w-5 opacity-0 group-hover:opacity-100 text-app-subtext hover:text-red-400" onClick={(e) => {
                                        e.stopPropagation();
                                        removeRoute(route.id);
                                    }}>
                                        <Trash2 className="w-3 h-3" />
                                    </Button>
                                </div>
                           </motion.div>
                       )
                   })}
               </AnimatePresence>
               
               <Button variant="ghost" className="w-full mt-2 text-xs text-app-subtext hover:text-app-accent border border-dashed border-app-border hover:border-app-accent/50" onClick={addRoute}>
                   <Plus className="w-3.5 h-3.5 mr-2" /> Add Mock
               </Button>
          </div>
      </div>
    </div>
  );
}
