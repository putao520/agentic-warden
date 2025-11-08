//! CLI 命令行参数解析和路由
//!
//! 使用 clap 定义命令行接口并进行参数解析

use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

/// MCP (Model Context Protocol) 服务器动作
#[derive(Subcommand, Debug, Clone)]
pub enum McpAction {
    /// 启动 MCP 服务器
    Server {
        /// 传输协议
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// 日志级别
        #[arg(long, default_value = "info")]
        log_level: String,
    },

    /// 测试 MCP 服务器配置
    Test,

    /// 显示 MCP 服务器状态
    Status,
}

/// Agentic-Warden - AI CLI 工具的统一管理和进程监控平台
#[derive(Parser, Debug, Clone)]
#[command(
    name = "agentic-warden",
    about = "AI CLI manager with process tracking and Google Drive sync",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// 显示 Dashboard（无参数时的默认行为）
    Dashboard,

    /// 显示任务状态 TUI
    Status,

    /// 启动 Provider 管理 TUI
    Provider,

    /// 推送目录到 Google Drive
    Push {
        /// 需要推送的目录（缺省为当前目录）
        #[arg(
            value_name = "DIR",
            value_parser = clap::value_parser!(PathBuf),
            num_args = 0..,
            trailing_var_arg = true
        )]
        dirs: Vec<PathBuf>,
    },

    /// 从 Google Drive 拉取文件
    Pull,

    /// 重置同步状态
    Reset,

    /// 列出远程文件
    List,

    /// 等待后台任务
    Wait,

    /// 显示使用示例
    #[command(alias = "demo")]
    Examples,

    /// 显示帮助信息
    Help {
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },

    /// MCP (Model Context Protocol) 服务器
    #[command(subcommand)]
    Mcp(McpAction),

    /// 捕获未显式声明的子命令（用于 AI CLI 选择器）
    #[command(external_subcommand)]
    External(Vec<String>),
}

impl Cli {
    /// 解析命令行参数并返回最终命令（默认 Dashboard）
    pub fn parse_command() -> Commands {
        Self::parse_command_from(std::env::args_os())
    }

    /// 尝试解析命令行参数（用于测试或自定义 argv）
    pub fn try_parse_command_from<I, T>(iter: I) -> Result<Commands, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut cli = Cli::try_parse_from(iter)?;
        Ok(cli.command.take().unwrap_or_else(|| Commands::Dashboard))
    }

    /// 解析命令行参数（失败时由 clap 处理错误输出和退出）
    pub fn parse_command_from<I, T>(iter: I) -> Commands
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        match Self::try_parse_command_from(iter) {
            Ok(command) => command,
            Err(err) => err.exit(),
        }
    }
}

/// AI CLI 命令解析结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCliArgs {
    pub selector: String,
    pub provider: Option<String>,
    pub prompt: Vec<String>,
}

impl AiCliArgs {
    /// 构建提示词字符串
    pub fn prompt_text(&self) -> String {
        self.prompt.join(" ")
    }
}

/// 将 external subcommand 解析为 AI CLI 参数
pub fn parse_external_as_ai_cli(tokens: &[String]) -> Result<AiCliArgs, String> {
    if tokens.is_empty() {
        return Err("No command provided".to_string());
    }

    let selector = tokens[0].clone();
    let mut provider: Option<String> = None;
    let mut prompt: Vec<String> = Vec::new();

    let mut iter = tokens.iter().skip(1);
    while let Some(token) = iter.next() {
        match token.as_str() {
            "-p" | "--provider" => {
                let value = expect_value(&mut iter, token)?;
                provider = Some(value);
            }
            _ => prompt.push(token.clone()),
        }
    }

    Ok(AiCliArgs {
        selector,
        provider,
        prompt,
    })
}

fn expect_value<'a, I>(iter: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .cloned()
        .ok_or_else(|| format!("Error: {} requires a value", flag))
}
