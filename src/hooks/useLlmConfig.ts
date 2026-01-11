import { useCallback } from "react";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import {
  createLlmConfig,
  type LlmConfigOverrides,
} from "../shared/api/llmConfig";

export function useLlmConfigBuilder() {
  const { provider, model, apiKey, baseUrl } = useSettingsStore(
    useShallow((state) => ({
      provider: state.provider,
      model: state.model,
      apiKey: state.apiKey,
      baseUrl: state.baseUrl,
    }))
  );

  return useCallback(
    (overrides?: LlmConfigOverrides) =>
      createLlmConfig({ provider, model, apiKey, baseUrl }, overrides),
    [provider, model, apiKey, baseUrl]
  );
}
