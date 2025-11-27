// Binary-specific modules
mod help;

use agentic_warden::cli_type::{parse_cli_selector_strict, parse_cli_type};
use agentic_warden::commands::ai_cli::AiCliCommand;
use agentic_warden::commands::{
    parse_external_as_ai_cli,
    parser::{McpAction, RolesAction},
    Cli, Commands,
};
use agentic_warden::error::ErrorCategory;
use agentic_warden::execute_update;
use agentic_warden::mcp::AgenticWardenMcpServer;
use agentic_warden::pwait_mode;
use agentic_warden::roles::RoleManager;
use agentic_warden::tui;
use agentic_warden::wait_mode;
use help::{print_command_help, print_general_help, print_quick_examples};
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // 处理外部AI CLI命令 (codex, claude, gemini)
    if args.len() >= 2 {
        match args[1].as_str() {
            "codex" | "claude" | "gemini" => {
                return match handle_external_ai_cli(&args).await {
                    Ok(code) => code,
                    Err(err) => {
                        eprintln!("{}", err);
                        ExitCode::from(1)
                    }
                };
            }
            _ => {}
        }
    }

    // 处理其他命令
    let command = Cli::parse_command();
    match main_impl(command).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::from(1)
        }
    }
}

async fn handle_external_ai_cli(args: &[String]) -> Result<ExitCode, String> {
    // args[0] 是程序名 "agentic-warden"
    // args[1] 是 CLI type "codex"/"claude"/"gemini"
    // args[2..] 是实际的 prompt 和选项

    if args.len() < 2 {
        return Err("Please specify AI CLI type".to_string());
    }

    let cli_type_str = &args[1];
    let remaining_args = &args[2..];

    // 解析 CLI 类型
    let cli_type = match parse_cli_type(cli_type_str) {
        Some(t) => t,
        None => return Err(format!("Unsupported AI CLI type: {}", cli_type_str)),
    };

    // Parse provider parameters
    let mut provider: Option<String> = None;
    let mut prompt_parts = Vec::new();

    let mut iter = remaining_args.iter().enumerate();
    while let Some((_i, token)) = iter.next() {
        match token.as_str() {
            "-p" | "--provider" => {
                if let Some((_, next_token)) = iter.next() {
                    provider = Some(next_token.clone());
                } else {
                    return Err("Missing provider name after -p/--provider flag".to_string());
                }
            }
            _ => {
                // This is part of the prompt
                prompt_parts.push(token.clone());
            }
        }
    }

    // Check if this is interactive mode (no prompt)
    if prompt_parts.is_empty() {
        println!(
            "🚀 Starting {} in interactive mode (provider: {:?})",
            cli_type_str, provider
        );

        // Interactive mode: start AI CLI directly with empty prompt
        let command = AiCliCommand::new(vec![cli_type], provider, String::new());
        command.execute().await.map_err(|e| e.to_string())
    } else {
        // Task mode: execute specific task
        let prompt_text = prompt_parts.join(" ");

        println!(
            "🚀 Starting {} with task: {} (provider: {:?})",
            cli_type_str, prompt_text, provider
        );

        let command = AiCliCommand::new(vec![cli_type], provider, prompt_text);
        command.execute().await.map_err(|e| e.to_string())
    }
}

async fn main_impl(command: Commands) -> Result<ExitCode, String> {
    match command {
        Commands::Dashboard => launch_tui(None).await,
        Commands::Status { tui } => {
            if tui {
                // 启动TUI界面
                launch_tui(Some(tui::ScreenType::Status)).await
            } else {
                // 显示文本摘要
                handle_status_command()
            }
        }
        Commands::Provider => launch_tui(Some(tui::ScreenType::Provider)).await,
        Commands::Push => launch_tui(Some(tui::ScreenType::Push)).await,
        Commands::Pull => launch_tui(Some(tui::ScreenType::Pull)).await,
        Commands::List => handle_sync_command("list", None).await,
        Commands::Wait { timeout, verbose } => {
            // CLI参数已完整实现，支持timeout和verbose参数
            if verbose {
                eprintln!(
                    "Waiting for all concurrent AI CLI tasks to complete (timeout: {})...",
                    timeout
                );
            }
            wait_mode::run().map_err(|e| e.to_string())?;
            Ok(ExitCode::from(0))
        }
        Commands::PWait { pid } => {
            // 等待指定进程的共享内存任务完成
            match pwait_mode::run_for_pid(pid) {
                Ok(report) => {
                    report.print();
                    Ok(ExitCode::from(0))
                }
                Err(pwait_mode::PWaitError::NoTasks) => {
                    eprintln!("No tasks found for PID {}", pid);
                    Ok(ExitCode::from(0))
                }
                Err(e) => {
                    eprintln!("Error waiting for tasks (PID {}): {}", pid, e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::Examples => {
            print_quick_examples().map_err(|e| format!("Failed to print examples: {}", e))?;
            Ok(ExitCode::from(0))
        }
        Commands::Help { command } => {
            match command {
                Some(topic) => {
                    print_command_help(&topic)
                        .map_err(|e| format!("Failed to print help: {}", e))?;
                }
                None => {
                    print_general_help().map_err(|e| format!("Failed to print help: {}", e))?;
                }
            }
            Ok(ExitCode::from(0))
        }
        Commands::Update { tool } => {
            let tool_name = tool.as_deref();
            match execute_update(tool_name).await {
                Ok(results) => {
                    let success_count = results.iter().filter(|(_, success, _)| *success).count();
                    let total_count = results.len();

                    if success_count == total_count {
                        println!("\n✅ All {} tool(s) updated successfully!", success_count);
                        Ok(ExitCode::from(0))
                    } else {
                        eprintln!(
                            "\n⚠️  {}/{} tool(s) updated successfully",
                            success_count, total_count
                        );
                        Ok(ExitCode::from(1))
                    }
                }
                Err(e) => {
                    eprintln!("❌ Update failed: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::Mcp(action) => handle_mcp_action(action).await,
        Commands::Roles(action) => handle_roles_command(action).await,
        Commands::External(tokens) => handle_external_command(tokens).await,
    }
}

async fn launch_tui(initial_screen: Option<tui::ScreenType>) -> Result<ExitCode, String> {
    color_eyre::install().map_err(|e| format!("Failed to install error handler: {}", e))?;

    let tui_result = match initial_screen {
        Some(screen) => tui::app::run_tui_app_with_screen(Some(screen)),
        None => tui::app::run_tui_app(),
    };

    tui_result.map_err(|e| format!("TUI error: {}", e))?;

    Ok(ExitCode::from(0))
}

/// 处理status命令（文本模式）
fn handle_status_command() -> Result<ExitCode, String> {
    use agentic_warden::storage::SharedMemoryStorage;
    use agentic_warden::task_record::TaskStatus;
    use agentic_warden::unified_registry::Registry;

    // 连接到当前进程的共享内存
    let storage = SharedMemoryStorage::connect()
        .map_err(|e| format!("Failed to connect to shared memory: {}", e))?;

    let registry = Registry::new(storage);

    // 获取所有任务条目
    let entries = registry
        .entries()
        .map_err(|e| format!("Failed to get task entries: {}", e))?;

    // 统计运行中的任务
    let running_count = entries
        .iter()
        .filter(|entry| entry.record.status == TaskStatus::Running)
        .count();

    // 输出结果
    if running_count == 0 {
        println!("No tasks!");
    } else {
        println!("running {} tasks!", running_count);
    }

    Ok(ExitCode::from(0))
}

async fn handle_external_command(tokens: Vec<String>) -> Result<ExitCode, String> {
    if tokens.is_empty() {
        return Err("No command provided".to_string());
    }

    let command_name = tokens[0].clone();
    let lower_command = command_name.to_lowercase();

    if lower_command == "help" {
        let topic = tokens.get(1).cloned();
        return handle_help_command(topic);
    }

    if tokens
        .iter()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_command_help(&command_name).map_err(|e| format!("Failed to print help: {}", e))?;
        return Ok(ExitCode::from(0));
    }

    let ai_args = parse_external_as_ai_cli(&tokens)?;
    let selector = parse_cli_selector_strict(&ai_args.selector).map_err(|err| {
        if err.category() == ErrorCategory::Validation {
            format!(
                "{}\n\nUse 'agentic-warden --help' for more information.",
                err.user_message()
            )
        } else {
            err.user_message()
        }
    })?;

    let ai_types = selector.types.clone();
    let ai_command = AiCliCommand::new(ai_types, ai_args.provider.clone(), ai_args.prompt_text());

    // 使用当前的异步运行时执行命令
    ai_command.execute().await.map_err(|e| e.to_string())
}

fn handle_help_command(topic: Option<String>) -> Result<ExitCode, String> {
    match topic {
        Some(cmd) => {
            print_command_help(&cmd).map_err(|e| format!("Failed to print help: {}", e))?;
        }
        None => {
            print_general_help().map_err(|e| format!("Failed to print help: {}", e))?;
        }
    }
    Ok(ExitCode::from(0))
}

/// 处理MCP命令
async fn handle_mcp_action(action: McpAction) -> Result<ExitCode, String> {
    match action {
        McpAction::List => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::List) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Add {
            name,
            command,
            args,
            description,
            category,
            env_vars,
            disabled,
        } => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};

            // 解析环境变量
            let mut env = Vec::new();
            for env_var in env_vars {
                let parts: Vec<&str> = env_var.splitn(2, '=').collect();
                if parts.len() == 2 {
                    env.push((parts[0].to_string(), parts[1].to_string()));
                } else {
                    eprintln!(
                        "Warning: Invalid env var format '{}', expected KEY=VALUE",
                        env_var
                    );
                }
            }

            match handle_mcp_command(McpCommand::Add {
                name,
                command,
                args,
                description,
                category,
                env,
                disabled,
            }) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Remove { name, yes } => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Remove { name, yes }) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Get { name } => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Get { name }) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Enable { name } => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Enable { name }) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Disable { name } => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Disable { name }) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Edit => {
            use agentic_warden::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Edit) {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Serve {
            transport,
            log_level,
        } => handle_mcp_serve(transport, log_level).await,
    }
}

async fn handle_mcp_serve(transport: String, log_level: String) -> Result<ExitCode, String> {
    // 初始化日志
    let log_level_filter = match log_level.to_lowercase().as_str() {
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level_filter)
        .with_target(false)
        .init();

    // Note: Claude Code hooks were removed in v6.0.0 (CC session history deleted)
    // No hooks installation/uninstallation needed

    // 创建MCP服务器（使用InProcessRegistry）
    // Provider配置通过supervisor模块管理，不需要在MCP server中直接管理
    let mcp_server = AgenticWardenMcpServer::bootstrap()
        .await
        .map_err(|e| format!("Failed to initialise MCP server: {e}"))?;

    match transport.as_str() {
        "stdio" => {
            // 使用stdio传输启动MCP服务器
            eprintln!("Starting Agentic-Warden MCP server with stdio transport...");

            match run_mcp_server_stdio(mcp_server).await {
                Ok(_) => {
                    eprintln!("MCP server stopped gracefully");
                    Ok(ExitCode::from(0))
                }
                Err(e) => {
                    eprintln!("MCP server error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        _ => Err(format!(
            "Unsupported transport: {}. Supported: stdio",
            transport
        )),
    }
    // HooksGuard automatically uninstalls hooks when dropped
}

/// Handle role management commands
async fn handle_roles_command(action: RolesAction) -> Result<ExitCode, String> {
    match action {
        RolesAction::List => {
            let manager = RoleManager::new().map_err(|e| format!("Failed to load roles: {}", e))?;
            let roles = manager
                .list_all_roles()
                .map_err(|e| format!("Failed to list roles: {}", e))?;

            if roles.is_empty() {
                if let Some(dir) = dirs::home_dir().map(|home| home.join(".aiw").join("role")) {
                    println!(
                        "No roles found. Add markdown role files under {}",
                        dir.display()
                    );
                } else {
                    println!("No roles found. Add markdown role files under ~/.aiw/role");
                }
            } else {
                println!("Available roles ({}):", roles.len());
                for role in roles {
                    println!(
                        "- {}: {} ({})",
                        role.name,
                        role.description,
                        role.file_path.display()
                    );
                }
            }

            Ok(ExitCode::from(0))
        }
    }
}

/// Handle sync command (push, pull, list)
async fn handle_sync_command(
    command: &str,
    _args: Option<Vec<PathBuf>>,
) -> Result<ExitCode, String> {
    match agentic_warden::sync::sync_command::handle_sync_command(command, None).await {
        Ok(code) => {
            // Convert i32 to u8 for ExitCode
            let exit_code = if code >= 0 && code <= 255 {
                code as u8
            } else {
                1 // Default error code
            };
            Ok(ExitCode::from(exit_code))
        }
        Err(e) => Err(format!("Sync command failed: {}", e)),
    }
}

/// 运行stdio传输的MCP服务器
async fn run_mcp_server_stdio(
    server: AgenticWardenMcpServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.run().await
}
