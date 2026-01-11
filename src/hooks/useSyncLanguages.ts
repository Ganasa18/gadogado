import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import { isTauri } from "../utils/tauri";

export function useSyncLanguages() {
  const { sourceLang, targetLang } = useSettingsStore(
    useShallow((state) => ({
      sourceLang: state.sourceLang,
      targetLang: state.targetLang,
    }))
  );

  useEffect(() => {
    if (!isTauri()) return;
    invoke("sync_languages", {
      source: sourceLang,
      target: targetLang,
    }).catch(console.error);
  }, [sourceLang, targetLang]);
}
