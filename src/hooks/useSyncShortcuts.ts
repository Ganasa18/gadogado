import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import { isTauri } from "../utils/tauri";

export function useSyncShortcuts() {
  const { shortcutsEnabled, translate, enhance, popup, terminal } =
    useSettingsStore(
      useShallow((state) => ({
        shortcutsEnabled: state.shortcutsEnabled,
        translate: state.shortcuts.translate,
        enhance: state.shortcuts.enhance,
        popup: state.shortcuts.popup,
        terminal: state.shortcuts.terminal,
      }))
    );

  useEffect(() => {
    if (!isTauri()) return;
    invoke("sync_shortcuts", {
      enabled: shortcutsEnabled,
      translate,
      enhance,
      popup,
      terminal,
    }).catch(console.error);
  }, [shortcutsEnabled, translate, enhance, popup, terminal]);
}
