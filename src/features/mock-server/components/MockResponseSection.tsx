// =============================================================================
// Mock Response Section Component
// Form for configuring response status, headers, and body
// =============================================================================

import { Copy } from "lucide-react";
import { Input } from "../../../shared/components/Input";
import { Select } from "../../../shared/components/Select";
import { TextArea } from "../../../shared/components/TextArea";
import { Button } from "../../../shared/components/Button";
import { KeyValueEditor } from "./KeyValueEditor";
import { FormDataEditor } from "./FormDataEditor";
import type { MockRoute, ResponseBodyType, RawSubType } from "../types";
import { createKeyValue, getPlaceholderForRawType } from "../types";

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

  const bodyType = route.response.bodyType;
  const rawSubType = route.response.rawSubType || "json";

  return (
    <section className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-500 delay-200">
      <div className="flex items-center justify-between border-b border-app-border pb-2">
        <div className="flex items-center gap-2 text-app-subtext">
          <Copy className="w-4 h-4 -scale-x-100" />
          <h3 className="text-xs font-bold uppercase tracking-widest">
            Mock Response
          </h3>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[10px] uppercase text-app-subtext font-semibold">
            Status
          </span>
          <Input
            type="number"
            className="w-20 h-7 text-xs bg-app-card border-app-border text-center font-mono"
            value={route.response.status}
            onChange={(e) =>
              updateResponse({
                ...route.response,
                status: parseInt(e.target.value) || 200,
              })
            }
          />
        </div>
      </div>

      {/* Body Type Selector */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <label className="text-[10px] uppercase text-app-subtext font-semibold">
            Body Type
          </label>
          <Select
            options={[
              { label: "None", value: "none" },
              { label: "Form Data", value: "form_data" },
              { label: "x-www-form-urlencoded", value: "form_urlencode" },
              { label: "Raw", value: "raw" },
            ]}
            value={bodyType}
            onChange={(v) =>
              updateResponse({ ...route.response, bodyType: v as ResponseBodyType })
            }
            className="h-8 bg-app-card border-app-border text-xs w-48"
          />
        </div>

        {/* Raw Body with subtype selector */}
        {bodyType === "raw" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-app-subtext">Raw Type</span>
              <Select
                options={[
                  { label: "Text", value: "text" },
                  { label: "JSON", value: "json" },
                  { label: "XML", value: "xml" },
                  { label: "HTML", value: "html" },
                  { label: "JavaScript", value: "javascript" },
                ]}
                value={rawSubType}
                onChange={(v) =>
                  updateResponse({ ...route.response, rawSubType: v as RawSubType })
                }
                className="h-7 bg-app-card border-app-border text-xs w-32"
              />
            </div>
            <div className="relative group">
              <TextArea
                className="font-mono text-xs min-h-50 bg-[#1e1e1e] border-app-border text-emerald-100/80 leading-relaxed"
                value={route.response.body}
                onChange={(e) =>
                  updateResponse({ ...route.response, body: e.target.value })
                }
                placeholder={getPlaceholderForRawType(rawSubType)}
              />
              <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                {rawSubType === "json" && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 px-2 bg-app-panel border border-app-border text-app-subtext shadow-sm text-[10px]"
                    onClick={() => {
                      try {
                        const fmt = JSON.stringify(
                          JSON.parse(route.response.body),
                          null,
                          2
                        );
                        updateResponse({ ...route.response, body: fmt });
                      } catch (e) {}
                    }}
                  >
                    Prettify
                  </Button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Form Data (multipart) */}
        {bodyType === "form_data" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-app-subtext">Form Fields</span>
            </div>
            <FormDataEditor
              items={route.response.formData || []}
              onChange={(items) =>
                updateResponse({ ...route.response, formData: items })
              }
              placeholder={{ key: "Key", value: "Value", fileValue: "File path" }}
              emptyMessage="No form fields defined"
              showAddButton={true}
              addButtonLabel="ADD FIELD"
              onAddClick={() =>
                updateResponse({
                  ...route.response,
                  formData: [
                    ...(route.response.formData || []),
                    { key: "", value: "", type: "text", enabled: true },
                  ],
                })
              }
            />
          </div>
        )}

        {/* x-www-form-urlencoded */}
        {bodyType === "form_urlencode" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-app-subtext">
                URL Encoded Fields
              </span>
            </div>
            <KeyValueEditor
              items={route.response.formUrlencode || []}
              onChange={(items) =>
                updateResponse({ ...route.response, formUrlencode: items })
              }
              placeholder={{ key: "Key", value: "Value" }}
              emptyMessage="No urlencoded fields defined"
              showAddButton={true}
              addButtonLabel="ADD FIELD"
              onAddClick={() =>
                updateResponse({
                  ...route.response,
                  formUrlencode: [
                    ...(route.response.formUrlencode || []),
                    createKeyValue(),
                  ],
                })
              }
            />
          </div>
        )}
      </div>
    </section>
  );
}
