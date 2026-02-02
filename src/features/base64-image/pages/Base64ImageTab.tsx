import { useCallback, useEffect, useRef, useState } from "react";
import {
  Image as ImageIcon,
  Upload,
  Download,
  Copy,
  Trash2,
  FileImage,
  CheckCircle2,
  AlertCircle,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Plus,
  X,
  RotateCcw,
} from "lucide-react";
import { Button } from "../../../shared/components/Button";

// ── Types ──────────────────────────────────────────────────

type ImageInfo = {
  width: number;
  height: number;
  size: number;
  mimeType: string;
};

type StoredImage = {
  id: string;
  name: string;
  base64: string;
  mimeType: string;
  width: number;
  height: number;
  size: number;
  createdAt: number;
};

// ── Constants ──────────────────────────────────────────────

const STORAGE_KEY = "base64-image-viewer-images";
const MAX_STORED = 20;

const MIME_SIGNATURES: Record<string, string> = {
  "/9j/": "image/jpeg",
  iVBOR: "image/png",
  R0lGO: "image/gif",
  UklGR: "image/webp",
  AAABA: "image/x-icon",
  PHN2Z: "image/svg+xml",
  Qk0: "image/bmp",
};

// ── Helpers ────────────────────────────────────────────────

function detectMimeFromBase64(b64: string): string {
  for (const [sig, mime] of Object.entries(MIME_SIGNATURES)) {
    if (b64.startsWith(sig)) return mime;
  }
  return "image/png";
}

function parseBase64Input(raw: string): {
  base64: string;
  mimeType: string;
} | null {
  const trimmed = raw.trim().replace(/^["'`]+|["'`]+$/g, "").trim();
  if (!trimmed) return null;

  const dataUriMatch = trimmed.match(
    /^data:(image\/[a-zA-Z0-9.+-]+);base64,(.+)$/s
  );
  if (dataUriMatch) {
    return {
      base64: dataUriMatch[2].replace(/[^A-Za-z0-9+/=]/g, ""),
      mimeType: dataUriMatch[1],
    };
  }

  const cleaned = trimmed.replace(/[^A-Za-z0-9+/=]/g, "");
  if (!cleaned) return null;
  const mimeType = detectMimeFromBase64(cleaned);
  return { base64: cleaned, mimeType };
}

function base64ToBlob(b64: string, mime: string): Blob {
  const byteChars = atob(b64);
  const byteArray = new Uint8Array(byteChars.length);
  for (let i = 0; i < byteChars.length; i++) {
    byteArray[i] = byteChars.charCodeAt(i);
  }
  return new Blob([byteArray], { type: mime });
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function loadStoredImages(): StoredImage[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveStoredImages(images: StoredImage[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(images.slice(0, MAX_STORED)));
  } catch {
    // localStorage full — silently fail
  }
}

function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

// ── Component ──────────────────────────────────────────────

export default function Base64ImageTab() {
  const [storedImages, setStoredImages] = useState<StoredImage[]>(loadStoredImages);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [zoom, setZoom] = useState(100);
  const [isDragOver, setIsDragOver] = useState(false);
  const [showInput, setShowInput] = useState(true);

  const base64Ref = useRef<string>("");
  const mimeRef = useRef<string>("");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const previewContainerRef = useRef<HTMLDivElement>(null);

  // Load selected image into preview
  const loadImagePreview = useCallback((img: StoredImage) => {
    try {
      const blob = base64ToBlob(img.base64, img.mimeType);
      const url = URL.createObjectURL(blob);
      base64Ref.current = img.base64;
      mimeRef.current = img.mimeType;

      setBlobUrl((prev) => {
        if (prev) URL.revokeObjectURL(prev);
        return url;
      });
      setImageInfo({
        width: img.width,
        height: img.height,
        size: img.size,
        mimeType: img.mimeType,
      });
      setSelectedId(img.id);
      setShowInput(false);
      setZoom(100);
      setError(null);
    } catch {
      setError("Failed to load stored image.");
    }
  }, []);

  // Auto-select first image on mount
  useEffect(() => {
    if (storedImages.length > 0 && !selectedId) {
      loadImagePreview(storedImages[0]);
    } else if (storedImages.length === 0) {
      setShowInput(true);
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Cleanup blob URL on unmount
  useEffect(() => {
    return () => {
      if (blobUrl) URL.revokeObjectURL(blobUrl);
    };
  }, [blobUrl]);

  const processBase64 = useCallback(
    (input: string, name?: string) => {
      setError(null);
      setCopied(false);

      const parsed = parseBase64Input(input);
      if (!parsed) {
        setError("No valid base64 data found.");
        return;
      }

      try {
        const blob = base64ToBlob(parsed.base64, parsed.mimeType);
        const url = URL.createObjectURL(blob);

        base64Ref.current = parsed.base64;
        mimeRef.current = parsed.mimeType;

        const img = new window.Image();
        img.onload = () => {
          setBlobUrl((prev) => {
            if (prev) URL.revokeObjectURL(prev);
            return url;
          });

          const info: ImageInfo = {
            width: img.naturalWidth,
            height: img.naturalHeight,
            size: blob.size,
            mimeType: parsed.mimeType,
          };
          setImageInfo(info);
          setZoom(100);
          setShowInput(false);

          // Save to localStorage
          const ext = parsed.mimeType.split("/")[1] || "img";
          const newStored: StoredImage = {
            id: generateId(),
            name: name || `image_${Date.now()}.${ext}`,
            base64: parsed.base64,
            mimeType: parsed.mimeType,
            width: img.naturalWidth,
            height: img.naturalHeight,
            size: blob.size,
            createdAt: Date.now(),
          };

          setStoredImages((prev) => {
            const updated = [newStored, ...prev].slice(0, MAX_STORED);
            saveStoredImages(updated);
            return updated;
          });
          setSelectedId(newStored.id);
        };
        img.onerror = () => {
          URL.revokeObjectURL(url);
          setError("Failed to load image. The base64 data may be corrupted.");
        };
        img.src = url;
      } catch {
        setError("Invalid base64 string. Could not decode.");
      }
    },
    []
  );

  const handlePaste = useCallback(
    (e: React.ClipboardEvent) => {
      e.preventDefault();
      const text = e.clipboardData.getData("text/plain");
      if (text) {
        processBase64(text);
        return;
      }
      const files = e.clipboardData.files;
      if (files.length > 0) handleFile(files[0]);
    },
    [processBase64] // eslint-disable-line react-hooks/exhaustive-deps
  );

  const handleFile = useCallback(
    (file: File) => {
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = () => processBase64(reader.result as string, file.name);
        reader.readAsDataURL(file);
      } else if (file.type === "text/plain" || file.name.endsWith(".txt")) {
        const reader = new FileReader();
        reader.onload = () => processBase64(reader.result as string, file.name);
        reader.readAsText(file);
      } else {
        setError("Unsupported file type. Use an image or a .txt file containing base64.");
      }
    },
    [processBase64]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      const files = e.dataTransfer.files;
      if (files.length > 0) {
        handleFile(files[0]);
        return;
      }
      const text = e.dataTransfer.getData("text/plain");
      if (text) processBase64(text);
    },
    [handleFile, processBase64]
  );

  const handleDeleteImage = useCallback(
    (id: string) => {
      setStoredImages((prev) => {
        const updated = prev.filter((img) => img.id !== id);
        saveStoredImages(updated);

        // If we deleted the selected one, pick another or show input
        if (selectedId === id) {
          if (updated.length > 0) {
            // Defer to avoid state conflict
            setTimeout(() => loadImagePreview(updated[0]), 0);
          } else {
            setBlobUrl((prev) => {
              if (prev) URL.revokeObjectURL(prev);
              return null;
            });
            setImageInfo(null);
            setSelectedId(null);
            setShowInput(true);
            base64Ref.current = "";
            mimeRef.current = "";
          }
        }
        return updated;
      });
    },
    [selectedId, loadImagePreview]
  );

  const handleClearAll = useCallback(() => {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    setBlobUrl(null);
    setImageInfo(null);
    setSelectedId(null);
    setError(null);
    setCopied(false);
    setZoom(100);
    setShowInput(true);
    base64Ref.current = "";
    mimeRef.current = "";
    setStoredImages([]);
    saveStoredImages([]);
  }, [blobUrl]);

  const handleDownload = useCallback(() => {
    if (!blobUrl || !imageInfo) return;
    const selected = storedImages.find((i) => i.id === selectedId);
    const ext = imageInfo.mimeType.split("/")[1] || "png";
    const a = document.createElement("a");
    a.href = blobUrl;
    a.download = selected?.name || `image.${ext}`;
    a.click();
  }, [blobUrl, imageInfo, storedImages, selectedId]);

  const handleCopyDataUri = useCallback(async () => {
    if (!base64Ref.current || !mimeRef.current) return;
    const dataUri = `data:${mimeRef.current};base64,${base64Ref.current}`;
    await navigator.clipboard.writeText(dataUri);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, []);

  // Scroll zoom with Ctrl+Wheel
  useEffect(() => {
    const container = previewContainerRef.current;
    if (!container) return;
    const onWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        setZoom((z) => Math.min(500, Math.max(10, z + (e.deltaY < 0 ? 10 : -10))));
      }
    };
    container.addEventListener("wheel", onWheel, { passive: false });
    return () => container.removeEventListener("wheel", onWheel);
  }, [blobUrl]);

  const hasImage = !!blobUrl && !!imageInfo;

  return (
    <div className="max-w-7xl mx-auto px-5 py-10 space-y-5 animate-in fade-in slide-in-from-bottom-4 duration-500">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-app-text">
            <ImageIcon className="w-5 h-5 text-app-accent" />
            <h3 className="text-2xl font-bold tracking-tight">
              Base64 Image Viewer
            </h3>
          </div>
          <p className="text-app-subtext text-xs uppercase tracking-widest font-medium opacity-70">
            Paste base64 or drop an image. All saved locally.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {storedImages.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClearAll}
              className="text-app-subtext gap-1.5 text-[11px]"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear All
            </Button>
          )}
        </div>
      </div>

      {/* Thumbnail Strip */}
      {storedImages.length > 0 && (
        <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-thin">
          {/* Add New Button */}
          <button
            onClick={() => {
              setShowInput(true);
              setSelectedId(null);
            }}
            className={`shrink-0 w-16 h-16 rounded-lg border-2 border-dashed flex items-center justify-center transition-colors ${
              showInput && !selectedId
                ? "border-app-accent bg-app-accent/10"
                : "border-app-border/40 hover:border-app-accent/50 hover:bg-app-accent/5"
            }`}
          >
            <Plus className="w-4 h-4 text-app-subtext" />
          </button>

          {storedImages.map((img) => (
            <div key={img.id} className="shrink-0 relative group">
              <button
                onClick={() => loadImagePreview(img)}
                className={`w-16 h-16 rounded-lg border-2 overflow-hidden transition-all ${
                  selectedId === img.id
                    ? "border-app-accent ring-1 ring-app-accent/30"
                    : "border-app-border/40 hover:border-app-accent/50"
                }`}
              >
                <img
                  src={`data:${img.mimeType};base64,${img.base64.slice(0, 1000)}...`}
                  alt={img.name}
                  className="w-full h-full object-cover"
                  // Use a tiny inline approach — thumbnail only needs enough to render
                  onError={(e) => {
                    // Fallback: load full base64 for thumbnail
                    (e.target as HTMLImageElement).src = `data:${img.mimeType};base64,${img.base64}`;
                  }}
                />
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteImage(img.id);
                }}
                className="absolute -top-1.5 -right-1.5 w-5 h-5 rounded-full bg-red-500/90 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Input Zone */}
      {(showInput || storedImages.length === 0) && (
        <div
          className={`bg-app-card border-2 border-dashed rounded-xl p-6 transition-colors ${
            isDragOver
              ? "border-app-accent bg-app-accent/5"
              : "border-app-border/60"
          }`}
          onDragOver={(e) => {
            e.preventDefault();
            setIsDragOver(true);
          }}
          onDragLeave={() => setIsDragOver(false)}
          onDrop={handleDrop}
        >
          <div className="space-y-4">
            <textarea
              placeholder="Paste base64 string here (Ctrl+V)..."
              className="w-full min-h-[80px] max-h-[80px] resize-none text-xs font-mono bg-app-panel/50 border border-app-border/40 rounded-lg p-3 text-app-text placeholder:text-app-subtext/50 focus:outline-none focus:ring-1 focus:ring-app-accent/50"
              onPaste={handlePaste}
            />

            <div className="flex items-center gap-3">
              <div className="h-px flex-1 bg-app-border/30" />
              <span className="text-[10px] text-app-subtext uppercase tracking-widest">
                or
              </span>
              <div className="h-px flex-1 bg-app-border/30" />
            </div>

            <div className="flex items-center justify-center gap-3">
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*,.txt"
                className="hidden"
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) handleFile(file);
                  if (fileInputRef.current) fileInputRef.current.value = "";
                }}
              />
              <Button
                variant="secondary"
                size="sm"
                onClick={() => fileInputRef.current?.click()}
                className="gap-2 text-app-subtext border-app-border/50 hover:bg-app-accent/10 hover:text-app-accent"
              >
                <Upload className="w-3.5 h-3.5" />
                Upload image or .txt
              </Button>
              <span className="text-[10px] text-app-subtext opacity-60">
                Drag & drop supported
              </span>
            </div>
          </div>

          {error && (
            <div className="mt-4 flex items-center gap-2 text-[11px] text-red-400">
              <AlertCircle className="w-3.5 h-3.5 shrink-0" />
              <span>{error}</span>
            </div>
          )}
        </div>
      )}

      {/* Image Preview + Info */}
      {hasImage && !showInput && (
        <div className="grid gap-4 lg:grid-cols-[1fr_260px]">
          {/* Preview Card */}
          <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3 min-w-0">
            <div className="flex items-center justify-between">
              <div className="text-xs font-semibold text-app-text flex items-center gap-2 min-w-0">
                <FileImage className="w-3.5 h-3.5 text-app-accent shrink-0" />
                <span className="truncate">
                  {storedImages.find((i) => i.id === selectedId)?.name || "Preview"}
                </span>
              </div>
              <div className="flex items-center gap-0.5 shrink-0">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setZoom((z) => Math.max(10, z - 25))}
                  className="h-7 w-7 p-0 text-app-subtext"
                >
                  <ZoomOut className="w-3.5 h-3.5" />
                </Button>
                <span className="text-[10px] text-app-subtext w-10 text-center tabular-nums">
                  {zoom}%
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setZoom((z) => Math.min(500, z + 25))}
                  className="h-7 w-7 p-0 text-app-subtext"
                >
                  <ZoomIn className="w-3.5 h-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setZoom(100)}
                  className="h-7 w-7 p-0 text-app-subtext"
                  title="Reset zoom"
                >
                  <Maximize2 className="w-3.5 h-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    // Fit to container
                    const container = previewContainerRef.current;
                    if (container && imageInfo) {
                      const cw = container.clientWidth - 32;
                      const ch = container.clientHeight - 32;
                      const fitZoom = Math.min(
                        (cw / imageInfo.width) * 100,
                        (ch / imageInfo.height) * 100,
                        100
                      );
                      setZoom(Math.round(Math.max(10, fitZoom)));
                    }
                  }}
                  className="h-7 w-7 p-0 text-app-subtext"
                  title="Fit to view"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                </Button>
              </div>
            </div>

            {/* Image container — fixed height, proper overflow scroll */}
            <div
              ref={previewContainerRef}
              className="overflow-auto rounded-lg border border-app-border/40 bg-app-panel"
              style={{ height: "420px" }}
            >
              <div
                className="min-h-full min-w-full flex items-center justify-center p-4"
                style={{
                  backgroundImage:
                    "repeating-conic-gradient(var(--color-app-border) 0% 25%, transparent 0% 50%)",
                  backgroundSize: "16px 16px",
                }}
              >
                <img
                  src={blobUrl}
                  alt="Decoded base64"
                  draggable={false}
                  className="block"
                  style={{
                    width: `${(imageInfo.width * zoom) / 100}px`,
                    height: `${(imageInfo.height * zoom) / 100}px`,
                    maxWidth: "none",
                    maxHeight: "none",
                    imageRendering: zoom > 200 ? "pixelated" : "auto",
                  }}
                />
              </div>
            </div>

            <div className="text-[10px] text-app-subtext opacity-60 text-center">
              Ctrl + Scroll to zoom
            </div>
          </div>

          {/* Info Sidebar */}
          <div className="space-y-3">
            {/* Image Info */}
            <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3">
              <div className="text-xs font-semibold text-app-text">Info</div>
              <div className="space-y-2.5">
                <InfoRow label="Type" value={imageInfo.mimeType.split("/")[1].toUpperCase()} />
                <InfoRow
                  label="Dimensions"
                  value={`${imageInfo.width} × ${imageInfo.height}`}
                />
                <InfoRow label="File Size" value={formatBytes(imageInfo.size)} />
                <InfoRow
                  label="Base64"
                  value={`${base64Ref.current.length.toLocaleString()} chars`}
                />
                <InfoRow label="Zoom" value={`${zoom}%`} />
              </div>
            </div>

            {/* Actions */}
            <div className="space-y-1.5">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleDownload}
                className="w-full gap-2 text-app-text hover:bg-app-accent/10 hover:text-app-accent"
              >
                <Download className="w-3.5 h-3.5" />
                Download
              </Button>

              <Button
                variant="secondary"
                size="sm"
                onClick={handleCopyDataUri}
                className="w-full gap-2 text-app-text hover:bg-app-accent/10 hover:text-app-accent"
              >
                {copied ? (
                  <>
                    <CheckCircle2 className="w-3.5 h-3.5 text-green-400" />
                    <span className="text-green-400">Copied!</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    Copy Data URI
                  </>
                )}
              </Button>

              {selectedId && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleDeleteImage(selectedId)}
                  className="w-full gap-2 text-red-400/70 hover:text-red-400 hover:bg-red-400/10"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  Delete
                </Button>
              )}
            </div>

            {/* Storage info */}
            <div className="text-[10px] text-app-subtext/50 text-center">
              {storedImages.length}/{MAX_STORED} images stored locally
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-[10px] text-app-subtext uppercase tracking-widest shrink-0">
        {label}
      </span>
      <span className="text-[11px] text-app-text font-mono truncate">{value}</span>
    </div>
  );
}
