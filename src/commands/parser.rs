//! CLI 命令行参数解析和路由
//!
//! 使用 clap 定义命令行接口并进行参数解析

use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

/// Claude Code Hooks 处理动作
#[derive(Subcommand, Debug, Clone)]
pub enum HooksAction {
    /// 处理 Claude Code hook 事件（从 stdin 读取）
    Handle,
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

    /// 显示任务状态
    Status {
        /// 启动TUI界面（默认显示文本摘要）
        #[arg(long)]
        tui: bool,
    },

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

    /// 等待所有并发AI CLI任务完成（跨进程）
    Wait {
        /// 超时时间（如: 12h, 30m, 1d）
        #[arg(long, default_value = "12h")]
        timeout: String,

        /// 显示详细进度信息
        #[arg(long, short = 'v')]
        verbose: bool,
    },

    /// 等待指定进程的共享任务完成
    #[command(name = "pwait")]
    PWait {
        /// 要等待的进程PID
        #[arg(value_name = "PID")]
        pid: u32,
    },

    /// 显示使用示例
    #[command(alias = "demo")]
    Examples,

    /// 显示帮助信息
    Help {
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },

    /// 更新 AI CLI 工具（未安装则安装，已安装则更新）
    Update {
        /// 指定要更新/安装的 AI CLI 工具（claude/codex/gemini）
        #[arg(value_name = "TOOL")]
        tool: Option<String>,
    },

    /// 启动 MCP (Model Context Protocol) 服务器
    Mcp {
        /// 传输协议
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// 日志级别
        #[arg(long, default_value = "info")]
        log_level: String,
    },

    /// Claude Code Hooks 处理
    #[command(subcommand)]
    Hooks(HooksAction),

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
        Ok(cli.command.take().unwrap_or(Commands::Dashboard))
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
