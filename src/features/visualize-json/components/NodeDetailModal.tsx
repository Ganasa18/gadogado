import { useEffect, useState } from "react";
import { Ban, Braces, Brackets, Check, Copy, Hash, ToggleLeft, Type, X } from "lucide-react";

import type { NodeDetail } from "../hooks/useNodeDetailModal";

interface NodeDetailModalProps {
  node: NodeDetail | null;
  onClose: () => void;
}

export default function NodeDetailModal({ node, onClose }: NodeDetailModalProps) {
  const [copiedContent, setCopiedContent] = useState(false);
  const [copiedPath, setCopiedPath] = useState(false);

  useEffect(() => {
    if (node) {
      setCopiedContent(false);
      setCopiedPath(false);
    }
  }, [node]);

  if (!node) return null;

  const getFormattedValue = (value: unknown): string => {
    if (value === null) return "null";
    if (typeof value === "string") return value;
    return JSON.stringify(value, null, 2);
  };

  const handleCopyContent = async () => {
    await navigator.clipboard.writeText(getFormattedValue(node.value));
    setCopiedContent(true);
    window.setTimeout(() => setCopiedContent(false), 1500);
  };

  const handleCopyPath = async () => {
    await navigator.clipboard.writeText(node.path);
    setCopiedPath(true);
    window.setTimeout(() => setCopiedPath(false), 1500);
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "object":
        return <Braces size={16} />;
      case "array":
        return <Brackets size={16} />;
      case "string":
        return <Type size={16} />;
      case "number":
        return <Hash size={16} />;
      case "boolean":
        return <ToggleLeft size={16} />;
      case "null":
        return <Ban size={16} />;
      default:
        return null;
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case "object":
        return "text-blue-400 bg-blue-500/10 border-blue-500/20";
      case "array":
        return "text-green-400 bg-green-500/10 border-green-500/20";
      case "string":
        return "text-orange-400 bg-orange-500/10 border-orange-500/20";
      case "number":
        return "text-purple-400 bg-purple-500/10 border-purple-500/20";
      case "boolean":
        return "text-pink-400 bg-pink-500/10 border-pink-500/20";
      case "null":
        return "text-gray-400 bg-gray-500/10 border-gray-500/20";
      default:
        return "text-app-subtext bg-app-panel border-app-border";
    }
  };

  return (
    <div
      className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-app-card border border-app-border rounded-xl shadow-2xl w-[90vw] max-w-lg max-h-[80vh] overflow-hidden animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className={`flex items-center justify-between px-4 py-3 border-b border-app-border ${getTypeColor(
            node.type,
          )}`}
        >
          <div className="flex items-center gap-3">
            <span className={`p-1.5 rounded-md ${getTypeColor(node.type)}`}>{getTypeIcon(node.type)}</span>
            <div>
              <h3 className="font-bold text-app-text">{node.label}</h3>
              <span className="text-[10px] text-app-subtext uppercase tracking-wider">{node.type}</span>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-app-bg/50 text-app-subtext hover:text-app-text transition"
          >
            <X size={18} />
          </button>
        </div>

        <div className="p-4 space-y-4">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-app-subtext uppercase font-semibold tracking-wide">Content</span>
              <button
                onClick={handleCopyContent}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-semibold transition ${
                  copiedContent
                    ? "bg-green-500/20 text-green-400 border border-green-500/30"
                    : "bg-app-panel hover:bg-app-accent/20 text-app-subtext hover:text-app-accent border border-app-border"
                }`}
              >
                {copiedContent ? <Check size={12} /> : <Copy size={12} />}
                {copiedContent ? "Copied!" : "Copy"}
              </button>
            </div>
            <div className="bg-app-bg rounded-lg border border-app-border p-3 max-h-[40vh] overflow-auto custom-scrollbar">
              <pre className="font-mono text-sm text-app-text whitespace-pre-wrap break-all select-text">
                {getFormattedValue(node.value)}
              </pre>
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-app-subtext uppercase font-semibold tracking-wide">JSON Path</span>
              <button
                onClick={handleCopyPath}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-semibold transition ${
                  copiedPath
                    ? "bg-green-500/20 text-green-400 border border-green-500/30"
                    : "bg-app-panel hover:bg-app-accent/20 text-app-subtext hover:text-app-accent border border-app-border"
                }`}
              >
                {copiedPath ? <Check size={12} /> : <Copy size={12} />}
                {copiedPath ? "Copied!" : "Copy"}
              </button>
            </div>
            <div className="bg-app-bg rounded-lg border border-app-border px-3 py-2">
              <code className="font-mono text-sm text-app-accent select-text">{node.path}</code>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
