export type QualityLevel = "good" | "ok" | "bad" | "unknown";

export function qualityLevel(score: number | null | undefined): QualityLevel {
  if (score === null || score === undefined) return "unknown";
  if (score >= 0.75) return "good";
  if (score >= 0.5) return "ok";
  return "bad";
}

export function qualityEmoji(score: number | null | undefined): string {
  const level = qualityLevel(score);
  if (level === "good") return "🟢";
  if (level === "ok") return "🟡";
  if (level === "bad") return "🔴";
  return "—";
}

export function fmtPct(score: number | null | undefined): string {
  if (score === null || score === undefined) return "—";
  return `${(score * 100).toFixed(0)}%`;
}

export function fmtMaybeInt(v: number | null | undefined): string {
  if (v === null || v === undefined) return "—";
  return `${v}`;
}

export function fmtDateTime(dt: string): string {
  const d = new Date(dt);
  if (Number.isNaN(d.getTime())) return dt;
  return d.toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function shortHash(hash: string): string {
  if (!hash) return "—";
  return hash.length <= 10 ? hash : `${hash.slice(0, 10)}…`;
}

export function gapLabel(gapType: string | null | undefined): string {
  if (!gapType) return "unknown";
  return gapType.replace(/_/g, " ");
}

export function gapSuggestion(gapType: string | null | undefined): string {
  switch (gapType) {
    case "no_results":
      return "No matching chunks retrieved. Likely missing coverage or wrong collection.";
    case "low_confidence":
      return "Retrieved chunks scored low. Improve OCR or chunk coherence.";
    case "partial_match":
      return "Some matches, but weak coverage. Add docs or adjust chunking settings.";
    default:
      return "Investigate query coverage and chunk quality.";
  }
}

export function sum(xs: number[]): number {
  return xs.reduce((a, b) => a + b, 0);
}
