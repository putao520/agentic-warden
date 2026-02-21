//! Agentic-Warden Library
//!
//! Universal AI agent manager with shared-memory task tracking, process management, and multi-agent orchestration.

pub mod cli_oauth;
pub mod cli_type;
pub mod auto_mode;
pub mod commands;
pub mod common;
pub mod config;
pub mod core;
pub mod error;
pub mod logging;
pub mod mcp;
pub mod mcp_routing;
pub mod platform;
pub mod provider;
pub mod pwait_mode;
pub mod registry;
pub mod registry_factory;
pub mod roles;
pub mod signal;
pub mod storage;
pub mod supervisor;
pub mod sync;
pub mod task_record;
pub mod tui;
pub mod unified_registry;
pub mod utils;
pub mod wait_mode;
pub mod worktree;

pub mod cli_manager;

// Re-export commonly used types for convenience
pub use cli_manager::{execute_update, execute_enhanced_update, CliTool, CliToolDetector, InstallType};
pub use core::models::*;
pub use core::process_tree::{get_process_tree, ProcessTreeError};
pub use error::RegistryError;
pub use registry_factory::{
    create_cli_registry, create_cli_registry_for_pid, create_cli_registry_with_namespace,
    create_mcp_registry, CliRegistry, McpRegistry,
};
pub use storage::{CleanupEvent, CleanupReason, RegistryEntry, TaskStorage};
pub use supervisor::ProcessError;
pub use task_record::{TaskRecord, TaskStatus};
pub use unified_registry::Registry;
pub use wait_mode::WaitError;
