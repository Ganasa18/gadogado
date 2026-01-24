import { ChangeEvent } from "react";
import {
  Download,
  FileJson,
  Loader2,
  RefreshCw,
  Trash2,
  Undo2,
  Wand2,
  X,
} from "lucide-react";

import { Button } from "../../../shared/components/Button";

interface InputSourcePanelProps {
  inputValue: string;
  onChangeInputValue: (value: string) => void;
  onParseJson: () => void;
  onClear: () => void;

  hasAiFixed: boolean;
  isFixing: boolean;
  aiError: string | null;
  onFixWithAi: () => void;
  onRegenerateAiFix: () => void;
  onRevertToOriginal: () => void;
  onClearAiError: () => void;

  onFileImport: (file: File) => void;
}

export default function InputSourcePanel({
  inputValue,
  onChangeInputValue,
  onParseJson,
  onClear,
  hasAiFixed,
  isFixing,
  aiError,
  onFixWithAi,
  onRegenerateAiFix,
  onRevertToOriginal,
  onClearAiError,
  onFileImport,
}: InputSourcePanelProps) {
  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      onFileImport(file);
    }
    event.target.value = "";
  };

  return (
    <div className="flex-none bg-app-card rounded-lg border border-app-border shadow-sm p-4">
      <div className="flex items-center gap-2 mb-4">
        <FileJson className="text-app-accent" size={18} />
        <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">Input Source</h3>
      </div>

      <div className="space-y-4">
        <div className="relative group">
          <textarea
            value={inputValue}
            onChange={(e) => onChangeInputValue(e.target.value)}
            placeholder='{"event_id": "evt_001", "action": "checkout"}'
            className={`w-full h-48 rounded-lg border bg-app-bg px-4 py-3 text-[13px] font-mono text-app-text focus:border-app-accent/50 focus:outline-none transition-all resize-none ${
              aiError ? "border-red-500/50" : "border-app-border"
            }`}
          />
          <div className="absolute top-2 right-2 flex gap-1">
            <button
              onClick={onFixWithAi}
              disabled={!inputValue.trim() || isFixing}
              className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-purple-500/10 text-purple-400 hover:bg-purple-500/20 hover:text-purple-300 disabled:opacity-40 disabled:cursor-not-allowed transition text-[11px] font-semibold border border-purple-500/20"
              title="Fix JSON with AI"
            >
              {isFixing ? <Loader2 size={12} className="animate-spin" /> : <Wand2 size={12} />}
              FIX AI
            </button>
          </div>
        </div>

        {aiError && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-xs">
            <X size={14} />
            <span className="flex-1">{aiError}</span>
            <button onClick={onClearAiError} className="hover:text-red-300 transition">
              <X size={12} />
            </button>
          </div>
        )}

        <div className="flex gap-2">
          <Button
            onClick={onParseJson}
            className="flex-1 bg-app-accent hover:bg-app-accent/90 text-white font-bold h-10"
          >
            Generate View
          </Button>

          {hasAiFixed && (
            <>
              <Button
                variant="secondary"
                onClick={onRegenerateAiFix}
                disabled={isFixing}
                className="px-3 border-app-border hover:bg-purple-500/10 hover:text-purple-400 hover:border-purple-500/20"
                title="Regenerate AI fix"
              >
                {isFixing ? <Loader2 size={18} className="animate-spin" /> : <RefreshCw size={18} />}
              </Button>
              <Button
                variant="secondary"
                onClick={onRevertToOriginal}
                className="px-3 border-app-border hover:bg-orange-500/10 hover:text-orange-400 hover:border-orange-500/20"
                title="Revert to original"
              >
                <Undo2 size={18} />
              </Button>
            </>
          )}

          <Button
            variant="secondary"
            onClick={onClear}
            className="px-3 border-app-border hover:bg-red-500/10 hover:text-red-500 hover:border-red-500/20"
          >
            <Trash2 size={18} />
          </Button>
        </div>

        <div className="relative overflow-hidden rounded-lg border border-dashed border-app-border p-4 hover:border-app-subtext transition cursor-pointer group">
          <input
            type="file"
            accept=".json,.yaml,.yml,.toml,.xml,.csv"
            onChange={handleFileChange}
            className="absolute inset-0 opacity-0 cursor-pointer z-10"
          />
          <div className="text-center">
            <div className="inline-flex p-2 rounded-full bg-app-panel mb-2 group-hover:text-app-accent transition">
              <Download size={16} />
            </div>
            <span className="text-xs font-semibold text-app-subtext block group-hover:text-app-text transition">
              Import File (JSON/YAML/TOML/XML/CSV)
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
