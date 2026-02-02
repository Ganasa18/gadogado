import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import { createLlmConfig } from "../shared/api/llmConfig";
import { isTauri } from "../utils/tauri";

let lastSyncedSignature: string | null = null;

export function useSyncConfig() {
  const { provider, model, baseUrl, getApiKey } = useSettingsStore(
    useShallow((state) => ({
      provider: state.provider,
      model: state.model,
      baseUrl: state.baseUrl,
      getApiKey: state.getApiKey,
    }))
  );
  const [hydrated, setHydrated] = useState(
    useSettingsStore.persist.hasHydrated()
  );

  useEffect(() => {
    return useSettingsStore.persist.onFinishHydration(() => {
      setHydrated(true);
    });
  }, []);

  useEffect(() => {
    if (!isTauri() || !hydrated) return;
    const apiKey = getApiKey(provider);
    const signature = `${provider}|${model}|${apiKey}|${baseUrl}`;
    if (signature === lastSyncedSignature) return;
    lastSyncedSignature = signature;
    const config = createLlmConfig(
      { provider, model, apiKey, baseUrl },
      { maxTokens: 1024, temperature: 0.7 }
    );
    invoke("sync_config", { config }).catch(console.error);
  }, [provider, model, baseUrl, getApiKey, hydrated]);
}
