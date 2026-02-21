// Binary-specific modules
mod help;

use aiw::cli_type::parse_cli_selector_strict;
use aiw::commands::ai_cli::AiCliCommand;
use aiw::commands::{parse_external_cli_args, parser::{ConfigAction, McpAction, RolesAction}, Cli, Commands};
use aiw::error::ErrorCategory;
use aiw::execute_enhanced_update;
use aiw::mcp::AgenticWardenMcpServer;
use aiw::commands::market::handle_plugin_action;
use aiw::pwait_mode;
use aiw::roles::RoleManager;
use aiw::tui;
use aiw::wait_mode;
use help::{print_command_help, print_general_help, print_quick_examples};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    let args: Vec<String> = std::env::args().collect();

    // 处理版本标志 - 在解析CLI之前检查
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("aiw {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::from(0);
    }

    // 处理外部AI CLI命令 (codex, claude, gemini)
    if args.len() >= 2 {
        match args[1].as_str() {
            "codex" | "claude" | "gemini" | "auto" => {
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
    if args.len() < 2 {
        return Err("Please specify AI CLI type".to_string());
    }

    if args[1].eq_ignore_ascii_case("auto") {
        return Ok(aiw::commands::auto::handle_auto_command(&args[1..]).await);
    }

    let tokens = args[1..].to_vec();
    let ai_args = parse_external_cli_args(&tokens)?;

    // selector 现在是 Option<String>，对于 claude/codex/gemini 命令一定是 Some
    let selector_str = ai_args.selector.as_ref().expect("selector should be set for external CLI commands");
    let selector = parse_cli_selector_strict(selector_str).map_err(|err| {
        if err.category() == ErrorCategory::Validation {
            format!(
                "{}\n\nUse 'agentic-warden --help' for more information.",
                err.user_message()
            )
        } else {
            err.user_message()
        }
    })?;

    if ai_args.prompt.is_empty() {
        println!(
            "🚀 Starting {} in interactive mode (provider: {:?})",
            selector_str, ai_args.provider
        );
    } else {
        println!(
            "🚀 Starting {} with task: {} (provider: {:?})",
            selector_str,
            ai_args.prompt_text(),
            ai_args.provider
        );
    }

    let ai_command = AiCliCommand::new(
        selector.types,
        ai_args.role.clone(),
        ai_args.provider.clone(),
        ai_args.prompt_text(),
        ai_args.cli_args.clone(),
        ai_args.cwd.clone(),
    );

    ai_command.execute().await.map_err(|e| e.to_string())
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
        Commands::Wait => {
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
        Commands::Version => {
            println!("aiw {}", env!("CARGO_PKG_VERSION"));
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
        Commands::Update => {
            match execute_enhanced_update().await {
                Ok((aiw_updated, cli_results)) => {
                    let cli_success_count = cli_results.iter().filter(|(_, success, _)| *success).count();
                    let cli_total_count = cli_results.len();

                    // Print AIW update status
                    if aiw_updated {
                        println!("\n✅ AIW updated successfully!");
                    } else {
                        println!("\n✅ AIW is already up to date!");
                    }

                    // Print AI CLI tools update status
                    if cli_total_count > 0 {
                        if cli_success_count == cli_total_count {
                            println!("✅ All {} AI CLI tool(s) updated successfully!", cli_success_count);
                        } else {
                            eprintln!(
                                "⚠️  {}/{} AI CLI tool(s) updated successfully",
                                cli_success_count, cli_total_count
                            );
                        }
                    } else {
                        println!("ℹ️  No AI CLI tools found to update");
                    }

                    Ok(ExitCode::from(0))
                }
                Err(e) => {
                    eprintln!("❌ Update failed: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        Commands::Mcp(action) => handle_mcp_action(action).await,
        Commands::Plugin(action) => handle_plugin_action(action).await.map_err(|e| e.to_string()),
        Commands::Roles(action) => handle_roles_command(action).await,
        Commands::Config(action) => handle_config_action(action),
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
    use aiw::storage::SharedMemoryStorage;
    use aiw::task_record::TaskStatus;
    use aiw::unified_registry::Registry;

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

    let ai_args = parse_external_cli_args(&tokens)?;

    // selector 现在是 Option<String>，对于 claude/codex/gemini 命令一定是 Some
    let selector_str = ai_args.selector.as_ref().expect("selector should be set for external CLI commands");
    let selector = parse_cli_selector_strict(selector_str).map_err(|err| {
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
    let ai_command = AiCliCommand::new(
        ai_types,
        ai_args.role.clone(),
        ai_args.provider.clone(),
        ai_args.prompt_text(),
        ai_args.cli_args.clone(),
        ai_args.cwd.clone(),
    );

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

fn handle_config_action(action: ConfigAction) -> Result<ExitCode, String> {
    match action {
        ConfigAction::CliOrder => Ok(aiw::commands::auto::handle_cli_order_command()),
    }
}

/// 处理MCP命令
async fn handle_mcp_action(action: McpAction) -> Result<ExitCode, String> {
    match action {
        McpAction::List => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::List).await {
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
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};

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
            })
            .await
            {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Remove { name, yes } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Remove { name, yes }).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Search {
            query,
            source,
            limit,
        } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Search {
                query,
                source,
                limit,
            })
            .await
            {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Install {
            name,
            source,
            env_vars,
            skip_env,
        } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
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

            match handle_mcp_command(McpCommand::Install {
                name,
                source,
                env,
                skip_env,
            })
            .await
            {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Info { name, source } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Info { name, source }).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Update => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Update).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Get { name } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Get { name }).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Enable { name } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Enable { name }).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Disable { name } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Disable { name }).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Edit => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Edit).await {
                Ok(_) => Ok(ExitCode::from(0)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(ExitCode::from(1))
                }
            }
        }
        McpAction::Browse { source } => {
            use aiw::commands::mcp::{handle_mcp_command, McpCommand};
            match handle_mcp_command(McpCommand::Browse { source }).await {
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
    // NOTE: global tracing subscriber is already set in main(), so we just
    // log a debug message here instead of re-initialising.
    tracing::debug!("MCP serve starting with log_level={}", log_level);

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
    use aiw::roles::builtin::list_builtin_roles;

    match action {
        RolesAction::List => {
            // List builtin roles first
            let builtin_roles = list_builtin_roles();
            println!("Builtin roles ({}):", builtin_roles.len());
            for role_name in &builtin_roles {
                println!("  {}", role_name);
            }

            // List user roles
            let manager = RoleManager::new().map_err(|e| format!("Failed to load roles: {}", e))?;
            let user_roles = manager
                .list_all_roles()
                .map_err(|e| format!("Failed to list roles: {}", e))?;

            if !user_roles.is_empty() {
                println!("\nUser roles ({}):", user_roles.len());
                for role in user_roles {
                    println!(
                        "  {}: {} ({})",
                        role.name,
                        role.description,
                        role.file_path.display()
                    );
                }
            }

            println!("\nUsage: aiw claude -r <role_name> \"your task\"");
            println!("Custom roles: ~/.aiw/role/*.md");

            Ok(ExitCode::from(0))
        }
    }
}

/// 运行stdio传输的MCP服务器
async fn run_mcp_server_stdio(
    server: AgenticWardenMcpServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.run().await
}
