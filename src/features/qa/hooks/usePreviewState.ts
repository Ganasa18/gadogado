import { useEffect, useRef, useState } from "react";

type UsePreviewStateOptions = {
  previewUrl: string | null;
  previewUrlValid: boolean;
  timeoutMs: number;
};

export default function usePreviewState({
  previewUrl,
  previewUrlValid,
  timeoutMs,
}: UsePreviewStateOptions) {
  const [previewLoading, setPreviewLoading] = useState(true);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewReloadToken, setPreviewReloadToken] = useState(0);
  const previewFrameRef = useRef<HTMLIFrameElement | null>(null);
  const previewTimeoutRef = useRef<number | null>(null);

  const handlePreviewLoad = () => {
    if (previewTimeoutRef.current) {
      window.clearTimeout(previewTimeoutRef.current);
    }
    setPreviewLoading(false);
    setPreviewError(null);
  };

  const handlePreviewError = () => {
    if (previewTimeoutRef.current) {
      window.clearTimeout(previewTimeoutRef.current);
    }
    setPreviewLoading(false);
    setPreviewError("Preview failed to load.");
  };

  const handleReloadPreview = () => {
    setPreviewReloadToken((value) => value + 1);
  };

  useEffect(() => {
    if (!previewUrlValid) {
      setPreviewLoading(false);
      setPreviewError(null);
      if (previewTimeoutRef.current) {
        window.clearTimeout(previewTimeoutRef.current);
      }
      return;
    }

    setPreviewLoading(true);
    setPreviewError(null);
    if (previewTimeoutRef.current) {
      window.clearTimeout(previewTimeoutRef.current);
    }
    previewTimeoutRef.current = window.setTimeout(() => {
      setPreviewLoading(false);
      setPreviewError("Preview failed to load.");
    }, timeoutMs);

    return () => {
      if (previewTimeoutRef.current) {
        window.clearTimeout(previewTimeoutRef.current);
      }
    };
  }, [previewUrlValid, previewReloadToken, previewUrl, timeoutMs]);

  return {
    previewLoading,
    previewError,
    previewReloadToken,
    previewFrameRef,
    handlePreviewLoad,
    handlePreviewError,
    handleReloadPreview,
  };
}
