import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "../utils/tauri";

interface ShortcutEventsOptions {
  onCapture: (payload: string) => void;
}

export function useShortcutEvents({ onCapture }: ShortcutEventsOptions) {
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlistenStart: (() => void) | undefined;
    let unlistenEnd: (() => void) | undefined;
    let unlistenCapture: (() => void) | undefined;

    listen("shortcut-start", (event) => {
      window.dispatchEvent(
        new CustomEvent("shortcut-start", { detail: event.payload })
      );
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenStart = unlisten;
      })
      .catch(console.error);

    listen("shortcut-end", (event) => {
      window.dispatchEvent(
        new CustomEvent("shortcut-end", { detail: event.payload })
      );
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenEnd = unlisten;
      })
      .catch(console.error);

    listen<string>("shortcut-capture", (event) => {
      onCapture(event.payload);
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenCapture = unlisten;
      })
      .catch(console.error);

    return () => {
      disposed = true;
      unlistenStart?.();
      unlistenEnd?.();
      unlistenCapture?.();
    };
  }, [onCapture]);
}
