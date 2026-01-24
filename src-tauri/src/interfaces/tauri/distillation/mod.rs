//! Distillation Commands Module
//!
//! This module contains all Tauri commands for model distillation:
//! - Correction Commands (Flow A - Collect Corrections)
//! - Dataset Commands (Flow B - Prepare Training Dataset)
//! - Model Commands
//! - Training Run Commands (Flow C - Run Training)
//! - Model Version Commands (Flow E - Promote Version + Rollback)
//! - Artifact and Backup Commands (Flow D - Evaluate + Compare)
//! - Evaluation Orchestrator (Rust -> Python evaluator)
//! - Python Orchestrator (Rust -> Python runner)
//! - Soft Labels Commands (Phase 1: Data Preparation)

pub mod common;
pub mod correction_commands;
pub mod dataset_commands;
pub mod model_commands;
pub mod training_run_commands;
pub mod model_version_commands;
pub mod artifact_commands;
pub mod evaluation_orchestrator;
pub mod python_orchestrator;
pub mod soft_label_commands;

// Re-export all commands for easier imports
pub use correction_commands::*;
pub use dataset_commands::*;
pub use model_commands::*;
pub use training_run_commands::*;
pub use model_version_commands::*;
pub use artifact_commands::*;
pub use evaluation_orchestrator::*;
pub use python_orchestrator::*;
pub use soft_label_commands::*;
