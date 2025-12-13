#![allow(dead_code)] // CLI类型定义，部分类型是API的一部分

use crate::cli_manager::CliToolDetector;
use crate::config::{CLAUDE_BIN, CODEX_BIN, GEMINI_BIN};
use crate::error::{errors, AgenticResult, AgenticWardenError};
use std::collections::HashSet;
use std::env;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliType {
    Claude,
    Codex,
    Gemini,
}

#[derive(Debug, Clone)]
pub struct CliSelector {
    pub types: Vec<CliType>,
}

impl CliSelector {
    pub fn all() -> Self {
        Self {
            types: vec![CliType::Claude, CliType::Codex, CliType::Gemini],
        }
    }

    pub fn from_single(cli_type: CliType) -> Self {
        Self {
            types: vec![cli_type],
        }
    }

    pub fn from_multiple(types: Vec<CliType>) -> Self {
        Self { types }
    }
}

impl CliType {
    pub fn command_name(&self) -> &str {
        match self {
            CliType::Claude => CLAUDE_BIN,
            CliType::Codex => CODEX_BIN,
            CliType::Gemini => GEMINI_BIN,
        }
    }

    pub fn env_var_name(&self) -> &str {
        match self {
            CliType::Claude => "CLAUDE_BIN",
            CliType::Codex => "CODEX_BIN",
            CliType::Gemini => "GEMINI_BIN",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CliType::Claude => "claude",
            CliType::Codex => "codex",
            CliType::Gemini => "gemini",
        }
    }

    /// 构建非交互式完整权限命令参数
    pub fn build_full_access_args(&self, prompt: &str) -> Vec<String> {
        self.build_full_access_args_with_cli(prompt, &[])
    }

    /// 构建非交互式完整权限命令参数，包含用户透传的CLI参数
    pub fn build_full_access_args_with_cli(&self, prompt: &str, cli_args: &[String]) -> Vec<String> {
        let mut args = match self {
            CliType::Claude => {
                vec![
                    "-p".to_string(),
                    "--dangerously-skip-permissions".to_string(),
                ]
            }
            CliType::Codex => {
                vec![
                    "exec".to_string(),
                    "--dangerously-bypass-approvals-and-sandbox".to_string(),
                ]
            }
            CliType::Gemini => {
                vec![
                    "-p".to_string(),
                    "--approval-mode".to_string(),
                    "yolo".to_string(),
                ]
            }
        };

        
        args.extend(cli_args.iter().cloned());
        args.push(prompt.to_string());
        args
    }

    /// 构建交互模式启动参数（不包含提示词）
    pub fn build_interactive_args(&self) -> Vec<String> {
        self.build_interactive_args_with_cli(&[])
    }

    /// 构建交互模式启动参数（包含用户透传的CLI参数）
    pub fn build_interactive_args_with_cli(&self, cli_args: &[String]) -> Vec<String> {
        let mut args = match self {
            CliType::Claude => {
                vec![
                    "-p".to_string(),
                    "--dangerously-skip-permissions".to_string(),
                ]
            }
            CliType::Codex => {
                vec![
                    "exec".to_string(),
                    "--dangerously-bypass-approvals-and-sandbox".to_string(),
                ]
            }
            CliType::Gemini => {
                vec!["--approval-mode".to_string(), "yolo".to_string()]
            }
        };

        
        args.extend(cli_args.iter().cloned());
        args
    }
}

pub fn parse_cli_type(arg: &str) -> Option<CliType> {
    match arg.to_lowercase().as_str() {
        "claude" => Some(CliType::Claude),
        "codex" => Some(CliType::Codex),
        "gemini" => Some(CliType::Gemini),
        _ => None,
    }
}

pub fn parse_cli_selector(arg: &str) -> Option<CliSelector> {
    parse_cli_selector_strict(arg).ok()
}

pub fn parse_cli_selector_strict(arg: &str) -> AgenticResult<CliSelector> {
    let arg = arg.trim();
    if arg.is_empty() {
        return Err(unsupported_selector(arg));
    }

    let mut types = Vec::new();
    let mut seen = HashSet::new();

    for part in arg.split('|') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }

        if token.eq_ignore_ascii_case("all") {
            let discovered = resolve_all_cli_types()?;
            for cli_type in discovered {
                if seen.insert(cli_type.clone()) {
                    types.push(cli_type);
                }
            }
            continue;
        }

        if let Some(cli_type) = parse_cli_type(token) {
            if seen.insert(cli_type.clone()) {
                types.push(cli_type);
            }
            continue;
        }

        return Err(unsupported_selector(token));
    }

    if types.is_empty() {
        Err(unsupported_selector(arg))
    } else {
        Ok(CliSelector::from_multiple(types))
    }
}

fn resolve_all_cli_types() -> AgenticResult<Vec<CliType>> {
    if let Some(overridden) = forced_cli_types_from_env()? {
        return Ok(overridden);
    }

    let mut detector = CliToolDetector::new();
    detector
        .detect_all_tools()
        .map_err(|err| detection_failure(err.to_string()))?;

    let mut installed = Vec::new();
    for tool in detector.get_tools() {
        if !tool.installed {
            continue;
        }
        if let Some(cli_type) = parse_cli_type(&tool.command) {
            if !installed.contains(&cli_type) {
                installed.push(cli_type);
            }
        }
    }

    if installed.is_empty() {
        Err(no_cli_detected_error())
    } else {
        Ok(installed)
    }
}

fn forced_cli_types_from_env() -> AgenticResult<Option<Vec<CliType>>> {
    match env::var("AGENTIC_WARDEN_FORCE_CLI_ALL") {
        Ok(value) => {
            let mut types = Vec::new();
            let mut seen = HashSet::new();
            for part in value.split(',') {
                let token = part.trim();
                if token.is_empty() {
                    continue;
                }
                if let Some(cli_type) = parse_cli_type(token) {
                    if seen.insert(cli_type.clone()) {
                        types.push(cli_type);
                    }
                } else {
                    return Err(unsupported_selector(token));
                }
            }
            if types.is_empty() {
                Err(no_cli_detected_error())
            } else {
                Ok(Some(types))
            }
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(detection_failure(err.to_string())),
    }
}

fn unsupported_selector(token: impl Into<String>) -> AgenticWardenError {
    let token = token.into();
    errors::validation_error(
        format!("Unsupported agent type '{token}'. Supported types: claude, codex, gemini, all, or combinations like claude|gemini"),
        Some("cli-selector".to_string()),
        Some(token),
    )
}

fn detection_failure(reason: impl Into<String>) -> AgenticWardenError {
    AgenticWardenError::Process {
        message: format!("Failed to detect installed AI CLI tools: {}", reason.into()),
        command: "cli_tool_detection".to_string(),
        source: None,
    }
}

fn no_cli_detected_error() -> AgenticWardenError {
    errors::validation_error(
        "No AI CLI installations detected. Install claude, codex or gemini CLI tools, or set AGENTIC_WARDEN_FORCE_CLI_ALL",
        Some("cli-tools".to_string()),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCategory;
    use serial_test::serial;
    use std::env;

    #[test]
    fn parse_selector_strict_allows_composites() {
        let selector =
            parse_cli_selector_strict("codex|claude|codex").expect("selector should parse");
        assert_eq!(selector.types, vec![CliType::Codex, CliType::Claude]);
    }

    #[test]
    fn parse_selector_strict_rejects_unknown_type() {
        let err = parse_cli_selector_strict("dragon").expect_err("unknown type should fail");
        assert_eq!(err.category(), ErrorCategory::Validation);
        assert!(err.user_message().contains("Unsupported agent type"));
    }

    #[serial]
    #[test]
    fn env_override_controls_all_selector() {
        let _guard = EnvGuard::set("AGENTIC_WARDEN_FORCE_CLI_ALL", "claude, gemini");
        let selector = parse_cli_selector_strict("all").expect("env override should work");
        assert_eq!(selector.types, vec![CliType::Claude, CliType::Gemini]);
    }

    #[serial]
    #[test]
    fn env_override_empty_errors() {
        let _guard = EnvGuard::set("AGENTIC_WARDEN_FORCE_CLI_ALL", "");
        let err = parse_cli_selector_strict("all").expect_err("empty override should fail");
        assert_eq!(err.category(), ErrorCategory::Validation);
        assert!(err
            .user_message()
            .contains("No AI CLI installations detected"));
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                env::set_var(self.key, value);
            } else {
                env::remove_var(self.key);
            }
        }
    }
}
