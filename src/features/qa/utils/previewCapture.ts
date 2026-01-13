import { convertFileSrc } from "@tauri-apps/api/core";
import type { ScreenshotResult } from "../../../types/qa/types";

export function resolveScreenshotSrc(
  payload: ScreenshotResult | string,
  isTauriApp: boolean
) {
  if (typeof payload === "string") {
    return payload.startsWith("data:")
      ? payload
      : isTauriApp
      ? convertFileSrc(payload)
      : payload;
  }
  if (payload.dataUrl) return payload.dataUrl;
  if (payload.path) {
    return isTauriApp ? convertFileSrc(payload.path) : payload.path;
  }
  return null;
}

/**
 * Generate a fallback placeholder image when capture fails due to cross-origin restrictions.
 */
export function generateFallbackDataUrl(
  width: number,
  height: number,
  message = "Preview capture unavailable"
): string {
  const safeWidth = Math.max(100, Math.floor(width));
  const safeHeight = Math.max(100, Math.floor(height));
  const canvas = document.createElement("canvas");
  canvas.width = safeWidth;
  canvas.height = safeHeight;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    // Return a minimal 1x1 transparent PNG
    return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
  }

  // Draw a styled placeholder
  ctx.fillStyle = "#1e1e2e";
  ctx.fillRect(0, 0, safeWidth, safeHeight);

  // Add diagonal stripes pattern
  ctx.strokeStyle = "#2a2a3e";
  ctx.lineWidth = 2;
  for (let i = -safeHeight; i < safeWidth; i += 20) {
    ctx.beginPath();
    ctx.moveTo(i, 0);
    ctx.lineTo(i + safeHeight, safeHeight);
    ctx.stroke();
  }

  // Add centered text
  ctx.fillStyle = "#6c7086";
  ctx.font = "14px -apple-system, BlinkMacSystemFont, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(message, safeWidth / 2, safeHeight / 2 - 10);

  // Add smaller subtext
  ctx.font = "11px -apple-system, BlinkMacSystemFont, sans-serif";
  ctx.fillStyle = "#585b70";
  ctx.fillText("Cross-origin content blocked", safeWidth / 2, safeHeight / 2 + 10);

  return canvas.toDataURL("image/png");
}

export async function captureFrameDataUrl(
  frame: HTMLIFrameElement
): Promise<string> {
  const { width, height } = frame.getBoundingClientRect();
  const safeWidth = Math.max(1, Math.floor(width));
  const safeHeight = Math.max(1, Math.floor(height));

  const doc = frame.contentDocument;
  if (!doc) {
    try {
      return await requestIframeCapture(frame);
    } catch (err) {
      console.warn("[QA Capture] Iframe capture failed, using fallback:", err);
      return generateFallbackDataUrl(safeWidth, safeHeight);
    }
  }

  try {
    return await renderDocumentToDataUrl(
      doc.documentElement,
      safeWidth,
      safeHeight
    );
  } catch (err) {
    if (isTaintedCanvasError(err)) {
      try {
        const sanitized = sanitizeDocumentElement(doc.documentElement);
        return await renderDocumentToDataUrl(sanitized, safeWidth, safeHeight);
      } catch (sanitizeErr) {
        console.warn("[QA Capture] Sanitized capture also failed, using fallback:", sanitizeErr);
        return generateFallbackDataUrl(safeWidth, safeHeight);
      }
    }
    console.warn("[QA Capture] Capture failed with unexpected error, using fallback:", err);
    return generateFallbackDataUrl(safeWidth, safeHeight);
  }
}

function requestIframeCapture(frame: HTMLIFrameElement) {
  return new Promise<string>((resolve, reject) => {
    const requestId = `capture-${Date.now()}-${Math.random()
      .toString(36)
      .slice(2, 8)}`;
    const timeoutId = window.setTimeout(() => {
      window.removeEventListener("message", handleMessage);
      reject(new Error("Preview capture timed out."));
    }, 8000);

    const handleMessage = (event: MessageEvent) => {
      if (event.source !== frame.contentWindow) return;
      if (!event.data || event.data.requestId !== requestId) return;
      if (event.data.type === "qa-recorder-capture") {
        window.clearTimeout(timeoutId);
        window.removeEventListener("message", handleMessage);
        if (typeof event.data.dataUrl !== "string") {
          reject(new Error("Preview capture returned invalid data."));
          return;
        }
        resolve(event.data.dataUrl);
        return;
      }
      if (event.data.type === "qa-recorder-capture-error") {
        window.clearTimeout(timeoutId);
        window.removeEventListener("message", handleMessage);
        reject(
          new Error(event.data.error || "Preview capture failed in iframe.")
        );
      }
    };

    window.addEventListener("message", handleMessage);
    frame.contentWindow?.postMessage(
      { type: "qa-recorder-command", action: "capture", requestId },
      "*"
    );
  });
}

function isTaintedCanvasError(err: unknown): boolean {
  if (!(err instanceof Error)) return false;
  return (
    err.message.includes("Tainted canvases") ||
    err.message.includes("SecurityError")
  );
}

function sanitizeDocumentElement(root: HTMLElement): HTMLElement {
  const clone = root.cloneNode(true) as HTMLElement;
  
  // Strip elements that commonly cause cross-origin issues
  const stripSelectors = [
    "img",
    "picture",
    "source",
    "video",
    "audio",
    "canvas",
    "iframe",
    "svg",
    "object",
    "embed",
    "link[rel=\"stylesheet\"]",
    "link[rel=\"icon\"]",
    "link[rel=\"preload\"]",
    "script",
  ];
  clone.querySelectorAll(stripSelectors.join(",")).forEach((el) => el.remove());

  // Strip font-face and all url() references from style elements
  clone.querySelectorAll("style").forEach((style) => {
    if (!style.textContent) return;
    let text = style.textContent;
    text = text.replace(/@font-face\s*\{[\s\S]*?\}/gi, "");
    text = text.replace(/@import\s+[^;]+;/gi, "");
    text = text.replace(/url\s*\([^)]*\)/gi, "none");
    style.textContent = text;
  });

  // Strip url() references from inline styles
  clone.querySelectorAll("[style]").forEach((el) => {
    const inline = el.getAttribute("style");
    if (!inline) return;
    const cleaned = inline.replace(/url\s*\([^)]*\)/gi, "none");
    el.setAttribute("style", cleaned);
  });

  // Strip src/srcset attributes that might point to external resources
  clone.querySelectorAll("[src], [srcset]").forEach((el) => {
    el.removeAttribute("src");
    el.removeAttribute("srcset");
  });

  // Strip background attributes (old HTML)
  clone.querySelectorAll("[background]").forEach((el) => {
    el.removeAttribute("background");
  });

  return clone;
}

async function renderDocumentToDataUrl(
  root: HTMLElement,
  width: number,
  height: number
): Promise<string> {
  const safeWidth = Math.max(1, Math.floor(width));
  const safeHeight = Math.max(1, Math.floor(height));
  const serialized = new XMLSerializer().serializeToString(root);
  const wrapped = `<div xmlns="http://www.w3.org/1999/xhtml">${serialized}</div>`;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${safeWidth}" height="${safeHeight}"><foreignObject width="100%" height="100%">${wrapped}</foreignObject></svg>`;
  const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(blob);

  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () =>
        reject(new Error("Failed to render preview snapshot."));
      image.src = url;
    });

    const canvas = document.createElement("canvas");
    canvas.width = safeWidth;
    canvas.height = safeHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas is not available for screenshot.");
    }
    ctx.drawImage(img, 0, 0, safeWidth, safeHeight);
    return canvas.toDataURL("image/png");
  } finally {
    URL.revokeObjectURL(url);
  }
}
