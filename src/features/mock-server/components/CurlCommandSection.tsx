// =============================================================================
// cURL Command Section Component
// Displays generated cURL command with copy button
// =============================================================================

import { Copy, Check } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import type { MockRoute } from "../types";

export interface CurlCommandSectionProps {
  route: MockRoute;
  baseUrl: string;
  generateCurlCommand: (route: MockRoute, baseUrl: string) => string;
  lastCopied: string | null;
  onCopyToClipboard: (label: string, value: string) => void;
}

export function CurlCommandSection({
  route,
  baseUrl,
  generateCurlCommand,
  lastCopied,
  onCopyToClipboard,
}: CurlCommandSectionProps) {
  const curlCommand = generateCurlCommand(route, baseUrl);
  const isCopied = lastCopied === "cURL";

  return (
    <section className="space-y-8 animate-in fade-in slide-in-from-top-2 duration-500">
      <div className="space-y-1">
        <h3 className="text-xs font-bold text-app-text uppercase tracking-widest">
          STEP 4: CURL GENERATOR
        </h3>
        <p className="text-[11px] text-app-subtext/60">Use this command to test your mock endpoint from a terminal.</p>
      </div>

      <div className="bg-app-card rounded-[24px] border border-app-border p-8 space-y-6 relative group overflow-hidden">
        <div className="absolute top-0 right-0 p-8">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => onCopyToClipboard("cURL", curlCommand)}
            className={`h-10 rounded-xl px-6 transition-all font-bold text-xs gap-2 ${
              isCopied 
                ? "bg-app-success text-white shadow-md" 
                : "bg-app-bg text-app-subtext hover:text-app-text border border-app-border"
            }`}
          >
            {isCopied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
            {isCopied ? "Copied!" : "Copy cURL"}
          </Button>
        </div>

        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-orange-500" />
            <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
              Terminal Command
            </span>
          </div>

          <div className="bg-app-bg p-6 rounded-2xl border border-app-border overflow-x-auto custom-scrollbar group-hover:border-app-subtext/20 transition-colors">
            <pre className="text-xs font-mono text-app-accent/80 whitespace-pre-wrap break-all leading-relaxed">
              {curlCommand}
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}
