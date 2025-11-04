//! Agentic-Warden Library
//!
//! Universal AI agent manager with shared-memory task tracking, process management, and multi-agent orchestration.

pub mod cli_type;
pub mod commands;
pub mod config;
pub mod core;
pub mod error;
pub mod logging;
pub mod platform;
pub mod provider;
pub mod registry;
pub mod signal;
pub mod supervisor;
pub mod sync;
pub mod task_record;
pub mod tui;
pub mod utils;
pub mod wait_mode;

pub mod cli_manager;

// Re-export commonly used types for convenience
pub use cli_manager::{CliTool, CliToolDetector, InstallType};
pub use core::{ProcessTreeError, ProcessTreeInfo, get_process_tree};
pub use core::models::*;
pub use registry::{RegistryEntry, RegistryError, TaskRegistry};
pub use supervisor::ProcessError;
pub use task_record::{TaskRecord, TaskStatus};
pub use wait_mode::WaitError;
