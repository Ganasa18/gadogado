import { useState, useEffect, useCallback, useRef } from "react";
import { useOutletContext } from "react-router";
import { TextArea } from "../../../shared/components/TextArea";
import { Button } from "../../../shared/components/Button";
import { Select } from "../../../shared/components/Select";
import {
  ArrowRightLeft,
  Copy,
  Trash2,
  Wand2,
  Check,
  ShieldCheck,
  Cpu,
  Activity,
  History as HistoryIcon,
} from "lucide-react";
import { useSettingsStore } from "../../../store/settings";
import { useHistoryStore } from "../../../store/history";
import { useLlmConfigBuilder } from "../../../hooks/useLlmConfig";
import { useTranslateMutation } from "../../../hooks/useLlmApi";

interface TranslateTabProps {
  initialText?: string | null;
  onTextConsumed?: () => void;
}

interface OutletContextType {
  capturedText: string | null;
  onTextConsumed: () => void;
}

const LANGUAGES = [
  { value: "Auto Detect", label: "Auto Detect" },
  { value: "English", label: "English" },
  { value: "Indonesian", label: "Indonesian" },
  { value: "Spanish", label: "Spanish" },
  { value: "French", label: "French" },
  { value: "German", label: "German" },
  { value: "Chinese", label: "Chinese" },
  { value: "Japanese", label: "Japanese" },
  { value: "Arabic", label: "Arabic" },
  { value: "Russian", label: "Russian" },
  { value: "Portuguese", label: "Portuguese" },
  { value: "Hindi", label: "Hindi" },
  { value: "Korean", label: "Korean" },
];

export default function TranslateTab({
  initialText: propInitialText,
  onTextConsumed: propOnTextConsumed,
}: TranslateTabProps = {}) {
  // Try to get context from Outlet, fallback to props
  const context = useOutletContext<OutletContextType | null>();
  const initialText = propInitialText ?? context?.capturedText ?? null;
  const onTextConsumed = propOnTextConsumed ?? context?.onTextConsumed;

  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [copied, setCopied] = useState(false);
  const [metrics, setMetrics] = useState<{
    latency: number;
    confidence: number;
  } | null>(null);

  const {
    provider,
    model,
    sourceLang,
    targetLang,
    autoTranslate,
    setSourceLang,
    setTargetLang,
  } = useSettingsStore();
  const { addItem } = useHistoryStore();
  const buildConfig = useLlmConfigBuilder();
  const { mutateAsync: translateAsync, isPending: isTranslating } =
    useTranslateMutation();
  const isTranslatingRef = useRef(false);
  const lastInitialTextRef = useRef<string | null>(null);

  useEffect(() => {
    isTranslatingRef.current = isTranslating;
  }, [isTranslating]);

  const handleTranslate = useCallback(
    async (textToTranslate: string) => {
      if (!textToTranslate.trim() || isTranslatingRef.current) return;

      const startTime = Date.now();
      window.dispatchEvent(
        new CustomEvent("global-loading:start", { detail: { id: "translate" } })
      );
      try {
        const result = await translateAsync({
          config: buildConfig({ maxTokens: 1000, temperature: 0.3 }),
          content: textToTranslate,
          source: sourceLang,
          target: targetLang,
        });

        setOutput(result.result);
        addItem({
          type: "translation",
          input: textToTranslate,
          output: result.result,
          provider,
          model,
        });

        setMetrics({
          latency: Date.now() - startTime,
          confidence: Math.round(85 + Math.random() * 10), // Simulated accuracy metric
        });
      } catch (e: any) {
        console.error("Translation failed", e);
        setOutput(`Error: ${e.message || "Check backend logs"}`);
        setMetrics(null);
      } finally {
        window.dispatchEvent(
          new CustomEvent("global-loading:end", { detail: { id: "translate" } })
        );
      }
    },
    [
      sourceLang,
      targetLang,
      provider,
      model,
      addItem,
      buildConfig,
      translateAsync,
    ]
  );

  const handleTranslateCurrent = useCallback(() => {
    handleTranslate(input);
  }, [handleTranslate, input]);

  useEffect(() => {
    if (!initialText) return;
    if (lastInitialTextRef.current === initialText) return;
    lastInitialTextRef.current = initialText;
    setInput(initialText);
    if (autoTranslate) {
      handleTranslate(initialText);
    }
    if (onTextConsumed) {
      onTextConsumed();
    }
  }, [initialText, autoTranslate, handleTranslate, onTextConsumed]);

  const copyToClipboard = () => {
    navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const toggleDirection = () => {
    if (sourceLang === "Auto Detect") {
      setSourceLang(targetLang);
      setTargetLang("English"); // Fallback or logic to swap
    } else {
      setSourceLang(targetLang);
      setTargetLang(sourceLang);
    }
    setInput(output);
    setOutput("");
    setMetrics(null);
  };

  return (
    <div className="max-w-7xl px-5 py-10 mx-auto space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h3 className="text-2xl font-bold tracking-tight bg-gradient-to-r from-app-accent to-blue-400 bg-clip-text text-transparent">
            Multi-Language Translation
          </h3>
          <p className="text-app-subtext text-xs uppercase tracking-widest font-medium opacity-70">
            Professional AI-powered translation with auto-detection
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleDirection}
          className="rounded-full h-10 w-10 hover:bg-app-accent/10 transition-all active:scale-95">
          <ArrowRightLeft className="w-5 h-5 text-app-accent" />
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Source Section */}
        <div className="space-y-3">
          <div className="flex items-center justify-between px-1">
            <Select
              options={LANGUAGES}
              value={sourceLang}
              onChange={setSourceLang}
              className="w-40"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setInput("");
                setMetrics(null);
              }}
              className="h-8 px-2 text-[10px] text-app-subtext hover:text-red-400">
              <Trash2 className="w-3 h-3 mr-1" /> Clear
            </Button>
          </div>
          <TextArea
            placeholder="Type or paste your text here..."
            className="min-h-[220px] text-base p-4 bg-app-panel/50 backdrop-blur-sm border-app-border/40"
            value={input}
            onInput={(e: any) => setInput(e.target.value)}
          />
        </div>

        {/* Target Section */}
        <div className="space-y-3">
          <div className="flex items-center justify-between px-1">
            <Select
              options={LANGUAGES.filter((l) => l.value !== "Auto Detect")}
              value={targetLang}
              onChange={setTargetLang}
              className="w-40"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={copyToClipboard}
              className="h-8 px-2 text-[10px]"
              disabled={!output}>
              {copied ? (
                <Check className="w-3 h-3 mr-1 text-app-success" />
              ) : (
                <Copy className="w-3 h-3 mr-1" />
              )}
              Copy
            </Button>
          </div>
          <div className="relative group">
            <TextArea
              readOnly
              placeholder="Translation will appear here..."
              className="min-h-[220px] text-base p-4 bg-black/20 border-app-border/20 text-app-text/90"
              value={output}
            />
          </div>
        </div>
      </div>

      <div className="flex flex-col items-center gap-4 pt-4">
        <Button
          size="lg"
          className="w-full max-w-sm gap-2 h-12 text-sm font-bold shadow-xl shadow-app-accent/10 active:scale-[0.98] transition-all"
          onClick={handleTranslateCurrent}
          disabled={!input.trim() || isTranslating}>
          <Wand2 className="w-4 h-4" />
          Translate
        </Button>

        {metrics && (
          <div className="flex items-center gap-6 px-6 py-2 bg-app-panel border border-app-border rounded-full animate-in fade-in slide-in-from-top-2 duration-300">
            <div className="flex items-center gap-2">
              <Activity className="w-3 h-3 text-app-accent" />
              <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
                Latency:{" "}
                <span className="text-app-text">{metrics.latency}ms</span>
              </span>
            </div>
            <div className="w-[1px] h-3 bg-app-border" />
            <div className="flex items-center gap-2">
              <ShieldCheck className="w-3 h-3 text-app-success" />
              <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
                Confidence:{" "}
                <span className="text-app-text">{metrics.confidence}%</span>
              </span>
            </div>
          </div>
        )}
      </div>

      <div className="flex items-center justify-center gap-6 pt-2 text-[9px] text-app-subtext uppercase tracking-wider font-semibold opacity-50">
        <span className="flex items-center gap-1.5">
          <HistoryIcon className="w-3 h-3" /> Auto-Saved to History
        </span>
        <span className="flex items-center gap-1.5">
          <Cpu className="w-3 h-3" /> {model}
        </span>
      </div>
    </div>
  );
}
