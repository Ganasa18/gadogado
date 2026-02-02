import { Copy, X } from "lucide-react";

type CurlPreviewModalProps = {
  curlCommand: string;
  curlError: string | null;
  curlCopied: boolean;
  onClose: () => void;
  onCopy: () => void;
};

export function CurlPreviewModal({
  curlCommand,
  curlError,
  curlCopied,
  onClose,
  onCopy,
}: CurlPreviewModalProps) {
  return (
    <div className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="w-full max-w-3xl bg-app-panel border border-app-border rounded-2xl shadow-2xl p-5 animate-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between gap-3 mb-4">
          <div>
            <div className="text-xs uppercase tracking-widest text-app-subtext">
              API Replay
            </div>
            <div className="text-sm font-semibold text-app-text">
              cURL Command Preview
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-app-border p-2 text-app-subtext hover:text-app-text hover:border-emerald-500/60 transition">
            <X className="w-4 h-4" />
          </button>
        </div>
        <div>
          {curlError ? (
            <div className="rounded-lg border border-red-900/50 bg-red-900/10 px-3 py-2 text-[11px] text-red-200">
              {curlError}
            </div>
          ) : (
            <pre className="max-h-64 overflow-auto rounded-lg border border-app-border bg-black/40 p-3 text-[11px] text-app-text whitespace-pre-wrap font-mono custom-scrollbar">
              {curlCommand}
            </pre>
          )}
        </div>
        <div className="mt-4 flex items-center justify-between">
          <span className="text-[10px] text-app-subtext">
            {curlCopied ? (
              <span className="text-emerald-400">Copied to clipboard!</span>
            ) : (
              "Use this in your terminal."
            )}
          </span>
          <button
            type="button"
            onClick={onCopy}
            disabled={!curlCommand}
            className="flex items-center gap-2 rounded-lg border border-app-border px-3 py-2 text-xs text-app-subtext hover:text-app-text hover:border-emerald-500/60 transition disabled:opacity-50">
            <Copy className="w-3.5 h-3.5" />
            Copy cURL
          </button>
        </div>
      </div>
    </div>
  );
}
