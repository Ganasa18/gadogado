import { useMemo, useState } from "react";
import { KeyRound, ShieldAlert, ShieldCheck } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { TextArea } from "../../../shared/components/TextArea";

type ParsedJwt = {
  header: Record<string, unknown> | null;
  payload: Record<string, unknown> | null;
  signature: string | null;
  headerRaw: string | null;
  payloadRaw: string | null;
  error: string | null;
  parts: string[];
};

function base64UrlToBytes(value: string): Uint8Array {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padding = normalized.length % 4;
  const padded = padding ? normalized + "=".repeat(4 - padding) : normalized;
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function decodeBase64Url(value: string): string {
  const bytes = base64UrlToBytes(value);
  return new TextDecoder().decode(bytes);
}

function safeParseJson(value: string): Record<string, unknown> | null {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function parseJwt(rawInput: string): ParsedJwt {
  const empty = {
    header: null,
    payload: null,
    signature: null,
    headerRaw: null,
    payloadRaw: null,
    error: null,
    parts: [],
  };

  const trimmed = rawInput.trim();
  if (!trimmed) {
    return empty;
  }

  const withoutBearer = trimmed.replace(/^Bearer\s+/i, "");
  const compact = withoutBearer.replace(/\s+/g, "");
  const parts = compact.split(".");

  if (parts.length !== 3) {
    return {
      ...empty,
      parts,
      error: "JWT must have exactly 3 parts separated by '.'",
    };
  }

  const [headerPart, payloadPart, signaturePart] = parts;
  try {
    const headerRaw = decodeBase64Url(headerPart);
    const payloadRaw = decodeBase64Url(payloadPart);
    const header = safeParseJson(headerRaw);
    const payload = safeParseJson(payloadRaw);

    return {
      header,
      payload,
      signature: signaturePart,
      headerRaw,
      payloadRaw,
      error:
        !header || !payload ? "Header or payload is not valid JSON." : null,
      parts,
    };
  } catch (err) {
    return {
      ...empty,
      parts,
      error: "Failed to decode token. Check base64url formatting.",
    };
  }
}

function formatJson(value: Record<string, unknown> | null): string {
  if (!value) return "";
  return JSON.stringify(value, null, 2);
}

export default function TokenTab() {
  const [tokenInput, setTokenInput] = useState("");

  const parsed = useMemo(() => parseJwt(tokenInput), [tokenInput]);
  const hasToken = tokenInput.trim().length > 0;

  return (
    <div className="max-w-7xl mx-auto px-5 py-10 space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-app-text">
            <KeyRound className="w-5 h-5 text-app-accent" />
            <h3 className="text-2xl font-bold tracking-tight">JWT Inspector</h3>
          </div>
          <p className="text-app-subtext text-xs uppercase tracking-widest font-medium opacity-70">
            Parse bearer tokens locally. No network calls.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setTokenInput("")}
          className="text-app-subtext">
          Clear
        </Button>
      </div>

      <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3">
        <label className="text-[10px] text-app-subtext uppercase tracking-widest">
          Bearer token
        </label>
        <TextArea
          value={tokenInput}
          onInput={(e: any) => setTokenInput(e.target.value)}
          placeholder="Paste a JWT or Authorization: Bearer <token> here..."
          className="min-h-[130px] text-xs font-mono bg-background border-app-border/50 text-app-text"
        />
        <div className="flex items-center gap-2 text-[10px] text-app-subtext">
          {parsed.error ? (
            <>
              <ShieldAlert className="w-3.5 h-3.5 text-red-400" />
              <span className="text-red-500">{parsed.error}</span>
            </>
          ) : hasToken ? (
            <>
              <ShieldCheck className="w-3.5 h-3.5 text-green-400" />
              <span>Decoded successfully. Signature not verified.</span>
            </>
          ) : (
            <span>Paste a token to see decoded header and payload.</span>
          )}
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3">
          <div className="text-xs font-semibold text-app-text">Header</div>
          <pre className="min-h-[220px] max-h-[320px] overflow-auto rounded-lg bg-app-panel border border-app-border/40 p-3 text-[11px] text-app-text whitespace-pre-wrap break-words">
            {parsed.header
              ? formatJson(parsed.header)
              : parsed.headerRaw
              ? parsed.headerRaw
              : "Header will appear here..."}
          </pre>
        </div>

        <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3">
          <div className="text-xs font-semibold text-app-text">Payload</div>
          <pre className="min-h-[220px] max-h-[320px] overflow-auto rounded-lg bg-app-panel border border-app-border/40 p-3 text-[11px] text-app-text whitespace-pre-wrap ">
            {parsed.payload
              ? formatJson(parsed.payload)
              : parsed.payloadRaw
              ? parsed.payloadRaw
              : "Payload will appear here..."}
          </pre>
        </div>

        <div className="bg-app-card border border-app-border rounded-xl p-4 space-y-3">
          <div className="text-xs font-semibold text-app-text">Signature</div>
          <div className="min-h-[220px] max-h-[320px] overflow-auto rounded-lg bg-app-panel border border-app-border/40 p-3 text-[11px] text-app-subtext whitespace-pre-wrap break-words">
            {parsed.signature || "Signature (base64url) will appear here..."}
          </div>
          <div className="text-[10px] text-app-subtext">
            Parts: {parsed.parts.length || 0}
          </div>
        </div>
      </div>
    </div>
  );
}
