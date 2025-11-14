//! Parser for Claude Code JSONL transcript format.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A single message from Claude Code transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeMessage {
    /// Message ID (optional, may not be in all formats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// Role of the message author.
    pub role: String,

    /// Message content.
    pub content: String,

    /// Timestamp of the message.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,

    /// Session ID (may be in JSONL, but we prefer from hook stdin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Parser for Claude Code JSONL transcript files.
pub struct ClaudeCodeTranscriptParser;

impl ClaudeCodeTranscriptParser {
    /// Parse a Claude Code JSONL transcript file.
    ///
    /// # Arguments
    /// * `path` - Path to the .jsonl file
    ///
    /// # Returns
    /// Vector of parsed messages, or error if file cannot be read/parsed.
    pub fn parse_file(path: &Path) -> Result<Vec<ClaudeCodeMessage>> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open transcript file: {:?}", path))?;

        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result
                .with_context(|| format!("Failed to read line {} from {:?}", line_num + 1, path))?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON line
            let message: ClaudeCodeMessage = serde_json::from_str(&line).with_context(|| {
                format!(
                    "Failed to parse JSON at line {} in {:?}: {}",
                    line_num + 1,
                    path,
                    line
                )
            })?;

            messages.push(message);
        }

        Ok(messages)
    }

    /// Get total message count without full parsing (for quick stats).
    pub fn count_messages(path: &Path) -> Result<usize> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open transcript file: {:?}", path))?;

        let reader = BufReader::new(file);
        let count = reader.lines().filter(|l| l.is_ok()).count();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_claude_code_jsonl() {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write sample JSONL
        writeln!(
            temp_file,
            r#"{{"role":"user","content":"Hello","timestamp":1700000000,"message_id":"msg-1"}}"#
        )
        .unwrap();
        writeln!(
            temp_file,
            r#"{{"role":"assistant","content":"Hi there!","timestamp":1700000001,"message_id":"msg-2"}}"#
        )
        .unwrap();

        let messages = ClaudeCodeTranscriptParser::parse_file(temp_file.path()).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "Hi there!");
    }
}
