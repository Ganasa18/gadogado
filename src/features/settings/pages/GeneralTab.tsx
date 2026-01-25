import { useEffect, useMemo, useState } from "react";
import { Switch } from "../../../shared/components/Switch";
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
import { useSettingsStore, type LLMProvider } from "../../../store/settings";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../../../store/toast";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";
import { useDebounce } from "../../../hooks/useDebounce";
import {
  useModelsQuery,
  useOpenRouterModelsQuery,
  useOpenRouterProvidersQuery,
} from "../../../hooks/useLlmApi";
import { isTauri } from "../../../utils/tauri";
import { useThemeStore } from "../../theme/themeStore";
import { LOCAL_LLM_BASE_URL } from "../../../shared/api/llmConfig";

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

// const OPENROUTER_EXAMPLE = `# Providers
// curl https://openrouter.ai/api/v1/providers \\
//   -H "Authorization: Bearer <token>"

// # Models
// curl https://openrouter.ai/api/v1/models \\
//   -H "Authorization: Bearer <token>"

// # Chat completion
// curl https://openrouter.ai/api/v1/chat/completions \\
//   -H "Content-Type: application/json" \\
//   -H "Authorization: Bearer $OPENROUTER_API_KEY" \\
//   -d '{
//   "model": "openai/gpt-5.2",
//   "messages": [
//     {
//       "role": "user",
//       "content": "What is the meaning of life?"
//     }
//   ]
// }'`;

export default function GeneralTab() {
  const {
    provider,
    model,
    baseUrl,
    localModels,
    embeddingProvider,
    embeddingModel,
    shortcutsEnabled,
    shortcuts,
    sourceLang,
    targetLang,
    aiOutputLanguage,
    getApiKey,
    setApiKey,
    setProvider,
    setModel,
    setBaseUrl,
    setLocalModels,
    setEmbeddingProvider,
    setEmbeddingModel,
    setShortcutsEnabled,
    setSourceLang,
    setTargetLang,
    setAiOutputLanguage,
  } = useSettingsStore();

  // Get the API key for the current provider
  const apiKey = getApiKey(provider);

  const {
    theme: appTheme,
    mode: appMode,
    setTheme: setAppTheme,
    setMode: setAppMode,
  } = useThemeStore();
  const [openRouterFilter] = useState("");
  const [apiKeyInput, setApiKeyInput] = useState(apiKey);
  const [selectedOpenRouterProvider, setSelectedOpenRouterProvider] =
    useState<string>("");

  // Debounce the filter input to prevent excessive filtering
  const debouncedFilter = useDebounce(openRouterFilter, 400);

  const { addToast } = useToastStore();
  const buildConfig = useLlmConfigBuilder();

  // Sync API key input when provider changes
  useEffect(() => {
    setApiKeyInput(apiKey);
  }, [provider, apiKey]);

  const isLocalProvider =
    provider === "local" || provider === "ollama" || provider === "llama_cpp";
  const isCliProxyProvider = provider === "cli_proxy";
  const shouldFetchModels = isLocalProvider || isCliProxyProvider;
  const isOpenRouter = provider === "openrouter";
  const hasApiKey = apiKey.trim().length > 0;
  const requiresApiKey =
    provider === "openai" ||
    provider === "gemini" ||
    provider === "openrouter" ||
    provider === "dll" ||
    provider === "cli_proxy";

  // Provider-specific labels
  const providerLabel: Partial<Record<LLMProvider, string>> = {
    openai: "OpenAI",
    gemini: "Google Gemini",
    openrouter: "OpenRouter",
    dll: "DLL",
    cli_proxy: "CLI Proxy",
  };

  const providerDefaults: Partial<Record<LLMProvider, string>> = {
    local: LOCAL_LLM_BASE_URL,
    openai: "https://api.openai.com/v1",
    gemini: "https://generativelanguage.googleapis.com/v1beta/models",
    ollama: "http://localhost:11434/v1",
    llama_cpp: "http://localhost:8080/v1",
    openrouter: "https://openrouter.ai/api/v1",
    dll: "",
    cli_proxy: "http://127.0.0.1:8317/v1",
  };
  const providerModels: Partial<Record<LLMProvider, string>> = {
    openai: "gpt-4o",
    gemini: "gemini-2.5-flash-lite",
    ollama: "llama3",
    llama_cpp: "llama-3-8b-instruct",
    openrouter: "openai/gpt-4",
    dll: "custom-model",
    cli_proxy: "gpt-5.1-codex",
  };

  const localConfig = useMemo(
    () => buildConfig({ maxTokens: 1024, temperature: 0.7 }),
    [buildConfig],
  );
  const modelsQuery = useModelsQuery(localConfig, shouldFetchModels);
  const openRouterConfig = useMemo(
    () => buildConfig({ maxTokens: 1024, temperature: 0.7 }),
    [buildConfig],
  );
  const openRouterModelsQuery = useOpenRouterModelsQuery(
    openRouterConfig,
    isOpenRouter && hasApiKey,
  );
  const openRouterProvidersQuery = useOpenRouterProvidersQuery(
    openRouterConfig,
    isOpenRouter && hasApiKey,
  );
  const openRouterModels = openRouterModelsQuery.data ?? [];
  const openRouterProviders = openRouterProvidersQuery.data ?? [];

  // Get current provider slug from model ID (e.g., "openai" from "openai/gpt-4")
  const currentModelProvider = useMemo(() => {
    const parts = model.split("/");
    return parts.length >= 2 ? parts[0] : "";
  }, [model]);

  // Sync selected provider when model changes
  useEffect(() => {
    if (
      isOpenRouter &&
      currentModelProvider &&
      currentModelProvider !== selectedOpenRouterProvider
    ) {
      setSelectedOpenRouterProvider(currentModelProvider);
    }
  }, [isOpenRouter, currentModelProvider, selectedOpenRouterProvider]);

  // Filter models by selected provider and search filter
  const openRouterFilteredModels = useMemo(() => {
    let filtered = openRouterModels;

    // First filter by selected provider
    if (selectedOpenRouterProvider) {
      filtered = filtered.filter((item) =>
        item.id.startsWith(selectedOpenRouterProvider + "/"),
      );
    }

    // Then apply search filter
    const filter = debouncedFilter.trim().toLowerCase();
    if (filter) {
      filtered = filtered.filter((item) => {
        const id = item.id?.toLowerCase() ?? "";
        const name = item.name?.toLowerCase() ?? "";
        const slug =
          typeof item.canonical_slug === "string"
            ? item.canonical_slug.toLowerCase()
            : "";
        return (
          id.includes(filter) || name.includes(filter) || slug.includes(filter)
        );
      });
    }

    return filtered;
  }, [debouncedFilter, openRouterModels, selectedOpenRouterProvider]);

  const openRouterSelectedModel = useMemo(
    () =>
      openRouterModels.find(
        (item) => item.id === model || item.canonical_slug === model,
      ),
    [model, openRouterModels],
  );

  useEffect(() => {
    if (!shouldFetchModels) return;
    if (!modelsQuery.data) return;
    // Deduplicate model IDs (cli_proxy may return duplicates)
    const uniqueModels = [...new Set(modelsQuery.data)];
    setLocalModels(uniqueModels);
    if (uniqueModels.length > 0 && !uniqueModels.includes(model)) {
      setModel(uniqueModels[0]);
    }
  }, [shouldFetchModels, modelsQuery.data, setLocalModels, setModel, model]);

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
                  const nextProvider = e.target.value as LLMProvider;
                  setProvider(nextProvider);
                  if (nextProvider === "local") {
                    setBaseUrl(providerDefaults.local ?? LOCAL_LLM_BASE_URL);
                    if (localModels.length > 0) {
                      setModel(localModels[0]);
                    }
                    return;
                  }
                  setBaseUrl(providerDefaults[nextProvider] ?? baseUrl);
                  setModel(providerModels[nextProvider] ?? model);
                }}
                className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                <option value="local">Local (LM Studio)</option>
                <option value="ollama">Ollama</option>
                <option value="llama_cpp">Llama.cpp</option>
                <option value="openai">OpenAI</option>
                <option value="gemini">Gemini</option>
                <option value="openrouter">OpenRouter</option>
                <option value="cli_proxy">CLI Proxy</option>
                <option value="dll">DLL</option>
              </select>
              <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
            </div>
            <div className="relative">
              {provider === "openrouter" ? (
                <select
                  value={selectedOpenRouterProvider}
                  onChange={(e: any) => {
                    const newProvider = e.target.value;
                    setSelectedOpenRouterProvider(newProvider);
                    // Auto-select the first model from the new provider
                    const firstModel = openRouterModels.find((m) =>
                      m.id.startsWith(newProvider + "/"),
                    );
                    if (firstModel) {
                      setModel(firstModel.id);
                    }
                  }}
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                  <option value="">Select Provider...</option>
                  {openRouterProviders.map((p) => (
                    <option
                      key={p.slug ?? p.id ?? p.name}
                      value={p.slug ?? p.id ?? ""}>
                      {p.name ?? p.slug ?? p.id}
                    </option>
                  ))}
                </select>
              ) : (
                <select
                  value={model}
                  onChange={(e: any) => setModel(e.target.value)}
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                  {shouldFetchModels ? (
                    localModels.length > 0 ? (
                      localModels.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))
                    ) : (
                      <option>No models found</option>
                    )
                  ) : provider === "gemini" ? (
                    <>
                      <option>gemini-2.5-flash-lite</option>
                      <option>gemini-2.0-flash-lite</option>
                      <option>gemini-3-flash-preview</option>
                      <option>text-embedding-004</option>
                      <option>embedding-001</option>
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
              )}
              <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
            </div>
          </div>

          {/* OpenRouter Model Selection (shown only when OpenRouter provider is selected) */}
          {provider === "openrouter" && (
            <div className="mt-3 pt-3 border-t border-app-border">
              <div className="text-[10px] text-app-subtext block mb-2">
                Select Model from{" "}
                {openRouterProviders.find(
                  (p) => (p.slug ?? p.id) === selectedOpenRouterProvider,
                )?.name ?? selectedOpenRouterProvider}
              </div>
              <div className="relative">
                <select
                  value={model}
                  onChange={(e: any) => setModel(e.target.value)}
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none"
                  disabled={!selectedOpenRouterProvider}>
                  {!selectedOpenRouterProvider ? (
                    <option>Select a provider first</option>
                  ) : openRouterFilteredModels.length === 0 ? (
                    <option>No models found</option>
                  ) : (
                    openRouterFilteredModels.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.name ?? item.id}
                      </option>
                    ))
                  )}
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
              </div>
              {openRouterSelectedModel && (
                <div className="mt-2 text-[10px] text-app-subtext">
                  <span className="text-app-text font-medium">Selected: </span>
                  {openRouterSelectedModel.name ?? openRouterSelectedModel.id}
                </div>
              )}
            </div>
          )}

          <div className="mt-4 pt-4 border-t border-app-border">
            <div className="text-[10px] text-app-subtext block mb-2">
              Embedding Model (RAG)
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="relative">
                <select
                  value={embeddingProvider}
                  onChange={(e: any) =>
                    setEmbeddingProvider(e.target.value as "local")
                  }
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                  <option value="local">Local (FastEmbed)</option>
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
              </div>
              <div className="relative">
                <select
                  value={embeddingModel}
                  onChange={(e: any) => setEmbeddingModel(e.target.value)}
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                  <option value="all-minilm-l6-v2">
                    all-MiniLM-L6-v2 (384 dim, fast)
                  </option>
                  <option value="nomic-embed-text-v1.5">
                    nomic-embed-text-v1.5 (768 dim, best)
                  </option>
                  <option value="bge-small-en-v1.5">
                    bge-small-en-v1.5 (384 dim)
                  </option>
                  <option value="multilingual-e5-small">
                    multilingual-e5-small (384 dim, multilingual)
                  </option>
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-gray-500 pointer-events-none" />
              </div>
            </div>
            <div className="text-[10px] text-app-subtext mt-2">
              Used for RAG vector search only.
            </div>
          </div>
          {/* Success Message */}
          {isLocalProvider && (
            <div className="bg-app-success-dim border border-app-success/50 text-app-success rounded px-3 py-2 text-[10px] flex items-center gap-2">
              <CheckCircle className="w-3 h-3" />
              <span>
                Local LLM provider active (LM Studio, Ollama, or Llama.cpp).
              </span>
            </div>
          )}
          {isCliProxyProvider && (
            <div className="bg-app-success-dim border border-app-success/50 text-app-success rounded px-3 py-2 text-[10px] flex items-center gap-2">
              <CheckCircle className="w-3 h-3" />
              <span>CLI Proxy provider active. Models fetched from proxy server.</span>
            </div>
          )}
          {provider !== "local" && (
            <div className="mt-3 space-y-2">
              {requiresApiKey && (
                <div>
                  <label className="text-[10px] text-app-subtext block mb-1">
                    {providerLabel[provider] || provider} API Key
                  </label>
                  <input
                    className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                    placeholder={`Paste your ${providerLabel[provider] || provider} API key`}
                    type="password"
                    value={apiKeyInput}
                    onInput={(e: any) => {
                      setApiKeyInput(e.target.value);
                      setApiKey(provider, e.target.value);
                    }}
                  />
                </div>
              )}
              <div>
                <label className="text-[10px] text-app-subtext block mb-1">
                  Base URL
                </label>
                <input
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                  placeholder="https://api.openai.com/v1"
                  type="text"
                  value={baseUrl}
                  onInput={(e: any) => setBaseUrl(e.target.value)}
                />
              </div>
              <button
                className="w-full bg-background border border-app-border rounded p-2 text-xs text-app-text hover:border-gray-500 transition"
                onClick={() => {
                  if (!isTauri()) {
                    addToast(
                      "Tauri runtime not available in browser mode",
                      "error",
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
              className="w-full bg-red-500/10 border border-red-500/20 rounded p-2 text-xs text-red-500 hover:bg-red-500/20 transition"
              onClick={() => {
                localStorage.removeItem("promptbridge-settings");
                localStorage.removeItem("promptbridge-history");
                window.location.reload();
              }}>
              Reset Local Storage
            </button>
          </div>
        </div>

        {/* {provider === "openrouter" && (
          <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
            <div className="flex items-center gap-2 mb-3 text-app-text font-medium">
              <Box className="w-4 h-4 text-blue-400" />
              <h3>OpenRouter Catalog</h3>
            </div>
            <div className="space-y-3">
              <div className="text-[10px] text-app-subtext">
                Browse and search across {openRouterProviders.length} providers and {openRouterModels.length} models.
              </div>
              <div className="flex items-center gap-2">
                <input
                  className="flex-1 bg-background border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                  placeholder="Search models by id or name..."
                  value={openRouterFilter}
                  onInput={(e: any) => setOpenRouterFilter(e.target.value)}
                />
                <button
                  className="bg-background border border-app-border rounded p-2 text-xs text-app-text hover:border-gray-500 transition"
                  onClick={refreshOpenRouter}
                  disabled={!hasApiKey}>
                  Refresh
                </button>
              </div>
              <div className="flex items-center justify-between text-[10px] text-app-subtext">
                <span>Total Models: {openRouterModels.length}</span>
                <span>Total Providers: {openRouterProviders.length}</span>
              </div>
              {!hasApiKey && (
                <div className="text-[10px] text-yellow-500">
                  Add your OpenRouter API key to load the catalog.
                </div>
              )}
              {openRouterModelsQuery.isLoading && (
                <div className="text-[10px] text-app-subtext">
                  Loading OpenRouter models...
                </div>
              )}
              {openRouterModelsQuery.error && (
                <div className="text-[10px] text-red-500">
                  Failed to load OpenRouter models.
                </div>
              )}
              {openRouterProvidersQuery.error && (
                <div className="text-[10px] text-red-500">
                  Failed to load OpenRouter providers.
                </div>
              )}
              {openRouterSelectedModel && (
                <div className="rounded-md border border-app-border bg-background p-3">
                  <div className="text-xs font-medium text-app-text">
                    {openRouterSelectedModel.name ??
                      openRouterSelectedModel.id}
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-[10px] text-app-subtext mt-2">
                    <div>
                      Context: {openRouterSelectedModel.context_length ?? "-"}
                    </div>
                    <div>
                      Max completion:{" "}
                      {openRouterSelectedModel.top_provider
                        ?.max_completion_tokens ?? "-"}
                    </div>
                    <div>
                      Prompt $:{" "}
                      {openRouterSelectedModel.pricing?.prompt ?? "-"}
                    </div>
                    <div>
                      Completion $:{" "}
                      {openRouterSelectedModel.pricing?.completion ?? "-"}
                    </div>
                  </div>
                  {openRouterSelectedModel.description && (
                    <div className="text-[10px] text-app-subtext mt-2">
                      {openRouterSelectedModel.description}
                    </div>
                  )}
                </div>
              )}
              {openRouterProviders.length > 0 && (
                <div>
                  <div className="text-[10px] text-app-subtext mb-2">
                    Available Providers
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-[10px] text-app-subtext">
                    {openRouterProviders.slice(0, 8).map((item) => (
                      <div key={item.slug ?? item.id ?? item.name} className="truncate px-2 py-1 bg-background rounded border border-app-border">
                        {item.name ?? item.slug ?? item.id}
                      </div>
                    ))}
                  </div>
                </div>
              )}
              <div className="rounded-md border border-app-border bg-background p-3">
                <div className="text-[10px] text-app-subtext mb-2">
                  Examples
                </div>
                <pre className="text-[10px] text-app-subtext font-mono whitespace-pre-wrap leading-relaxed">
                  {OPENROUTER_EXAMPLE}
                </pre>
              </div>
            </div>
          </div>
        )} */}

        {/* Card 2: Languages */}
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex justify-between items-center mb-3">
            <div className="flex items-center gap-2 text-app-text font-medium">
              <Languages className="w-4 h-4 text-app-subtext" />
              <h3>Languages</h3>
            </div>
            <span className="text-[10px] text-app-subtext">
              Max 5000 characters per translation
            </span>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex-1">
              <label className="text-[10px] text-app-subtext block mb-1">
                From
              </label>
              <div className="relative">
                <select
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none outline-none"
                  value={sourceLang}
                  onChange={(e: any) => setSourceLang(e.target.value)}>
                  {LANGUAGES.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang}
                    </option>
                  ))}
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-app-subtext pointer-events-none" />
              </div>
            </div>
            <button className="mt-5 text-app-subtext hover:text-app-text transition">
              <ArrowRightLeft className="w-3.5 h-3.5" />
            </button>
            <div className="flex-1">
              <label className="text-[10px] text-app-subtext block mb-1">
                To
              </label>
              <div className="relative">
                <select
                  className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none outline-none"
                  value={targetLang}
                  onChange={(e: any) => setTargetLang(e.target.value)}>
                  {LANGUAGES.filter((lang) => lang !== "Auto Detect").map(
                    (lang) => (
                      <option key={lang} value={lang}>
                        {lang}
                      </option>
                    ),
                  )}
                </select>
                <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-app-subtext pointer-events-none" />
              </div>
            </div>
          </div>
          <div className="mt-4 pt-4 border-t border-app-border">
            <label className="text-[10px] text-app-subtext block mb-1">
              AI Output Language
            </label>
            <div className="relative">
              <select
                className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none outline-none"
                value={aiOutputLanguage}
                onChange={(e: any) => setAiOutputLanguage(e.target.value)}>
                {LANGUAGES.filter((lang) => lang !== "Auto Detect").map(
                  (lang) => (
                    <option key={lang} value={lang}>
                      {lang}
                    </option>
                  ),
                )}
              </select>
              <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-app-subtext pointer-events-none" />
            </div>
            <div className="text-[10px] text-app-subtext mt-2">
              Applied to AI summaries and test cases.
            </div>
          </div>
        </div>

        {shortcutsEnabled && (
          <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
            <div className="flex items-center gap-2 mb-4 text-app-text font-medium">
              <Keyboard className="w-4 h-4 text-app-subtext" />
              <h3>Shortcuts</h3>
            </div>
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-app-text text-xs">
                  Translate & Replace
                </span>
                <span className="bg-[#1e293b] text-blue-300 border border-blue-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.translate}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-app-text text-xs">Translate & Popup</span>
                <span className="bg-[#143325] text-green-300 border border-green-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.popup}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-app-text text-xs">Enhance Prompt</span>
                <span className="bg-[#332e18] text-yellow-200 border border-yellow-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.enhance}
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-app-text text-xs">
                  Terminal Translate
                </span>
                <span className="bg-[#1e2a2f] text-cyan-200 border border-cyan-800/50 px-2 py-1 rounded text-[10px] font-mono shadow-sm">
                  {shortcuts.terminal}
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Card 4: Appearance */}
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-4 text-app-text font-medium">
            <Sliders className="w-4 h-4 text-blue-400" />
            <h3>Appearance</h3>
          </div>
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-[10px] text-app-subtext block mb-1">
                  Theme Palette
                </label>
                <div className="relative">
                  <select
                    value={appTheme}
                    onChange={(e: any) => setAppTheme(e.target.value)}
                    className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                    <option value="default">Default</option>
                    <option value="pastel-blue">Pastel Blue</option>
                    <option value="pastel-green">Pastel Green</option>
                  </select>
                  <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-app-subtext pointer-events-none" />
                </div>
              </div>
              <div>
                <label className="text-[10px] text-app-subtext block mb-1">
                  Color Mode
                </label>
                <div className="relative">
                  <select
                    value={appMode}
                    onChange={(e: any) => setAppMode(e.target.value)}
                    className="w-full bg-background border border-app-border rounded p-2 px-3 text-xs appearance-none cursor-pointer hover:border-gray-500 transition outline-none">
                    <option value="light">Light</option>
                    <option value="dark">Dark</option>
                    <option value="system">System</option>
                  </select>
                  <ChevronDown className="w-3 h-3 absolute right-3 top-2.5 text-app-subtext pointer-events-none" />
                </div>
              </div>
            </div>

            <div className="flex justify-between items-center pt-2 border-t border-app-border">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-md bg-[#1d3326] text-green-500 flex items-center justify-center text-lg border border-green-500/30 shadow-[0_0_10px_rgba(34,197,94,0.1)]">
                  <Power className="w-5 h-5" />
                </div>
                <div>
                  <div className="text-app-text font-medium text-xs leading-tight">
                    Shortcuts Active
                  </div>
                  <div className="text-[10px] text-app-subtext leading-tight">
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
