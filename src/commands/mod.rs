//! CLI 命令处理模块
//!
//! 处理所有命令行接口的解析和路由

pub mod ai_cli;
pub mod auto;
pub mod market;
pub mod mcp;
pub mod parser;
pub mod tui_commands;

// Re-exports (used by main.rs)
pub use parser::*;
