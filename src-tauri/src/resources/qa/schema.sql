CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  goal TEXT NOT NULL,
  session_type TEXT NOT NULL,
  is_positive_case INTEGER NOT NULL DEFAULT 1,
  target_url TEXT,
  api_base_url TEXT,
  auth_profile_json TEXT,
  source_session_id TEXT,
  app_version TEXT,
  os TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at
  ON sessions(started_at);

CREATE TABLE IF NOT EXISTS session_runs (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  run_type TEXT NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  triggered_by TEXT NOT NULL,
  source_run_id TEXT,
  checkpoint_id TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  meta_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_session_runs_session
  ON session_runs(session_id, started_at);

CREATE INDEX IF NOT EXISTS idx_session_runs_status
  ON session_runs(status);

CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  run_id TEXT,
  checkpoint_id TEXT,
  seq INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  origin TEXT,
  recording_mode TEXT,
  selector TEXT,
  element_text TEXT,
  value TEXT,
  url TEXT,
  screenshot_id TEXT,
  meta_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_session_seq
  ON events(session_id, seq);

CREATE INDEX IF NOT EXISTS idx_events_session_ts
  ON events(session_id, ts);

CREATE INDEX IF NOT EXISTS idx_events_run_seq
  ON events(run_id, seq);

CREATE INDEX IF NOT EXISTS idx_events_checkpoint
  ON events(checkpoint_id);

CREATE TABLE IF NOT EXISTS api_calls (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  event_request_id TEXT,
  event_response_id TEXT,
  method TEXT NOT NULL,
  url TEXT NOT NULL,
  request_headers_json TEXT,
  request_body_json TEXT,
  request_body_hash TEXT,
  response_status INTEGER,
  response_headers_json TEXT,
  response_body_hash TEXT,
  timing_ms INTEGER,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_calls_session
  ON api_calls(session_id, created_at);

CREATE INDEX IF NOT EXISTS idx_api_calls_run
  ON api_calls(run_id);

CREATE TABLE IF NOT EXISTS ai_actions (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  checkpoint_id TEXT,
  seq INTEGER NOT NULL,
  action_type TEXT NOT NULL,
  selector TEXT,
  value TEXT,
  description TEXT,
  status TEXT NOT NULL,
  resulting_event_id TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_actions_run_seq
  ON ai_actions(run_id, seq);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  event_id TEXT,
  type TEXT NOT NULL,
  path TEXT NOT NULL,
  mime TEXT,
  width INTEGER,
  height INTEGER,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_artifacts_session
  ON artifacts(session_id);

CREATE INDEX IF NOT EXISTS idx_artifacts_event
  ON artifacts(event_id);

CREATE TABLE IF NOT EXISTS checkpoints (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  title TEXT,
  start_event_seq INTEGER NOT NULL,
  end_event_seq INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_session_seq
  ON checkpoints(session_id, seq);

CREATE TABLE IF NOT EXISTS checkpoint_summaries (
  id TEXT PRIMARY KEY,
  checkpoint_id TEXT NOT NULL,
  summary_text TEXT NOT NULL,
  entities_json TEXT,
  risks_json TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checkpoint_summaries_checkpoint
  ON checkpoint_summaries(checkpoint_id);

CREATE TABLE IF NOT EXISTS test_cases (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  checkpoint_id TEXT,
  type TEXT NOT NULL,
  title TEXT NOT NULL,
  steps_json TEXT NOT NULL,
  expected TEXT,
  priority TEXT,
  status TEXT,
  dedup_hash TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_test_cases_session
  ON test_cases(session_id);

CREATE INDEX IF NOT EXISTS idx_test_cases_checkpoint
  ON test_cases(checkpoint_id);

CREATE INDEX IF NOT EXISTS idx_test_cases_dedup
  ON test_cases(dedup_hash);

CREATE TABLE IF NOT EXISTS test_case_runs (
  id TEXT PRIMARY KEY,
  test_case_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  result_json TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_test_case_runs_session
  ON test_case_runs(session_id, started_at);

CREATE INDEX IF NOT EXISTS idx_test_case_runs_case
  ON test_case_runs(test_case_id);

CREATE TABLE IF NOT EXISTS replay_runs (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  checkpoint_id TEXT,
  run_id TEXT,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  meta_json TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_replay_runs_session
  ON replay_runs(session_id);

CREATE INDEX IF NOT EXISTS idx_replay_runs_checkpoint
  ON replay_runs(checkpoint_id);

CREATE TABLE IF NOT EXISTS llm_runs (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL,
  scope_id TEXT NOT NULL,
  model TEXT NOT NULL,
  prompt_version TEXT,
  input_digest TEXT,
  input_summary TEXT,
  output_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_llm_runs_scope
  ON llm_runs(scope, scope_id);

CREATE INDEX IF NOT EXISTS idx_llm_runs_model
  ON llm_runs(model);

CREATE TABLE IF NOT EXISTS run_stream_events (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  channel TEXT NOT NULL,
  level TEXT NOT NULL,
  message TEXT NOT NULL,
  payload_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_run_stream_events_run_seq
  ON run_stream_events(run_id, seq);
