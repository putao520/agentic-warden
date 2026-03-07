//! CLI 参数解析模块
//!
//! 提供统一的 CLI 参数解析结构，分离 AIW 自有参数和透传参数

use crate::cli_type::{parse_cli_type, CliType};
use std::path::PathBuf;

/// AIW 自有参数（抽取后不转发）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiwArgs {
    /// 角色名称
    pub role: Option<String>,
    /// AIW Provider 名称（使用 -mp/--aiw-provider 参数设置）
    pub provider: Option<String>,
    /// 工作目录
    pub cwd: Option<PathBuf>,
}

/// CLI 调用的完整信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInvocation {
    /// CLI 类型
    pub cli_type: CliType,
    /// AIW 自有参数
    pub aiw_args: AiwArgs,
    /// 剩余参数原样透传，不做任何解析
    pub remaining_args: Vec<String>,
}

impl CliInvocation {
    /// 解析 external 命令（claude/codex/gemini 命令）
    ///
    /// # 参数
    /// - `tokens`: 命令行参数数组，第一个元素应该是 CLI 类型（claude/codex/gemini）
    ///
    /// # 返回
    /// - `Ok(CliInvocation)`: 解析后的调用信息
    /// - `Err(String)`: 解析错误信息
    pub fn from_external(tokens: &[String]) -> Result<Self, String> {
        if tokens.is_empty() {
            return Err("No command provided".to_string());
        }

        // 第一个 token 是 CLI 类型
        let cli_type = parse_cli_type(&tokens[0])
            .ok_or_else(|| format!("Unknown CLI type: {}", tokens[0]))?;

        // 解析剩余参数
        Self::parse_with_type(cli_type, &tokens[1..])
    }

    /// 解析 auto 命令
    ///
    /// # 参数
    /// - `tokens`: 命令行参数数组（不包含 "auto" 本身）
    pub fn from_auto(tokens: &[String]) -> Result<Self, String> {
        Self::parse_with_type(CliType::Auto, tokens)
    }

    /// 核心解析逻辑
    ///
    /// 从 tokens 中提取 AIW 自有参数和透传参数
    fn parse_with_type(cli_type: CliType, tokens: &[String]) -> Result<Self, String> {
        let (aiw_args, remaining_args) = extract_aiw_args(tokens);

        Ok(Self {
            cli_type,
            aiw_args,
            remaining_args,
        })
    }

    /// 判断是否是 Auto 模式
    pub fn is_auto(&self) -> bool {
        matches!(self.cli_type, CliType::Auto)
    }

    /// 判断是否是交互模式（remaining_args 为空）
    pub fn is_interactive(&self) -> bool {
        self.remaining_args.is_empty()
    }
}

/// 提取 AIW 固定参数，返回 (AiwArgs, 剩余参数)
fn extract_aiw_args(tokens: &[String]) -> (AiwArgs, Vec<String>) {
    let mut aiw_args = AiwArgs::default();
    let mut remaining = Vec::new();
    let mut iter = tokens.iter().peekable();

    while let Some(token) = iter.next() {
        match token.to_lowercase().as_str() {
            "-r" | "--role" => {
                if let Some(value) = iter.next() {
                    aiw_args.role = Some(value.clone());
                }
            }
            "-mp" | "--aiw-provider" => {
                if let Some(value) = iter.next() {
                    aiw_args.provider = Some(value.clone());
                }
            }
            "-c" | "--cwd" | "-C" => {
                if let Some(value) = iter.next() {
                    aiw_args.cwd = Some(PathBuf::from(value));
                }
            }
            _ => {
                // 其他参数原样保留
                remaining.push(token.clone());
            }
        }
    }

    (aiw_args, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_mode_no_args() {
        let inv = CliInvocation::from_external(&["claude".to_string()]).unwrap();
        assert!(inv.is_interactive());
        assert_eq!(inv.cli_type, CliType::Claude);
        assert!(inv.remaining_args.is_empty());
        assert!(inv.aiw_args.role.is_none());
    }

    #[test]
    fn test_non_interactive_mode_with_prompt() {
        let inv = CliInvocation::from_external(&["claude".to_string(), "hello".to_string()]).unwrap();
        assert!(!inv.is_interactive());
        assert_eq!(inv.remaining_args, vec!["hello"]);
        assert_eq!(inv.cli_type, CliType::Claude);
    }

    #[test]
    fn test_aiw_args_extraction() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "-r".to_string(),
            "senior".to_string(),
            "-mp".to_string(),
            "openrouter".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.role, Some("senior".to_string()));
        assert_eq!(inv.aiw_args.provider, Some("openrouter".to_string()));
        assert!(inv.remaining_args.is_empty());
    }

    #[test]
    fn test_remaining_args_preserved() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "--some-flag".to_string(),
            "value".to_string(),
        ]).unwrap();
        assert_eq!(inv.remaining_args, vec!["--some-flag", "value"]);
        assert!(!inv.is_interactive());
    }

    #[test]
    fn test_mixed_args() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "-r".to_string(),
            "senior".to_string(),
            "--cli-flag".to_string(),
            "do something".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.role, Some("senior".to_string()));
        // 所有非 AIW 参数原样保留
        assert_eq!(inv.remaining_args, vec!["--cli-flag", "do something"]);
    }

    #[test]
    fn test_auto_mode() {
        let inv = CliInvocation::from_auto(&["hello".to_string(), "world".to_string()]).unwrap();
        assert!(inv.is_auto());
        assert_eq!(inv.remaining_args, vec!["hello", "world"]);
        assert_eq!(inv.cli_type, CliType::Auto);
    }

    #[test]
    fn test_cwd_extraction() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "-c".to_string(),
            "/path/to/dir".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.cwd, Some(PathBuf::from("/path/to/dir")));
    }

    #[test]
    fn test_uppercase_cwd_extraction() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "-C".to_string(),
            "/another/path".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.cwd, Some(PathBuf::from("/another/path")));
    }

    #[test]
    fn test_unknown_cli_type() {
        let result = CliInvocation::from_external(&["unknown".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown CLI type"));
    }

    #[test]
    fn test_empty_tokens() {
        let result = CliInvocation::from_external(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No command provided");
    }

    #[test]
    fn test_multiple_remaining_flags() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "--flag1".to_string(),
            "val1".to_string(),
            "--flag2".to_string(),
            "val2".to_string(),
        ]).unwrap();
        assert_eq!(inv.remaining_args, vec!["--flag1", "val1", "--flag2", "val2"]);
    }

    #[test]
    fn test_role_flag_extraction() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "--role".to_string(),
            "developer".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.role, Some("developer".to_string()));
    }

    #[test]
    fn test_provider_flag_extraction() {
        let inv = CliInvocation::from_external(&[
            "claude".to_string(),
            "--aiw-provider".to_string(),
            "anthropic".to_string(),
        ]).unwrap();
        assert_eq!(inv.aiw_args.provider, Some("anthropic".to_string()));
    }

    #[test]
    fn test_codex_type() {
        let inv = CliInvocation::from_external(&["codex".to_string()]).unwrap();
        assert_eq!(inv.cli_type, CliType::Codex);
    }

    #[test]
    fn test_gemini_type() {
        let inv = CliInvocation::from_external(&["gemini".to_string()]).unwrap();
        assert_eq!(inv.cli_type, CliType::Gemini);
    }
}
