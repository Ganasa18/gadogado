import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { QaSession } from "../../../types/qa/types";
import { useQaSessionStore } from "../../../store/qaSession";
import { parseCurlCommand } from "../utils/curlParser";

export const API_METHODS = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "OPTIONS",
] as const;

const BODY_METHODS = new Set(["POST", "PUT", "PATCH"]);

export const HEADER_KEY_SUGGESTIONS = [
  "Content-Type",
  "Accept",
  "Authorization",
  "User-Agent",
  "Cache-Control",
  "Accept-Language",
  "X-Request-Id",
  "X-Trace-Id",
];

type ToastType = "success" | "error" | "info";

type ToastHandler = (message: string, type: ToastType) => void;

export type ApiKeyValueRow = {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
};

export type ApiFormRow = {
  id: string;
  key: string;
  value: string;
  file: File | null;
  enabled: boolean;
};

export type ApiResponsePayload = {
  status: number;
  durationMs: number;
  headers: ApiKeyValueRow[];
  body: string;
  contentType: string | null;
};

type AuthHeader = {
  key: string;
  value: string;
};

function parseAuthHeaders(authProfileJson?: string | null): AuthHeader[] {
  if (!authProfileJson) return [];
  try {
    const parsed = JSON.parse(authProfileJson);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return Object.entries(parsed)
        .filter(([key]) => typeof key === "string")
        .map(([key, entryValue]) => ({
          key,
          value:
            typeof entryValue === "string"
              ? entryValue
              : JSON.stringify(entryValue),
        }));
    }
  } catch {
    return [];
  }
  return [];
}

const createRowId = () =>
  crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

const createKeyValueRow = (overrides: Partial<ApiKeyValueRow> = {}) => ({
  id: createRowId(),
  key: "",
  value: "",
  enabled: true,
  ...overrides,
});

const createFormRow = (overrides: Partial<ApiFormRow> = {}) => ({
  id: createRowId(),
  key: "",
  value: "",
  file: null,
  enabled: true,
  ...overrides,
});

const fileToBase64 = (file: File) =>
  new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string") {
        const [, base64] = reader.result.split(",");
        resolve(base64 ?? "");
      } else {
        resolve("");
      }
    };
    reader.onerror = () => reject(new Error("Failed to read file"));
    reader.readAsDataURL(file);
  });

export type ApiRequestBuilderState = {
  apiEventType: string;
  apiMethod: (typeof API_METHODS)[number];
  apiEndpoint: string;
  apiHeaders: ApiKeyValueRow[];
  apiParams: ApiKeyValueRow[];
  apiBodyTab: "json" | "form";
  apiBodyJson: string;
  apiFormData: ApiFormRow[];
  apiActiveTab: "params" | "headers" | "body" | "response";
  apiResponse: ApiResponsePayload | null;
  apiResponseError: string | null;
  apiSending: boolean;
  supportsBody: boolean;
  requestUrlPreview: string;
  formattedResponseBody: string;
};

export type ApiRequestBuilderActions = {
  setApiEventType: (value: string) => void;
  setApiMethod: (value: (typeof API_METHODS)[number]) => void;
  setApiEndpoint: (value: string) => void;
  setApiBodyTab: (value: "json" | "form") => void;
  setApiBodyJson: (value: string) => void;
  setApiActiveTab: (value: "params" | "headers" | "body" | "response") => void;
  setApiFormData: (updater: (rows: ApiFormRow[]) => ApiFormRow[]) => void;
  setApiHeaders: (updater: (rows: ApiKeyValueRow[]) => ApiKeyValueRow[]) => void;
  setApiParams: (updater: (rows: ApiKeyValueRow[]) => ApiKeyValueRow[]) => void;
  handleSendApiRequest: () => Promise<void>;

  resetApiState: () => void;
  importCurl: (command: string) => boolean;
};

type ApiRequestBuilderArgs = {
  session: QaSession | null;
  isTauriApp: boolean;
  addToast: ToastHandler;
  onEventsRecorded: () => void;
};

export function useApiRequestBuilder({
  session,
  isTauriApp,
  addToast,
  onEventsRecorded,
}: ApiRequestBuilderArgs): {
  state: ApiRequestBuilderState;
  actions: ApiRequestBuilderActions;
} {
  const [apiEventType, setApiEventType] = useState("curl_request_success");
  const [apiMethod, setApiMethod] = useState<(typeof API_METHODS)[number]>("GET");
  const [apiEndpoint, setApiEndpoint] = useState("");
  const [apiHeaders, setApiHeaders] = useState<ApiKeyValueRow[]>([]);
  const [apiParams, setApiParams] = useState<ApiKeyValueRow[]>([]);
  const [apiBodyTab, setApiBodyTab] = useState<"json" | "form">("json");
  const [apiBodyJson, setApiBodyJson] = useState("");
  const [apiFormData, setApiFormData] = useState<ApiFormRow[]>([]);
  const [apiActiveTab, setApiActiveTab] = useState<
    "params" | "headers" | "body" | "response"
  >("params");
  const [apiResponse, setApiResponse] = useState<ApiResponsePayload | null>(null);
  const [apiResponseError, setApiResponseError] = useState<string | null>(null);
  const [apiSending, setApiSending] = useState(false);
  const { activeRunId, setActiveRunId } = useQaSessionStore();

  const supportsBody = BODY_METHODS.has(apiMethod);

  const resolveApiUrl = () => {
    const endpoint = apiEndpoint.trim();
    const baseUrl = session?.api_base_url?.trim() ?? "";

    if (!endpoint) {
      return baseUrl;
    }
    if (endpoint.startsWith("http://") || endpoint.startsWith("https://")) {
      return endpoint;
    }
    if (!baseUrl) {
      return endpoint;
    }
    return `${baseUrl.replace(/\/$/, "")}/${endpoint.replace(/^\//, "")}`;
  };

  const buildRequestUrl = () => {
    const baseUrl = resolveApiUrl();
    if (!baseUrl) return "";
    try {
      const url = new URL(baseUrl);
      apiParams
        .filter((param) => param.enabled && param.key.trim().length > 0)
        .forEach((param) => {
          url.searchParams.append(param.key.trim(), param.value.trim());
        });
      return url.toString();
    } catch {
      return baseUrl;
    }
  };

  const requestUrlPreview = useMemo(
    () => buildRequestUrl(),
    [apiEndpoint, apiParams, session?.api_base_url]
  );

  const formattedResponseBody = useMemo(() => {
    if (!apiResponse) return "";
    const body = apiResponse.body ?? "";
    const looksLikeJson =
      apiResponse.contentType?.includes("json") ||
      body.trim().startsWith("{") ||
      body.trim().startsWith("[");
    if (!looksLikeJson) return body;
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }, [apiResponse]);

  const resetApiState = () => {
    const authHeaders = parseAuthHeaders(session?.auth_profile_json).map((item) =>
      createKeyValueRow({ key: item.key, value: item.value })
    );

    setApiEventType("curl_request_success");
    setApiMethod("GET");
    setApiEndpoint("");
    setApiHeaders(authHeaders.length ? authHeaders : [createKeyValueRow()]);
    setApiParams([createKeyValueRow()]);
    setApiBodyTab("json");
    setApiBodyJson("");
    setApiFormData([createFormRow()]);
    setApiActiveTab("params");
    setApiResponse(null);
    setApiResponseError(null);
  };

  useEffect(() => {
    if (!session || session.session_type !== "api") return;
    resetApiState();
    setActiveRunId(null);
  }, [session?.id, session?.session_type, session?.auth_profile_json, setActiveRunId]);

  useEffect(() => {
    if (!supportsBody && apiActiveTab === "body") {
      setApiActiveTab("params");
    }
  }, [supportsBody, apiActiveTab]);

  const handleSendApiRequest = async () => {
    if (!session) return;
    if (!isTauriApp) {
      addToast("QA sessions are only available in the Tauri app", "error");
      return;
    }

    const requestUrl = buildRequestUrl();
    if (!requestUrl) {
      addToast("API endpoint or base URL is required", "error");
      return;
    }
    try {
      new URL(requestUrl);
    } catch {
      addToast("Enter a full URL or set a base URL", "error");
      return;
    }

    if (supportsBody && apiBodyTab === "json" && apiBodyJson.trim()) {
      try {
        JSON.parse(apiBodyJson);
      } catch {
        addToast("Request body is not valid JSON", "error");
        return;
      }
    }

    setApiSending(true);
    setApiResponseError(null);
    try {
      let runId = activeRunId;
      if (!runId) {
        const run = await invoke<{ id: string }>("qa_start_run", {
          sessionId: session.id,
          runType: "record",
          mode: "api",
          triggeredBy: "user",
          sourceRunId: null,
          checkpointId: null,
          metaJson: null,
        });
        runId = run.id;
        setActiveRunId(run.id);
      }
      const formDataPayload = await Promise.all(
        apiFormData.map(async (field) => {
          if (!field.file) {
            return {
              key: field.key,
              value: field.value,
              enabled: field.enabled,
            };
          }
          const base64 = await fileToBase64(field.file);
          return {
            key: field.key,
            value: field.value,
            enabled: field.enabled,
            fileName: field.file.name,
            fileBase64: base64,
            contentType: field.file.type,
          };
        })
      );

      const requestHeaders = apiHeaders.map(({ key, value, enabled }) => ({
        key,
        value,
        enabled,
      }));
      const requestParams = apiParams.map(({ key, value, enabled }) => ({
        key,
        value,
        enabled,
      }));

      const response = await invoke<{
        status: number;
        durationMs: number;
        headers: ApiKeyValueRow[];
        body: string;
        contentType: string | null;
      }>("qa_execute_api_request", {
        request: {
          method: apiMethod,
          url: resolveApiUrl(),
          headers: requestHeaders,
          queryParams: requestParams,
          bodyType: supportsBody ? apiBodyTab : null,
          bodyJson: supportsBody && apiBodyTab === "json" ? apiBodyJson : null,
          formData: supportsBody && apiBodyTab === "form" ? formDataPayload : [],
          source: "manual",
        },
      });

      setApiResponse({
        status: response.status,
        durationMs: response.durationMs,
        headers: response.headers.map((header) =>
          createKeyValueRow({
            key: header.key,
            value: header.value,
            enabled: header.enabled,
          })
        ),
        body: response.body,
        contentType: response.contentType,
      });
      setApiActiveTab("response");

      await invoke("qa_record_event", {
        event: {
          eventType: apiEventType,
          url: requestUrl,
          runId,
          origin: "user",
          recordingMode: "api",
          metaJson: JSON.stringify({
            method: apiMethod,
            url: requestUrl,
            status: response.status,
            timing_ms: response.durationMs,
            response_headers: response.headers,
            response_body: response.body,
            request_headers: requestHeaders,
            query_params: requestParams,
            body_type: supportsBody ? apiBodyTab : null,
            request_body: supportsBody && apiBodyTab === "json" ? apiBodyJson : undefined,
          }),
        },
        sessionId: session.id,
      });

      onEventsRecorded();
    } catch (err) {
      console.error(err);
      setApiResponseError("Failed to send API request.");
      addToast("Failed to send API request", "error");
    } finally {
      setApiSending(false);
    }
  };

  const importCurl = (command: string) => {
    const parsed = parseCurlCommand(command);
    if (!parsed) return false;

    // Check if method is valid
    const method = API_METHODS.find(m => m === parsed.method.toUpperCase()) || "GET";
    setApiMethod(method);
    
    setApiEndpoint(parsed.url);

    const newHeaders = parsed.headers.map(h => createKeyValueRow({ key: h.key, value: h.value }));
    setApiHeaders(newHeaders.length > 0 ? newHeaders : [createKeyValueRow()]);
    
    // Clear params as they often come in URL. 
    // Ideally we could parse them out of URL but leaving them in URL string works too.
    setApiParams([createKeyValueRow()]);

    if (parsed.body) {
        setApiBodyTab("json");
        try {
           const pretty = JSON.stringify(JSON.parse(parsed.body), null, 2);
           setApiBodyJson(pretty);
        } catch {
           setApiBodyJson(parsed.body);
        }
    } else {
        setApiBodyJson("");
    }
    
    return true;
  };

  return {
    state: {
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
    },
    actions: {
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
      resetApiState,
      importCurl,
    },

  };
}
