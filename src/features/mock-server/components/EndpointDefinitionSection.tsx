// =============================================================================
// Endpoint Definition Section Component
// Form for editing endpoint method and path
// =============================================================================

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
    <section className="space-y-8 animate-in fade-in slide-in-from-top-2 duration-500">
      <div className="space-y-1">
        <h3 className="text-xs font-bold text-app-text uppercase tracking-widest">
          STEP 1: BASE CONFIGURATION
        </h3>
        <p className="text-[11px] text-app-subtext/60">Set the request method and target URL path for this mock.</p>
      </div>

      <div className="grid grid-cols-[160px_1fr] gap-6">
        <div className="space-y-2">
          <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
            Method
          </label>
          <Select
            options={METHODS.map((m) => ({ label: m, value: m }))}
            value={route.method}
            onChange={(v) =>
              onUpdateRoute((r) => ({ ...r, method: v as HttpMethod }))
            }
            className="h-11 bg-app-card border-app-border rounded-xl font-bold text-xs"
          />
        </div>
        <div className="space-y-2">
          <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
            Request URL Path
          </label>
          <Input
            value={route.path}
            onChange={(e) =>
              onUpdateRoute((r) => ({ ...r, path: e.target.value }))
            }
            className="h-11 bg-app-card border-app-border rounded-xl font-mono text-sm text-app-text focus:ring-app-accent focus:border-app-accent transition-all"
            placeholder="/api/v1/resource"
          />
        </div>
      </div>
    </section>
  );
}
