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
-- NOTE: source_correction_id reference added later after corrections table exists
CREATE TABLE IF NOT EXISTS dataset_items (
  item_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES datasets(dataset_id) ON DELETE CASCADE,
  prompt TEXT NOT NULL,
  expected_output TEXT,
  metadata_json TEXT,
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
-- NOTE: base_version_id reference added later after model_versions table exists
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

-- Teacher-generated soft labels for knowledge distillation.
-- Stores cached probability distributions from teacher models, enabling offline training.
CREATE TABLE IF NOT EXISTS soft_labels (
  soft_label_id TEXT PRIMARY KEY,
  prompt TEXT NOT NULL,
  prompt_hash TEXT NOT NULL UNIQUE,  -- SHA256 hash for deduplication lookup
  teacher_model_id TEXT NOT NULL REFERENCES models(model_id),
  teacher_output TEXT NOT NULL,  -- Text output from teacher
  soft_label_type TEXT NOT NULL CHECK(soft_label_type IN ('logits', 'one_hot', 'text_only')),
  -- soft_label_type values:
  --   'logits': Full probability distribution from local teacher (Float32 array in soft_labels_blob)
  --   'one_hot': One-hot distribution for API teachers (Float32 array in soft_labels_blob)
  --   'text_only': Text output only, use supervised loss (soft_labels_blob is NULL)
  soft_labels_blob BLOB,  -- Binary: Float32 array [seq_len, vocab_size] or NULL
  temperature REAL NOT NULL DEFAULT 1.0,
  metadata_json TEXT,  -- Additional metadata (generation params, token timestamps, etc.)
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Link corrections to soft labels (many-to-many, supports reusing soft labels across corrections)
CREATE TABLE IF NOT EXISTS correction_soft_labels (
  correction_id TEXT NOT NULL REFERENCES corrections(correction_id) ON DELETE CASCADE,
  soft_label_id TEXT NOT NULL REFERENCES soft_labels(soft_label_id) ON DELETE CASCADE,
  PRIMARY KEY (correction_id, soft_label_id)
);

-- Link dataset items to soft labels (for golden sets and synthetic data)
CREATE TABLE IF NOT EXISTS dataset_item_soft_labels (
  item_id TEXT NOT NULL REFERENCES dataset_items(item_id) ON DELETE CASCADE,
  soft_label_id TEXT NOT NULL REFERENCES soft_labels(soft_label_id) ON DELETE CASCADE,
  PRIMARY KEY (item_id, soft_label_id)
);

-- Link training runs to soft labels (specifies which soft labels to use for each run)
-- This enables explicit soft label selection per run, with fallback to correction/dataset defaults
CREATE TABLE IF NOT EXISTS run_soft_labels (
  run_id TEXT NOT NULL REFERENCES training_runs(run_id) ON DELETE CASCADE,
  soft_label_id TEXT NOT NULL REFERENCES soft_labels(soft_label_id) ON DELETE CASCADE,
  PRIMARY KEY (run_id, soft_label_id)
);

-- Indexes.
CREATE INDEX IF NOT EXISTS idx_corrections_created_at ON corrections(created_at);
CREATE INDEX IF NOT EXISTS idx_training_runs_status_start ON training_runs(status, start_time);
CREATE INDEX IF NOT EXISTS idx_run_corrections_run ON run_corrections(run_id);
CREATE INDEX IF NOT EXISTS idx_run_corrections_correction ON run_corrections(correction_id);
CREATE INDEX IF NOT EXISTS idx_versions_model_created ON model_versions(model_id, created_at);
CREATE INDEX IF NOT EXISTS idx_eval_version ON evaluation_metrics(version_id);
CREATE INDEX IF NOT EXISTS idx_logs_run_time ON training_logs(run_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_soft_labels_prompt_hash ON soft_labels(prompt_hash);
CREATE INDEX IF NOT EXISTS idx_soft_labels_teacher_model ON soft_labels(teacher_model_id);
CREATE INDEX IF NOT EXISTS idx_soft_labels_teacher_hash ON soft_labels(teacher_model_id, prompt_hash);

-- Deferred foreign keys (added after all tables exist to avoid circular dependencies)
-- These are created without actual foreign key constraints to avoid creation order issues
-- The application layer is responsible for maintaining referential integrity
