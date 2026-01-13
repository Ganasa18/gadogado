import { useState } from "react";
import { Terminal, X, ChevronDown } from "lucide-react";
import type { QaSession } from "../../../types/qa/types";
import {
  API_METHODS,
  HEADER_KEY_SUGGESTIONS,
  type ApiFormRow,
  type ApiKeyValueRow,
  type ApiResponsePayload,
  useApiRequestBuilder,
} from "../hooks/useApiRequestBuilder";

const API_EVENT_TYPES = [
  "curl_request_success",
  "curl_request_validation_error",
  "curl_request_auth_error",
  "curl_request_server_error",
  "curl_request_unknown",
];

const createKeyValueRow = (overrides: Partial<ApiKeyValueRow> = {}) => ({
  id: crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
  key: "",
  value: "",
  enabled: true,
  ...overrides,
});

const createFormRow = (overrides: Partial<ApiFormRow> = {}) => ({
  id: crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
  key: "",
  value: "",
  file: null,
  enabled: true,
  ...overrides,
});

type ApiRequestPanelProps = {
  session: QaSession | null;
  isTauriApp: boolean;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  onEventsRecorded: () => void;
  onEndSession: () => void;
  canEndSession: boolean;
};

export default function ApiRequestPanel({
  session,
  isTauriApp,
  addToast,
  onEventsRecorded,
  onEndSession,
  canEndSession,
}: ApiRequestPanelProps) {
  const { state, actions } = useApiRequestBuilder({
    session,
    isTauriApp,
    addToast,
    onEventsRecorded,
  });

  const {
    apiEventType,
    apiMethod,
    apiEndpoint,
    apiHeaders,
    apiParams,
    apiBodyTab,
    apiBodyJson,
    apiFormData,
    apiActiveTab,
    apiResponse,
    apiResponseError,
    apiSending,
    supportsBody,
    requestUrlPreview,
    formattedResponseBody,
  } = state;

  const {
    setApiEventType,
    setApiMethod,
    setApiEndpoint,
    setApiBodyTab,
    setApiBodyJson,
    setApiActiveTab,
    setApiFormData,
    setApiHeaders,
    setApiParams,
    handleSendApiRequest,
    importCurl,
  } = actions;

  const [isCurlModalOpen, setIsCurlModalOpen] = useState(false);
  const [curlCommand, setCurlCommand] = useState("");

  const handleImportCurl = () => {
    if (!curlCommand.trim()) return;
    const success = importCurl(curlCommand);
    if (success) {
      setIsCurlModalOpen(false);
      setCurlCommand("");
      addToast("Imported cURL command", "success");
    } else {
      addToast("Failed to parse cURL command", "error");
    }
  };

  return (
    <section className="space-y-4 col-span-2">
      <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
        <div className="flex items-center justify-between gap-2">
          <div className="text-[11px] text-app-subtext uppercase tracking-wide">
            API Session
          </div>
          <button
            type="button"
            onClick={onEndSession}
            disabled={!canEndSession}
            className="text-[10px] px-3 py-1 rounded border border-red-500/40 text-red-200 hover:border-red-500/70 transition disabled:opacity-50">
            End Session
          </button>
        </div>
        <div className="mt-2 text-sm text-app-text font-semibold">
          Postman-style request builder
        </div>
        <div className="mt-2 text-[11px] text-app-subtext">
          Responses are recorded as events (e.g.{" "}
          <span className="text-blue-200">curl_request_success</span>,{" "}
          <span className="text-blue-200">curl_request_validation_error</span>).
        </div>
        <div className="mt-3 grid grid-cols-1 gap-2 text-[11px]">
          <div className="rounded-md border border-app-border bg-black/20 p-2">
            <div className="text-[10px] text-gray-500">Base URL</div>
            <div className="text-gray-300">{session?.api_base_url || "n/a"}</div>
          </div>
          <div className="rounded-md border border-app-border bg-black/20 p-2">
            <div className="text-[10px] text-gray-500">Auth profile</div>
            <div className="text-gray-300">
              {session?.auth_profile_json ? "Configured" : "n/a"}
            </div>
          </div>
        </div>
      </div>

      <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="text-[11px] text-app-subtext uppercase tracking-wide">
            API Request
          </div>
          <div className="text-[10px] text-app-subtext">Build and send API calls</div>
        </div>
        
        <button
            onClick={() => setIsCurlModalOpen(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-black/40 border border-app-border rounded hover:bg-black/60 transition text-xs text-emerald-400"
        >
            <Terminal className="w-3.5 h-3.5" />
            <span>Import cURL</span>
        </button>

        <div className="flex flex-col gap-3">
          <div className="flex flex-col md:flex-row gap-2">
            <div className="relative w-full md:w-32">
              <select
                className="w-full bg-[#181818] border border-app-border rounded p-2 pr-8 text-xs outline-none focus:border-gray-500 transition appearance-none cursor-pointer"
                value={apiMethod}
                onChange={(event) =>
                  setApiMethod(event.target.value as (typeof API_METHODS)[number])
                }>
                {API_METHODS.map((method) => (
                  <option key={method} value={method}>
                    {method}
                  </option>
                ))}
              </select>
              <div className="absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none text-app-subtext">
                <ChevronDown className="w-3.5 h-3.5" />
              </div>
            </div>
            <input
              className="flex-1 bg-[#181818] border border-app-border rounded p-2 text-xs outline-none focus:border-gray-500 transition"
              placeholder={
                session?.api_base_url
                  ? "/v1/users/123"
                  : "https://api.example.com/v1/users"
              }
              value={apiEndpoint}
              onChange={(event) => setApiEndpoint(event.target.value)}
            />
            <button
              type="button"
              onClick={handleSendApiRequest}
              disabled={apiSending}
              className="w-full md:w-auto bg-[#1a2a3a] border border-blue-800/40 rounded px-4 py-2 text-xs text-blue-200 hover:border-blue-500/60 transition disabled:opacity-50">
              {apiSending ? "Sending..." : "Send"}
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">
                Event type
              </label>
              <div className="relative">
                <select
                  className="w-full bg-[#181818] border border-app-border rounded p-2 pr-8 text-xs outline-none focus:border-gray-500 transition appearance-none cursor-pointer"
                  value={apiEventType}
                  onChange={(event) => setApiEventType(event.target.value)}>
                  {API_EVENT_TYPES.map((eventType) => (
                    <option key={eventType} value={eventType}>
                      {eventType}
                    </option>
                  ))}
                </select>
                <div className="absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none text-app-subtext">
                  <ChevronDown className="w-3.5 h-3.5" />
                </div>
              </div>
            </div>
            <div className="text-[10px] text-app-subtext">
              Full URL: {requestUrlPreview || "n/a"}
            </div>
          </div>

          <ApiTabs activeTab={apiActiveTab} onChange={setApiActiveTab} />

          {apiActiveTab === "params" && (
            <ApiKeyValueTable
              rows={apiParams}
              emptyLabel="param"
              onAdd={() => setApiParams((rows) => [...rows, createKeyValueRow()])}
              onUpdate={(id, updates) =>
                setApiParams((rows) =>
                  rows.map((row) => (row.id === id ? { ...row, ...updates } : row))
                )
              }
              onRemove={(id) =>
                setApiParams((rows) =>
                  rows.length > 1 ? rows.filter((row) => row.id !== id) : rows
                )
              }
            />
          )}

          {apiActiveTab === "headers" && (
            <ApiKeyValueTable
              rows={apiHeaders}
              emptyLabel="header"
              headerSuggestions={HEADER_KEY_SUGGESTIONS}
              onAdd={() => setApiHeaders((rows) => [...rows, createKeyValueRow()])}
              onUpdate={(id, updates) =>
                setApiHeaders((rows) =>
                  rows.map((row) => (row.id === id ? { ...row, ...updates } : row))
                )
              }
              onRemove={(id) =>
                setApiHeaders((rows) =>
                  rows.length > 1 ? rows.filter((row) => row.id !== id) : rows
                )
              }
            />
          )}

          {apiActiveTab === "body" && (
            <ApiBodyTab
              supportsBody={supportsBody}
              bodyTab={apiBodyTab}
              bodyJson={apiBodyJson}
              formData={apiFormData}
              onTabChange={setApiBodyTab}
              onBodyJsonChange={setApiBodyJson}
              onFormDataChange={setApiFormData}
            />
          )}

          {apiActiveTab === "response" && (
            <ApiResponsePanel
              response={apiResponse}
              responseError={apiResponseError}
              formattedBody={formattedResponseBody}
            />
          )}
        </div>
      </div>
      
      {/* Import cURL Modal */}
      {isCurlModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
          <div className="w-full max-w-lg bg-app-card border border-app-border rounded-lg shadow-xl flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-app-border">
              <div className="flex items-center gap-2 font-medium text-sm text-app-text">
                <Terminal className="w-4 h-4 text-emerald-400" />
                Import cURL Command
              </div>
              <button onClick={() => setIsCurlModalOpen(false)} className="text-app-subtext hover:text-app-text">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-4 space-y-4">
              <div className="text-xs text-app-subtext">
                Paste a cURL command below to populate the request fields.
              </div>
              <textarea
                value={curlCommand}
                onChange={(e) => setCurlCommand(e.target.value)}
                placeholder="curl -X POST https://api.example.com/data -H 'Content-Type: application/json' -d '...'"
                className="w-full h-32 bg-black/30 border border-app-border rounded p-3 text-xs font-mono outline-none focus:border-emerald-500/50 transition resize-none text-app-text"
                autoFocus
              />
              <div className="flex justify-end gap-2">
                <button
                  onClick={() => setIsCurlModalOpen(false)}
                  className="px-3 py-1.5 rounded border border-app-border text-xs text-app-subtext hover:bg-white/5 transition"
                >
                  Cancel
                </button>
                <button
                  onClick={handleImportCurl}
                  disabled={!curlCommand.trim()}
                  className="px-3 py-1.5 rounded bg-emerald-600/20 border border-emerald-500/50 text-xs text-emerald-100 hover:bg-emerald-600/30 transition disabled:opacity-50"
                >
                  Import
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

type ApiTabsProps = {
  activeTab: "params" | "headers" | "body" | "response";
  onChange: (tab: "params" | "headers" | "body" | "response") => void;
};

function ApiTabs({ activeTab, onChange }: ApiTabsProps) {
  const tabs = ["params", "headers", "body", "response"] as const;
  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-app-border pb-2 text-[10px]">
      {tabs.map((tab) => (
        <button
          key={tab}
          type="button"
          onClick={() => onChange(tab)}
          className={`px-3 py-1 rounded border transition ${
            activeTab === tab
              ? "bg-emerald-700/20 border-emerald-500/60 text-emerald-100"
              : "bg-[#181818] border-app-border text-app-subtext"
          }`}>
          {tab.toUpperCase()}
        </button>
      ))}
    </div>
  );
}

type ApiKeyValueTableProps = {
  rows: ApiKeyValueRow[];
  emptyLabel: string;
  headerSuggestions?: string[];
  onAdd: () => void;
  onUpdate: (id: string, updates: Partial<ApiKeyValueRow>) => void;
  onRemove: (id: string) => void;
};

function ApiKeyValueTable({
  rows,
  emptyLabel,
  headerSuggestions,
  onAdd,
  onUpdate,
  onRemove,
}: ApiKeyValueTableProps) {
  const datalistId = headerSuggestions ? "qa-header-keys" : undefined;

  return (
    <div className="space-y-2">
      {headerSuggestions && (
        <datalist id={datalistId}>
          {headerSuggestions.map((header) => (
            <option key={header} value={header} />
          ))}
        </datalist>
      )}
      {rows.map((row) => (
        <div
          key={row.id}
          className="grid grid-cols-[auto_1fr_1fr_auto] gap-2 items-center">
          <input
            type="checkbox"
            checked={row.enabled}
            onChange={(event) =>
              onUpdate(row.id, { enabled: event.target.checked })
            }
          />
          <input
            className="bg-[#181818] border border-app-border rounded p-2 text-xs"
            placeholder={emptyLabel}
            list={datalistId}
            value={row.key}
            onChange={(event) => onUpdate(row.id, { key: event.target.value })}
          />
          <input
            className="bg-[#181818] border border-app-border rounded p-2 text-xs"
            placeholder="value"
            value={row.value}
            onChange={(event) => onUpdate(row.id, { value: event.target.value })}
          />
          <button
            type="button"
            onClick={() => onRemove(row.id)}
            className="text-[10px] text-red-200">
            Remove
          </button>
        </div>
      ))}
      <button type="button" onClick={onAdd} className="text-[10px] text-emerald-200">
        + Add {emptyLabel}
      </button>
    </div>
  );
}

type ApiBodyTabProps = {
  supportsBody: boolean;
  bodyTab: "json" | "form";
  bodyJson: string;
  formData: ApiFormRow[];
  onTabChange: (tab: "json" | "form") => void;
  onBodyJsonChange: (value: string) => void;
  onFormDataChange: (updater: (rows: ApiFormRow[]) => ApiFormRow[]) => void;
};

function ApiBodyTab({
  supportsBody,
  bodyTab,
  bodyJson,
  formData,
  onTabChange,
  onBodyJsonChange,
  onFormDataChange,
}: ApiBodyTabProps) {
  return (
    <div className="space-y-3">
      {!supportsBody && (
        <div className="text-[11px] text-app-subtext">
          Selected method does not support a request body.
        </div>
      )}
      {supportsBody && (
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onTabChange("json")}
            className={`px-3 py-1 rounded border text-[10px] transition ${
              bodyTab === "json"
                ? "bg-blue-700/30 border-blue-500/60 text-blue-100"
                : "bg-[#181818] border-app-border text-app-subtext"
            }`}>
            Raw JSON
          </button>
          <button
            type="button"
            onClick={() => onTabChange("form")}
            className={`px-3 py-1 rounded border text-[10px] transition ${
              bodyTab === "form"
                ? "bg-blue-700/30 border-blue-500/60 text-blue-100"
                : "bg-[#181818] border-app-border text-app-subtext"
            }`}>
            Form-data
          </button>
        </div>
      )}
      {supportsBody && bodyTab === "json" && (
        <textarea
          className="w-full min-h-[160px] bg-[#181818] border border-app-border rounded p-2 text-xs outline-none focus:border-gray-500 transition resize-y font-mono"
          placeholder='{"name":"Ada"}'
          value={bodyJson}
          onChange={(event) => onBodyJsonChange(event.target.value)}
        />
      )}
      {supportsBody && bodyTab === "form" && (
        <div className="space-y-2">
          {formData.map((field) => (
            <div
              key={field.id}
              className="grid grid-cols-[auto_1fr_1fr_1fr_auto] gap-2 items-center">
              <input
                type="checkbox"
                checked={field.enabled}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, enabled: event.target.checked }
                        : row
                    )
                  )
                }
              />
              <input
                className="bg-[#181818] border border-app-border rounded p-2 text-xs"
                placeholder="key"
                value={field.key}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, key: event.target.value }
                        : row
                    )
                  )
                }
              />
              <input
                className="bg-[#181818] border border-app-border rounded p-2 text-xs"
                placeholder="value"
                value={field.value}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, value: event.target.value }
                        : row
                    )
                  )
                }
              />
              <input
                type="file"
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, file: event.target.files?.[0] ?? null }
                        : row
                    )
                  )
                }
              />
              <button
                type="button"
                onClick={() =>
                  onFormDataChange((rows) =>
                    rows.length > 1
                      ? rows.filter((row) => row.id !== field.id)
                      : rows
                  )
                }
                className="text-[10px] text-red-200">
                Remove
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={() => onFormDataChange((rows) => [...rows, createFormRow()])}
            className="text-[10px] text-emerald-200">
            + Add field
          </button>
        </div>
      )}
    </div>
  );
}

type ApiResponsePanelProps = {
  response: ApiResponsePayload | null;
  responseError: string | null;
  formattedBody: string;
};

function ApiResponsePanel({
  response,
  responseError,
  formattedBody,
}: ApiResponsePanelProps) {
  return (
    <div className="space-y-3">
      {responseError && (
        <div className="text-[11px] text-red-200">{responseError}</div>
      )}
      {!response && !responseError && (
        <div className="text-[11px] text-app-subtext">
          Send a request to view the response.
        </div>
      )}
      {response && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-2 text-[11px]">
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Status</div>
              <div className="text-gray-300">{response.status}</div>
            </div>
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Time</div>
              <div className="text-gray-300">{response.durationMs} ms</div>
            </div>
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Content-Type</div>
              <div className="text-gray-300">
                {response.contentType || "n/a"}
              </div>
            </div>
          </div>

          <div>
            <div className="text-[10px] text-gray-500 mb-1">Response Headers</div>
            <div className="max-h-[160px] overflow-y-auto space-y-1 text-[10px]">
              {response.headers.length === 0 && (
                <div className="text-app-subtext">No headers available.</div>
              )}
              {response.headers.map((header) => (
                <div key={header.id} className="text-app-subtext">
                  {header.key}: {header.value}
                </div>
              ))}
            </div>
          </div>

          <div>
            <div className="text-[10px] text-gray-500 mb-1">Response Body</div>
            <pre className="bg-black/30 border border-app-border rounded p-3 text-[11px] text-app-text max-h-[320px] overflow-auto whitespace-pre-wrap font-mono">
              {formattedBody || "(empty)"}
            </pre>
          </div>
        </>
      )}
    </div>
  );
}
