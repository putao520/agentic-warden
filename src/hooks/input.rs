//! Claude Code hook input data structures.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Input data from Claude Code hooks (SessionEnd, PreCompact).
///
/// This structure is received via stdin when Claude Code triggers a hook.
///
/// # Example JSON Input
/// ```json
/// {
///   "session_id": "session-abc123",
///   "transcript_path": "/home/user/.claude/sessions/2025-11-14.jsonl",
///   "hook_event_name": "SessionEnd",
///   "cwd": "/home/user/project",
///   "permission_mode": "normal"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeHookInput {
    /// Session ID from Claude Code.
    ///
    /// **Important**: This is the canonical source of session_id, not the JSONL file.
    pub session_id: String,

    /// Path to the JSONL transcript file.
    pub transcript_path: String,

    /// Name of the hook event that triggered this call.
    ///
    /// Typical values: "SessionEnd", "PreCompact"
    pub hook_event_name: String,

    /// Current working directory when hook was triggered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Permission mode of the Claude Code session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

impl ClaudeCodeHookInput {
    /// Parse hook input from stdin.
    pub fn from_stdin() -> anyhow::Result<Self> {
        let input: Self = serde_json::from_reader(std::io::stdin())?;
        Ok(input)
    }

    /// Get transcript path as PathBuf.
    pub fn transcript_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.transcript_path)
    }
}
