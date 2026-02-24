//! CLI 命令行参数解析和路由
//!
//! 使用 clap 定义命令行接口并进行参数解析

use clap::{Parser, Subcommand};
use std::ffi::OsString;

/// 统一的 CLI 参数解析结果
///
/// 用于所有 AI CLI 命令（auto, claude, codex, gemini）的参数解析
/// - auto 命令: selector = None
/// - claude/codex/gemini 命令: selector = Some("claude"|"codex"|"gemini")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCliArgs {
    /// CLI 选择器（auto 命令为 None）
    pub selector: Option<String>,
    /// 角色名称
    pub role: Option<String>,
    /// AIW Provider 名称（使用 -mp/--aiw-provider 参数设置）
    pub provider: Option<String>,
    /// 透传给底层 AI CLI 的参数
    pub cli_args: Vec<String>,
    /// 用户提示词
    pub prompt: Vec<String>,
    /// 工作目录
    pub cwd: Option<std::path::PathBuf>,
}

impl ParsedCliArgs {
    /// 构建提示词字符串
    pub fn prompt_text(&self) -> String {
        self.prompt.join(" ")
    }
}

/// AI CLI 角色管理动作
#[derive(Subcommand, Debug, Clone)]
pub enum RolesAction {
    /// 列出所有可用的角色配置
    List,
}

/// 配置管理动作
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// 管理 AI CLI 执行顺序
    #[command(name = "cli-order")]
    CliOrder,
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

    /// 搜索MCP服务器
    Search {
        /// 搜索关键词
        query: String,
        /// 指定来源 (registry|smithery)
        #[arg(long)]
        source: Option<String>,
        /// 返回结果数量限制
        #[arg(long)]
        limit: Option<usize>,
    },

    /// 安装MCP服务器
    Install {
        /// 服务器名称
        name: String,
        /// 指定来源 (当名称未带前缀时)
        #[arg(long)]
        source: Option<String>,
        /// 环境变量 (KEY=VALUE，可多次使用)
        #[arg(long = "env")]
        env_vars: Vec<String>,
        /// 跳过环境变量配置
        #[arg(long = "skip-env")]
        skip_env: bool,
    },

    /// 查看MCP服务器信息
    Info {
        /// 服务器名称
        name: String,
        /// 指定来源查询
        #[arg(long)]
        source: Option<String>,
    },

    /// 更新仓库索引缓存
    Update,

    /// 交互式浏览所有MCP服务器
    Browse {
        /// 指定来源 (registry|smithery)
        #[arg(long)]
        source: Option<String>,
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

/// 插件市场管理动作
#[derive(Subcommand, Debug, Clone)]
pub enum MarketplaceAction {
    /// 添加插件市场源
    Add {
        /// GitHub仓库/本地路径/远程URL
        #[arg(value_name = "REPO_URL")]
        repo_url: String,
        /// 市场源别名
        #[arg(long)]
        name: Option<String>,
    },
    /// 列出市场源
    List,
    /// 移除市场源
    Remove {
        /// 市场源名称
        name: String,
    },
    /// 更新市场源索引
    Update {
        /// 市场源名称（可选）
        name: Option<String>,
    },
}

/// 插件市场动作
#[derive(Subcommand, Debug, Clone)]
pub enum PluginAction {
    /// 市场源管理
    #[command(subcommand)]
    Marketplace(MarketplaceAction),

    /// 浏览MCP插件
    Browse {
        /// 指定市场源
        #[arg(long)]
        market: Option<String>,
        /// 指定分类
        #[arg(long)]
        category: Option<String>,
        /// 指定标签（逗号分隔）
        #[arg(long)]
        tags: Option<String>,
    },

    /// 搜索插件
    Search {
        /// 搜索关键词
        query: String,
        /// 指定市场源
        #[arg(long)]
        market: Option<String>,
    },

    /// 查看插件详情
    Info {
        /// 插件名称@市场
        plugin: String,
    },

    /// 安装插件
    Install {
        /// 插件名称@市场
        plugin: String,
        /// 环境变量 (KEY=VALUE格式，可多次使用)
        #[arg(long = "env")]
        env_vars: Vec<String>,
        /// 跳过环境变量配置
        #[arg(long = "skip-env")]
        skip_env: bool,
    },

    /// 列出已安装插件
    List {
        /// 显示已禁用插件
        #[arg(long = "show-disabled")]
        show_disabled: bool,
    },

    /// 移除插件
    Remove {
        /// 插件名称
        plugin: String,
    },

    /// 启用插件
    Enable {
        /// 插件名称
        plugin: String,
    },

    /// 禁用插件
    Disable {
        /// 插件名称
        plugin: String,
    },
}

/// AIW - AI CLI 工具的统一管理和进程监控平台
#[derive(Parser, Debug, Clone)]
#[command(
    name = "aiw",
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

    /// 插件市场管理
    #[command(subcommand)]
    Plugin(PluginAction),

    /// 配置管理
    #[command(subcommand)]
    Config(ConfigAction),

    /// AI CLI 角色管理
    #[command(subcommand)]
    Roles(RolesAction),

    /// 显示版本信息
    #[command(name = "v")]
    Version,

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

/// 解析 AI CLI 命令参数（auto/claude/codex/gemini）
///
/// # 参数
/// - `tokens`: 命令行参数数组（不包含命令名本身）
///
/// # 返回
/// - `Ok(ParsedCliArgs)`: 解析后的参数
/// - `Err(String)`: 解析错误信息
///
/// # 示例
/// ```ignore
/// // auto 命令
/// let args = parse_cli_args(&["-mp", "glm", "echo test"]);
/// // args.selector == None
///
/// // claude 命令
/// let args = parse_cli_args(&["claude", "-r", "common", "-mp", "glm", "echo test"]);
/// // args.selector == Some("claude")
/// ```
pub fn parse_cli_args(tokens: &[String]) -> Result<ParsedCliArgs, String> {
    let mut role: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut cwd: Option<std::path::PathBuf> = None;
    let mut cli_args: Vec<String> = Vec::new();
    let mut prompt: Vec<String> = Vec::new();

    // Pass 1: extract AIW-owned flags (-r, -mp, -C) from tokens, leave the rest
    let mut remaining: Vec<(usize, String)> = Vec::new();
    let mut iter = tokens.iter().enumerate().peekable();

    while let Some((idx, token)) = iter.next() {
        // Once we hit "--", everything after is prompt (preserve for pass 2)
        if token == "--" {
            remaining.push((idx, token.clone()));
            for (i, t) in iter.by_ref() {
                remaining.push((i, t.clone()));
            }
            break;
        }

        match token.to_lowercase().as_str() {
            "-r" | "--role" => {
                if role.is_some() {
                    return Err("Error: role specified multiple times".to_string());
                }
                let (_, value) = iter
                    .next()
                    .ok_or_else(|| "Missing role name after -r/--role flag".to_string())?;
                if value.starts_with('-') || value.is_empty() || value == "--" {
                    return Err("Missing role name after -r/--role flag".to_string());
                }
                role = Some(value.clone());
            }
            "-mp" | "--aiw-provider" => {
                if provider.is_some() {
                    return Err("Error: provider specified multiple times".to_string());
                }
                let (_, value) = iter
                    .next()
                    .ok_or_else(|| "Missing provider name after -mp/--aiw-provider flag".to_string())?;
                if value.starts_with('-') || value.is_empty() || value == "--" {
                    return Err("Missing provider name after -mp/--aiw-provider flag".to_string());
                }
                provider = Some(value.clone());
            }
            "-C" | "--cwd" => {
                if cwd.is_some() {
                    return Err("Error: working directory specified multiple times".to_string());
                }
                let (_, value) = iter
                    .next()
                    .ok_or_else(|| "Missing directory path after -C/--cwd flag".to_string())?;
                if value.starts_with('-') || value.is_empty() || value == "--" {
                    return Err("Missing directory path after -C/--cwd flag".to_string());
                }
                cwd = Some(std::path::PathBuf::from(value));
            }
            _ => {
                remaining.push((idx, token.clone()));
            }
        }
    }

    // Pass 2: parse remaining tokens into cli_args and prompt
    let mut iter2 = remaining.iter().peekable();

    while let Some((idx, token)) = iter2.next() {
        if token == "--" {
            prompt.extend(iter2.map(|(_, t)| t.clone()));
            break;
        }

        if token.starts_with('-') {
            // Check if this flag separates CLI args from prompt
            if is_prompt_separator_flag(token) {
                prompt.extend(iter2.map(|(_, t)| t.clone()));
                break;
            }

            cli_args.push(token.clone());

            if let Some((_, next)) = iter2.peek() {
                if *next != "--" && !next.starts_with('-') && !is_valueless_flag(token) {
                    let (_, value_token) = iter2.next().expect("peeked token should exist");
                    cli_args.push(value_token.clone());
                }
            }

            continue;
        }

        // First non-flag token becomes the start of the prompt
        prompt.push(token.clone());
        prompt.extend(iter2.map(|(_, t)| t.clone()));
        break;
    }

    Ok(ParsedCliArgs {
        selector: None,  // 默认为 None，由调用方决定是否需要 selector
        role,
        provider,
        cli_args,
        prompt,
        cwd,
    })
}

/// 解析 external AI CLI 命令（claude/codex/gemini）
///
/// 与 `parse_cli_args` 的区别是会自动从第一个 token 提取 selector
///
/// # 参数
/// - `tokens`: 命令行参数数组，第一个元素应该是 CLI 类型（claude/codex/gemini）
///
/// # 返回
/// - `Ok(ParsedCliArgs)`: 解析后的参数，selector 为第一个 token
/// - `Err(String)`: 解析错误信息
pub fn parse_external_cli_args(tokens: &[String]) -> Result<ParsedCliArgs, String> {
    if tokens.is_empty() {
        return Err("No command provided".to_string());
    }

    let selector = tokens[0].clone();
    let mut args = parse_cli_args(&tokens[1..])?;
    args.selector = Some(selector);
    Ok(args)
}

fn is_valueless_flag(flag: &str) -> bool {
    // Only AIW's own valueless flags, not AI CLI flags
    // AI CLI flags are transparently forwarded - we don't need to understand them
    matches!(
        flag,
        "-c"
            | "--dangerously-skip-permissions"
            | "--dangerously-bypass-approvals-and-sandbox"
    ) || flag.starts_with("--no-")
}

fn is_prompt_separator_flag(_flag: &str) -> bool {
    // All flags are forwarded to AI CLI, none are prompt separators
    // The prompt is determined by the first non-flag argument
    false
}
