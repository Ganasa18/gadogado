import type { ApiFormRow, ApiKeyValueRow } from "../../hooks/useApiRequestBuilder";

export const createKeyValueRow = (overrides: Partial<ApiKeyValueRow> = {}) => ({
  id: crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
  key: "",
  value: "",
  enabled: true,
  ...overrides,
});

export const createFormRow = (overrides: Partial<ApiFormRow> = {}) => ({
  id: crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
  key: "",
  value: "",
  file: null,
  enabled: true,
  ...overrides,
});
