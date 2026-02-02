// =============================================================================
// Multi-Response Editor Component
// Manages multiple payload→response mappings for a route
// =============================================================================

import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "../../../shared/components/Button";
import { ResponseConfigEditor } from "./ResponseConfigEditor";
import { type MockRoute } from "../types";

export interface MultiResponseEditorProps {
  route: MockRoute;
  onUpdateRoute: (updater: (route: MockRoute) => MockRoute) => void;
}

export function MultiResponseEditor({ route, onUpdateRoute }: MultiResponseEditorProps) {
  const mappings = route.multiResponses || [];
  const [selectedMappingId, setSelectedMappingId] = useState<string | null>(
    mappings[0]?.id || null
  );

  const selectedMapping = mappings.find((m) => m.id === selectedMappingId);

  const removeMapping = (id: string) => {
    onUpdateRoute((r) => ({
      ...r,
      multiResponses: (r.multiResponses || []).filter((m) => m.id !== id),
    }));
    if (selectedMappingId === id) {
      const remaining = mappings.filter((m) => m.id !== id);
      setSelectedMappingId(remaining[0]?.id || null);
    }
  };

  const updateMapping = (id: string, updates: Partial<typeof mappings[0]>) => {
    onUpdateRoute((r) => ({
      ...r,
      multiResponses: (r.multiResponses || []).map((m) =>
        m.id === id ? { ...m, ...updates } : m
      ),
    }));
  };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-top-2 duration-500">
      {/* Mapping List Sidebar */}
      <div className="grid grid-cols-3 gap-6">
        {/* Left: List of mappings */}
        <div className="col-span-1 space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
              Payload Mappings ({mappings.length})
            </label>
          </div>

          <p className="text-[10px] text-app-subtext/70">
            Add/edit payloads in Body Validation. Configure responses here.
          </p>

           <div className="space-y-2 max-h-[400px] overflow-y-auto custom-scrollbar">
             {mappings.map((mapping) => (
               <div
                 key={mapping.id}
                 className={`p-3 rounded-xl border cursor-pointer transition-all relative group ${
                   selectedMappingId === mapping.id
                     ? "bg-app-accent/10 border-app-accent"
                     : "bg-app-card border-app-border hover:border-app-accent/50"
                 }`}
                 onClick={() => setSelectedMappingId(mapping.id)}
               >
                 <input
                   type="text"
                   className="bg-transparent text-xs font-bold text-app-text w-full focus:outline-none"
                   value={mapping.name}
                   onChange={(e) => updateMapping(mapping.id, { name: e.target.value })}
                   onClick={(e) => e.stopPropagation()}
                   placeholder="Mapping name"
                 />
                 <p className="text-[10px] text-app-subtext truncate mt-1">
                   Response: {mapping.response.status}
                 </p>
                 <button
                   type="button"
                   onClick={(e) => {
                     e.stopPropagation();
                     removeMapping(mapping.id);
                   }}
                   className="absolute top-2 right-2 text-app-subtext/50 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity">
                   <Trash2 className="w-3 h-3" />
                 </button>
               </div>
             ))}
           </div>
         </div>

        {/* Right: Selected mapping editor */}
        <div className="col-span-2">
          {selectedMapping ? (
            <div className="space-y-6 animate-in fade-in slide-in-from-top-2 duration-500">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-bold text-app-text">Edit Response</h4>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => removeMapping(selectedMapping.id)}
                  className="h-8 px-3 text-[10px] text-red-400 hover:text-red-300 hover:bg-red-500/10">
                  <Trash2 className="w-3 h-3 mr-1" />
                  Delete Mapping
                </Button>
              </div>

              <ResponseConfigEditor
                response={selectedMapping.response}
                onChange={(response) => updateMapping(selectedMapping.id, { response })}
              />
            </div>
          ) : (
            <div className="text-center text-app-subtext py-12">
              <p className="text-sm">No mappings configured.</p>
              <p className="text-xs mt-2">Add a mapping in Body Validation, then set its response here.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
