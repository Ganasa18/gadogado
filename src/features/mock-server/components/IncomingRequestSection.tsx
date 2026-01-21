// =============================================================================
// Incoming Request Section Component
// Form for configuring request matchers (headers, body validation)
// =============================================================================

import { ChevronRight } from "lucide-react";
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
}

export function IncomingRequestSection({
  route,
  onUpdateRoute,
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

  return (
    <section className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-500 delay-100">
      <div className="flex items-center justify-between border-b border-app-border pb-2">
        <div className="flex items-center gap-2 text-app-subtext">
          <ChevronRight className="w-4 h-4" />
          <h3 className="text-xs font-bold uppercase tracking-widest">
            Incoming Request
          </h3>
        </div>
      </div>

      <div className="space-y-4">
        {/* Headers Matcher */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="text-[10px] uppercase text-app-subtext font-semibold">
              Headers Matcher
            </label>
            <Button
              size="sm"
              variant="ghost"
              className="h-6 text-[10px] text-app-accent"
              onClick={() =>
                updateHeaders([...route.matchers.headers, createKeyValue()])
              }>
              ADD HEADER
            </Button>
          </div>
          <KeyValueEditor
            items={route.matchers.headers}
            onChange={updateHeaders}
            placeholder={{ key: "Header Name", value: "Value" }}
            emptyMessage="No header requirements defined"
            showAddButton={false}
          />
        </div>

        {/* Body Validation */}
        <div className="space-y-4 pt-4">
          <div className="flex items-center justify-between">
            <label className="text-[10px] uppercase text-app-subtext font-semibold">
              Body Validation Schema
            </label>
            <Select
              options={[
                { label: "None", value: "none" },
                { label: "Raw JSON", value: "raw_json" },
                { label: "Raw XML", value: "raw_xml" },
                { label: "Form Data", value: "form_data" },
                { label: "x-www-form-urlencoded", value: "form_urlencode" },
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
                      },
                );
              }}
              className="h-7 bg-app-card border-app-border text-xs w-40"
            />
          </div>

          {/* Raw JSON/XML Body Validation */}
          {(bodyType === "raw_json" || bodyType === "raw_xml") && (
            <div className="relative group">
              <TextArea
                className="font-mono text-xs min-h-30 bg-app-card/50 border-app-border leading-relaxed"
                value={route.matchers.body?.value || ""}
                placeholder={
                  bodyType === "raw_json"
                    ? '{ "key": "value"... }'
                    : '<?xml version="1.0"?><root></root>'
                }
                onChange={(e) =>
                  updateBody(
                    route.matchers.body
                      ? { ...route.matchers.body, value: e.target.value }
                      : null,
                  )
                }
              />
              <div className="absolute bottom-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <span className="text-[10px] text-app-subtext bg-app-bg px-2 py-1 rounded border border-app-border">
                  Mode:{" "}
                  {bodyMode === "exact"
                    ? "Exact Match"
                    : bodyMode === "regex"
                      ? "Regex"
                      : "Contains"}
                </span>
              </div>
            </div>
          )}

          {/* Form Data Validation */}
          {bodyType === "form_data" && (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-app-subtext">
                  Expected Form Fields
                </span>
              </div>
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
                  key: "Key",
                  value: "Expected value",
                  fileValue: "File path",
                }}
                emptyMessage="No form field validation defined"
                showAddButton={true}
                addButtonLabel="ADD FIELD"
                onAddClick={() =>
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
                }
              />
            </div>
          )}

          {/* x-www-form-urlencoded Validation */}
          {bodyType === "form_urlencode" && (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-app-subtext">
                  Expected URL Encoded Fields
                </span>
              </div>
              <KeyValueEditor
                items={route.matchers.body?.formUrlencode || []}
                onChange={(items) =>
                  updateBody(
                    route.matchers.body
                      ? { ...route.matchers.body, formUrlencode: items }
                      : null,
                  )
                }
                placeholder={{ key: "Key", value: "Expected value" }}
                emptyMessage="No urlencoded field validation defined"
                showAddButton={true}
                addButtonLabel="ADD FIELD"
                onAddClick={() =>
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
                }
              />
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
