import { useEffect, useState, useCallback, useRef } from "react";
import { Copy, RotateCcw, X } from "lucide-react";
import { TextArea } from "./TextArea";
import { Button } from "./Button";
import { useSettingsStore } from "../../store/settings";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import { useTranslateMutation } from "../../hooks/useLlmApi";

const LANGUAGES = [
  "Auto Detect",
  "English",
  "Indonesian",
  "Spanish",
  "French",
  "German",
  "Chinese",
  "Japanese",
  "Arabic",
  "Russian",
  "Portuguese",
  "Hindi",
  "Korean",
];

interface PopupTranslateDialogProps {
  open: boolean;
  initialText: string | null;
  onClose: () => void;
}

export default function PopupTranslateDialog({
  open,
  initialText,
  onClose,
}: PopupTranslateDialogProps) {
  const {
    sourceLang,
    targetLang,
    autoTranslate,
    setSourceLang,
    setTargetLang,
  } = useSettingsStore();
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const buildConfig = useLlmConfigBuilder();
  const { mutateAsync: translateAsync, isPending: isTranslating } =
    useTranslateMutation();
  const isTranslatingRef = useRef(false);
  const lastAutoTextRef = useRef<string | null>(null);

  useEffect(() => {
    isTranslatingRef.current = isTranslating;
  }, [isTranslating]);

  const runTranslate = useCallback(
    async (text: string) => {
      if (!text.trim() || isTranslatingRef.current) return;
      setError(null);
      try {
        const result = await translateAsync({
          config: buildConfig({ maxTokens: 1000, temperature: 0.3 }),
          content: text,
          source: sourceLang,
          target: targetLang,
        });
        setOutput(result.result);
      } catch (e: any) {
        setOutput("");
        setError(e?.message || "Translation failed. Check backend logs.");
      }
    },
    [buildConfig, sourceLang, targetLang, translateAsync]
  );

  useEffect(() => {
    if (!open) {
      lastAutoTextRef.current = null;
      return;
    }
    const nextText = initialText ?? "";
    setInput(nextText);
    setOutput("");
    setError(null);
    if (
      autoTranslate &&
      nextText.trim() &&
      lastAutoTextRef.current !== nextText
    ) {
      lastAutoTextRef.current = nextText;
      runTranslate(nextText);
    }
  }, [open, initialText, autoTranslate, runTranslate]);

  useEffect(() => {
    if (!open) return;
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, onClose]);

  const handleCopy = () => {
    if (!output) return;
    navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/70 backdrop-blur-md animate-in fade-in duration-200">
      <div className="w-full max-w-3xl bg-app-panel border border-app-border rounded-2xl shadow-2xl p-6 animate-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <span className="text-xs uppercase tracking-widest text-app-subtext">
              Popup Translate
            </span>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="h-8 w-8">
            <X className="w-4 h-4" />
          </Button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-[10px] text-gray-500 uppercase tracking-wider">
                From
              </label>
              <div className="relative">
                <select
                  className="bg-[#18181b] border border-app-border rounded px-3 py-1.5 text-xs appearance-none outline-none"
                  value={sourceLang}
                  onChange={(e: any) => setSourceLang(e.target.value)}>
                  {LANGUAGES.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <TextArea
              className="min-h-[180px] text-sm p-3 bg-app-panel/60 border-app-border/40"
              value={input}
              onInput={(e: any) => setInput(e.target.value)}
              placeholder="Captured text will appear here..."
            />
          </div>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-[10px] text-gray-500 uppercase tracking-wider">
                To
              </label>
              <div className="relative">
                <select
                  className="bg-[#18181b] border border-app-border rounded px-3 py-1.5 text-xs appearance-none outline-none"
                  value={targetLang}
                  onChange={(e: any) => setTargetLang(e.target.value)}>
                  {LANGUAGES.filter((lang) => lang !== "Auto Detect").map(
                    (lang) => (
                      <option key={lang} value={lang}>
                        {lang}
                      </option>
                    )
                  )}
                </select>
              </div>
            </div>
            <TextArea
              readOnly
              className="min-h-[180px] text-sm p-3 bg-black/20 border-app-border/20 text-app-text/90"
              value={output}
              placeholder="Translation will appear here..."
            />
            {error && <div className="text-[10px] text-red-400">{error}</div>}
          </div>
        </div>

        <div className="mt-4 flex flex-col md:flex-row md:items-center gap-3 justify-between">
          <div className="text-[10px] text-app-subtext uppercase tracking-widest">
            {isTranslating
              ? "Translating..."
              : autoTranslate
              ? "Auto translate enabled"
              : "Manual translate"}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopy}
              className="h-9 px-3 text-xs"
              disabled={!output}>
              <Copy className="w-3 h-3 mr-1" />
              {copied ? "Copied" : "Copy"}
            </Button>
            <Button
              size="sm"
              onClick={() => runTranslate(input)}
              className="h-9 px-4 text-xs font-bold"
              disabled={!input.trim() || isTranslating}>
              <RotateCcw className="w-3 h-3 mr-1" />
              {isTranslating ? "Translating" : "Regenerate"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
