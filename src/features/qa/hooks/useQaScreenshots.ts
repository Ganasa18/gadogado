import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { QaEvent } from "../../../types/qa/types";
import { resolveScreenshotSrc } from "../utils/previewCapture";
import { isTauri } from "../../../utils/tauri";

export type ScreenshotItem = {
  id: string;
  src: string;
  ts: number;
  eventType?: string;
  nodeName?: string;
};

export default function useQaScreenshots(sessionId: string) {
  const [screenshots, setScreenshots] = useState<ScreenshotItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadScreenshots = useCallback(async () => {
    console.log("[useQaScreenshots] loadScreenshots called, sessionId:", sessionId);
    if (!sessionId) {
      console.log("[useQaScreenshots] Early return: sessionId is empty");
      return;
    }
    setLoading(true);
    setError(null);
    console.log("[useQaScreenshots] Calling qa_list_screenshots with sessionId:", sessionId);
    try {
      const events = await invoke<QaEvent[]>("qa_list_screenshots", { sessionId });
      const isTauriApp = isTauri();
      console.log("[useQaScreenshots] Received events count:", events.length);
      
      const items: ScreenshotItem[] = [];
      for (const e of events) {
          if (!e.screenshot_path) {
            console.log("[useQaScreenshots] Event missing screenshot_path:", e.id);
            continue;
          }
          const src = resolveScreenshotSrc(e.screenshot_path, isTauriApp);
          console.log("[useQaScreenshots] Event screenshot processed:", e.id, "path:", e.screenshot_path, "src:", src);
          if (src) {
              items.push({
                  id: e.id,
                  src,
                  ts: e.ts,
                  eventType: e.event_type,
                  nodeName: (e.element_text || e.selector) ?? undefined
              });
          }
      }
      console.log("[useQaScreenshots] Final screenshots count:", items.length);
      setScreenshots(items);
    } catch (err) {
      console.error("Failed to load screenshots:", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    loadScreenshots();
  }, [loadScreenshots]);

  return { screenshots, loading, error, reload: loadScreenshots };
}
