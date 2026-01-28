// =============================================================================
// Mock Server - Type Definitions and Helper Functions
// =============================================================================

// Constants
// -----------------------------------------------------------------------------

export const METHODS = ["GET", "POST", "PATCH", "DELETE", "OPTIONS", "PUT"] as const;
export const MATCH_MODES = ["exact", "contains", "regex"] as const;

// Type Definitions
// -----------------------------------------------------------------------------

export type HttpMethod = (typeof METHODS)[number];
export type MatchMode = (typeof MATCH_MODES)[number];
export type ResponseBodyType = "none" | "form_data" | "form_urlencode" | "raw";
export type RawSubType = "text" | "json" | "xml" | "html" | "javascript";
export type BodyType = "raw_json" | "raw_xml" | "form_data" | "form_urlencode";

export interface MockKeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

export interface FormDataItem {
  key: string;
  value: string;
  type: 'text' | 'file';
  enabled: boolean;
}

export interface MockBodyMatch {
  mode: MatchMode;
  value: string;
  bodyType?: BodyType;
  formData?: FormDataItem[];
  formUrlencode?: MockKeyValue[];
  validationStrategy?: 'exact' | 'key_only';
}

export interface MockRouteMatchers {
  queryParams: MockKeyValue[];
  headers: MockKeyValue[];
  body?: MockBodyMatch | null;
}

export interface MockResponse {
  status: number;
  headers: MockKeyValue[];
  body: string;
  bodyType: ResponseBodyType;
  rawSubType?: RawSubType;
  formData?: FormDataItem[];
  formUrlencode?: MockKeyValue[];
  delayMs?: number | null;
}

export interface MockRoute {
  id: string;
  name: string;
  enabled: boolean;
  method: HttpMethod;
  path: string;
  matchers: MockRouteMatchers;
  response: MockResponse;
}

export interface MockServerConfig {
  port: number;
  routes: MockRoute[];
}

export interface MockServerStatus {
  running: boolean;
  port: number;
  url: string;
  routeCount: number;
}

export interface LogEntry {
  time: string;
  level: string;
  source: string;
  message: string;
}

// Factory Functions
// -----------------------------------------------------------------------------

export const createKeyValue = (): MockKeyValue => ({
  key: "",
  value: "",
  enabled: true,
});

export const createRoute = (): MockRoute => ({
  id:
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `mock_${Date.now()}_${Math.random().toString(36).slice(2)}`,
  name: "New mock",
  enabled: true,
  method: "GET",
  path: "/api/v1/resource",
  matchers: {
    queryParams: [],
    headers: [],
    body: null,
  },
  response: {
    status: 200,
    headers: [
      { key: "Content-Type", value: "application/json", enabled: true }
    ],
    body: "{\n  \"status\": \"success\",\n  \"data\": {\n    \"id\": \"123\",\n    \"message\": \"Hello World\"\n  }\n}",
    bodyType: "raw",
    rawSubType: "json",
    formData: [],
    formUrlencode: [],
    delayMs: null,
  },
});

// Utility Functions
// -----------------------------------------------------------------------------

export const cloneRoute = (route: MockRoute): MockRoute =>
  JSON.parse(JSON.stringify(route)) as MockRoute;

export const cloneConfig = (config: MockServerConfig): MockServerConfig =>
  JSON.parse(JSON.stringify(config)) as MockServerConfig;

export const getPlaceholderForRawType = (subType?: RawSubType): string => {
  switch (subType) {
    case "json":
      return '{\n  "status": "success",\n  "data": {}\n}';
    case "xml":
      return '<?xml version="1.0"?>\n<root></root>';
    case "html":
      return '<!DOCTYPE html>\n<html><body></body></html>';
    case "javascript":
      return '// JavaScript code\nconsole.log("Hello");';
    default:
      return 'Raw text content';
  }
};

export const normalizeConfig = (config: MockServerConfig): MockServerConfig => ({
  ...config,
  routes: config.routes.map((route) => ({
    ...route,
    matchers: {
      queryParams: route.matchers.queryParams ?? [],
      headers: route.matchers.headers ?? [],
      body: route.matchers.body ? {
        ...route.matchers.body,
        bodyType: route.matchers.body.bodyType ?? "raw_json",
        formData: route.matchers.body.formData ?? [],
        formUrlencode: route.matchers.body.formUrlencode ?? [],
        validationStrategy: route.matchers.body.validationStrategy ?? "exact",
      } : null,
    },
    response: {
      ...route.response,
      headers: route.response.headers ?? [],
      bodyType: route.response.bodyType ?? "raw",
      rawSubType: route.response.rawSubType ?? "json",
      formData: route.response.formData ?? [],
      formUrlencode: route.response.formUrlencode ?? [],
      delayMs: route.response.delayMs ?? null,
    },
  })),
});

export const serializeConfig = (config: MockServerConfig | null): string =>
  config ? JSON.stringify(normalizeConfig(config)) : "";

// Method Badge Colors
// -----------------------------------------------------------------------------

export const getMethodBadgeColors = (method: string): string => {
  const colors: Record<string, string> = {
    GET: "text-green-400 bg-green-400/10 border-green-400/20",
    POST: "text-blue-400 bg-blue-400/10 border-blue-400/20",
    PUT: "text-orange-400 bg-orange-400/10 border-orange-400/20",
    DELETE: "text-red-400 bg-red-400/10 border-red-400/20",
    PATCH: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  };
  return colors[method] || "text-gray-400 bg-gray-400/10 border-gray-400/20";
};

// Content Type Mapping for cURL
// -----------------------------------------------------------------------------

export const getContentTypeForRawType = (subType: RawSubType): string => {
  const contentTypeMap: Record<RawSubType, string> = {
    json: "application/json",
    xml: "application/xml",
    html: "text/html",
    text: "text/plain",
    javascript: "application/javascript",
  };
  return contentTypeMap[subType];
};

// cURL Command Generator
// -----------------------------------------------------------------------------

export const generateCurlCommand = (
  route: MockRoute,
  baseUrl: string
): string => {
  const query = (route.matchers.queryParams || [])
    .filter((q) => q.enabled && q.key)
    .map((q) => {
      const key = encodeURIComponent(q.key);
      const value = encodeURIComponent(q.value || "");
      return `${key}=${value}`;
    })
    .join("&");
  const url = query ? `${baseUrl}${route.path}?${query}` : `${baseUrl}${route.path}`;

  let cmd = `curl -X ${route.method} "${url}"`;

  const requestHeaders = route.matchers.headers || [];
  requestHeaders.forEach(h => {
    if (h.enabled && h.key) cmd += ` \\\n  -H "${h.key}: ${h.value}"`;
  });

  const hasContentTypeHeader = requestHeaders.some(
    (h) => h.enabled && h.key?.toLowerCase() === "content-type"
  );
  const bodyMatcher = route.matchers.body || null;
  const bodyType = bodyMatcher?.bodyType;

  if (route.method !== 'GET') {
    switch (bodyType) {
      case 'raw_json':
        if (!hasContentTypeHeader) {
          cmd += ` \\\n  -H "Content-Type: application/json"`;
        }
        if (bodyMatcher?.value) {
          cmd += ` \\\n  --data '${bodyMatcher.value.replace(/'/g, "'\\''")}'`;
        }
        break;
      case 'raw_xml':
        if (!hasContentTypeHeader) {
          cmd += ` \\\n  -H "Content-Type: application/xml"`;
        }
        if (bodyMatcher?.value) {
          cmd += ` \\\n  --data '${bodyMatcher.value.replace(/'/g, "'\\''")}'`;
        }
        break;
      case 'form_data':
        if (bodyMatcher?.formData && bodyMatcher.formData.length > 0) {
          bodyMatcher.formData.forEach(item => {
            if (item.enabled && item.key) {
              if (item.type === 'file') {
                cmd += ` \\\n  -F "${item.key}=@${item.value || 'path/to/file'}"`;
              } else {
                cmd += ` \\\n  -F "${item.key}=${item.value}"`;
              }
            }
          });
        }
        break;
      case 'form_urlencode':
        if (!hasContentTypeHeader) {
          cmd += ` \\\n  -H "Content-Type: application/x-www-form-urlencoded"`;
        }
        if (bodyMatcher?.formUrlencode && bodyMatcher.formUrlencode.length > 0) {
          bodyMatcher.formUrlencode.forEach(item => {
            if (item.enabled && item.key) {
              cmd += ` \\\n  --data-urlencode "${item.key}=${item.value}"`;
            }
          });
        }
        break;
    }
  }

  return cmd;
};
