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
pub mod process_registry;
pub mod provider;
pub mod pwait_mode;
pub mod registry;
pub mod registry_factory;
pub mod signal;
pub mod storage;
pub mod supervisor;
pub mod sync;
pub mod task_record;
pub mod tui;
pub mod utils;
pub mod wait_mode;

pub mod cli_manager;

// Re-export commonly used types for convenience
pub use cli_manager::{CliTool, CliToolDetector, InstallType};
pub use core::models::*;
pub use core::process_tree::{get_process_tree, ProcessTreeError};
pub use error::RegistryError;
pub use process_registry::InProcessRegistry;
pub use registry::{RegistryEntry, TaskRegistry};
pub use registry_factory::{RegistryFactory, RegistryType, TaskSource};
pub use supervisor::ProcessError;
pub use task_record::{TaskRecord, TaskStatus};
pub use wait_mode::WaitError;
