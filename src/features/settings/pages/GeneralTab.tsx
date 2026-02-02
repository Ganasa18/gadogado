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
    <div className="flex flex-col bg-app-bg text-app-text min-h-full overflow-y-auto pb-10">
      {/* Header Section */}
      <header className="p-6 md:p-8 pb-4">
        <div className="flex items-center gap-1 text-[10px] text-app-subtext mb-1 uppercase tracking-wider font-semibold">
          <span>Configuration</span>
          <span className="opacity-50 mx-1">&gt;</span>
          <span className="text-app-text/70">General</span>
        </div>
        <h1 className="text-2xl md:text-3xl font-bold bg-gradient-to-r from-app-text to-app-text/60 bg-clip-text text-transparent">
          General Settings
        </h1>
      </header>

      <main className="px-6 md:px-8 grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left Column */}
        <div className="lg:col-span-7 flex flex-col gap-6">
          {/* Card 1: Translation Model */}
          <section className="bg-app-card rounded-xl border border-app-border/40 p-6 shadow-xl backdrop-blur-sm relative overflow-hidden group">
            <div className="absolute top-0 left-0 w-1 h-full bg-primary/40 group-hover:bg-primary transition-colors duration-300"></div>
            <div className="flex items-start gap-4 mb-6">
              <div className="p-3 rounded-lg bg-primary/10 text-primary border border-primary/20 shadow-inner">
                <Box className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-base font-semibold text-app-text leading-tight">Translation Model</h3>
                <p className="text-[11px] text-app-subtext mt-1">Configure your primary translation engine</p>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-5 mb-5">
              <div className="space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">CLI Proxy</label>
                <div className="relative group/select">
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
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm focus:ring-1 focus:ring-primary/30">
                    <option value="local">Local (LM Studio)</option>
                    <option value="ollama">Ollama</option>
                    <option value="llama_cpp">Llama.cpp</option>
                    <option value="openai">OpenAI</option>
                    <option value="gemini">Gemini</option>
                    <option value="openrouter">OpenRouter</option>
                    <option value="cli_proxy">CLI Proxy</option>
                    <option value="dll">DLL</option>
                  </select>
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none group-hover/select:text-app-text transition-colors" />
                </div>
              </div>
              <div className="space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">Model Selection</label>
                <div className="relative group/select">
                  {provider === "openrouter" ? (
                    <select
                      value={selectedOpenRouterProvider}
                      onChange={(e: any) => {
                        const newProvider = e.target.value;
                        setSelectedOpenRouterProvider(newProvider);
                        const firstModel = openRouterModels.find((m) =>
                          m.id.startsWith(newProvider + "/"),
                        );
                        if (firstModel) {
                          setModel(firstModel.id);
                        }
                      }}
                      className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm focus:ring-1 focus:ring-primary/30">
                      <option value="">Select Provider...</option>
                      {openRouterProviders.map((p) => (
                        <option key={p.slug ?? p.id ?? p.name} value={p.slug ?? p.id ?? ""}>
                          {p.name ?? p.slug ?? p.id}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <select
                      value={model}
                      onChange={(e: any) => setModel(e.target.value)}
                      className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm focus:ring-1 focus:ring-primary/30">
                      {shouldFetchModels ? (
                        localModels.length > 0 ? (
                          localModels.map((m) => (
                            <option key={m} value={m}>{m}</option>
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
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none group-hover/select:text-app-text transition-colors" />
                </div>
              </div>

              {provider === "openrouter" && (
                <div className="space-y-1.5 mt-3 pt-4 border-t border-app-border/40 md:col-span-2">
                  <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">
                    Model Selection from {openRouterProviders.find(p => (p.slug ?? p.id) === selectedOpenRouterProvider)?.name ?? selectedOpenRouterProvider}
                  </label>
                  <div className="relative group/select">
                    <select
                      value={model}
                      onChange={(e: any) => setModel(e.target.value)}
                      className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm focus:ring-1 focus:ring-primary/30"
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
                    <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none group-hover/select:text-app-text transition-colors" />
                  </div>
                  {openRouterSelectedModel && (
                    <div className="mt-1.5 px-1 flex items-center justify-between">
                      <span className="text-[10px] text-app-subtext/60">Selected: <span className="text-app-text font-medium">{(openRouterSelectedModel as any)?.name ?? (openRouterSelectedModel as any)?.id}</span></span>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="space-y-1.5 mb-6">
              <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">Embedding Model (RAG)</label>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
                <div className="relative group/select">
                  <select
                    value={embeddingProvider}
                    onChange={(e: any) => setEmbeddingProvider(e.target.value as "local")}
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm">
                    <option value="local">Local (FastEmbed)</option>
                  </select>
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none" />
                </div>
                <div className="relative group/select">
                  <select
                    value={embeddingModel}
                    onChange={(e: any) => setEmbeddingModel(e.target.value)}
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none cursor-pointer hover:border-app-text/30 transition outline-none shadow-sm">
                    <option value="all-minilm-l6-v2">all-MiniLM-L6-v2 (384 dim, fast)</option>
                    <option value="nomic-embed-text-v1.5">nomic-embed-text-v1.5 (768 dim, best)</option>
                    <option value="bge-small-en-v1.5">bge-small-en-v1.5 (384 dim)</option>
                    <option value="multilingual-e5-small">multilingual-e5-small (384 dim, multi)</option>
                  </select>
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none" />
                </div>
              </div>
            </div>

            {/* Provider Specific Status Messages */}
            {isLocalProvider && (
              <div className="bg-app-success-dim border border-app-success/20 text-app-success rounded-lg px-4 py-3 text-[10px] flex items-center gap-3 animate-in fade-in slide-in-from-top-1 duration-300 shadow-sm">
                <CheckCircle className="w-4 h-4 shrink-0" />
                <span className="font-medium tracking-wide">Local LLM provider active (LM Studio, Ollama, or Llama.cpp).</span>
              </div>
            )}
            {isCliProxyProvider && (
              <div className="bg-app-success-dim border border-app-success/20 text-app-success rounded-lg px-4 py-3 text-[10px] flex items-center gap-3 animate-in fade-in slide-in-from-top-1 duration-300 shadow-sm">
                <CheckCircle className="w-4 h-4 shrink-0" />
                <span className="font-medium tracking-wide">CLI Proxy provider active. Models fetched from proxy server.</span>
              </div>
            )}
          </section>

          {/* Card 2: Language Preferences */}
          <section className="bg-app-card rounded-xl border border-app-border/40 p-6 shadow-xl backdrop-blur-sm relative overflow-hidden group">
            <div className="absolute top-0 left-0 w-1 h-full bg-purple-500/40 group-hover:bg-purple-500 transition-colors duration-300"></div>
            <div className="flex justify-between items-start mb-6">
              <div className="flex items-start gap-4">
                <div className="p-3 rounded-lg bg-purple-500/10 text-purple-400 border border-purple-500/20 shadow-inner">
                  <Languages className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="text-base font-semibold text-app-text leading-tight">Language Preferences</h3>
                  <p className="text-[11px] text-app-subtext mt-1">Define translation workflow</p>
                </div>
              </div>
              <span className="text-[9px] font-bold text-app-subtext/80 uppercase tracking-widest mt-1 px-2 py-1 bg-app-bg/50 rounded-md border border-app-border/40">
                MAX 5000 CHR
              </span>
            </div>

            <div className="flex flex-col md:flex-row items-center gap-4 md:gap-5 mb-6 relative">
              <div className="flex-1 w-full space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">From</label>
                <div className="relative group/select">
                  <select
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none outline-none hover:border-app-text/30 transition shadow-sm"
                    value={sourceLang}
                    onChange={(e: any) => setSourceLang(e.target.value)}>
                    {LANGUAGES.map((lang) => (
                      <option key={lang} value={lang}>{lang}</option>
                    ))}
                  </select>
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none" />
                </div>
              </div>

              <div className="shrink-0 flex items-center justify-center -mb-2 md:mb-0 md:mt-5">
                <button
                  onClick={() => {
                    const temp = sourceLang;
                    if (targetLang !== "Auto Detect") {
                      setSourceLang(targetLang);
                      setTargetLang(temp === "Auto Detect" ? "English" : temp);
                    }
                  }}
                  className="p-2.5 rounded-full bg-app-bg border border-app-border text-app-subtext hover:text-app-text hover:border-app-text/30 hover:scale-110 active:scale-95 transition-all shadow-md group/swap">
                  <ArrowRightLeft className="w-4 h-4 group-hover/swap:rotate-180 transition-transform duration-500" />
                </button>
              </div>

              <div className="flex-1 w-full space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">To</label>
                <div className="relative group/select">
                  <select
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none outline-none hover:border-app-text/30 transition shadow-sm"
                    value={targetLang}
                    onChange={(e: any) => setTargetLang(e.target.value)}>
                    {LANGUAGES.filter((lang) => lang !== "Auto Detect").map((lang) => (
                      <option key={lang} value={lang}>{lang}</option>
                    ))}
                  </select>
                  <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none" />
                </div>
              </div>
            </div>

            <div className="pt-5 border-t border-app-border/40 space-y-1.5">
              <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">AI Output Language</label>
              <div className="relative group/select">
                <select
                  className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs appearance-none outline-none hover:border-app-text/30 transition shadow-sm"
                  value={aiOutputLanguage}
                  onChange={(e: any) => setAiOutputLanguage(e.target.value)}>
                  {LANGUAGES.filter((lang) => lang !== "Auto Detect").map((lang) => (
                    <option key={lang} value={lang}>{lang}</option>
                  ))}
                </select>
                <ChevronDown className="w-3.5 h-3.5 absolute right-4 top-1/2 -translate-y-1/2 text-app-subtext/60 pointer-events-none" />
              </div>
              <p className="text-[10px] text-app-subtext/60 italic px-0.5">Applied to AI summaries and test cases generated by the assistant.</p>
            </div>
          </section>
        </div>

        {/* Right Column */}
        <div className="lg:col-span-5 flex flex-col gap-6">
          {/* Card 3: API Config */}
          <section className="bg-app-card rounded-xl border border-app-border/40 p-6 shadow-xl backdrop-blur-sm relative overflow-hidden group">
            <div className="absolute top-0 left-0 w-1 h-full bg-orange-500/40 group-hover:bg-orange-500 transition-colors duration-300"></div>
            <div className="flex items-start gap-4 mb-6">
              <div className="p-3 rounded-lg bg-orange-500/10 text-orange-400 border border-orange-500/20 shadow-inner">
                <Sliders className="w-5 h-5 -rotate-90" />
              </div>
              <div>
                <h3 className="text-base font-semibold text-app-text leading-tight">API Config</h3>
                <p className="text-[11px] text-app-subtext mt-1">Security & connectivity</p>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-5 mb-5">
              <div className="space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">
                  {requiresApiKey ? (providerLabel[provider] || provider) : (provider === "local" ? "Ollama/LM Studio" : provider)} API KEY
                </label>
                <div className="relative group/input">
                  <input
                    className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 pr-10 text-xs outline-none focus:border-app-text/30 transition shadow-inner font-mono tracking-widest"
                    placeholder="••••••••••••••••••••••••••••••••"
                    type="password"
                    value={apiKeyInput}
                    onInput={(e: any) => {
                      setApiKeyInput(e.target.value);
                      setApiKey(provider, e.target.value);
                    }}
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <label className="text-[9px] text-app-subtext/80 font-bold uppercase tracking-widest block px-0.5">Base URL</label>
                <input
                  className="w-full bg-background border border-app-border rounded-lg p-2.5 px-4 text-xs outline-none focus:border-app-text/30 transition shadow-inner font-mono"
                  placeholder="http://127.0.0.1:8317/v1"
                  type="text"
                  value={baseUrl}
                  onInput={(e: any) => setBaseUrl(e.target.value)}
                />
              </div>
            </div>

            <button
              className="w-full bg-app-success/10 hover:bg-app-success/20 text-app-success border border-app-success/30 rounded-lg p-3 text-xs font-bold tracking-wider uppercase flex items-center justify-center gap-2.5 transition-all shadow-lg shadow-app-success/10 hover:shadow-app-success/20 group/save"
              onClick={() => {
                if (!isTauri()) {
                  addToast("Tauri runtime not available in browser mode", "error");
                  return;
                }
                const config = buildConfig({ maxTokens: 1024, temperature: 0.7 });
                invoke("sync_config", { config })
                  .then(() => addToast("Configuration saved successfully", "success"))
                  .catch((err) => {
                    console.error(err);
                    addToast("Failed to save configuration", "error");
                  });
              }}>
              <CheckCircle className="w-4 h-4 group-hover/save:scale-110 transition-transform" />
              Save Settings
            </button>
          </section>

          {/* Card 4: Quick Shortcuts */}
          <section className="bg-app-card rounded-xl border border-app-border/40 p-6 shadow-xl backdrop-blur-sm relative overflow-hidden group">
            <div className="absolute top-0 left-0 w-1 h-full bg-blue-500/40 group-hover:bg-blue-500 transition-colors duration-300"></div>
            <div className="flex items-start gap-4 mb-6">
              <div className="p-3 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20 shadow-inner">
                <Keyboard className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-base font-semibold text-app-text leading-tight">Quick Shortcuts</h3>
                <p className="text-[11px] text-app-subtext mt-1">Productivity hotkeys</p>
              </div>
            </div>

            <div className="space-y-2.5">
              {[
                { label: "Translate & Replace", key: shortcuts.translate, color: "text-primary bg-primary/10 border-primary/20" },
                { label: "Translate & Popup", key: shortcuts.popup, color: "text-app-success bg-app-success/10 border-app-success/20" },
                { label: "Enhance Prompt", key: shortcuts.enhance, color: "text-amber-400 bg-amber-400/10 border-amber-400/20" }
              ].map((item, idx) => (
                <div key={idx} className="flex justify-between items-center p-3 rounded-lg bg-background/40 hover:bg-background/60 border border-app-border/30 transition-colors group/item">
                  <span className="text-xs text-app-text group-hover/item:text-app-text font-medium">{item.label}</span>
                  <div className={`flex items-center gap-1.5 ${item.color} border px-2.5 py-1 rounded-md shadow-sm`}>
                    <span className="text-[9px] font-bold tracking-tighter opacity-70">Ctrl</span>
                    <span className="text-[9px] font-bold tracking-tighter opacity-70">Alt</span>
                    <span className="text-[10px] font-bold px-1 min-w-[1.2rem] text-center">{item.key.slice(-1).toUpperCase()}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>

        {/* Card 5: Appearance & Debug - FULL WIDTH */}
        <section className="lg:col-span-12 bg-app-card/60 rounded-xl border border-app-border/30 p-5 shadow-lg backdrop-blur-sm mt-2">
          <div className="flex justify-between items-center mb-5 pb-4 border-b border-app-border/30">
            <div className="flex items-center gap-3">
              <div className={`w-10 h-10 rounded-xl flex items-center justify-center text-lg border transition-all duration-500 ${shortcutsEnabled ? 'bg-app-success/20 text-app-success border-app-success/30 shadow-[0_0_15px_rgba(16,185,129,0.1)]' : 'bg-app-subtext/10 text-app-subtext border-app-border/50'}`}>
                <Power className={`w-5 h-5 ${shortcutsEnabled ? 'animate-pulse' : ''}`} />
              </div>
              <div>
                <div className="text-app-text font-bold text-xs tracking-tight">System Controls</div>
                <div className={`text-[10px] font-medium ${shortcutsEnabled ? 'text-app-success' : 'text-app-subtext'}`}>
                  {shortcutsEnabled ? "Global Hub Active" : "Shortcuts Hibernating"}
                </div>
              </div>
            </div>
            <Switch checked={shortcutsEnabled} onCheckedChange={setShortcutsEnabled} />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-5">
            <div className="space-y-1.5">
              <label className="text-[9px] text-app-subtext/70 font-bold uppercase tracking-wider block">Palette</label>
              <select
                value={appTheme}
                onChange={(e: any) => setAppTheme(e.target.value)}
                className="w-full bg-background/50 border border-app-border/50 rounded-lg p-2 text-[11px] outline-none hover:border-app-text/30 transition">
                <option value="default">Default</option>
                <option value="pastel-blue">Pastel Blue</option>
                <option value="pastel-green">Pastel Green</option>
              </select>
            </div>
            <div className="space-y-1.5">
              <label className="text-[9px] text-app-subtext/70 font-bold uppercase tracking-wider block">Mode</label>
              <select
                value={appMode}
                onChange={(e: any) => setAppMode(e.target.value)}
                className="w-full bg-background/50 border border-app-border/50 rounded-lg p-2 text-[11px] outline-none hover:border-app-text/30 transition">
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
              </select>
            </div>
          </div>

          <button
            className="w-full bg-red-500/5 hover:bg-red-500/10 border border-red-500/10 hover:border-red-500/20 rounded-lg py-2 text-[10px] text-red-500/60 hover:text-red-500 transition-all font-medium tracking-wide uppercase"
            onClick={() => {
              if (confirm("Reset application data? This cannot be undone.")) {
                localStorage.removeItem("promptbridge-settings");
                localStorage.removeItem("promptbridge-history");
                window.location.reload();
              }
            }}>
            Emergency Data Reset
          </button>
        </section>
      </main>
    </div>
  );
}
