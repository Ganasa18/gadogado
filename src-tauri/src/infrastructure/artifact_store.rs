use crate::domain::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

fn io_err(msg: impl Into<String>) -> AppError {
    AppError::Internal(msg.into())
}

fn invalid_input(msg: impl Into<String>) -> AppError {
    AppError::ValidationError(msg.into())
}

/// Artifact layout for the offline training/distillation subsystem.
///
/// This is intentionally rooted under the per-user `app_data_dir` so we can:
/// - keep artifacts out of the app bundle
/// - support atomic updates and rollback
/// - apply retention policies without touching immutable app resources
#[derive(Debug, Clone)]
pub struct TrainingArtifactLayout {
    root: PathBuf,
    models_base: PathBuf,
    models_versions: PathBuf,
    runs: PathBuf,
    evaluations: PathBuf,
}

impl TrainingArtifactLayout {
    pub fn new(app_data_dir: &Path) -> Self {
        let root = app_data_dir.join("training");
        let models_base = root.join("models").join("base");
        let models_versions = root.join("models").join("versions");
        let runs = root.join("runs");
        let evaluations = root.join("evaluations");
        Self {
            root,
            models_base,
            models_versions,
            runs,
            evaluations,
        }
    }

    pub fn ensure(&self) -> Result<()> {
        ensure_dir(&self.root)?;
        ensure_dir(&self.models_base)?;
        ensure_dir(&self.models_versions)?;
        ensure_dir(&self.runs)?;
        ensure_dir(&self.evaluations)?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn models_base_dir(&self) -> &Path {
        &self.models_base
    }

    pub fn models_versions_dir(&self) -> &Path {
        &self.models_versions
    }

    pub fn runs_dir(&self) -> &Path {
        &self.runs
    }

    pub fn evaluations_dir(&self) -> &Path {
        &self.evaluations
    }

    pub fn version_dir(&self, version_id: &str) -> PathBuf {
        self.models_versions.join(version_id)
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs.join(run_id)
    }

    pub fn evaluation_dir(&self, eval_id: &str) -> PathBuf {
        self.evaluations.join(eval_id)
    }
}

fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|e| io_err(format!("Failed to create dir {}: {e}", path.display())))?;
    Ok(())
}

pub fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    {
        let mut file = fs::File::create(&tmp_path).map_err(|e| {
            io_err(format!(
                "Failed to create temp file {}: {e}",
                tmp_path.display()
            ))
        })?;
        file.write_all(bytes).map_err(|e| {
            io_err(format!(
                "Failed to write temp file {}: {e}",
                tmp_path.display()
            ))
        })?;
        file.sync_all().ok();
    }

    // Best-effort atomic replace.
    // - Rename is atomic when destination does not exist.
    // - On Windows, rename cannot replace; we move old away then swap.
    if path.exists() {
        let backup = path.with_extension(format!("bak-{}", Uuid::new_v4()));
        fs::rename(path, &backup).map_err(|e| {
            io_err(format!(
                "Failed to move existing file {} to {}: {e}",
                path.display(),
                backup.display()
            ))
        })?;

        fs::rename(&tmp_path, path).map_err(|e| {
            io_err(format!(
                "Failed to rename temp file {} to {}: {e}",
                tmp_path.display(),
                path.display()
            ))
        })?;

        let _ = fs::remove_file(&backup);
        Ok(())
    } else {
        fs::rename(&tmp_path, path).map_err(|e| {
            io_err(format!(
                "Failed to rename temp file {} to {}: {e}",
                tmp_path.display(),
                path.display()
            ))
        })?;
        Ok(())
    }
}

pub fn atomic_write_dir(
    target_dir: &Path,
    populate: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    if let Some(parent) = target_dir.parent() {
        ensure_dir(parent)?;
    }

    let tmp_dir = target_dir.with_extension(format!("tmp-{}", Uuid::new_v4()));
    ensure_dir(&tmp_dir)?;

    if let Err(e) = populate(&tmp_dir) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(e);
    }

    if target_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(invalid_input(format!(
            "Target directory already exists: {}",
            target_dir.display()
        )));
    }

    fs::rename(&tmp_dir, target_dir).map_err(|e| {
        io_err(format!(
            "Failed to rename temp dir {} to {}: {e}",
            tmp_dir.display(),
            target_dir.display()
        ))
    })?;

    Ok(())
}

pub fn sha256_hex_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| {
        io_err(format!(
            "Failed to open file for hashing {}: {e}",
            path.display()
        ))
    })?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = file.read(&mut buf).map_err(|e| {
            io_err(format!(
                "Failed to read file for hashing {}: {e}",
                path.display()
            ))
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

pub fn dir_size_bytes(dir: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(dir)
        .map_err(|e| io_err(format!("Failed to read dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| io_err(format!("Failed dir entry: {e}")))?;
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| io_err(format!("Failed to stat {}: {e}", path.display())))?;

        if meta.is_dir() {
            total += dir_size_bytes(&path)?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

#[derive(Debug, Clone)]
pub struct RunRetentionPolicy {
    pub max_age_days: u64,
    pub max_runs: usize,
}

#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub deleted_run_ids: Vec<String>,
    pub freed_bytes: u64,
}

/// Best-effort cleanup of old run folders.
///
/// `protected_run_ids` should include any runs that produced promoted versions.
/// Backup strategy for training DB.
///
/// Provides:
/// - Daily rolling backups (keeps last N days)
/// - On-demand backup before version promotion
/// - Automatic cleanup of old backups
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Directory where backups are stored.
    pub backup_dir: PathBuf,
    /// Maximum number of daily backups to keep.
    pub max_daily_backups: usize,
    /// Prefix for backup files.
    pub prefix: String,
}

impl BackupConfig {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            backup_dir: app_data_dir.join("backups").join("training"),
            max_daily_backups: 7,
            prefix: "training_db".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub backup_path: PathBuf,
    pub size_bytes: u64,
    pub timestamp: String,
}

/// Ensure there is at least one backup for the current UTC day.
///
/// This is best-effort and is designed to be called at app startup.
/// - If a backup already exists for today, no new backup is created.
/// - Old backups are still cleaned up according to the retention policy.
pub fn ensure_daily_backup(db_path: &Path, config: &BackupConfig) -> Result<Option<BackupResult>> {
    ensure_dir(&config.backup_dir)?;

    let today_prefix = format!("{}_{}_", config.prefix, chrono::Utc::now().format("%Y%m%d"));

    let mut has_today = false;
    for entry in fs::read_dir(&config.backup_dir).map_err(|e| {
        io_err(format!(
            "Failed to read backup dir {}: {e}",
            config.backup_dir.display()
        ))
    })? {
        let entry = entry.map_err(|e| io_err(format!("Failed dir entry: {e}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.starts_with(&today_prefix) {
            continue;
        }

        // Ignore promotion backups for daily-check purposes.
        if file_name.contains("pre_promote") {
            continue;
        }

        has_today = true;
        break;
    }

    let created = if has_today {
        None
    } else {
        Some(backup_training_db(db_path, config, Some("daily"))?)
    };

    // Always enforce retention.
    let _ = cleanup_old_backups(config)?;

    Ok(created)
}

/// Create a backup of the training database.
///
/// The backup filename includes the current timestamp and an optional reason tag.
pub fn backup_training_db(
    db_path: &Path,
    config: &BackupConfig,
    reason: Option<&str>,
) -> Result<BackupResult> {
    ensure_dir(&config.backup_dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let reason_suffix = reason.map(|r| format!("_{}", r)).unwrap_or_default();
    let backup_name = format!("{}_{}{}.db", config.prefix, timestamp, reason_suffix);
    let backup_path = config.backup_dir.join(&backup_name);

    // Read the source database
    let db_bytes = fs::read(db_path).map_err(|e| {
        io_err(format!(
            "Failed to read training DB for backup {}: {e}",
            db_path.display()
        ))
    })?;

    // Write backup atomically
    atomic_write_bytes(&backup_path, &db_bytes)?;

    let size_bytes = db_bytes.len() as u64;

    Ok(BackupResult {
        backup_path,
        size_bytes,
        timestamp,
    })
}

/// Backup before a version promotion (special tagged backup).
pub fn backup_before_promotion(
    db_path: &Path,
    config: &BackupConfig,
    version_id: &str,
) -> Result<BackupResult> {
    let reason = format!("pre_promote_{}", &version_id[..8.min(version_id.len())]);
    backup_training_db(db_path, config, Some(&reason))
}

/// Cleanup old backups according to retention policy.
///
/// Keeps the most recent `max_daily_backups` regular backups.
/// Promotion backups are kept indefinitely (or until manually deleted).
pub fn cleanup_old_backups(config: &BackupConfig) -> Result<Vec<PathBuf>> {
    let mut deleted = Vec::new();

    if !config.backup_dir.exists() {
        return Ok(deleted);
    }

    let mut regular_backups: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in fs::read_dir(&config.backup_dir).map_err(|e| {
        io_err(format!(
            "Failed to read backup dir {}: {e}",
            config.backup_dir.display()
        ))
    })? {
        let entry = entry.map_err(|e| io_err(format!("Failed dir entry: {e}")))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip promotion backups (they contain "pre_promote")
        if file_name.contains("pre_promote") {
            continue;
        }

        // Only consider files matching our prefix
        if !file_name.starts_with(&config.prefix) {
            continue;
        }

        let meta = entry
            .metadata()
            .map_err(|e| io_err(format!("Failed to stat {}: {e}", path.display())))?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        regular_backups.push((path, mtime));
    }

    // Sort by modification time, newest first
    regular_backups.sort_by(|a, b| b.1.cmp(&a.1));

    // Delete backups beyond the retention limit
    for (path, _) in regular_backups.into_iter().skip(config.max_daily_backups) {
        if fs::remove_file(&path).is_ok() {
            deleted.push(path);
        }
    }

    Ok(deleted)
}

/// List all available backups.
pub fn list_backups(config: &BackupConfig) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();

    if !config.backup_dir.exists() {
        return Ok(backups);
    }

    for entry in fs::read_dir(&config.backup_dir).map_err(|e| {
        io_err(format!(
            "Failed to read backup dir {}: {e}",
            config.backup_dir.display()
        ))
    })? {
        let entry = entry.map_err(|e| io_err(format!("Failed dir entry: {e}")))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.starts_with(&config.prefix) {
            continue;
        }

        let meta = entry
            .metadata()
            .map_err(|e| io_err(format!("Failed to stat {}: {e}", path.display())))?;

        let is_promotion_backup = file_name.contains("pre_promote");

        backups.push(BackupInfo {
            path: path.clone(),
            file_name: file_name.to_string(),
            size_bytes: meta.len(),
            is_promotion_backup,
            modified: meta.modified().ok(),
        });
    }

    // Sort by modification time, newest first
    backups.sort_by(|a, b| b.modified.cmp(&a.modified));

    Ok(backups)
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub file_name: String,
    pub size_bytes: u64,
    pub is_promotion_backup: bool,
    pub modified: Option<SystemTime>,
}

/// Restore the training database from a backup.
///
/// Creates a backup of the current DB before restoring.
pub fn restore_from_backup(
    backup_path: &Path,
    db_path: &Path,
    config: &BackupConfig,
) -> Result<BackupResult> {
    // First, backup the current database
    let pre_restore = backup_training_db(db_path, config, Some("pre_restore"))?;

    // Read the backup
    let backup_bytes = fs::read(backup_path).map_err(|e| {
        io_err(format!(
            "Failed to read backup file {}: {e}",
            backup_path.display()
        ))
    })?;

    // Write to the DB path atomically
    atomic_write_bytes(db_path, &backup_bytes)?;

    Ok(pre_restore)
}

pub fn cleanup_old_runs(
    layout: &TrainingArtifactLayout,
    policy: &RunRetentionPolicy,
    protected_run_ids: &HashSet<String>,
) -> Result<CleanupReport> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(policy.max_age_days * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut entries: Vec<(String, SystemTime)> = Vec::new();
    for entry in fs::read_dir(layout.runs_dir()).map_err(|e| {
        io_err(format!(
            "Failed to read runs dir {}: {e}",
            layout.runs_dir().display()
        ))
    })? {
        let entry = entry.map_err(|e| io_err(format!("Failed dir entry: {e}")))?;
        let meta = entry
            .metadata()
            .map_err(|e| io_err(format!("Failed to stat {}: {e}", entry.path().display())))?;
        if !meta.is_dir() {
            continue;
        }

        let run_id = entry.file_name().to_string_lossy().to_string();
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push((run_id, mtime));
    }

    // Oldest first.
    entries.sort_by_key(|(_, t)| *t);

    let mut report = CleanupReport {
        deleted_run_ids: Vec::new(),
        freed_bytes: 0,
    };

    for (idx, (run_id, mtime)) in entries.into_iter().enumerate() {
        if protected_run_ids.contains(&run_id) {
            continue;
        }

        let too_old = mtime < cutoff;
        let over_limit = idx >= policy.max_runs;
        if !too_old && !over_limit {
            continue;
        }

        let run_dir = layout.run_dir(&run_id);
        let size = dir_size_bytes(&run_dir).unwrap_or(0);
        if fs::remove_dir_all(&run_dir).is_ok() {
            report.deleted_run_ids.push(run_id);
            report.freed_bytes += size;
        }
    }

    Ok(report)
}
