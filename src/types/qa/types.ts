export type QaSessionType = "browser" | "api";

export interface QaSession {
  id: string;
  title: string;
  goal: string;
  session_type: QaSessionType;
  is_positive_case: boolean;
  target_url?: string | null;
  api_base_url?: string | null;
  auth_profile_json?: string | null;
  source_session_id?: string | null;
  app_version?: string | null;
  os?: string | null;
  started_at: number;
  ended_at?: string | null;
  notes?: string | null;
}

export interface QaEvent {
  id: string;
  session_id: string;
  run_id?: string | null;
  checkpoint_id?: string | null;
  seq: number;
  ts: number;
  event_type: string;
  origin?: string | null;
  recording_mode?: string | null;
  selector?: string | null;
  element_text?: string | null;
  value?: string | null;
  url?: string | null;
  screenshot_id?: string | null;
  screenshot_path?: string | null;
  meta_json?: string | null;
}

export type ScreenshotResult = {
  path?: string | null;
  dataUrl?: string | null;
};

export type QaEventPage = {
  events: QaEvent[];
  total: number;
  page: number;
  pageSize: number;
};

export type QaCheckpoint = {
  id: string;
  sessionId: string;
  seq: number;
  title?: string | null;
  startEventSeq: number;
  endEventSeq: number;
  createdAt: number;
};

export type QaCheckpointSummary = {
  id: string;
  checkpointId: string;
  summaryText: string;
  entitiesJson?: string | null;
  risksJson?: string | null;
  createdAt: number;
};

export type QaTestCase = {
  id: string;
  sessionId: string;
  checkpointId?: string | null;
  type: string;
  title: string;
  stepsJson: string;
  expected?: string | null;
  priority?: string | null;
  status?: string | null;
  dedupHash: string;
  createdAt: number;
};

export type QaLlmRun = {
  id: string;
  scope: string;
  scopeId: string;
  model: string;
  promptVersion?: string | null;
  inputDigest?: string | null;
  inputSummary?: string | null;
  outputJson: string;
  createdAt: number;
};

export type QaSessionRun = {
  id: string;
  sessionId: string;
  runType: string;
  mode: string;
  status: string;
  triggeredBy: string;
  sourceRunId?: string | null;
  checkpointId?: string | null;
  startedAt: number;
  endedAt?: number | null;
  metaJson?: string | null;
};

export type QaRunStreamEvent = {
  id: string;
  runId: string;
  seq: number;
  ts: number;
  channel: string;
  level: string;
  message: string;
  payloadJson?: string | null;
};

export type ExploreResult = {
  checkpoints: QaCheckpoint[];
  summaries: QaCheckpointSummary[];
  testCases: QaTestCase[];
  llmRuns: QaLlmRun[];
  postSubmitDetected: boolean;
  detectedPatterns: string[];
};
