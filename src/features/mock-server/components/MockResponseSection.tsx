// =============================================================================
// Mock Response Section Component
// Form for configuring response status, headers, and body
// =============================================================================

import { ResponseConfigEditor } from "./ResponseConfigEditor";
import { MultiResponseEditor } from "./MultiResponseEditor";
import type { MockRoute, ResponseStrategy } from "../types";

export interface MockResponseSectionProps {
  route: MockRoute;
  onUpdateRoute: (updater: (route: MockRoute) => MockRoute) => void;
}

export function MockResponseSection({
  route,
  onUpdateRoute,
}: MockResponseSectionProps) {
  const updateResponse = (response: typeof route.response) => {
    onUpdateRoute((r) => ({ ...r, response }));
  };

  const responseStrategy: ResponseStrategy = route.responseStrategy || "single";

  return (
    <section className="space-y-10 animate-in fade-in slide-in-from-top-2 duration-500">
      <div className="space-y-1">
        <h3 className="text-xs font-bold text-app-text uppercase tracking-widest">
          STEP 3: RESPONSE CONFIGURATION
        </h3>
        <p className="text-[11px] text-app-subtext/60">
          {responseStrategy === "multi"
            ? "Configure response for each payload mapping in the Validation tab."
            : "Define the payload and status code your mock server will return."}
        </p>
      </div>

      {responseStrategy === "single" ? (
        <div className="space-y-6">
          <ResponseConfigEditor response={route.response} onChange={updateResponse} />
        </div>
      ) : (
        <MultiResponseEditor route={route} onUpdateRoute={onUpdateRoute} />
      )}
    </section>
  );
}
