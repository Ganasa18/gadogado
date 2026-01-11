import { useEffect, useMemo } from "react";
import { Switch } from "../../shared/components/Switch";
import {
  Box,
  Languages,
  ArrowRightLeft,
  Keyboard,
  Sliders,
  Power,
  CheckCircle,
  ChevronDown,
} from "lucide-react";
import { useSettingsStore } from "../../store/settings";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../../store/toast";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import { useModelsQuery } from "../../hooks/useLlmApi";
import { isTauri } from "../../utils/tauri";

const LANGUAGES = [
  "Auto Detect",
  "English",
  "Indonesian",
  "Spanish",
  "French",
  "German",
  "Chinese",
  "Japanese",
  "Arabic",
  "Russian",
  "Portuguese",
  "Hindi",
  "Korean",
];

export default function GeneralTab() {
  const {
    provider,
    model,
    apiKey,
    baseUrl,
    localModels,
    shortcutsEnabled,
    shortcuts,
    sourceLang,
    targetLang,
    setProvider,
    setModel,
    setApiKey,
    setBaseUrl,
    setLocalModels,
    setShortcutsEnabled,
    setSourceLang,
    setTargetLang,
  } = useSettingsStore();
  const { addToast } = useToastStore();
  const buildConfig = useLlmConfigBuilder();

  const isLocalProvider = provider === "local";
  const providerDefaults: Partial<Record<string, string>> = {
    openai: "https://api.openai.com/v1",
    google: "https://generativelanguage.googleapis.com/v1beta/models",
  };
  const providerModels: Partial<Record<string, string>> = {
    openai: "gpt-4o",
    google: "gemini-2.5-flash-lite",
    dll: "custom-model",
  };

  const localConfig = useMemo(
    () => buildConfig({ maxTokens: 1024, temperature: 0.7 }),
    [buildConfig]
  );
  const modelsQuery = useModelsQuery(localConfig, isLocalProvider);

  useEffect(() => {
    if (!isLocalProvider) return;
    if (!modelsQuery.data) return;
    setLocalModels(modelsQuery.data);
    if (modelsQuery.data.length > 0 && !model) {
      setModel(modelsQuery.data[0]);
    }
  }, [isLocalProvider, modelsQuery.data, setLocalModels, setModel, model]);

  useEffect(() => {
    console.log("[LLM] Settings changed", { provider, model, baseUrl });
  }, [provider, model, baseUrl]);

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      {/* Left Column - Settings */}
      <aside className="w-full overflow-y-auto p-4 flex flex-col gap-4">
        {/* Card 1: Translation Model */}
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-3 text-app-text font-medium">
            <Box className="w-4 h-4 text-blue-400" />
            <h3>Translation Model</h3>
          </div>
          <div className="grid grid-cols-2 gap-3 mb-3">
            <div className="relative">
              <select
                value={provider}
                onChange={(e: any) => {
                  const nextProvider = e.target.value as
                    | "local"
                    | "openai"
                    | "google"
                    | "dll";
                  setProvider(nextProvider);
                  if (nextProvider !== "local") {
                    setBaseUrl(providerDefaults[nextProvider] ?? baseUrl);
                    setModel(providerModels[nextProvider] ?? model);
                  } else if (localModels.length > 0) {
                    setModel(localModels[0]);
                  }
                }}
                className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                <option value="local">Free For Local LLM</option>
                <option value="openai">Open Api</option>
                <option value="google">Google</option>
                <option value="dll">DLL</option>
              </select>
              <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
            </div>
            <div className="relative">
              <select
                value={model}
                onChange={(e: any) => setModel(e.target.value)}
                className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                {isLocalProvider ? (
                  localModels.length > 0 ? (
                    localModels.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))
                  ) : (
                    <option>No models found</option>
                  )
                ) : provider === "google" ? (
                  <>
                    <option>gemini-2.5-flash-lite</option>
                    <option>gemini-2.0-flash-lite</option>
                    <option>gemini-3-flash-preview</option>
                  </>
                ) : provider === "openai" ? (
                  <>
                    <option>gpt-4o</option>
                    <option>gpt-4o-mini</option>
                  </>
                ) : (
                  <option>custom-model</option>
                )}
              </select>
              <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
            </div>
          </div>
          {/* Success Message */}
          {isLocalProvider && (
            <div className="bg-[#1d3326] border border-green-600/50 text-green-400 rounded px-3 py-2 text-[10px] flex items-center gap-2">
              <CheckCircle className="w-3 h-3" />
              <span>
                Free For Local LLM (Local LLM via Backend) - Ensure LM Studio is
                active.
              </span>
            </div>
          )}
          {!isLocalProvider && (
            <div className="mt-3 space-y-2">
              <div>
                <label className="text-[10px] text-gray-500 block mb-1">
                  API Key
                </label>
                <input
                  className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                  placeholder="Paste your API key"
                  type="password"
                  value={apiKey}
                  onInput={(e: any) => setApiKey(e.target.value)}
                />
              </div>
              <div>
                <label className="text-[10px] text-gray-500 block mb-1">
                  Base URL
                </label>
                <input
                  className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                  placeholder="https://api.openai.com/v1"
                  type="text"
                  value={baseUrl}
                  onInput={(e: any) => setBaseUrl(e.target.value)}
                />
              </div>
              <button
                className="w-full bg-[#1e1e1e] border border-app-border rounded p-2 text-xs text-gray-300 hover:border-gray-500 transition"
                onClick={() => {
                  if (!isTauri()) {
                    addToast(
                      "Tauri runtime not available in browser mode",
                      "error"
                    );
                    return;
                  }
                  const config = buildConfig({
                    maxTokens: 1024,
                    temperature: 0.7,
                  });
                  console.log("[LLM] Saving API settings", config);
                  invoke("sync_config", { config })
                    .then(() => {
                      addToast("Configuration saved successfully", "success");
                    })
                    .catch((err) => {
                      console.error(err);
                      addToast("Failed to save configuration", "error");
                    });
                }}>
                Save API Settings
              </button>
            </div>
          )}
          <div className="mt-3">
            <button
              className="w-full bg-[#2a1d1d] border border-red-900/40 rounded p-2 text-xs text-red-200 hover:border-red-700/60 transition"
              onClick={() => {
                localStorage.removeItem("promptbridge-settings");
                localStorage.removeItem("promptbridge-history");
                window.location.reload();
              }}>
              Reset Local Storage
            </button>
          </div>
        </div>

        {/* Card 2: Languages */}
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex justify-between items-center mb-3">
            <div className="flex items-center gap-2 text-app-text font-medium">
              <Languages className="w-4 h-4 text-gray-400" />
              <h3>Languages</h3>
            </div>
            <span className="text-[10px] text-gray-500">
              Max 5000 characters per translation
            </span>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex-1">
              <label className="text-[10px] text-gray-500 block mb-1">
                From
              </label>
              <div className="relative">
                <select
                  className="w-full bg-[#18181b] border border-app-border rounded p-2 px-3 text-xs appearance-none outline-none"
                  value={sourceLang}
                  onChange={(e: any) => setSourceLang(e.target.value)}>
                  {LANGUAGES.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang}
                    </option>
                  ))}
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
              </div>
            </div>
            <button className="mt-5 text-gray-500 hover:text-white transition">
              <ArrowRightLeft className="w-3.5 h-3.5" />
            </button>
            <div className="flex-1">
              <label className="text-[10px] text-gray-500 block mb-1">To</label>
              <div className="relative">
                <select
                  className="w-full bg-[#18181b] border border-app-border rounded p-2 px-3 text-xs appearance-none outline-none"
                  value={targetLang}
                  onChange={(e: any) => setTargetLang(e.target.value)}>
                  {LANGUAGES.filter((lang) => lang !== "Auto Detect").map(
                    (lang) => (
                      <option key={lang} value={lang}>
                        {lang}
                      </option>
                    )
                  )}
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
              </div>
            </div>
          </div>
        </div>

        {shortcutsEnabled && (
          <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
            <div className="flex items-center gap-2 mb-4 text-app-text font-medium">
              <Keyboard className="w-4 h-4 text-gray-400" />
              <h3>Shortcuts</h3>
            </div>
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-gray-300 text-xs">
                  Translate & Replace
                </span>
                <span className="bg-[#1e293b] text-blue-300 border border-blue-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.translate}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-300 text-xs">Translate & Popup</span>
                <span className="bg-[#143325] text-green-300 border border-green-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.popup}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-300 text-xs">Enhance Prompt</span>
                <span className="bg-[#332e18] text-yellow-200 border border-yellow-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.enhance}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-300 text-xs">Terminal Translate</span>
                <span className="bg-[#1e2a2f] text-cyan-200 border border-cyan-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.terminal}
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Card 4: Application Preferences */}
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-4 text-app-text font-medium">
            <Sliders className="w-4 h-4 text-blue-400" />
            <h3>Application Preferences</h3>
          </div>
          <div className="space-y-4">
            <div className="flex justify-between items-center">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-md bg-[#1d3326] text-green-500 flex items-center justify-center text-lg border border-green-500/30 shadow-[0_0_10px_rgba(34,197,94,0.1)]">
                  <Power className="w-5 h-5" />
                </div>
                <div>
                  <div className="text-gray-200 font-medium text-xs leading-tight">
                    Shortcuts Active
                  </div>
                  <div className="text-[10px] text-gray-500 leading-tight">
                    {shortcutsEnabled
                      ? "Shortcuts are enabled"
                      : "Shortcuts are disabled"}
                  </div>
                </div>
              </div>
              <Switch
                checked={shortcutsEnabled}
                onCheckedChange={setShortcutsEnabled}
              />
            </div>
          </div>
        </div>
      </aside>
    </div>
  );
}
