import { ArrowRight, Globe, Info, Loader2, X } from "lucide-react";
import type { WebCrawlMode } from "../api";

type Props = {
  webUrl: string;
  onChangeWebUrl: (v: string) => void;
  maxPages: number;
  onChangeMaxPages: (v: number) => void;
  maxDepth: number;
  onChangeMaxDepth: (v: number) => void;
  webCrawlMode: WebCrawlMode;
  onChangeWebCrawlMode: (v: WebCrawlMode) => void;
  isCrawling: boolean;
  onStart: () => void;
  onClose: () => void;
  selectedCollectionId: number | null;
};

export function RagTabWebImportPanel(props: Props) {
  const {
    webUrl,
    onChangeWebUrl,
    maxPages,
    onChangeMaxPages,
    maxDepth,
    onChangeMaxDepth,
    webCrawlMode,
    onChangeWebCrawlMode,
    isCrawling,
    onStart,
    onClose,
    selectedCollectionId,
  } = props;

  return (
    <div className="p-6 border-b border-app-border bg-app-card/30">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Globe className="w-4 h-4 text-app-accent" />
          <h3 className="text-sm font-semibold">Import from website</h3>
        </div>
        <button onClick={onClose} className="p-1.5 text-app-text-muted hover:text-app-text transition-colors">
          <X className="w-4 h-4" />
        </button>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
        <div className="md:col-span-2">
          <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">URL</label>
          <input
            value={webUrl}
            onChange={(e) => onChangeWebUrl(e.target.value)}
            placeholder="https://docs.example.com"
            className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
          />
        </div>
        <div>
          <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">Max pages</label>
          <input
            type="number"
            min={1}
            value={maxPages}
            onChange={(e) => onChangeMaxPages(Number(e.target.value) || 1)}
            className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
          />
        </div>
        <div>
          <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">Max depth</label>
          <input
            type="number"
            min={1}
            value={maxDepth}
            onChange={(e) => onChangeMaxDepth(Number(e.target.value) || 1)}
            className="w-full bg-app-bg border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
          />
        </div>
      </div>

      <div className="mt-4">
        <label className="text-[10px] text-app-subtext block mb-2 uppercase tracking-wider">Crawl Mode</label>
        <div className="flex gap-3">
          <button
            onClick={() => onChangeWebCrawlMode("html")}
            className={`flex-1 px-4 py-2.5 rounded-lg text-sm border transition-all ${
              webCrawlMode === "html"
                ? "bg-app-accent text-white border-app-accent"
                : "bg-app-bg border-app-border text-app-text hover:border-app-accent/50"
            }`}>
            <div className="font-medium">HTML (Fast)</div>
            <div className="text-[10px] opacity-75 mt-0.5">Best for simple sites</div>
          </button>
          <button
            onClick={() => onChangeWebCrawlMode("ocr")}
            className={`flex-1 px-4 py-2.5 rounded-lg text-sm border transition-all ${
              webCrawlMode === "ocr"
                ? "bg-app-accent text-white border-app-accent"
                : "bg-app-bg border-app-border text-app-text hover:border-app-accent/50"
            }`}>
            <div className="font-medium">OCR (Accurate)</div>
            <div className="text-[10px] opacity-75 mt-0.5">For JS-heavy sites</div>
          </button>
        </div>
      </div>

      <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-xs text-app-text-muted">
          <Info className="w-3.5 h-3.5" />
          {webCrawlMode === "html"
            ? "Crawls only same-domain links to keep the import scoped."
            : "Uses Playwright screenshots + Tesseract OCR. Requires Node.js."}
        </div>
        <button
          onClick={onStart}
          disabled={isCrawling || selectedCollectionId === null || !webUrl.trim()}
          className="flex items-center gap-2 px-4 py-2 bg-app-accent text-white rounded-lg text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
          {isCrawling ? <Loader2 className="w-4 h-4 animate-spin" /> : <ArrowRight className="w-4 h-4" />}
          {webCrawlMode === "ocr" ? "Start OCR capture" : "Start crawl"}
        </button>
      </div>
    </div>
  );
}
