pub mod commands_registry;
pub mod distillation;
pub mod rag_commands;

pub(crate) mod core_commands;
pub(crate) mod mock_server_commands;
pub mod qa;
pub(crate) mod state;

pub use core_commands::*;
pub use distillation::*;
pub use mock_server_commands::*;
pub use qa::*;
pub use state::{cleanup_child_processes, AppState};

pub(crate) use state::{DistillTrainerHandle, QaRecorderHandle};
