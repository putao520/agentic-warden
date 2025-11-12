//! Memory module - Integrated semantic memory functionality from gmemory
//!
//! Provides conversation memory, semantic search, and TODO tracking capabilities
//! for AI CLI tools managed by Agentic-Warden.

pub mod config;
pub mod vector_store;
pub mod embedding;
pub mod semantic_search;
pub mod todo_manager;
pub mod memory_manager;

pub use config::MemoryConfig;
pub use memory_manager::MemoryManager;
pub use todo_manager::{TodoManager, TodoItem, TodoStatus, TodoPriority};