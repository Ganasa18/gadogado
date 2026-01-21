// =============================================================================
// Copy to Clipboard Hook
// Manages clipboard functionality with feedback
// =============================================================================

import { useCallback, useState } from "react";

export interface UseCopyToClipboardReturn {
  lastCopied: string | null;
  copyToClipboard: (label: string, value: string) => void;
}

export interface UseCopyToClipboardProps {
  feedbackDuration?: number;
}

/**
 * Hook for copying text to clipboard with temporary feedback
 */
export function useCopyToClipboard({
  feedbackDuration = 1500,
}: UseCopyToClipboardProps = {}): UseCopyToClipboardReturn {
  const [lastCopied, setLastCopied] = useState<string | null>(null);

  const copyToClipboard = useCallback((label: string, value: string) => {
    if (!value) return;
    navigator.clipboard.writeText(value);
    setLastCopied(label);
    window.setTimeout(() => setLastCopied(null), feedbackDuration);
  }, [feedbackDuration]);

  return {
    lastCopied,
    copyToClipboard,
  };
}
