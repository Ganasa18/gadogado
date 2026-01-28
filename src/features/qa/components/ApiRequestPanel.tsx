import { useState } from "react";
import { Terminal, ChevronDown } from "lucide-react";
import type { QaSession } from "../../../types/qa/types";
import {
  API_METHODS,
  HEADER_KEY_SUGGESTIONS,
  useApiRequestBuilder,
} from "../hooks/useApiRequestBuilder";

import { ApiBodyTab } from "./apiRequestPanel/ApiBodyTab";
import { ApiKeyValueTable } from "./apiRequestPanel/ApiKeyValueTable";
import { ApiResponsePanel } from "./apiRequestPanel/ApiResponsePanel";
import { ApiTabs } from "./apiRequestPanel/ApiTabs";
import { CurlImportModal } from "./apiRequestPanel/CurlImportModal";
import { createKeyValueRow } from "./apiRequestPanel/rowFactories";

const API_EVENT_TYPES = [
  "curl_request_success",
  "curl_request_validation_error",
  "curl_request_auth_error",
  "curl_request_server_error",
  "curl_request_unknown",
];


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
      
      {isCurlModalOpen && (
        <CurlImportModal
          curlCommand={curlCommand}
          setCurlCommand={setCurlCommand}
          onClose={() => setIsCurlModalOpen(false)}
          onImport={handleImportCurl}
        />
      )}
    </section>
  );
}
