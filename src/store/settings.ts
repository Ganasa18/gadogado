import { create } from "zustand";
import { persist } from "zustand/middleware";

export type LLMProvider =
  | "local"
  | "openai"
  | "gemini"
  | "ollama"
  | "llama_cpp"
  | "dll";

export type EmbeddingProvider = "local";

export interface PromptTemplate {
  id: string;
  name: string;
  systemPrompt: string;
  isDefault: boolean;
}

export const DEFAULT_ENHANCE_PROMPT =
  "You are an expert prompt engineer. Improve the following prompt to be more precise, descriptive, and effective for large language models. Ensure clarity and remove ambiguity. Return ONLY the enhanced prompt. Do not include any explanations.";

export const DEFAULT_TEMPLATES: PromptTemplate[] = [
  {
    id: "default",
    name: "Default Enhancement",
    systemPrompt: DEFAULT_ENHANCE_PROMPT,
    isDefault: true,
  },
  {
    id: "code-review",
    name: "Code Review",
    systemPrompt:
      "You are an expert code reviewer. Analyze the following code and provide constructive feedback on quality, best practices, and potential improvements. Focus on clarity, efficiency, and maintainability. Return ONLY the review comments.",
    isDefault: true,
  },
  {
    id: "writing-assistant",
    name: "Writing Assistant",
    systemPrompt:
      "You are a professional writing assistant. Improve the following text for clarity, grammar, style, and readability. Maintain the original tone and intent. Return ONLY the improved text.",
    isDefault: true,
  },
  {
    id: "technical-docs",
    name: "Technical Docs",
    systemPrompt:
      "You are a technical documentation expert. Transform the following content into clear, well-structured technical documentation. Use appropriate formatting and examples where helpful. Return ONLY the formatted documentation.",
    isDefault: true,
  },
];

export interface SettingsState {
  provider: LLMProvider;
  model: string;
  apiKey: string;
  baseUrl: string;
  localModels: string[];
  embeddingProvider: EmbeddingProvider;
  embeddingModel: string;
  shortcutsEnabled: boolean;
  autoTranslate: boolean;
  shortcuts: Record<"translate" | "popup" | "enhance" | "terminal", string>;
  sourceLang: string;
  targetLang: string;
  aiOutputLanguage: string;
  promptTemplates: PromptTemplate[];
  activeTemplateId: string;
  setProvider: (provider: LLMProvider) => void;
  setModel: (model: string) => void;
  setApiKey: (key: string) => void;
  setBaseUrl: (url: string) => void;
  setLocalModels: (models: string[]) => void;
  setEmbeddingProvider: (provider: EmbeddingProvider) => void;
  setEmbeddingModel: (model: string) => void;
  setShortcutsEnabled: (enabled: boolean) => void;
  setAutoTranslate: (enabled: boolean) => void;
  setShortcut: (
    action: "translate" | "popup" | "enhance" | "terminal",
    combo: string
  ) => void;
  resetShortcuts: () => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  setAiOutputLanguage: (lang: string) => void;
  setActiveTemplateId: (id: string) => void;
  addPromptTemplate: (template: Omit<PromptTemplate, "id" | "isDefault">) => void;
  updatePromptTemplate: (id: string, updates: Partial<PromptTemplate>) => void;
  deletePromptTemplate: (id: string) => void;
  restoreDefaultTemplates: () => void;
  // Navigation Management
  navSettings: Record<string, { visible: boolean; order: number }>;
  sectionSettings: Record<string, { order: number; visible: boolean }>; // visible defaults to true if missing
  toggleNavVisibility: (path: string) => void;
  setNavOrder: (path: string, order: number) => void;
  setSectionOrder: (sectionId: string, order: number) => void;
  resetNavSettings: () => void;
}

const DEFAULT_SHORTCUTS: SettingsState["shortcuts"] = {
  translate: "Ctrl + Alt + Q",
  popup: "Ctrl + Alt + P",
  enhance: "Ctrl + Alt + E",
  terminal: "Ctrl + Alt + T",
};

const LEGACY_SHORTCUTS = {
  translate: new Set(["Ctrl + Alt + T"]),
  terminal: new Set(["Ctrl + Alt + R", "Ctrl + Alt + U"]),
};

const PROVIDER_BASE_URLS: Record<LLMProvider, string> = {
  local: "http://localhost:1234/v1",
  openai: "https://api.openai.com/v1",
  gemini: "https://generativelanguage.googleapis.com/v1beta/models",
  ollama: "http://localhost:11434/v1",
  llama_cpp: "http://localhost:8080/v1",
  dll: "",
};

export const DEFAULT_MODELS: Record<LLMProvider, string> = {
  local: "local-model",
  openai: "gpt-4o",
  gemini: "gemini-2.0-flash",
  ollama: "llama3",
  llama_cpp: "llama-3-8b-instruct",
  dll: "",
};

export interface ProviderConfig {
  baseUrl: string;
  defaultModel: string;
  requiresApiKey: boolean;
  label: string;
}

export const PROVIDER_CONFIGS: Record<LLMProvider, ProviderConfig> = {
  local: {
    baseUrl: PROVIDER_BASE_URLS.local,
    defaultModel: DEFAULT_MODELS.local,
    requiresApiKey: false,
    label: "Local (LM Studio)",
  },
  openai: {
    baseUrl: PROVIDER_BASE_URLS.openai,
    defaultModel: DEFAULT_MODELS.openai,
    requiresApiKey: true,
    label: "OpenAI",
  },
  gemini: {
    baseUrl: PROVIDER_BASE_URLS.gemini,
    defaultModel: DEFAULT_MODELS.gemini,
    requiresApiKey: true,
    label: "Google Gemini",
  },
  ollama: {
    baseUrl: PROVIDER_BASE_URLS.ollama,
    defaultModel: DEFAULT_MODELS.ollama,
    requiresApiKey: false,
    label: "Ollama",
  },
  llama_cpp: {
    baseUrl: PROVIDER_BASE_URLS.llama_cpp,
    defaultModel: DEFAULT_MODELS.llama_cpp,
    requiresApiKey: false,
    label: "Llama.cpp",
  },
  dll: {
    baseUrl: PROVIDER_BASE_URLS.dll,
    defaultModel: DEFAULT_MODELS.dll,
    requiresApiKey: false,
    label: "DLL Plugin",
  },
};

export const isKeylessProvider = (provider: LLMProvider): boolean => {
  return !PROVIDER_CONFIGS[provider].requiresApiKey;
};

const normalizeProvider = (value?: string): LLMProvider => {
  if (value === "google" || value === "gemini") return "gemini";
  if (value === "openai") return "openai";
  if (value === "ollama") return "ollama";
  if (value === "llama_cpp") return "llama_cpp";
  if (value === "dll") return "dll";
  return "local";
};

function normalizeShortcuts(
  shortcuts?: SettingsState["shortcuts"]
): SettingsState["shortcuts"] {
  if (!shortcuts) {
    return DEFAULT_SHORTCUTS;
  }
  const nextShortcuts: SettingsState["shortcuts"] = {
    ...DEFAULT_SHORTCUTS,
    ...shortcuts,
  };
  if (LEGACY_SHORTCUTS.translate.has(nextShortcuts.translate)) {
    nextShortcuts.translate = DEFAULT_SHORTCUTS.translate;
  }
  if (LEGACY_SHORTCUTS.terminal.has(nextShortcuts.terminal)) {
    nextShortcuts.terminal = DEFAULT_SHORTCUTS.terminal;
  }
  return nextShortcuts;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      provider: "local",
      model: "local-model",
      apiKey: "",
      baseUrl: PROVIDER_BASE_URLS.local,
      localModels: [],
      embeddingProvider: "local",
      embeddingModel: "all-minilm-l6-v2",
      shortcutsEnabled: true,
      autoTranslate: true,
      shortcuts: DEFAULT_SHORTCUTS,
      sourceLang: "Auto Detect",
      targetLang: "English",
      aiOutputLanguage: "English",
      promptTemplates: DEFAULT_TEMPLATES,
      activeTemplateId: "default",
      setProvider: (provider) => set({ provider }),
      setModel: (model) => set({ model }),
      setApiKey: (apiKey) => set({ apiKey }),
      setBaseUrl: (baseUrl) => set({ baseUrl }),
      setLocalModels: (localModels) => set({ localModels }),
      setEmbeddingProvider: (embeddingProvider) => set({ embeddingProvider }),
      setEmbeddingModel: (embeddingModel) => set({ embeddingModel }),
      setShortcutsEnabled: (shortcutsEnabled) => set({ shortcutsEnabled }),
      setAutoTranslate: (autoTranslate) => set({ autoTranslate }),
      setShortcut: (action, combo) =>
        set((state) => ({
          shortcuts: {
            ...state.shortcuts,
            [action]: combo,
          },
        })),
      resetShortcuts: () => set({ shortcuts: DEFAULT_SHORTCUTS }),
      setSourceLang: (sourceLang) => set({ sourceLang }),
      setTargetLang: (targetLang) => set({ targetLang }),
      setAiOutputLanguage: (aiOutputLanguage) => set({ aiOutputLanguage }),
      setActiveTemplateId: (activeTemplateId) => set({ activeTemplateId }),
      addPromptTemplate: (template) =>
        set((state) => ({
          promptTemplates: [
            ...state.promptTemplates,
            { ...template, id: crypto.randomUUID(), isDefault: false },
          ],
        })),
      updatePromptTemplate: (id, updates) =>
        set((state) => ({
          promptTemplates: state.promptTemplates.map((t) =>
            t.id === id ? { ...t, ...updates } : t
          ),
        })),
      deletePromptTemplate: (id) =>
        set((state) => ({
          promptTemplates: state.promptTemplates.filter((t) => t.id !== id),
          activeTemplateId:
            state.activeTemplateId === id ? "default" : state.activeTemplateId,
        })),
      restoreDefaultTemplates: () =>
        set({
          promptTemplates: DEFAULT_TEMPLATES,
          activeTemplateId: "default",
        }),
      // Navigation Management Implementation
      navSettings: {},
      toggleNavVisibility: (path) =>
        set((state) => {
          const current = state.navSettings[path] || { visible: true, order: 0 };
          return {
            navSettings: {
              ...state.navSettings,
              [path]: { ...current, visible: !current.visible },
            },
          };
        }),
      setNavOrder: (path, order) =>
        set((state) => {
          const current = state.navSettings[path] || { visible: true, order: 0 };
          return {
            navSettings: {
              ...state.navSettings,
              [path]: { ...current, order },
            },
          };
        }),
      sectionSettings: {},
      setSectionOrder: (sectionId, order) =>
        set((state) => {
          const current = state.sectionSettings[sectionId] || { visible: true, order: 0 };
          return {
            sectionSettings: {
              ...state.sectionSettings,
              [sectionId]: { ...current, order },
            },
          };
        }),
      resetNavSettings: () => set({ navSettings: {}, sectionSettings: {} }),
    }),
    {
      name: "promptbridge-settings",
      version: 2,
      migrate: (state) => {
        const persisted = state as SettingsState & {
          mode?: string;
          baseUrl?: string;
        };
        if (persisted.provider) {
          const provider = normalizeProvider(persisted.provider);
          return {
            ...persisted,
            provider,
            baseUrl:
              persisted.baseUrl ?? PROVIDER_BASE_URLS[provider] ?? persisted.baseUrl,
            embeddingProvider: persisted.embeddingProvider ?? "local",
            embeddingModel: persisted.embeddingModel ?? "all-minilm-l6-v2",
            aiOutputLanguage: persisted.aiOutputLanguage ?? "English",
            shortcuts: normalizeShortcuts(persisted.shortcuts),
          };
        }
        const provider = normalizeProvider(persisted.mode);
        const baseUrl = persisted.baseUrl ?? PROVIDER_BASE_URLS[provider];
        return {
          ...persisted,
          provider,
          baseUrl,
          embeddingProvider: persisted.embeddingProvider ?? "local",
          embeddingModel: persisted.embeddingModel ?? "all-minilm-l6-v2",
          aiOutputLanguage: persisted.aiOutputLanguage ?? "English",
          shortcuts: normalizeShortcuts(persisted.shortcuts),
        };
      },
    }
  )
);
