//! CLI 命令行参数解析和路由
//!
//! 使用 clap 定义命令行接口并进行参数解析

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Agentic-Warden - AI CLI 工具的统一管理和进程监控平台
#[derive(Parser)]
#[command(name = "agentic-warden")]
#[command(about = "AI CLI manager with process tracking and Google Drive sync")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start AI CLI with optional provider
    #[command(name = "codex|claude|gemini|all")]
    AiCli {
        /// Provider name (-p or --provider)
        #[arg(short = 'p', long = "provider")]
        provider: Option<String>,
        /// Prompt or task description
        prompt: String,
    },

    /// Show dashboard (default when no arguments)
    Dashboard,

    /// Show task status
    Status,

    /// Push directories to Google Drive
    Push {
        /// Directories to push (default: current directory)
        #[arg(default_value = ".")]
        dirs: Vec<PathBuf>,
    },

    /// Pull from Google Drive
    Pull,

    /// Reset sync state
    Reset,

    /// List remote files
    List,

    /// Provider management (deprecated - use TUI)
    Provider,

    /// Show version information
    #[command(name = "--version")]
    Version,

    /// Show help
    #[command(name = "--help")]
    Help,

    /// Show quick examples
    Examples,

    /// Wait for background tasks
    Wait,
}

impl Cli {
    /// 解析命令行参数
    pub fn parse_args() -> Self {
        // 处理特殊的复合命令（如 codex|claude）
        let args: Vec<String> = std::env::args().collect();

        // 检查是否是复合AI命令
        if args.len() > 1 {
            let first_arg = &args[1];
            if first_arg.contains('|') || first_arg == "all" {
                // 将复合命令转换为标准格式
                return Self::parse_ai_command(&args);
            }
        }

        // 标准解析
        Self::try_parse().unwrap_or_else(|_| {
            // 如果没有参数，默认显示 dashboard
            Cli {
                command: Commands::Dashboard,
            }
        })
    }

    /// 处理复合AI命令
    fn parse_ai_command(args: &[String]) -> Self {
        let ai_types = &args[1];
        let mut provider: Option<String> = None;
        let mut prompt_parts: Vec<String> = Vec::new();
        let mut i = 2; // 跳过程序名和AI类型

        while i < args.len() {
            match args[i].as_str() {
                "-p" | "--provider" => {
                    if i + 1 < args.len() {
                        provider = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Error: -p/--provider requires a value");
                        std::process::exit(1);
                    }
                }
                _ => {
                    prompt_parts.push(args[i].clone());
                    i += 1;
                }
            }
        }

        let prompt = prompt_parts.join(" ");

        Cli {
            command: Commands::AiCli {
                provider,
                prompt,
            },
        }
    }
}