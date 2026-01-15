import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import { createEmbeddingConfig } from "../shared/api/llmConfig";
import { isTauri } from "../utils/tauri";

let lastSyncedSignature: string | null = null;

export function useSyncEmbeddingConfig() {
  const { embeddingProvider, embeddingModel } = useSettingsStore(
    useShallow((state) => ({
      embeddingProvider: state.embeddingProvider,
      embeddingModel: state.embeddingModel,
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
    const signature = `${embeddingProvider}|${embeddingModel}`;
    if (signature === lastSyncedSignature) return;
    lastSyncedSignature = signature;
    const config = createEmbeddingConfig({
      provider: embeddingProvider,
      model: embeddingModel,
    });
    invoke("sync_embedding_config", { config }).catch(console.error);
  }, [embeddingProvider, embeddingModel, hydrated]);
}
