//! 核心业务逻辑模块
//!
//! 包含进程树管理、任务跟踪、共享内存等核心功能

pub mod models;
pub mod process_tree;
pub mod shared_map;

// Re-exports (used by lib.rs)
pub use models::ProcessTreeInfo;
pub use process_tree::*;
