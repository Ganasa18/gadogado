// =============================================================================
// Payload Mapping Editor Component
// Edits a single payload→response mapping
// =============================================================================

import { Trash2 } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { TextArea } from "../../../shared/components/TextArea";
import { ResponseConfigEditor } from "./ResponseConfigEditor";
import type { PayloadResponseMapping, MultiResponseMatchMode } from "../types";

export interface PayloadMappingEditorProps {
  mapping: PayloadResponseMapping;
  matchMode: MultiResponseMatchMode;
  onUpdate: (updates: Partial<PayloadResponseMapping>) => void;
  onDelete: () => void;
}

export function PayloadMappingEditor({
  mapping,
  matchMode,
  onUpdate,
  onDelete,
}: PayloadMappingEditorProps) {
  const isKeyMatch = matchMode === "key_match";

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-top-2 duration-500">
      {/* Header with delete button */}
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-bold text-app-text">Edit Mapping</h4>
        <Button
          size="sm"
          variant="ghost"
          onClick={onDelete}
          className="h-8 px-3 text-[10px] text-red-400 hover:text-red-300 hover:bg-red-500/10">
          <Trash2 className="w-3 h-3 mr-1" />
          Delete Mapping
        </Button>
      </div>

      {/* Payload Input */}
      <div className="space-y-2">
        <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
          {isKeyMatch ? "Expected Keys (JSON for key extraction)" : "Expected Payload (Exact JSON Match)"}
        </label>
        <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
          <TextArea
            className="font-mono text-xs min-h-[200px] bg-transparent border-0 leading-relaxed text-app-text p-4 focus:ring-0"
            value={mapping.payload}
            onChange={(e) => onUpdate({ payload: e.target.value })}
            placeholder='{\n  "key": "value"\n}'
          />
        </div>
        <p className="text-[10px] text-app-subtext px-1">
          {isKeyMatch
            ? "This JSON defines the keys to match. Requests with matching keys will use this response."
            : "The incoming request body must match this exactly (whitespace ignored)."}
        </p>
        {isKeyMatch && (
          <div className="flex items-center gap-2 px-2 py-1.5 bg-blue-500/10 border border-blue-500/20 rounded-lg">
            <div className="w-1.5 h-1.5 rounded-full bg-blue-500" />
            <span className="text-[10px] text-blue-400">
              Keys from this JSON will be extracted and matched against incoming requests
            </span>
          </div>
        )}
      </div>

      {/* Response Configuration */}
      <div className="space-y-4">
        <h5 className="text-xs font-bold text-app-text uppercase tracking-widest">
          Response to Return
        </h5>

        <ResponseConfigEditor
          response={mapping.response}
          onChange={(response) => onUpdate({ response })}
        />
      </div>
    </div>
  );
}
