import { MAX_TEXT_LENGTH } from "./constants";
import type { QaEventPayload } from "./types";

export function buildSelector(element: Element): string | undefined {
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

export function getElementText(element: Element): string | undefined {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    return normalizeText(
      element.getAttribute("aria-label") || element.placeholder || element.name,
    );
  }

  if (element instanceof HTMLSelectElement) {
    const selected = element.selectedOptions?.[0]?.textContent;
    return normalizeText(selected || element.getAttribute("aria-label") || element.name);
  }

  return normalizeText(element.textContent);
}

export function getElementValue(element: Element): string | undefined {
  if (element instanceof HTMLInputElement) return element.value;
  if (element instanceof HTMLTextAreaElement) return element.value;
  if (element instanceof HTMLSelectElement) return element.value;
  if (element instanceof HTMLElement && element.isContentEditable) {
    return element.innerText;
  }
  return undefined;
}

export function maskValue(element: Element, value?: string): string | undefined {
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

export function stringifyMeta(meta: Record<string, unknown>): string | undefined {
  const cleaned: Record<string, unknown> = {};
  Object.entries(meta).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") return;
    cleaned[key] = value;
  });
  return Object.keys(cleaned).length > 0 ? JSON.stringify(cleaned) : undefined;
}

export function isEditableTargetForManualClick(target: Element): boolean {
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

export function shouldSkipManualClickFromPayload(payload: QaEventPayload): boolean {
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

function escapeSelector(value: string): string {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(value);
  }
  return value.replace(/["\\]/g, "\\$&");
}
