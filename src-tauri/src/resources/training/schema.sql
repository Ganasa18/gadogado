PRAGMA foreign_keys = ON;

-- Registered models (teacher or student).
CREATE TABLE IF NOT EXISTS models (
  model_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  provider TEXT NOT NULL CHECK(provider IN ('local', 'api')),
  model_family TEXT,
  default_artifact_path TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Logical datasets (golden/corrections/synthetic).
CREATE TABLE IF NOT EXISTS datasets (
  dataset_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK(type IN ('corrections', 'golden', 'synthetic')),
  description TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Dataset items for golden/synthetic (and optionally corrections-derived snapshots).
CREATE TABLE IF NOT EXISTS dataset_items (
  item_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES datasets(dataset_id) ON DELETE CASCADE,
  prompt TEXT NOT NULL,
  expected_output TEXT,
  metadata_json TEXT,
  source_correction_id TEXT REFERENCES corrections(correction_id) ON DELETE SET NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Human corrections.
CREATE TABLE IF NOT EXISTS corrections (
  correction_id TEXT PRIMARY KEY,
  prompt TEXT NOT NULL,
  student_output TEXT NOT NULL,
  corrected_output TEXT NOT NULL,
  accuracy_rating INTEGER NOT NULL CHECK(accuracy_rating BETWEEN 1 AND 5),
  relevance_rating INTEGER CHECK(relevance_rating BETWEEN 1 AND 5),
  safety_rating INTEGER CHECK(safety_rating BETWEEN 1 AND 5),
  domain_notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Normalized tags.
CREATE TABLE IF NOT EXISTS tags (
  tag_id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS correction_tags (
  correction_id TEXT NOT NULL REFERENCES corrections(correction_id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
  PRIMARY KEY (correction_id, tag_id)
);

-- Training runs.
CREATE TABLE IF NOT EXISTS training_runs (
  run_id TEXT PRIMARY KEY,
  student_model_id TEXT NOT NULL REFERENCES models(model_id),
  base_version_id TEXT REFERENCES model_versions(version_id),
  teacher_model_id TEXT REFERENCES models(model_id),
  method TEXT NOT NULL CHECK(method IN ('fine_tune', 'knowledge_distillation', 'hybrid')),
  status TEXT NOT NULL CHECK(status IN ('queued', 'running', 'completed', 'failed', 'cancelled', 'rolled_back')),
  start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
  end_time DATETIME,
  hyperparams_json TEXT NOT NULL,
  seed INTEGER,
  failure_reason TEXT
);

-- Run -> corrections mapping (reproducible selection + split + weight).
CREATE TABLE IF NOT EXISTS run_corrections (
  run_id TEXT NOT NULL REFERENCES training_runs(run_id) ON DELETE CASCADE,
  correction_id TEXT NOT NULL REFERENCES corrections(correction_id) ON DELETE CASCADE,
  split TEXT NOT NULL CHECK(split IN ('train', 'val', 'test')),
  weight REAL NOT NULL DEFAULT 1.0 CHECK(weight > 0),
  PRIMARY KEY (run_id, correction_id)
);

-- Run -> dataset mapping (for golden sets, synthetic, etc.).
CREATE TABLE IF NOT EXISTS run_datasets (
  run_id TEXT NOT NULL REFERENCES training_runs(run_id) ON DELETE CASCADE,
  dataset_id TEXT NOT NULL REFERENCES datasets(dataset_id) ON DELETE CASCADE,
  split TEXT NOT NULL CHECK(split IN ('train', 'val', 'test')),
  weight REAL NOT NULL DEFAULT 1.0 CHECK(weight > 0),
  PRIMARY KEY (run_id, dataset_id, split)
);

-- Produced model versions.
CREATE TABLE IF NOT EXISTS model_versions (
  version_id TEXT PRIMARY KEY,
  model_id TEXT NOT NULL REFERENCES models(model_id) ON DELETE CASCADE,
  run_id TEXT REFERENCES training_runs(run_id) ON DELETE SET NULL,
  parent_version_id TEXT REFERENCES model_versions(version_id) ON DELETE SET NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  is_promoted INTEGER NOT NULL DEFAULT 0 CHECK(is_promoted IN (0, 1)),
  promoted_at DATETIME,
  artifact_path TEXT NOT NULL,
  artifact_hash TEXT,
  artifact_size_bytes INTEGER,
  notes TEXT
);

-- Active version pointer per model.
CREATE TABLE IF NOT EXISTS model_actives (
  model_id TEXT PRIMARY KEY REFERENCES models(model_id) ON DELETE CASCADE,
  version_id TEXT NOT NULL REFERENCES model_versions(version_id),
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Training logs.
CREATE TABLE IF NOT EXISTS training_logs (
  log_id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL REFERENCES training_runs(run_id) ON DELETE CASCADE,
  epoch INTEGER NOT NULL,
  step INTEGER NOT NULL,
  loss REAL,
  lr REAL,
  temperature REAL,
  cpu_util REAL,
  ram_usage_mb INTEGER,
  gpu_util REAL,
  timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Evaluation metrics.
CREATE TABLE IF NOT EXISTS evaluation_metrics (
  metric_id INTEGER PRIMARY KEY AUTOINCREMENT,
  version_id TEXT NOT NULL REFERENCES model_versions(version_id) ON DELETE CASCADE,
  dataset_id TEXT NOT NULL REFERENCES datasets(dataset_id) ON DELETE CASCADE,
  metric_name TEXT NOT NULL,
  metric_value REAL NOT NULL,
  evaluated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(version_id, dataset_id, metric_name)
);

-- Run artifacts for retention/cleanup.
CREATE TABLE IF NOT EXISTS run_artifacts (
  artifact_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES training_runs(run_id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('config', 'log', 'checkpoint', 'adapter', 'merged_model', 'gguf')),
  path TEXT NOT NULL,
  hash TEXT,
  size_bytes INTEGER,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes.
CREATE INDEX IF NOT EXISTS idx_corrections_created_at ON corrections(created_at);
CREATE INDEX IF NOT EXISTS idx_training_runs_status_start ON training_runs(status, start_time);
CREATE INDEX IF NOT EXISTS idx_run_corrections_run ON run_corrections(run_id);
CREATE INDEX IF NOT EXISTS idx_run_corrections_correction ON run_corrections(correction_id);
CREATE INDEX IF NOT EXISTS idx_versions_model_created ON model_versions(model_id, created_at);
CREATE INDEX IF NOT EXISTS idx_eval_version ON evaluation_metrics(version_id);
CREATE INDEX IF NOT EXISTS idx_logs_run_time ON training_logs(run_id, timestamp);
