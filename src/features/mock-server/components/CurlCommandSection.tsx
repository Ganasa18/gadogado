// =============================================================================
// cURL Command Section Component
// Displays generated cURL command with copy button
// =============================================================================

import { Copy } from "lucide-react";
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

  return (
    <section className="pt-8 pb-10">
      <div className="rounded-lg bg-app-panel border border-app-border p-4 space-y-3">
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-bold uppercase tracking-widest text-app-subtext">
            Generate Request cURL
          </h4>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => onCopyToClipboard("cURL", curlCommand)}
            className="text-app-accent hover:bg-app-accent/10"
          >
            <Copy className="w-3.5 h-3.5 mr-2" />
            {lastCopied === "cURL" ? "Copied!" : "Copy cURL"}
          </Button>
        </div>
        <div className="bg-[#101010] p-3 rounded border border-app-border overflow-x-auto">
          <pre className="text-[10px] font-mono text-app-text/70 whitespace-pre-wrap break-all">
            {curlCommand}
          </pre>
        </div>
      </div>
    </section>
  );
}
