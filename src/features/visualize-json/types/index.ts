export interface JsonNode {
  key: string;
  value: any;
  type: 'object' | 'array' | 'string' | 'number' | 'boolean' | 'null';
  path: string;
  depth: number;
  children?: JsonNode[];
  expanded?: boolean;
}

export interface HistoryItem {
  id: string;
  data: any;
  format: 'json' | 'csv' | 'xml' | 'yaml' | 'toml';
  filename?: string;
  timestamp: string;
}

export interface VisualizationState {
  json: JsonNode | null;
  history: any[];
  currentPath: string;
}

export interface LocalStorageData {
  visualization: VisualizationState | null;
  lastOpened: string;
}
