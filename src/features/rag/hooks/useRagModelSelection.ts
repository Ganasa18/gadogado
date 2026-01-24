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

  const localConfig = buildConfig({ maxTokens: 1024, temperature: 0.7 });
  const modelsQuery = useModelsQuery(localConfig, isLocalProvider);

  useEffect(() => {
    if (!isLocalProvider) return;
    if (!modelsQuery.data) return;

    setLocalModels(modelsQuery.data);
    if (modelsQuery.data.length > 0 && !modelsQuery.data.includes(model)) {
      setModel(modelsQuery.data[0]);
    }
  }, [isLocalProvider, modelsQuery.data, setLocalModels, setModel, model]);

  useEffect(() => {
    if (isLocalProvider) {
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
  }, [isLocalProvider, localModels, model, provider, setModel, modelsQuery.data]);

  return {
    provider,
    model,
    localModels,
    setModel,
    setLocalModels,
    buildConfig,
    isLocalProvider,
    modelsQuery,
  };
}
