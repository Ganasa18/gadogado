import { useEffect, useRef } from "react";
import { useLocation } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import { useQaSessionStore } from "../store/qaSession";
import { useToastStore } from "../store/toast";
import { isTauri } from "../utils/tauri";

const INPUT_DEBOUNCE_MS = 350;
const MAX_TEXT_LENGTH = 160;

interface QaEventPayload {
  eventType: string;
  selector?: string;
  elementText?: string;
  value?: string;
  url?: string;
  metaJson?: string;
  runId?: string;
  origin?: string;
  recordingMode?: string;
}

export function useQaEventRecorder() {
  const location = useLocation();
  const { addToast } = useToastStore();
  const {
    recordingSessionId,
    activeRunId,
    recordingMode,
    recordingDelay,
    isRecordingArmed,
    setIsRecordingArmed,
  } = useQaSessionStore();
  const inputTimersRef = useRef(new Map<Element, number>());
  const lastPointerRef = useRef<{ x: number; y: number } | null>(null);
  const lastRouteRef = useRef<string | null>(null);
  const isArmedRef = useRef(false);
  const recordingModeRef = useRef(recordingMode);
  const recordingDelayRef = useRef(recordingDelay);
  const isRecordingArmedRef = useRef(isRecordingArmed);
  const sessionRef = useRef<string | null>(null);
  const retryTimeoutsRef = useRef<number[]>([]);
  const recordingDelayTimeoutRef = useRef<number | null>(null);

  sessionRef.current = recordingSessionId;

  useEffect(() => {
    recordingModeRef.current = recordingMode;
  }, [recordingMode]);

  useEffect(() => {
    recordingDelayRef.current = recordingDelay;
  }, [recordingDelay]);

  useEffect(() => {
    isRecordingArmedRef.current = isRecordingArmed;
  }, [isRecordingArmed]);

  useEffect(() => {
    if (!isTauri() || !recordingSessionId) {
      return;
    }

    console.log("[QA Recorder] Recording session started:", recordingSessionId);
    isArmedRef.current = false;

    const recordEventWithDelay = (payload: QaEventPayload) => {
      const sessionId = sessionRef.current;
      if (!sessionId) return;

      // Clear any pending delay
      if (recordingDelayTimeoutRef.current) {
        window.clearTimeout(recordingDelayTimeoutRef.current);
      }

      // Apply recording delay
      const delay = recordingDelayRef.current || 0;
      recordingDelayTimeoutRef.current = window.setTimeout(() => {
        console.log("[QA Recorder] Recording event:", payload.eventType, payload);
        invoke("qa_record_event", { event: payload, sessionId }).catch((err) => {
          console.error("[QA Recorder] Failed to record QA event", err);
        });
      }, delay);
    };

    const allowedEventTypes = new Set(["click", "input", "submit"]);

    const recordEvent = (payload: QaEventPayload) => {
      const normalizedType = payload.eventType.toLowerCase();
      if (!allowedEventTypes.has(normalizedType)) {
        return;
      }
      const mode = recordingModeRef.current;
      const armed = isRecordingArmedRef.current;

      // In manual mode, only record if armed
      if (mode === "manual" && !armed) {
        console.log("[QA Recorder] Manual mode: event ignored (not armed)");
        return;
      }

      recordEventWithDelay({
        ...payload,
        eventType: normalizedType,
        runId: activeRunId ?? payload.runId,
        origin: payload.origin ?? "user",
        recordingMode: payload.recordingMode ?? mode,
      });

      // In manual mode, disarm after recording one event
      if (mode === "manual") {
        isRecordingArmedRef.current = false;
        setIsRecordingArmed(false);
        addToast("Event recorded. Click 'Record Next' to capture another.", "success");
      }
    };

    const armRecording = () => {
      if (isArmedRef.current) return;
      isArmedRef.current = true;
      const mode = recordingModeRef.current;
      const delay = recordingDelayRef.current;

      if (mode === "auto") {
        addToast(
          `Recording started (${delay}ms delay). All actions will be recorded automatically.`,
          "info"
        );
      } else {
        addToast(
          "Manual recording mode. Click 'Record Next' to capture each event.",
          "info"
        );
      }
    };

    const shouldIgnoreTarget = (target: Element, root?: Element | null) => {
      if (root && !root.contains(target)) {
        return true;
      }
      return Boolean(target.closest("[data-qa-record-ignore]"));
    };

    const getCoordinates = (event: Event) => {
      if (event instanceof MouseEvent) {
        return { x: event.clientX, y: event.clientY };
      }
      return lastPointerRef.current ?? undefined;
    };

    const handlePointerDown = (event: Event, root?: Element | null) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (shouldIgnoreTarget(target, root)) return;
      const coords = getCoordinates(event);
      if (coords) {
        lastPointerRef.current = coords;
      }
    };

    const resolveUrl = (target: Element) =>
      target.ownerDocument?.defaultView?.location?.href ?? window.location.href;

    const isSyntheticEvent = (event: Event) => event.isTrusted === false;

    const handleClick = (event: Event, root?: Element | null) => {
      if (isSyntheticEvent(event)) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (shouldIgnoreTarget(target, root)) return;
      if (!isArmedRef.current) {
        armRecording();
      }
      if (
        recordingModeRef.current === "manual" &&
        isRecordingArmedRef.current &&
        isEditableTargetForManualClick(target)
      ) {
        return;
      }

      recordEvent({
        eventType: "click",
        selector: buildSelector(target),
        elementText: getElementText(target),
        url: resolveUrl(target),
        metaJson: stringifyMeta({
          tag: target.tagName.toLowerCase(),
          coordinates: getCoordinates(event),
        }),
      });
    };

    const handleInput = (event: Event, root?: Element | null) => {
      if (isSyntheticEvent(event)) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (shouldIgnoreTarget(target, root)) return;
      if (
        !(
          target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement ||
          target instanceof HTMLSelectElement
        )
      ) {
        return;
      }
      if (!isArmedRef.current) {
        armRecording();
      }

      const inputType = (event as InputEvent).inputType;
      const previousTimer = inputTimersRef.current.get(target);
      if (previousTimer) {
        window.clearTimeout(previousTimer);
      }

      const nextTimer = window.setTimeout(() => {
        inputTimersRef.current.delete(target);
        const rawValue = getElementValue(target);
        const maskedValue = maskValue(target, rawValue);

        recordEvent({
          eventType: "input",
          selector: buildSelector(target),
          elementText: getElementText(target),
          value: maskedValue,
          url: resolveUrl(target),
          metaJson: stringifyMeta({
            tag: target.tagName.toLowerCase(),
            inputType,
            type: target instanceof HTMLInputElement ? target.type : undefined,
            coordinates: getCoordinates(event),
          }),
        });
      }, INPUT_DEBOUNCE_MS);

      inputTimersRef.current.set(target, nextTimer);
    };

    const handleSubmit = (event: Event, root?: Element | null) => {
      if (isSyntheticEvent(event)) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (shouldIgnoreTarget(target, root)) return;
      if (!isArmedRef.current) {
        armRecording();
      }

      const form = target instanceof HTMLFormElement ? target : target.closest("form");
      const element = form ?? target;

      recordEvent({
        eventType: "submit",
        selector: buildSelector(element),
        elementText: getElementText(element),
        url: resolveUrl(element),
        metaJson: stringifyMeta({
          tag: element.tagName.toLowerCase(),
          action: form?.action,
          method: form?.method,
          coordinates: getCoordinates(event),
        }),
      });
    };

    const attachListeners = (doc: Document, root?: Element | null) => {
      const onPointerDown = (event: Event) => handlePointerDown(event, root);
      const onClick = (event: Event) => handleClick(event, root);
      const onInput = (event: Event) => handleInput(event, root);
      const onSubmit = (event: Event) => handleSubmit(event, root);
      doc.addEventListener("pointerdown", onPointerDown, true);
      doc.addEventListener("click", onClick, true);
      doc.addEventListener("input", onInput, true);
      doc.addEventListener("submit", onSubmit, true);
      return () => {
        doc.removeEventListener("pointerdown", onPointerDown, true);
        doc.removeEventListener("click", onClick, true);
        doc.removeEventListener("input", onInput, true);
        doc.removeEventListener("submit", onSubmit, true);
      };
    };

    const MAX_FRAME_ATTACH_ATTEMPTS = 5;
    const FRAME_RETRY_DELAYS = [0, 100, 300, 500, 1000]; // Progressive delays

    const root = document.querySelector("[data-qa-record-root]");
    const cleanupMain = attachListeners(document, root);
    let cleanupFrame: (() => void) | null = null;

    const attachFrameListenersWithRetry = (
      frame: HTMLIFrameElement | null,
      attemptIndex = 0
    ) => {
      if (!frame) {
        console.warn("[QA Recorder] No frame element found");
        return;
      }
      if (attemptIndex >= MAX_FRAME_ATTACH_ATTEMPTS) {
        console.warn("[QA Recorder] Max retry attempts reached, giving up");
        return;
      }

      console.log(`[QA Recorder] Attempt ${attemptIndex + 1}/${MAX_FRAME_ATTACH_ATTEMPTS} to attach iframe listeners`);

      try {
        const frameDoc = frame.contentDocument;
        if (!frameDoc) {
          console.warn(`[QA Recorder] contentDocument is null on attempt ${attemptIndex + 1}, will retry...`);
          // Retry with delay
          const delay = FRAME_RETRY_DELAYS[attemptIndex] || 1000;
          const timeoutId = window.setTimeout(() => {
            attachFrameListenersWithRetry(frame, attemptIndex + 1);
          }, delay);
          retryTimeoutsRef.current.push(timeoutId);
          return;
        }

        // Success - clean up previous listeners
        if (cleanupFrame) {
          cleanupFrame();
        }
        cleanupFrame = attachListeners(frameDoc, null);
        console.log("[QA Recorder] ✓ Iframe listeners attached successfully");
      } catch (err) {
        console.warn(`[QA Recorder] Error on attempt ${attemptIndex + 1}:`, err);
        // Retry for same-origin access errors
        if (attemptIndex < MAX_FRAME_ATTACH_ATTEMPTS - 1) {
          const delay = FRAME_RETRY_DELAYS[attemptIndex] || 1000;
          const timeoutId = window.setTimeout(() => {
            attachFrameListenersWithRetry(frame, attemptIndex + 1);
          }, delay);
          retryTimeoutsRef.current.push(timeoutId);
        }
      }
    };

    const frame = document.querySelector<HTMLIFrameElement>(
      "[data-qa-preview-frame]"
    );

    if (frame) {
      console.log("[QA Recorder] Frame found, src:", frame.src);
      try {
        const frameOrigin = new URL(frame.src || "about:blank").origin;
        const parentOrigin = window.location.origin;
        console.log("[QA Recorder] Frame origin:", frameOrigin);
        console.log("[QA Recorder] Parent origin:", parentOrigin);

        if (frameOrigin !== parentOrigin && frameOrigin !== "null") {
          console.warn(
            "[QA Recorder] Cross-origin iframe detected, will use postMessage instead"
          );
        }
      } catch (err) {
        console.warn("[QA Recorder] Could not parse frame origin:", err);
      }
    } else {
      console.warn("[QA Recorder] Frame element not found in DOM");
    }

    // Listen for postMessage events from iframe
    const handleIframeMessage = (event: MessageEvent) => {
      if (event.data?.type === "qa-recorder-ready") {
        console.log("[QA Recorder] Iframe recorder script ready");
        return;
      }

      if (event.data?.type === "qa-recorder-event") {
        const payload = event.data.payload as QaEventPayload;
        console.log(
          "[QA Recorder] Received event from iframe:",
          payload.eventType
        );
        if (
          recordingModeRef.current === "manual" &&
          isRecordingArmedRef.current &&
          shouldSkipManualClickFromPayload(payload)
        ) {
          return;
        }
        recordEvent(payload);
      }
    };

    window.addEventListener("message", handleIframeMessage);

    // Try to inject recorder script into iframe
    const injectRecorderScript = (targetFrame: HTMLIFrameElement) => {
      try {
        const frameDoc = targetFrame.contentDocument;
        if (frameDoc) {
          // Same-origin: inject directly
          console.log("[QA Recorder] Same-origin iframe, injecting script...");
          const script = frameDoc.createElement("script");
          script.src = "/qa-recorder-inject.js";
          frameDoc.head.appendChild(script);
          console.log("[QA Recorder] Script injected into same-origin iframe");
        } else {
          // Cross-origin: cannot inject directly
          console.warn(
            "[QA Recorder] Cross-origin iframe, cannot inject script directly"
          );
          console.warn(
            "[QA Recorder] User must manually add the recorder script to the target page"
          );
        }
      } catch (err) {
        console.warn("[QA Recorder] Could not inject script:", err);
      }
    };

    if (frame) {
      injectRecorderScript(frame);
    }

    // Initial attempt with retry logic (for same-origin fallback)
    attachFrameListenersWithRetry(frame, 0);

    // Re-attach on iframe load (for future reloads)
    const handleFrameLoad = () => attachFrameListenersWithRetry(frame, 0);
    frame?.addEventListener("load", handleFrameLoad);

    // Watch for iframe src changes with MutationObserver
    let observer: MutationObserver | null = null;
    if (frame) {
      observer = new MutationObserver(() => {
        attachFrameListenersWithRetry(frame, 0);
      });
      observer.observe(frame, { attributes: true, attributeFilter: ["src"] });
    }

    return () => {
      isArmedRef.current = false;
      setIsRecordingArmed(false);
      isRecordingArmedRef.current = false;
      cleanupMain();
      if (cleanupFrame) {
        cleanupFrame();
        cleanupFrame = null;
      }
      frame?.removeEventListener("load", handleFrameLoad);
      observer?.disconnect();
      window.removeEventListener("message", handleIframeMessage);

      // Clear recording delay timeout
      if (recordingDelayTimeoutRef.current) {
        window.clearTimeout(recordingDelayTimeoutRef.current);
        recordingDelayTimeoutRef.current = null;
      }

      // Clear any pending retry timeouts
      retryTimeoutsRef.current.forEach((timeoutId) =>
        window.clearTimeout(timeoutId)
      );
      retryTimeoutsRef.current = [];

      inputTimersRef.current.forEach((timer) => window.clearTimeout(timer));
      inputTimersRef.current.clear();
    };
  }, [recordingSessionId, addToast]);

  useEffect(() => {
    if (!isTauri() || !recordingSessionId || !isArmedRef.current) {
      lastRouteRef.current = null;
      return;
    }

    const sessionId = sessionRef.current;
    if (!sessionId) return;

    const routeKey = `${location.pathname}${location.search}${location.hash}`;
    if (lastRouteRef.current === routeKey) return;
    lastRouteRef.current = routeKey;

    invoke("qa_record_event", {
      event: {
        eventType: "navigation",
        url: window.location.href,
        metaJson: stringifyMeta({
          pathname: location.pathname,
          search: location.search,
          hash: location.hash,
        }),
      },
      sessionId,
    }).catch((err) => {
      console.error("Failed to record QA navigation event", err);
    });
  }, [recordingSessionId, location.pathname, location.search, location.hash]);
}

function buildSelector(element: Element): string | undefined {
  const prioritized = [
    "data-testid",
    "data-purpose",
    "id",
    "name",
    "aria-label",
    "role",
  ];

  for (const attr of prioritized) {
    const value = element.getAttribute(attr);
    if (!value) continue;
    if (attr === "id") {
      return `#${escapeSelector(value)}`;
    }
    return `${element.tagName.toLowerCase()}[${attr}="${escapeSelector(value)}"]`;
  }

  const path: string[] = [];
  let current: Element | null = element;

  const rootBody = element.ownerDocument?.body;
  for (let depth = 0; current && current !== rootBody && depth < 4; depth += 1) {
    const tagName = current.tagName.toLowerCase();
    const index = nthOfType(current);
    path.unshift(`${tagName}:nth-of-type(${index})`);
    current = current.parentElement;
  }

  return path.length > 0 ? path.join(" > ") : undefined;
}

function nthOfType(element: Element): number {
  let index = 1;
  let sibling = element.previousElementSibling;
  while (sibling) {
    if (sibling.tagName === element.tagName) {
      index += 1;
    }
    sibling = sibling.previousElementSibling;
  }
  return index;
}

function getElementText(element: Element): string | undefined {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    return normalizeText(
      element.getAttribute("aria-label") || element.placeholder || element.name
    );
  }

  if (element instanceof HTMLSelectElement) {
    const selected = element.selectedOptions?.[0]?.textContent;
    return normalizeText(
      selected || element.getAttribute("aria-label") || element.name
    );
  }

  return normalizeText(element.textContent);
}

function getElementValue(element: Element): string | undefined {
  if (element instanceof HTMLInputElement) return element.value;
  if (element instanceof HTMLTextAreaElement) return element.value;
  if (element instanceof HTMLSelectElement) return element.value;
  if (element instanceof HTMLElement && element.isContentEditable) {
    return element.innerText;
  }
  return undefined;
}

function maskValue(element: Element, value?: string): string | undefined {
  if (!value || value.trim().length === 0) return undefined;
  if (element instanceof HTMLInputElement && element.type === "password") {
    return "[masked]";
  }
  const label = [
    element.getAttribute("name"),
    element.getAttribute("id"),
    element.getAttribute("aria-label"),
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  if (label.includes("password")) {
    return "[masked]";
  }
  return value;
}

function normalizeText(value?: string | null): string | undefined {
  if (!value) return undefined;
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  return trimmed.length > MAX_TEXT_LENGTH
    ? trimmed.slice(0, MAX_TEXT_LENGTH)
    : trimmed;
}

function isEditableTargetForManualClick(target: Element): boolean {
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
    return true;
  }
  if (target instanceof HTMLInputElement) {
    const inputType = target.type.toLowerCase();
    return !["button", "submit", "reset", "image"].includes(inputType);
  }
  if (target instanceof HTMLElement && target.isContentEditable) {
    return true;
  }
  return false;
}

function shouldSkipManualClickFromPayload(payload: QaEventPayload): boolean {
  if (payload.eventType !== "click" || !payload.metaJson) return false;
  try {
    const meta = JSON.parse(payload.metaJson) as {
      tag?: string;
      type?: string;
      isEditable?: boolean;
    };
    if (meta.isEditable) {
      if (meta.tag === "input") {
        const inputType = (meta.type || "").toLowerCase();
        return !["button", "submit", "reset", "image"].includes(inputType);
      }
      return true;
    }
    if (meta.tag === "input") {
      const inputType = (meta.type || "").toLowerCase();
      return !["button", "submit", "reset", "image"].includes(inputType);
    }
    return meta.tag === "textarea" || meta.tag === "select";
  } catch {
    return false;
  }
}

function stringifyMeta(meta: Record<string, unknown>): string | undefined {
  const cleaned: Record<string, unknown> = {};
  Object.entries(meta).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") return;
    cleaned[key] = value;
  });
  return Object.keys(cleaned).length > 0 ? JSON.stringify(cleaned) : undefined;
}

function escapeSelector(value: string): string {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(value);
  }
  return value.replace(/["\\]/g, "\\$&");
}
