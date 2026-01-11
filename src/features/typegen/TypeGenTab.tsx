import { useEffect, useMemo, useState } from "react";
import { TextArea } from "../../shared/components/TextArea";
import { Button } from "../../shared/components/Button";
import { Select } from "../../shared/components/Select";
import { Input } from "../../shared/components/Input";
import { Copy, Check, Code2 } from "lucide-react";
import { useSettingsStore } from "../../store/settings";
import { useHistoryStore } from "../../store/history";
import { useLlmConfigBuilder } from "../../hooks/useLlmConfig";
import { useTypegenMutation } from "../../hooks/useLlmApi";

const LANGUAGES = [
  { value: "TypeScript", label: "TypeScript" },
  { value: "Go", label: "Go" },
  { value: "Rust", label: "Rust" },
  { value: "Dart", label: "Dart" },
  { value: "Flutter", label: "Flutter" },
  { value: "Java", label: "Java" },
  { value: "PHP", label: "PHP" },
];

const MODES = [
  { value: "auto", label: "Auto" },
  { value: "offline", label: "Offline" },
  { value: "llm", label: "LLM" },
];

export default function TypeGenTab() {
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [copied, setCopied] = useState(false);
  const [rootName, setRootName] = useState("Root");
  const [language, setLanguage] = useState("TypeScript");
  const [mode, setMode] = useState("auto");
  const [jsonError, setJsonError] = useState<string | null>(null);
  const { provider, model } = useSettingsStore();
  const { addItem } = useHistoryStore();
  const buildConfig = useLlmConfigBuilder();
  const typegenMutation = useTypegenMutation();
  const isLoading = typegenMutation.isPending;

  const jsonIsValid = useMemo(() => {
    if (!input.trim()) {
      return false;
    }
    try {
      JSON.parse(input);
      return true;
    } catch (err: any) {
      return false;
    }
  }, [input]);

  useEffect(() => {
    if (!input.trim()) {
      setJsonError(null);
      return;
    }
    try {
      JSON.parse(input);
      setJsonError(null);
    } catch (err: any) {
      setJsonError(err?.message || "Invalid JSON");
    }
  }, [input]);

  const handleGenerate = async () => {
    if (!input.trim() || !jsonIsValid || isLoading) return;

    window.dispatchEvent(
      new CustomEvent("global-loading:start", { detail: { id: "typegen" } })
    );

    try {
      const result = await typegenMutation.mutateAsync({
        config: buildConfig({ maxTokens: 2000, temperature: 0.2 }),
        json: input,
        language,
        root_name: rootName,
        mode,
      });

      setOutput(result.result);
      addItem({
        type: "typegen",
        input,
        output: result.result,
        provider,
        model,
        topic: `${language} ${rootName}`.trim(),
      });
    } catch (e: any) {
      console.error("Type generation failed", e);
      setOutput(`Error: ${e.message || "Check backend logs"}`);
    } finally {
      window.dispatchEvent(
        new CustomEvent("global-loading:end", { detail: { id: "typegen" } })
      );
    }
  };

  const copyToClipboard = () => {
    if (!output) return;
    navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="max-w-5xl mx-auto p-4 space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Code2 className="w-5 h-5 text-app-accent" />
            <h3 className="text-2xl font-bold tracking-tight">
              Type Generator
            </h3>
          </div>
          <p className="text-app-subtext text-xs uppercase tracking-widest font-medium opacity-70">
            Generate type-safe models from JSON responses
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setInput("");
            setOutput("");
          }}
          className="text-app-subtext">
          Clear
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <div className="space-y-2">
          <label className="text-[10px] uppercase tracking-widest text-app-subtext">
            Mode
          </label>
          <Select
            options={MODES}
            value={mode}
            onChange={setMode}
            searchable={false}
          />
        </div>
        <div className="space-y-2">
          <label className="text-[10px] uppercase tracking-widest text-app-subtext">
            Language
          </label>
          <Select options={LANGUAGES} value={language} onChange={setLanguage} />
        </div>
        <div className="space-y-2">
          <label className="text-[10px] uppercase tracking-widest text-app-subtext">
            Root Type Name
          </label>
          <Input
            value={rootName}
            onInput={(e: any) => setRootName(e.target.value)}
            placeholder="Root"
          />
        </div>
      </div>

      <div className="flex justify-center">
        <Button
          size="lg"
          className="w-full max-w-sm gap-2 h-12 text-sm font-bold shadow-xl shadow-app-accent/10 active:scale-[0.98] transition-all"
          onClick={handleGenerate}
          disabled={!input.trim() || !jsonIsValid || isLoading}>
          {isLoading ? "Generating..." : "Generate Types"}
        </Button>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <div className="space-y-2">
          <div className="flex items-center justify-between px-1">
            <span className="text-[10px] uppercase tracking-widest text-app-subtext font-bold">
              JSON Input
            </span>
            <span
              className={`text-[10px] uppercase tracking-widest font-bold ${
                jsonError ? "text-red-400" : "text-app-success"
              }`}>
              {input.trim()
                ? jsonError
                  ? "Invalid JSON"
                  : "Valid JSON"
                : "Waiting"}
            </span>
          </div>
          <TextArea
            placeholder="Paste JSON response here..."
            className="min-h-[240px] text-sm font-mono bg-black/20 border-app-border/40"
            value={input}
            onInput={(e: any) => setInput(e.target.value)}
          />
          {jsonError && <p className="text-[11px] text-red-400">{jsonError}</p>}
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between px-1">
            <span className="text-[10px] uppercase tracking-widest text-app-subtext font-bold">
              Generated Types
            </span>
            <Button
              variant="ghost"
              size="sm"
              onClick={copyToClipboard}
              className="h-6 px-2 text-[10px]"
              disabled={!output}>
              {copied ? (
                <Check className="w-3 h-3 mr-1 text-app-success" />
              ) : (
                <Copy className="w-3 h-3 mr-1" />
              )}
              Copy
            </Button>
          </div>
          <TextArea
            readOnly
            placeholder="Generated types appear here..."
            className="min-h-[240px] text-sm font-mono bg-app-panel/40 border-app-border/40"
            value={output}
          />
        </div>
      </div>
    </div>
  );
}
