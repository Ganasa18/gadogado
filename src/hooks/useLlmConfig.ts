import { useCallback } from "react";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import {
  createLlmConfig,
  type LlmConfigOverrides,
} from "../shared/api/llmConfig";

export function useLlmConfigBuilder() {
  const { provider, model, baseUrl, getApiKey } = useSettingsStore(
    useShallow((state) => ({
      provider: state.provider,
      model: state.model,
      baseUrl: state.baseUrl,
      getApiKey: state.getApiKey,
    }))
  );

  return useCallback(
    (overrides?: LlmConfigOverrides) => {
      const apiKey = getApiKey(provider);
      return createLlmConfig({ provider, model, apiKey, baseUrl }, overrides);
    },
    [provider, model, baseUrl, getApiKey]
  );
}
