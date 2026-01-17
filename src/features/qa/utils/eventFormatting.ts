import type { QaEvent, QaSession } from "../../../types/qa/types";

function pickFirstString(values: Array<string | null | undefined>) {
  return values.find(
    (value) => typeof value === "string" && value.trim().length > 0
  );
}

export function extractPreviewUrl(session: QaSession | null): string | null {
  if (!session) return null;
  if (session.target_url) return session.target_url;
  if (!session.notes) return null;
  const trimmed = session.notes.trim();
  if (!trimmed) return null;
  try {
    const parsed = JSON.parse(trimmed);
    if (parsed && typeof parsed === "object") {
      const url =
        typeof (parsed as { preview_url?: unknown }).preview_url === "string"
          ? (parsed as { preview_url: string }).preview_url
          : typeof (parsed as { target_url?: unknown }).target_url === "string"
          ? (parsed as { target_url: string }).target_url
          : null;
      return url ?? null;
    }
  } catch {
    return trimmed;
  }
  return null;
}

export function isValidUrl(value: string) {
  try {
    const url = new URL(value);
    return Boolean(url.protocol && url.hostname);
  } catch {
    return false;
  }
}

export function formatTimestamp(timestamp: number | string) {
  const date =
    typeof timestamp === "string" && /^\d+$/.test(timestamp)
      ? new Date(Number(timestamp))
      : new Date(timestamp);
  return date.toLocaleString();
}

export function formatEventSeq(seq: number) {
  return seq.toString().padStart(3, "0");
}

export function formatEventTime(timestamp: number) {
  const date = new Date(timestamp);
  const time = date.toLocaleTimeString([], {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  const ms = String(date.getMilliseconds()).padStart(3, "0");
  return `${time}.${ms}`;
}

function parseEventMeta(event: QaEvent): Record<string, unknown> | null {
  if (!event.meta_json) return null;
  try {
    const parsed = JSON.parse(event.meta_json);
    return parsed && typeof parsed === "object"
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

export function getEventDetails(event: QaEvent) {
  const normalized = event.event_type.toLowerCase();
  if (normalized.includes("curl") || normalized.includes("api")) {
    const meta = parseEventMeta(event);
    const method =
      typeof meta?.method === "string" ? meta.method.toUpperCase() : "REQUEST";
    const rawStatus =
      typeof meta?.status === "number"
        ? meta.status
        : typeof meta?.status_code === "number"
        ? meta.status_code
        : typeof meta?.status === "string"
        ? Number(meta.status)
        : typeof meta?.status_code === "string"
        ? Number(meta.status_code)
        : null;
    const status = typeof rawStatus === "number" && Number.isFinite(rawStatus)
      ? rawStatus
      : null;
    const url =
      typeof meta?.url === "string"
        ? meta.url
        : pickFirstString([event.url, event.selector, event.element_text]) || "n/a";
    const primary = `${method} ${url}`;
    const secondary = status ? `status ${status}` : "status n/a";
    return { primary, secondary };
  }

  const primary =
    pickFirstString([event.selector, event.element_text, event.url]) || "n/a";
  const secondary =
    pickFirstString(
      [event.value, event.url, event.element_text, event.selector].filter(
        (value) => value !== primary
      )
    ) || "n/a";
  return { primary, secondary };
}

export function getEventBadgeClasses(eventType: string) {
  const normalized = eventType.toLowerCase();
  if (normalized.includes("navigate") || normalized.includes("route")) {
    return "bg-sky-500/10 text-sky-300 border-sky-500/20";
  }
  if (normalized.includes("input") || normalized.includes("change")) {
    return "bg-purple-500/10 text-purple-300 border-purple-500/20";
  }
  if (normalized.includes("click")) {
    return "bg-emerald-500/10 text-emerald-300 border-emerald-500/20";
  }
  if (normalized.includes("submit")) {
    return "bg-amber-500/10 text-amber-300 border-amber-500/20";
  }
  if (normalized.includes("curl") || normalized.includes("api")) {
    return "bg-blue-500/10 text-blue-300 border-blue-500/20";
  }
  return "bg-slate-500/10 text-slate-300 border-slate-500/20";
}

export function formatEventMetadata(event: QaEvent) {
  let meta: unknown = null;
  if (event.meta_json) {
    try {
      meta = JSON.parse(event.meta_json);
    } catch {
      meta = event.meta_json;
    }
  }
  const payload = {
    event_type: event.event_type,
    selector: event.selector ?? undefined,
    element_text: event.element_text ?? undefined,
    value: event.value ?? undefined,
    url: event.url ?? undefined,
    screenshot_id: event.screenshot_id ?? undefined,
    timestamp: event.ts,
    meta,
  };
  return JSON.stringify(payload, null, 2);
}
