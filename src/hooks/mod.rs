//! Claude Code hooks integration for automatic conversation history indexing.
//!
//! This module handles Claude Code hook events (SessionEnd, PreCompact) to automatically
//! capture and index conversation history into the vector database for semantic search.

mod handler;
mod input;
mod parser;

pub use handler::HookHandler;
pub use input::ClaudeCodeHookInput;
pub use parser::ClaudeCodeTranscriptParser;

use anyhow::Result;

/// Main entry point for hook processing.
///
/// Reads hook input from stdin, parses transcript, generates embeddings,
/// and stores in vector database.
pub async fn handle_hook_from_stdin() -> Result<()> {
    let handler = HookHandler::new().await?;
    handler.handle_from_stdin().await
}
