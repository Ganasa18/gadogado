// =============================================================================
// Incoming Request Section Component
// Form for configuring request matchers (headers, body validation)
// =============================================================================

import { ShieldCheck, Layers } from "lucide-react";
import { Select } from "../../../shared/components/Select";
import { TextArea } from "../../../shared/components/TextArea";
import { Button } from "../../../shared/components/Button";
import { KeyValueEditor } from "./KeyValueEditor";
import { FormDataEditor } from "./FormDataEditor";
import type { MockRoute, BodyType } from "../types";
import { createKeyValue } from "../types";

export interface IncomingRequestSectionProps {
  route: MockRoute;
  onUpdateRoute: (updater: (route: MockRoute) => MockRoute) => void;
  showOnly?: 'headers' | 'body';
}

export function IncomingRequestSection({
  route,
  onUpdateRoute,
  showOnly,
}: IncomingRequestSectionProps) {
  const updateHeaders = (headers: typeof route.matchers.headers) => {
    onUpdateRoute((r) => ({
      ...r,
      matchers: { ...r.matchers, headers },
    }));
  };

  const updateBody = (body: typeof route.matchers.body) => {
    onUpdateRoute((r) => ({
      ...r,
      matchers: { ...r.matchers, body },
    }));
  };

  const bodyType = route.matchers.body?.bodyType || "none";
  const bodyMode = route.matchers.body?.mode || "contains";
  const validationStrategy = route.matchers.body?.validationStrategy || "exact";

  return (
    <div className="space-y-12">
      {/* Step 2: Headers Matcher */}
      {(!showOnly || showOnly === 'headers') && (
        <section className="space-y-6">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h3 className="text-xs font-bold text-app-text uppercase tracking-widest flex items-center gap-2">
                STEP 2: HEADERS MATCHER
              </h3>
              <p className="text-[11px] text-app-subtext/60">Specify required headers for this endpoint to match.</p>
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="h-9 px-4 rounded-xl bg-app-card text-app-subtext hover:text-app-text border border-app-border transition-all text-[11px] font-bold"
              onClick={() =>
                updateHeaders([...route.matchers.headers, createKeyValue()])
              }>
              + ADD HEADER
            </Button>
          </div>

          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <KeyValueEditor
              items={route.matchers.headers}
              onChange={updateHeaders}
              placeholder={{ key: "X-Header-Name", value: "expected-value" }}
              emptyMessage="No header requirements defined yet."
              showAddButton={false}
            />
          </div>
        </section>
      )}

      {/* Step 3: Body Validation */}
      {(!showOnly || showOnly === 'body') && (
        <section className="space-y-6 pt-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h3 className="text-xs font-bold text-app-text uppercase tracking-widest flex items-center gap-2">
                BODY VALIDATION
              </h3>
              <p className="text-[11px] text-app-subtext/60">Define how the request body should be validated.</p>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-6">
            <div className="space-y-2">
              <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
                Body Type
              </label>
              <Select
                options={[
                  { label: "No Body Validation", value: "none" },
                  { label: "Application/JSON", value: "raw_json" },
                  { label: "Application/XML", value: "raw_xml" },
                  { label: "Multipart Form Data", value: "form_data" },
                  { label: "URL Encoded Form", value: "form_urlencode" },
                ]}
                value={bodyType}
                onChange={(v) => {
                  const newBodyType = v as BodyType;
                  updateBody(
                    v === "none"
                      ? null
                      : {
                          mode: bodyMode,
                          bodyType: newBodyType,
                          value: route.matchers.body?.value || "",
                          formData: route.matchers.body?.formData || [],
                          formUrlencode: route.matchers.body?.formUrlencode || [],
                          validationStrategy: validationStrategy,
                        },
                  );
                }}
                className="h-11 bg-app-card border-app-border rounded-xl text-xs w-full"
              />
            </div>

            {bodyType !== 'none' && (
              <div className="space-y-2">
                <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
                  Validation Strategy
                </label>
                <div className="flex bg-app-card p-1 rounded-xl border border-app-border h-11">
                  <button
                    onClick={() => updateBody(route.matchers.body ? { ...route.matchers.body, validationStrategy: 'exact' } : null)}
                    className={`flex-1 flex items-center justify-center gap-2 rounded-lg text-[10px] font-bold transition-all ${
                      validationStrategy === 'exact' 
                        ? "bg-app-accent text-white" 
                        : "text-app-subtext hover:text-app-text"
                    }`}>
                    <ShieldCheck className="w-3 h-3" />
                    EXACT VALUE
                  </button>
                  <button
                    onClick={() => updateBody(route.matchers.body ? { ...route.matchers.body, validationStrategy: 'key_only' } : null)}
                    className={`flex-1 flex items-center justify-center gap-2 rounded-lg text-[10px] font-bold transition-all ${
                      validationStrategy === 'key_only' 
                        ? "bg-app-accent text-white" 
                        : "text-app-subtext hover:text-app-text"
                    }`}>
                    <Layers className="w-3 h-3" />
                    KEY MATCH ONLY
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Raw JSON/XML Body Validation */}
          {(bodyType === "raw_json" || bodyType === "raw_xml") && (
            <div className="space-y-2 animate-in fade-in slide-in-from-top-2 duration-300">
              <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
                {bodyType === "raw_json" ? "EXPECTED JSON BODY" : "EXPECTED XML BODY"}
              </label>
              <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
                <TextArea
                  className="font-mono text-xs min-h-[300px] bg-transparent border-0 leading-relaxed text-app-text p-6 focus:ring-0"
                  value={route.matchers.body?.value || ""}
                  placeholder={
                    bodyType === "raw_json"
                      ? '{\n  "status": "success",\n  "id": 123\n}'
                      : '<?xml version="1.0"?>\n<root>\n  <id>123</id>\n</root>'
                  }
                  onChange={(e) =>
                    updateBody(
                      route.matchers.body
                        ? { ...route.matchers.body, value: e.target.value }
                        : null,
                    )
                  }
                />
              </div>
            </div>
          )}

          {/* Form Data Validation */}
          {bodyType === "form_data" && (
            <div className="space-y-4 animate-in fade-in slide-in-from-top-2 duration-300">
              <div className="flex items-center justify-between px-1">
                <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
                  Expected Form Fields
                </span>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 px-3 rounded-lg bg-app-card text-app-subtext hover:text-app-text border border-app-border transition-all text-[10px] font-bold"
                  onClick={() =>
                    updateBody(
                      route.matchers.body
                        ? {
                            ...route.matchers.body,
                            formData: [
                              ...(route.matchers.body.formData || []),
                              { key: "", value: "", type: "text", enabled: true },
                            ],
                          }
                        : null,
                    )
                  }>
                  + ADD FIELD
                </Button>
              </div>
              <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
                <FormDataEditor
                  items={route.matchers.body?.formData || []}
                  onChange={(items) =>
                    updateBody(
                      route.matchers.body
                        ? { ...route.matchers.body, formData: items }
                        : null,
                    )
                  }
                  placeholder={{
                    key: "Field Key",
                    value: "Expected value",
                    fileValue: "File path pattern",
                  }}
                  emptyMessage="No form field validation defined."
                  showAddButton={false}
                />
              </div>
            </div>
          )}

          {/* x-www-form-urlencoded Validation */}
          {bodyType === "form_urlencode" && (
            <div className="space-y-4 animate-in fade-in slide-in-from-top-2 duration-300">
              <div className="flex items-center justify-between px-1">
                <span className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
                  Expected URL Encoded Fields
                </span>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 px-3 rounded-lg bg-app-card text-app-subtext hover:text-app-text border border-app-border transition-all text-[10px] font-bold"
                  onClick={() =>
                    updateBody(
                      route.matchers.body
                        ? {
                            ...route.matchers.body,
                            formUrlencode: [
                              ...(route.matchers.body.formUrlencode || []),
                              createKeyValue(),
                            ],
                          }
                        : null,
                    )
                  }>
                  + ADD FIELD
                </Button>
              </div>
              <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
                <KeyValueEditor
                  items={route.matchers.body?.formUrlencode || []}
                  onChange={(items) =>
                    updateBody(
                      route.matchers.body
                        ? { ...route.matchers.body, formUrlencode: items }
                        : null,
                    )
                  }
                  placeholder={{ key: "key", value: "expected_value" }}
                  emptyMessage="No urlencoded field validation defined."
                  showAddButton={false}
                />
              </div>
            </div>
          )}
        </section>
      )}
    </div>
  );
}
