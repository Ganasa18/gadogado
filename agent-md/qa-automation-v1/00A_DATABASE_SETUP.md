# DATABASE SCHEMA — SQLite (MVP AI QA Recorder)

## Design Principles

- SQLite is the single source of truth
- LLM is stateless; database stores all memory
- No binary data in DB (only file paths)
- Explicit relations, no cascading deletes
- Simple INTEGER timestamps (epoch ms)

---

## Tasks (Progress)

- [x] Finalize schema.sql location and load path
- [x] Implement DB init + table creation on startup
  - Run log: Initialized QA database at: C:\Users\YSHA-PC\AppData\Roaming\com.ysha-pc.gadogado\qa_recorder.db
  - Run log: QA database file created: C:\Users\YSHA-PC\AppData\Roaming\com.ysha-pc.gadogado\qa_recorder.db (110592 bytes)
  - Run log: QA database ready, proceeding to app database init
- [x] Create all indexes during init
- [ ] Validate foreign key rules in code (no cascade deletes)
- [x] Add basic DB health check on app launch

---

## Schema Location

- src-tauri/src/resources/qa/schema.sql

---

## TABLE: sessions

Purpose:
Represents one QA recording session (browser or API).


Columns:

- id TEXT PRIMARY KEY
- title TEXT NOT NULL
- goal TEXT NOT NULL
- session_type TEXT NOT NULL -- browser | api
- is_positive_case INTEGER NOT NULL DEFAULT 1
- target_url TEXT
- api_base_url TEXT
- auth_profile_json TEXT
- source_session_id TEXT
- app_version TEXT
- os TEXT
- started_at INTEGER NOT NULL
- ended_at INTEGER
- notes TEXT


Indexes:

- idx_sessions_started_at

---

## TABLE: events

Purpose:
Stores recorded user actions or API calls in sequence.


Columns:

- id TEXT PRIMARY KEY
- session_id TEXT NOT NULL
- seq INTEGER NOT NULL
- ts INTEGER NOT NULL
- event_type TEXT NOT NULL
- selector TEXT
- element_text TEXT
- value TEXT
- url TEXT
- screenshot_id TEXT
- meta_json TEXT

Indexes:

- idx_events_session_seq (session_id, seq)
- idx_events_session_ts (session_id, ts)

---

## TABLE: artifacts

Purpose:
Stores metadata for screenshots or videos saved on disk.

Columns:

- id TEXT PRIMARY KEY
- session_id TEXT NOT NULL
- event_id TEXT
- type TEXT NOT NULL -- screenshot | video
- path TEXT NOT NULL
- mime TEXT
- width INTEGER
- height INTEGER
- created_at INTEGER NOT NULL

Indexes:

- idx_artifacts_session
- idx_artifacts_event

---

## TABLE: checkpoints

Purpose:
Splits a session into logical flow segments to control LLM context.

Columns:

- id TEXT PRIMARY KEY
- session_id TEXT NOT NULL
- seq INTEGER NOT NULL
- title TEXT
- start_event_seq INTEGER NOT NULL
- end_event_seq INTEGER NOT NULL
- created_at INTEGER NOT NULL

Indexes:

- idx_checkpoints_session_seq

---

## TABLE: checkpoint_summaries

Purpose:
Stores compressed memory per checkpoint for LLM usage.

Columns:

- id TEXT PRIMARY KEY
- checkpoint_id TEXT NOT NULL
- summary_text TEXT NOT NULL
- entities_json TEXT
- risks_json TEXT
- created_at INTEGER NOT NULL

Indexes:

- idx_checkpoint_summaries_checkpoint

---

## TABLE: test_cases

Purpose:
Stores AI-generated and curated test cases.

Columns:

- id TEXT PRIMARY KEY
- session_id TEXT NOT NULL
- checkpoint_id TEXT
- type TEXT NOT NULL -- negative | edge | exploratory | regression
- title TEXT NOT NULL
- steps_json TEXT NOT NULL
- expected TEXT
- priority TEXT -- P0 | P1 | P2
- status TEXT -- new | running | passed | failed
- dedup_hash TEXT NOT NULL
- created_at INTEGER NOT NULL

Indexes:

- idx_test_cases_session
- idx_test_cases_checkpoint
- idx_test_cases_dedup

---

## TABLE: replay_runs

Purpose:
Stores replay outcomes for browser and API sessions.

Columns:

- id TEXT PRIMARY KEY
- session_id TEXT NOT NULL
- checkpoint_id TEXT
- mode TEXT NOT NULL -- browser | api
- status TEXT NOT NULL -- running | passed | failed
- error TEXT
- meta_json TEXT -- includes API method/status when event_type is api_*
- started_at INTEGER NOT NULL
- ended_at INTEGER

Indexes:

- idx_replay_runs_session
- idx_replay_runs_checkpoint

---

## TABLE: llm_runs


Purpose:
Audit log of all LLM calls (debugging, dedup, cost tracking).

Columns:

- id TEXT PRIMARY KEY
- scope TEXT NOT NULL -- chunk | checkpoint | session
- scope_id TEXT NOT NULL
- model TEXT NOT NULL
- prompt_version TEXT
- input_digest TEXT
- input_summary TEXT
- output_json TEXT NOT NULL
- created_at INTEGER NOT NULL

Indexes:

- idx_llm_runs_scope
- idx_llm_runs_model

---

## FOREIGN KEY RULES (MVP)

- Foreign keys are logical only
- Do NOT use ON DELETE CASCADE
- Cleanup is handled explicitly in code

---

## schema.sql (READY TO USE)

```sql
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

CREATE TABLE IF NOT EXISTS replay_runs (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  checkpoint_id TEXT,
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
```
