//! AI CLI 启动命令处理逻辑
//!
//! 处理 codex、claude、gemini 等 AI CLI 的启动和管理

use crate::cli_type::{parse_cli_selector, CliType};
use crate::registry::TaskRegistry;
use crate::supervisor;
use anyhow::Result;
use std::ffi::OsString;
use std::process::ExitCode;

/// AI CLI 启动参数
pub struct AiCliCommand {
    pub ai_types: Vec<CliType>,
    pub provider: Option<String>,
    pub prompt: String,
}

impl AiCliCommand {
    /// 创建新的 AI CLI 命令
    pub fn new(ai_types: Vec<CliType>, provider: Option<String>, prompt: String) -> Self {
        Self {
            ai_types,
            provider,
            prompt,
        }
    }

    /// 执行 AI CLI 命令
    pub async fn execute(&self) -> Result<ExitCode> {
        let registry = TaskRegistry::connect()?;

        // 检查是否是交互模式（无提示词）
        if self.prompt.is_empty() {
            // 交互模式仅支持单个 CLI
            if self.ai_types.len() != 1 {
                return Err(anyhow::anyhow!(
                    "Interactive mode only supports single CLI. Please provide a task description for multiple CLI execution."
                ));
            }

            let cli_type = &self.ai_types[0];
            let exit_code = supervisor::start_interactive_cli(&registry, cli_type, self.provider.clone())?;
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else {
            // 任务模式
            if self.ai_types.len() == 1 {
                // 单个 CLI 执行
                let cli_type = &self.ai_types[0];
                let cli_args = cli_type.build_full_access_args(&self.prompt);
                let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

                let exit_code = supervisor::execute_cli(&registry, cli_type, &os_args, self.provider.clone())?;
                Ok(ExitCode::from((exit_code & 0xFF) as u8))
            } else {
                // 多个 CLI 批量执行
                println!(
                    "Starting tasks for CLI(s): {}",
                    self.ai_types
                        .iter()
                        .map(|t| t.display_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                let cli_selector = crate::cli_type::CliSelector {
                    types: self.ai_types.clone(),
                };

                let exit_codes = supervisor::execute_multiple_clis(
                    &registry,
                    &cli_selector,
                    &self.prompt,
                    self.provider.clone(),
                )?;

                // 返回第一个失败的 exit code，或者 0 如果全部成功
                let final_exit_code = exit_codes
                    .iter()
                    .find(|&&code| code != 0)
                    .copied()
                    .unwrap_or(0);

                Ok(ExitCode::from((final_exit_code & 0xFF) as u8))
            }
        }
    }
}

/// 解析 AI 类型字符串
pub fn parse_ai_types(input: &str) -> Result<Vec<CliType>> {
    if input == "all" {
        // 返回所有可用的 AI 类型
        Ok(vec![CliType::Codex, CliType::Claude, CliType::Gemini])
    } else {
        // 解析复合类型如 "codex|claude"
        let ai_types: Vec<CliType> = input
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                parse_cli_selector(s).and_then(|selector| {
                    if selector.types.len() == 1 {
                        Some(selector.types[0].clone())
                    } else {
                        eprintln!("Warning: Multiple CLI types detected in '{}', skipping", s);
                        None
                    }
                })
            })
            .collect();
        Ok(ai_types)
    }
}