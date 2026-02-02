import { useEffect } from "react";
import { PROVIDER_MODEL_OPTIONS, useSettingsStore } from "../../../store/settings";
import { useModelsQuery } from "../../../hooks/useLlmApi";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";

export function useRagModelSelection() {
  const { provider, model, localModels, setModel, setLocalModels } =
    useSettingsStore();
  const buildConfig = useLlmConfigBuilder();

  const isLocalProvider =
    provider === "local" || provider === "ollama" || provider === "llama_cpp";

  const isCliProxyProvider = provider === "cli_proxy";
  const shouldFetchModels = isLocalProvider || isCliProxyProvider;

  const localConfig = buildConfig({ maxTokens: 1024, temperature: 0.7 });
  const modelsQuery = useModelsQuery(localConfig, shouldFetchModels);

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
    // For providers that fetch models from server (local, ollama, llama_cpp, cli_proxy)
    if (shouldFetchModels) {
      if (localModels.length > 0 && !localModels.includes(model)) {
        setModel(localModels[0]);
      }
      return;
    }
    if (provider === "gemini") {
      const models = PROVIDER_MODEL_OPTIONS.gemini;
      if (models && models.length > 0 && !models.includes(model)) {
        setModel(models[0]);
      }
      return;
    }
    if (provider === "openai") {
      const models = PROVIDER_MODEL_OPTIONS.openai;
      if (models && models.length > 0 && !models.includes(model)) {
        setModel(models[0]);
      }
      return;
    }
    if (provider === "openrouter") {
      const models =
        (modelsQuery.data && modelsQuery.data.length > 0
          ? modelsQuery.data
          : PROVIDER_MODEL_OPTIONS.openrouter) ?? [];
      if (models.length > 0 && !models.includes(model)) {
        setModel(models[0]);
      }
    }
  }, [shouldFetchModels, localModels, model, provider, setModel, modelsQuery.data]);

  return {
    provider,
    model,
    localModels,
    setModel,
    setLocalModels,
    buildConfig,
    isLocalProvider,
    isCliProxyProvider,
    shouldFetchModels,
    modelsQuery,
  };
}
