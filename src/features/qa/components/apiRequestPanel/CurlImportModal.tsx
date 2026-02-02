import { Terminal, X } from "lucide-react";

type CurlImportModalProps = {
  curlCommand: string;
  setCurlCommand: (value: string) => void;
  onClose: () => void;
  onImport: () => void;
};

export function CurlImportModal({
  curlCommand,
  setCurlCommand,
  onClose,
  onImport,
}: CurlImportModalProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="w-full max-w-lg bg-app-card border border-app-border rounded-lg shadow-xl flex flex-col">
        <div className="flex items-center justify-between p-4 border-b border-app-border">
          <div className="flex items-center gap-2 font-medium text-sm text-app-text">
            <Terminal className="w-4 h-4 text-emerald-400" />
            Import cURL Command
          </div>
          <button
            onClick={onClose}
            className="text-app-subtext hover:text-app-text">
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="p-4 space-y-4">
          <div className="text-xs text-app-subtext">
            Paste a cURL command below to populate the request fields.
          </div>
          <textarea
            value={curlCommand}
            onChange={(e) => setCurlCommand(e.target.value)}
            placeholder="curl -X POST https://api.example.com/data -H 'Content-Type: application/json' -d '...'"
            className="w-full h-32 bg-black/30 border border-app-border rounded p-3 text-xs font-mono outline-none focus:border-emerald-500/50 transition resize-none text-app-text"
            autoFocus
          />
          <div className="flex justify-end gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded border border-app-border text-xs text-app-subtext hover:bg-white/5 transition">
              Cancel
            </button>
            <button
              onClick={onImport}
              disabled={!curlCommand.trim()}
              className="px-3 py-1.5 rounded bg-emerald-600/20 border border-emerald-500/50 text-xs text-emerald-100 hover:bg-emerald-600/30 transition disabled:opacity-50">
              Import
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
