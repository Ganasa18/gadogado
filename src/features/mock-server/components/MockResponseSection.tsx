// =============================================================================
// Mock Response Section Component
// Form for configuring response status, headers, and body
// =============================================================================

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
    <section className="space-y-10 animate-in fade-in slide-in-from-top-2 duration-500">
      <div className="space-y-1">
        <h3 className="text-xs font-bold text-app-text uppercase tracking-widest">
          STEP 3: RESPONSE CONFIGURATION
        </h3>
        <p className="text-[11px] text-app-subtext/60">Define the payload and status code your mock server will return.</p>
      </div>

      <div className="grid grid-cols-2 gap-6">
        <div className="space-y-2">
          <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
            Status Code
          </label>
          <div className="flex gap-2">
            <Select
              options={[
                { label: "200 OK", value: "200" },
                { label: "201 Created", value: "201" },
                { label: "400 Bad Request", value: "400" },
                { label: "401 Unauthorized", value: "401" },
                { label: "403 Forbidden", value: "403" },
                { label: "404 Not Found", value: "404" },
                { label: "500 Internal Server Error", value: "500" },
              ]}
              value={route.response.status.toString()}
              onChange={(v) =>
                updateResponse({
                  ...route.response,
                  status: parseInt(v) || 200,
                })
              }
              className="h-11 bg-app-card border-app-border rounded-xl text-xs flex-1 font-bold"
            />
            <Input
              type="number"
              className="w-20 h-11 bg-app-card border-app-border rounded-xl text-center font-mono text-sm text-app-text"
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

        <div className="space-y-2">
          <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest px-1">
            Content-Type
          </label>
          <Select
            options={[
              { label: "None", value: "none" },
              { label: "Application/JSON", value: "raw_json" },
              { label: "Application/XML", value: "raw_xml" },
              { label: "Multipart Form Data", value: "form_data" },
              { label: "URL Encoded Form", value: "form_urlencode" },
              { label: "Plain Text", value: "raw" },
            ]}
            value={bodyType === 'raw' ? `raw_${rawSubType}` : bodyType}
            onChange={(v) => {
              if (v.startsWith('raw_')) {
                updateResponse({ 
                  ...route.response, 
                  bodyType: 'raw', 
                  rawSubType: v.replace('raw_', '') as RawSubType 
                });
              } else {
                updateResponse({ 
                  ...route.response, 
                  bodyType: v as ResponseBodyType,
                  rawSubType: 'json'
                });
              }
            }}
            className="h-11 bg-app-card border-app-border rounded-xl text-xs w-full font-bold"
          />
        </div>
      </div>

      {/* Response Body Editor */}
      <div className="space-y-2">
        <div className="flex items-center justify-between px-1">
          <label className="text-[10px] font-bold text-app-subtext uppercase tracking-widest">
            Response Body ({bodyType.replace('_', ' ').toUpperCase()})
          </label>
          {bodyType === "raw" && rawSubType === "json" && (
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2 text-[10px] text-app-subtext hover:text-app-text"
              onClick={() => {
                try {
                  const fmt = JSON.stringify(JSON.parse(route.response.body), null, 2);
                  updateResponse({ ...route.response, body: fmt });
                } catch (e) {}
              }}>
              Prettify JSON
            </Button>
          )}
        </div>

        {bodyType === "raw" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <TextArea
              className="font-mono text-xs min-h-[400px] bg-transparent border-0 leading-relaxed text-app-text p-6 focus:ring-0"
              value={route.response.body}
              onChange={(e) => updateResponse({ ...route.response, body: e.target.value })}
              placeholder={getPlaceholderForRawType(rawSubType)}
            />
          </div>
        ) : bodyType === "form_data" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <FormDataEditor
              items={route.response.formData || []}
              onChange={(items) => updateResponse({ ...route.response, formData: items })}
              placeholder={{ key: "Key", value: "Value", fileValue: "Path" }}
              emptyMessage="No form fields defined yet."
              showAddButton={true}
              onAddClick={() => updateResponse({
                ...route.response,
                formData: [...(route.response.formData || []), { key: "", value: "", type: "text", enabled: true }]
              })}
            />
          </div>
        ) : bodyType === "form_urlencode" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <KeyValueEditor
              items={route.response.formUrlencode || []}
              onChange={(items) => updateResponse({ ...route.response, formUrlencode: items })}
              placeholder={{ key: "Key", value: "Value" }}
              emptyMessage="No urlencoded fields defined yet."
              showAddButton={true}
              onAddClick={() => updateResponse({
                ...route.response,
                formUrlencode: [...(route.response.formUrlencode || []), createKeyValue()]
              })}
            />
          </div>
        ) : (
          <div className="h-32 flex items-center justify-center bg-app-card rounded-2xl border border-app-border text-app-subtext/30 text-xs italic">
            No response body will be sent.
          </div>
        )}
      </div>
    </section>
  );
}
