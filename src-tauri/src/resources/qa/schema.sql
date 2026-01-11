CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  goal TEXT NOT NULL,
  is_positive_case INTEGER NOT NULL DEFAULT 1,
  app_version TEXT,
  os TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at
  ON sessions(started_at);

CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  event_type TEXT NOT NULL,
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
