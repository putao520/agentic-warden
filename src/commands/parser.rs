//! CLI 命令行参数解析和路由
//!
//! 使用 clap 定义命令行接口并进行参数解析

use clap::{Parser, Subcommand};
use std::ffi::OsString;

/// AI CLI 角色管理动作
#[derive(Subcommand, Debug, Clone)]
pub enum RolesAction {
    /// 列出所有可用的角色配置
    List,
}

/// MCP服务器管理动作
#[derive(Subcommand, Debug, Clone)]
pub enum McpAction {
    /// 列出所有MCP服务器
    List,

    /// 添加MCP服务器
    Add {
        /// 服务器名称
        name: String,
        /// 可执行命令
        command: String,
        /// 命令参数
        args: Vec<String>,
        /// 服务器描述
        #[arg(long)]
        description: Option<String>,
        /// 服务器分类
        #[arg(long)]
        category: Option<String>,
        /// 环境变量 (KEY=VALUE格式，可多次使用)
        #[arg(long = "env")]
        env_vars: Vec<String>,
        /// 添加但不启用
        #[arg(long)]
        disabled: bool,
    },

    /// 移除MCP服务器
    Remove {
        /// 服务器名称
        name: String,
        /// 跳过确认提示
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// 获取服务器详细配置
    Get {
        /// 服务器名称
        name: String,
    },

    /// 启用服务器
    Enable {
        /// 服务器名称
        name: String,
    },

    /// 禁用服务器
    Disable {
        /// 服务器名称
        name: String,
    },

    /// 在编辑器中编辑配置文件
    Edit,

    /// 启动MCP服务器（内部使用）
    Serve {
        /// 传输协议
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// 日志级别
        #[arg(long, default_value = "info")]
        log_level: String,
    },
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

    /// 推送配置到 Google Drive
    Push,

    /// 从 Google Drive 拉取文件
    Pull,

    /// 列出远程文件
    List,

    /// 等待所有并发AI CLI任务完成（跨进程）
    Wait,

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

    /// 更新 AI CLI 工具和 AIW 自身
    Update,

    /// MCP服务器管理
    #[command(subcommand)]
    Mcp(McpAction),

    /// AI CLI 角色管理
    #[command(subcommand)]
    Roles(RolesAction),

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
