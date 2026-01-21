// =============================================================================
// Mock Server Feature - Public API
// =============================================================================

// Main export (default)
export { default } from "./pages/MockServerTab";
export { default as MockServerTab } from "./pages/MockServerTab";

// Types
export type {
  HttpMethod,
  MatchMode,
  ResponseBodyType,
  RawSubType,
  BodyType,
  MockKeyValue,
  FormDataItem,
  MockBodyMatch,
  MockRouteMatchers,
  MockResponse,
  MockRoute,
  MockServerConfig,
  MockServerStatus,
  LogEntry,
} from "./types";

// Re-export hooks for convenience
export {
  useMockServerConfig,
  useRouteManagement,
  useTrafficLogs,
  useCurlGenerator,
  useCopyToClipboard,
} from "./hooks";

// Re-export components for convenience
export {
  MethodBadge,
  KeyValueEditor,
  FormDataEditor,
  EndpointSidebar,
  QuickSwitcherSidebar,
  TrafficLogsView,
  RouteEditorHeader,
  EndpointDefinitionSection,
  IncomingRequestSection,
  MockResponseSection,
  CurlCommandSection,
} from "./components";

// Re-export API client for advanced use cases
export { mockServerApi, getMockServerLogs } from "./api/client";
