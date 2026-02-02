// =============================================================================
// Response Config Editor Component
// Shared response configuration UI (status, content-type, body)
// =============================================================================

import { Input } from "../../../shared/components/Input";
import { Select } from "../../../shared/components/Select";
import { TextArea } from "../../../shared/components/TextArea";
import { Button } from "../../../shared/components/Button";
import { KeyValueEditor } from "./KeyValueEditor";
import { FormDataEditor } from "./FormDataEditor";
import type { MockResponse, ResponseBodyType, RawSubType } from "../types";
import { createKeyValue, getPlaceholderForRawType } from "../types";

export interface ResponseConfigEditorProps {
  response: MockResponse;
  onChange: (response: MockResponse) => void;
}

export function ResponseConfigEditor({
  response,
  onChange,
}: ResponseConfigEditorProps) {
  const bodyType = response.bodyType;
  const rawSubType = response.rawSubType || "json";

  return (
    <div className="space-y-6">
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
              value={response.status.toString()}
              onChange={(v) =>
                onChange({
                  ...response,
                  status: parseInt(typeof v === "string" ? v : v[0]) || 200,
                })
              }
              className="h-11 bg-app-card border-app-border rounded-xl text-xs flex-1 font-bold"
              searchable={false}
            />
            <Input
              type="number"
              className="w-20 h-11 bg-app-card border-app-border rounded-xl text-center font-mono text-sm text-app-text"
              value={response.status}
              onChange={(e) =>
                onChange({
                  ...response,
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
              const value = typeof v === "string" ? v : v[0];
              if (value.startsWith('raw_')) {
                onChange({
                  ...response,
                  bodyType: 'raw',
                  rawSubType: value.replace('raw_', '') as RawSubType
                });
              } else {
                onChange({
                  ...response,
                  bodyType: value as ResponseBodyType,
                  rawSubType: 'json'
                });
              }
            }}
            className="h-11 bg-app-card border-app-border rounded-xl text-xs w-full font-bold"
            searchable={false}
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
                  const fmt = JSON.stringify(JSON.parse(response.body), null, 2);
                  onChange({ ...response, body: fmt });
                } catch (e) {}
              }}>
              Prettify JSON
            </Button>
          )}
        </div>

        {bodyType === "raw" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <TextArea
              className="font-mono text-xs min-h-[300px] bg-transparent border-0 leading-relaxed text-app-text p-6 focus:ring-0"
              value={response.body}
              onChange={(e) => onChange({ ...response, body: e.target.value })}
              placeholder={getPlaceholderForRawType(rawSubType)}
            />
          </div>
        ) : bodyType === "form_data" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <FormDataEditor
              items={response.formData || []}
              onChange={(items) => onChange({ ...response, formData: items })}
              placeholder={{ key: "Key", value: "Value", fileValue: "Path" }}
              emptyMessage="No form fields defined yet."
              showAddButton={true}
              onAddClick={() => onChange({
                ...response,
                formData: [...(response.formData || []), { key: "", value: "", type: "text", enabled: true }]
              })}
            />
          </div>
        ) : bodyType === "form_urlencode" ? (
          <div className="bg-app-card rounded-2xl border border-app-border overflow-hidden">
            <KeyValueEditor
              items={response.formUrlencode || []}
              onChange={(items) => onChange({ ...response, formUrlencode: items })}
              placeholder={{ key: "Key", value: "Value" }}
              emptyMessage="No urlencoded fields defined yet."
              showAddButton={true}
              onAddClick={() => onChange({
                ...response,
                formUrlencode: [...(response.formUrlencode || []), createKeyValue()]
              })}
            />
          </div>
        ) : (
          <div className="h-32 flex items-center justify-center bg-app-card rounded-2xl border border-app-border text-app-subtext/30 text-xs italic">
            No response body will be sent.
          </div>
        )}
      </div>
    </div>
  );
}
