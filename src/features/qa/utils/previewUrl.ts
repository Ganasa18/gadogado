export function getProxiedPreviewUrl(
  previewUrl: string | null,
  previewUrlValid: boolean
): string | null {
  if (!previewUrl || !previewUrlValid) return null;

  try {
    const url = new URL(previewUrl);
    const currentOrigin = window.location.origin;
    const previewOrigin = `${url.protocol}//${url.host}`;

    if (previewOrigin === currentOrigin) {
      console.log(`[QA Session] Same origin, using direct URL: ${previewUrl}`);
      return previewUrl;
    }

    console.log(
      `[QA Session] Cross-origin detected (${previewOrigin} !== ${currentOrigin}), using proxy`
    );
    return `http://localhost:3001/api/qa/proxy?url=${encodeURIComponent(
      previewUrl
    )}`;
  } catch {
    return previewUrl;
  }
}
