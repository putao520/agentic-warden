//! Codex Warden Library
//!
//! A supervisor wrapper around the Codex CLI with shared-memory task tracking
//! and process tree-based isolation features.

pub mod cli_type;
pub mod config;
pub mod logging;
pub mod platform;
pub mod process_tree;
pub mod registry;
pub mod shared_map;
pub mod signal;
pub mod supervisor;
pub mod sync;
pub mod task_record;
pub mod utils;
pub mod wait_mode;

pub mod cli_manager;
pub mod provider;
pub mod tui;

// Re-export commonly used types for convenience
pub use process_tree::{ProcessTreeError, ProcessTreeInfo, get_process_tree};
pub use registry::{RegistryEntry, RegistryError, TaskRegistry};
pub use supervisor::ProcessError;
pub use task_record::{TaskRecord, TaskStatus};
pub use wait_mode::WaitError;
pub use cli_manager::{CliManager, CliTool, InstallType};
