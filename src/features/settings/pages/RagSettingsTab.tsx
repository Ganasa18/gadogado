import { useEffect, useState } from "react";
import { Switch } from "../../../shared/components/Switch";
import {
  Brain,
  Zap,
  MemoryStick,
  Sliders,
  Info,
  Save,
  Loader2,
} from "lucide-react";
import { useSettingsStore } from "../../../store/settings";
import { useToastStore } from "../../../store/toast";
import {
  getRagGlobalSettings,
  updateRagGlobalSettings,
  getModelContextLimit,
} from "../../../features/rag/api/contextSettings";
import type {
  RagContextSettings,
  ModelContextLimit,
} from "../../../store/settings";

export default function RagSettingsTab() {
  const { provider, model } = useSettingsStore();
  const { addToast } = useToastStore();

  const [settings, setSettings] = useState<RagContextSettings | null>(null);
  const [modelLimit, setModelLimit] = useState<ModelContextLimit | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSettings();
    loadModelLimit();
  }, [provider, model]);

  const loadSettings = async () => {
    try {
      setIsLoading(true);
      const s = await getRagGlobalSettings();
      setSettings(s);
    } catch (error) {
      addToast(`Failed to load RAG settings: ${error}`, "error");
    } finally {
      setIsLoading(false);
    }
  };

  const loadModelLimit = async () => {
    try {
      const limit = await getModelContextLimit(provider, model);
      setModelLimit(limit);
    } catch (error) {
      console.error("Failed to load model limit:", error);
    }
  };

  const handleSave = async () => {
    if (!settings) return;
    setIsSaving(true);
    try {
      await updateRagGlobalSettings(settings);
      addToast("RAG settings saved successfully", "success");
    } catch (error) {
      addToast(`Failed to save RAG settings: ${error}`, "error");
    } finally {
      setIsSaving(false);
    }
  };

  const getStrategyDescription = (strategy: string) => {
    switch (strategy) {
      case "adaptive":
        return "Automatically selects strategy based on model context window size";
      case "truncate":
        return "Simply removes oldest messages when context is full (most efficient)";
      case "summarize":
        return "Summarizes old messages using LLM (best for large context models)";
      case "hybrid":
        return "Summarizes very old messages, keeps recent verbatim (balanced)";
      default:
        return "";
    }
  };

  const applyPreset = (preset: "small" | "medium" | "large") => {
    if (!settings) return;

    const presets = {
      small: {
        maxContextTokens: 4000,
        maxHistoryMessages: 5,
        enableCompaction: true,
        compactionStrategy: "adaptive" as const,
        summaryThreshold: 3,
        reservedForResponse: 1024,
        smallModelThreshold: 8000,
        largeModelThreshold: 32000,
      },
      medium: {
        maxContextTokens: 16000,
        maxHistoryMessages: 10,
        enableCompaction: true,
        compactionStrategy: "adaptive" as const,
        summaryThreshold: 5,
        reservedForResponse: 2048,
        smallModelThreshold: 8000,
        largeModelThreshold: 32000,
      },
      large: {
        maxContextTokens: 128000,
        maxHistoryMessages: 20,
        enableCompaction: true,
        compactionStrategy: "summarize" as const,
        summaryThreshold: 10,
        reservedForResponse: 4096,
        smallModelThreshold: 8000,
        largeModelThreshold: 32000,
      },
    };

    setSettings({ ...settings, ...presets[preset] });
  };

  if (isLoading) {
    return (
      <div className="flex bg-app-bg text-app-text min-h-full p-6 justify-center items-center">
        <Loader2 className="w-6 h-6 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="flex bg-app-bg text-app-text min-h-full p-6">
      <aside className="w-full max-w-4xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center gap-3 mb-6">
          <Brain className="w-6 h-6 text-primary" />
          <div>
            <h1 className="text-xl font-semibold">RAG Context Settings</h1>
            <p className="text-sm text-app-subtext">
              Configure context window and memory management for RAG queries
            </p>
          </div>
        </div>

        {/* Current Model Info */}
        {modelLimit && (
          <div className="bg-app-card rounded-lg border border-app-border p-4">
            <div className="flex items-center gap-2 mb-3">
              <Info className="w-4 h-4 text-blue-400" />
              <h3 className="font-medium">Current Model Capacity</h3>
            </div>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <div className="text-app-subtext">Provider</div>
                <div className="font-medium capitalize">{provider}</div>
              </div>
              <div>
                <div className="text-app-subtext">Model</div>
                <div className="font-medium">{model}</div>
              </div>
              <div>
                <div className="text-app-subtext">Context Window</div>
                <div className="font-medium">
                  {modelLimit.contextWindow.toLocaleString()} tokens
                </div>
              </div>
              <div>
                <div className="text-app-subtext">Max Output</div>
                <div className="font-medium">
                  {modelLimit.maxOutputTokens.toLocaleString()} tokens
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Context Window Settings */}
        {settings && (
          <div className="bg-app-card rounded-lg border border-app-border p-4">
            <div className="flex items-center gap-2 mb-4">
              <MemoryStick className="w-4 h-4 text-primary" />
              <h3 className="font-medium">Context Window</h3>
            </div>

            <div className="space-y-4">
              {/* Max Context Tokens */}
              <div>
                <label className="text-sm text-app-subtext block mb-2">
                  Max Context Tokens
                  <span className="ml-2 text-xs text-primary">
                    ({(settings.maxContextTokens || 8000).toLocaleString()})
                  </span>
                </label>
                <input
                  type="range"
                  min="2000"
                  max={modelLimit?.contextWindow || 128000}
                  step={1000}
                  value={settings.maxContextTokens || 8000}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      maxContextTokens: parseInt(e.target.value),
                    })
                  }
                  className="w-full"
                />
                <div className="flex justify-between text-xs text-app-subtext mt-1">
                  <span>2K</span>
                  <span>
                    {(modelLimit?.contextWindow || 128000).toLocaleString()}
                  </span>
                </div>
              </div>

              {/* Max History Messages */}
              <div>
                <label className="text-sm text-app-subtext block mb-2">
                  Max History Messages: {settings.maxHistoryMessages || 10}
                </label>
                <input
                  type="number"
                  min="1"
                  max="50"
                  value={settings.maxHistoryMessages || 10}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      maxHistoryMessages: parseInt(e.target.value) || 10,
                    })
                  }
                  className="w-full bg-background border border-app-border rounded p-2"
                />
              </div>

              {/* Reserved for Response */}
              <div>
                <label className="text-sm text-app-subtext block mb-2">
                  Reserved for Response:{" "}
                  {(settings.reservedForResponse || 2048).toLocaleString()} tokens
                </label>
                <input
                  type="range"
                  min="512"
                  max="8192"
                  step="512"
                  value={settings.reservedForResponse || 2048}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      reservedForResponse: parseInt(e.target.value),
                    })
                  }
                  className="w-full"
                />
              </div>
            </div>
          </div>
        )}

        {/* Compaction Settings */}
        {settings && (
          <div className="bg-app-card rounded-lg border border-app-border p-4">
            <div className="flex items-center gap-2 mb-4">
              <Zap className="w-4 h-4 text-yellow-400" />
              <h3 className="font-medium">Context Compaction</h3>
            </div>

            <div className="space-y-4">
              {/* Enable Compaction */}
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm font-medium">Enable Compaction</div>
                  <div className="text-xs text-app-subtext">
                    Summarize older messages when context is full
                  </div>
                </div>
                <Switch
                  checked={settings.enableCompaction}
                  onCheckedChange={(checked) =>
                    setSettings({
                      ...settings,
                      enableCompaction: checked,
                    })
                  }
                />
              </div>

              {/* Compaction Strategy */}
              {settings.enableCompaction && (
                <>
                  <div>
                    <label className="text-sm text-app-subtext block mb-2">
                      Compaction Strategy
                    </label>
                    <select
                      value={settings.compactionStrategy}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          compactionStrategy: e.target.value as any,
                        })
                      }
                      className="w-full bg-background border border-app-border rounded p-2">
                      <option value="adaptive">
                        Adaptive (Auto-detect) - Recommended
                      </option>
                      <option value="truncate">
                        Truncate Oldest (Most Efficient)
                      </option>
                      <option value="summarize">
                        Summarize All (Best Quality)
                      </option>
                      <option value="hybrid">Hybrid (Balanced)</option>
                    </select>
                    <div className="text-xs text-app-subtext mt-1">
                      {getStrategyDescription(settings.compactionStrategy)}
                    </div>
                  </div>

                  {/* Adaptive Strategy Thresholds */}
                  {settings.compactionStrategy === "adaptive" && (
                    <div className="bg-app-bg p-3 rounded border border-app-border">
                      <div className="text-sm font-medium mb-2">
                        Model Size Thresholds
                      </div>
                      <div className="space-y-3">
                        <div>
                          <label className="text-xs text-app-subtext block mb-1">
                            Small Model Threshold:{" "}
                            {(settings.smallModelThreshold || 8000).toLocaleString()}{" "}
                            tokens
                          </label>
                          <input
                            type="range"
                            min="4000"
                            max="16000"
                            step="1000"
                            value={settings.smallModelThreshold || 8000}
                            onChange={(e) =>
                              setSettings({
                                ...settings,
                                smallModelThreshold: parseInt(e.target.value),
                              })
                            }
                            className="w-full"
                          />
                          <div className="text-xs text-app-subtext mt-1">
                            Below this = use efficient truncation (for local
                            models)
                          </div>
                        </div>
                        <div>
                          <label className="text-xs text-app-subtext block mb-1">
                            Large Model Threshold:{" "}
                            {(settings.largeModelThreshold || 32000).toLocaleString()}{" "}
                            tokens
                          </label>
                          <input
                            type="range"
                            min="16000"
                            max="128000"
                            step="1000"
                            value={settings.largeModelThreshold || 32000}
                            onChange={(e) =>
                              setSettings({
                                ...settings,
                                largeModelThreshold: parseInt(e.target.value),
                              })
                            }
                            className="w-full"
                          />
                          <div className="text-xs text-app-subtext mt-1">
                            Above this = use full summarization (for cloud
                            models)
                          </div>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Summary Threshold */}
                  <div>
                    <label className="text-sm text-app-subtext block mb-2">
                      Summarize after: {settings.summaryThreshold || 5} messages
                    </label>
                    <input
                      type="range"
                      min="3"
                      max="20"
                      value={settings.summaryThreshold || 5}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          summaryThreshold: parseInt(e.target.value),
                        })
                      }
                      className="w-full"
                    />
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        {/* Quick Presets */}
        <div className="bg-app-card rounded-lg border border-app-border p-4">
          <div className="flex items-center gap-2 mb-4">
            <Sliders className="w-4 h-4 text-purple-400" />
            <h3 className="font-medium">Quick Presets</h3>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <button
              onClick={() => applyPreset("small")}
              className="p-3 border border-app-border rounded hover:border-primary transition text-left">
              <div className="font-medium text-sm">Small Local</div>
              <div className="text-xs text-app-subtext">
                4K context, Adaptive
              </div>
            </button>

            <button
              onClick={() => applyPreset("medium")}
              className="p-3 border border-app-border rounded hover:border-primary transition text-left">
              <div className="font-medium text-sm">Medium</div>
              <div className="text-xs text-app-subtext">
                16K context, Adaptive
              </div>
            </button>

            <button
              onClick={() => applyPreset("large")}
              className="p-3 border border-app-border rounded hover:border-primary transition text-left">
              <div className="font-medium text-sm">Large Cloud</div>
              <div className="text-xs text-app-subtext">
                128K+ context, Summarize
              </div>
            </button>
          </div>
        </div>

        {/* Save Button */}
        <div className="flex justify-end">
          <button
            onClick={handleSave}
            disabled={isSaving}
            className="px-6 py-2 bg-primary text-white rounded hover:opacity-90 transition disabled:opacity-50 flex items-center gap-2">
            {isSaving ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Saving...
              </>
            ) : (
              <>
                <Save className="w-4 h-4" />
                Save Settings
              </>
            )}
          </button>
        </div>
      </aside>
    </div>
  );
}
