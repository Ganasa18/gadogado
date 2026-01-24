import { useState } from "react";
import { Check, Code, Copy } from "lucide-react";

import type { JsonNode } from "../../types";

interface JsonViewProps {
  json: JsonNode;
}

export default function JsonView({ json }: JsonViewProps) {
  const [copiedToClipboard, setCopiedToClipboard] = useState(false);

  const handleCopyJson = () => {
    const jsonString = JSON.stringify(json.value, null, 2);
    navigator.clipboard.writeText(jsonString);
    setCopiedToClipboard(true);
    window.setTimeout(() => setCopiedToClipboard(false), 2000);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex-none px-4 py-3 border-b border-app-border flex items-center justify-between bg-app-panel">
        <div className="flex items-center gap-2">
          <Code className="text-app-accent" size={18} />
          <h3 className="text-sm font-bold uppercase tracking-wider text-app-text">Formatted JSON</h3>
        </div>
        <button
          onClick={handleCopyJson}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-app-accent hover:bg-app-accent/90 text-white text-sm font-semibold transition"
        >
          {copiedToClipboard ? (
            <>
              <Check size={16} />
              Copied!
            </>
          ) : (
            <>
              <Copy size={16} />
              Copy JSON
            </>
          )}
        </button>
      </div>
      <div className="flex-1 overflow-auto p-0 bg-app-bg/50">
        <pre className="json-formatter p-6 text-sm font-mono text-app-text whitespace-pre-wrap break-all leading-relaxed select-text">
          {JSON.stringify(json.value, null, 2)}
        </pre>
      </div>
    </div>
  );
}
