#![allow(dead_code)]
// TODO: tighten this allow once all commands/state are wired.

pub mod commands_registry;
pub mod distillation;
pub mod rag_commands;

pub(crate) mod core_commands;
pub(crate) mod mock_server_commands;
pub mod qa;
pub(crate) mod state;

pub use state::{cleanup_child_processes, AppState};

pub(crate) use state::{DistillTrainerHandle, QaRecorderHandle};
