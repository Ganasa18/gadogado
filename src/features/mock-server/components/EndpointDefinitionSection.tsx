// =============================================================================
// Endpoint Definition Section Component
// Form for editing endpoint method and path
// =============================================================================

import { LayoutGrid } from "lucide-react";
import { Input } from "../../../shared/components/Input";
import { Select } from "../../../shared/components/Select";
import type { MockRoute, HttpMethod } from "../types";
import { METHODS } from "../types";

export interface EndpointDefinitionSectionProps {
  route: MockRoute;
  onUpdateRoute: (updater: (route: MockRoute) => MockRoute) => void;
}

export function EndpointDefinitionSection({
  route,
  onUpdateRoute,
}: EndpointDefinitionSectionProps) {
  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 text-app-subtext">
        <LayoutGrid className="w-4 h-4" />
        <h3 className="text-xs font-bold uppercase tracking-widest">
          Endpoint Definition
        </h3>
      </div>
      <div className="grid grid-cols-[120px_1fr] gap-4">
        <div className="space-y-1">
          <label className="text-[10px] uppercase text-app-subtext font-semibold pl-1">
            Method
          </label>
          <Select
            options={METHODS.map((m) => ({ label: m, value: m }))}
            value={route.method}
            onChange={(v) =>
              onUpdateRoute((r) => ({ ...r, method: v as HttpMethod }))
            }
            className="h-10 bg-app-card border-app-border"
          />
        </div>
        <div className="space-y-1">
          <label className="text-[10px] uppercase text-app-subtext font-semibold pl-1">
            Request URL Path
          </label>
          <Input
            value={route.path}
            onChange={(e) =>
              onUpdateRoute((r) => ({ ...r, path: e.target.value }))
            }
            className="h-10 bg-app-card border-app-border font-mono text-sm"
            placeholder="/api/v1/resource"
          />
        </div>
      </div>
    </section>
  );
}
