use crate::config::{CLAUDE_BIN, CODEX_BIN, GEMINI_BIN};

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
        match self {
            CliType::Claude => {
                vec![
                    "-p".to_string(),
                    "--dangerously-skip-permissions".to_string(),
                    prompt.to_string(),
                ]
            }
            CliType::Codex => {
                vec![
                    "exec".to_string(),
                    "--dangerously-bypass-approvals-and-sandbox".to_string(),
                    prompt.to_string(),
                ]
            }
            CliType::Gemini => {
                vec![
                    "--approval-mode".to_string(),
                    "yolo".to_string(),
                    prompt.to_string(),
                ]
            }
        }
    }

    /// 构建交互模式启动参数（不包含提示词）
    pub fn build_interactive_args(&self) -> Vec<String> {
        match self {
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
        }
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
    let arg = arg.to_lowercase();

    if arg == "all" {
        return Some(CliSelector::all());
    }

    // 处理组合语法：claude|gemini|codex
    if arg.contains('|') {
        let mut types = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for part in arg.split('|') {
            if let Some(cli_type) = parse_cli_type(part.trim()) {
                // 避免重复
                if seen.insert(cli_type.clone()) {
                    types.push(cli_type);
                }
            } else {
                return None; // 无效的CLI类型
            }
        }

        if types.is_empty() {
            None
        } else {
            Some(CliSelector::from_multiple(types))
        }
    } else {
        // 单个CLI
        parse_cli_type(&arg).map(CliSelector::from_single)
    }
}
