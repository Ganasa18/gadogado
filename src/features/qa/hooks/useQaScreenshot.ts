import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { RefObject } from "react";
import { resolveScreenshotSrc } from "../utils/previewCapture";
import { ScreenshotResult } from "../../../types/qa/types";
import type { CaptureMode } from "../../../store/qaSession";

type UseQaScreenshotOptions = {
  sessionId: string;
  isTauriApp: boolean;
  previewFrameRef: RefObject<HTMLIFrameElement | null>;
  captureMode: CaptureMode;
};

/**
 * Get the screen coordinates of an element (iframe).
 * This accounts for:
 * - Window position on screen
 * - DPR (device pixel ratio) scaling
 * - Scroll position
 */
function getFrameScreenCoordinates(frame: HTMLIFrameElement): {
  x: number;
  y: number;
  width: number;
  height: number;
} {
  const rect = frame.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;

  // Get window position on screen
  const screenX = window.screenX || window.screenLeft || 0;
  const screenY = window.screenY || window.screenTop || 0;

  // Account for browser chrome (address bar, tabs, etc.)
  // outerHeight - innerHeight gives approximate chrome height
  const chromeHeight = window.outerHeight - window.innerHeight;
  const chromeWidth = window.outerWidth - window.innerWidth;

  // Calculate screen position
  // Note: rect is in CSS pixels, we need to convert to screen pixels
  const x = Math.round((screenX + chromeWidth / 2 + rect.left) * dpr);
  const y = Math.round((screenY + chromeHeight + rect.top) * dpr);
  const width = Math.round(rect.width * dpr);
  const height = Math.round(rect.height * dpr);

  return { x, y, width, height };
}

async function getWindowScreenCoordinates(): Promise<{
  x: number;
  y: number;
  width: number;
  height: number;
} | null> {
  try {
    const appWindow = getCurrentWindow();
    const [position, size] = await Promise.all([
      appWindow.outerPosition(),
      appWindow.outerSize(),
    ]);
    return {
      x: position.x,
      y: position.y,
      width: size.width,
      height: size.height,
    };
  } catch {
    return null;
  }
}

export default function useQaScreenshot({
  sessionId,
  isTauriApp,
  previewFrameRef,
  captureMode,
}: UseQaScreenshotOptions) {
  const [screenshotLoading, setScreenshotLoading] = useState(false);
  const [screenshotError, setScreenshotError] = useState<string | null>(null);
  const [screenshotSrc, setScreenshotSrc] = useState<string | null>(null);
  const captureInFlightRef = useRef(false);

  const captureScreenshot = async (silent = false, eventId?: string | null) => {
    if (!sessionId) return;
    if (!isTauriApp) {
      if (!silent) {
        setScreenshotError("Screenshot capture requires the Tauri app.");
      }
      return;
    }

    const frame = previewFrameRef.current;
    if (captureMode === "windowed_frame" && !frame) {
      if (!silent) {
        setScreenshotError("Preview frame is not available.");
      }
      return;
    }

    if (captureInFlightRef.current) {
      return;
    }
    captureInFlightRef.current = true;

    setScreenshotLoading(true);
    if (!silent) {
      setScreenshotError(null);
    }

    try {
      let coords: { x: number; y: number; width: number; height: number };
      if (captureMode === "full_screen") {
        coords = { x: 0, y: 0, width: 1, height: 1 };
      } else {
        const windowCoords = await getWindowScreenCoordinates();
        if (windowCoords) {
          coords = windowCoords;
        } else if (frame) {
          coords = getFrameScreenCoordinates(frame);
        } else {
          throw new Error("Preview frame is not available.");
        }
      }

      console.log("[QA Screenshot] Capturing native screenshot:", {
        ...coords,
        captureMode,
      });

      // Use native screenshot capture
      const result = await invoke<ScreenshotResult | string>(
        "qa_capture_native_screenshot",
        {
          sessionId,
          x: coords.x,
          y: coords.y,
          width: coords.width,
          height: coords.height,
          eventId: eventId ?? undefined,
          captureMode,
        }
      );

      const nextSrc = resolveScreenshotSrc(result, isTauriApp);
      if (!nextSrc) {
        throw new Error("Screenshot payload missing.");
      }
      setScreenshotSrc(nextSrc);
      setScreenshotError(null);
      console.log("[QA Screenshot] Native screenshot captured successfully");
    } catch (err) {
      console.error("[QA Screenshot] Native capture failed:", err);
      if (!silent) {
        const message =
          err instanceof Error && err.message
            ? err.message
            : "Failed to capture screenshot.";
        setScreenshotError(message);
      }
    } finally {
      captureInFlightRef.current = false;
      setScreenshotLoading(false);
    }
  };

  useEffect(() => {
    setScreenshotSrc(null);
    setScreenshotError(null);
  }, [sessionId]);

  return {
    screenshotLoading,
    screenshotError,
    screenshotSrc,
    captureScreenshot,
  };
}
