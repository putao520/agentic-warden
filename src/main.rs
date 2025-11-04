mod cli_manager;
mod cli_type;
mod commands;
mod config;
mod core;
mod error;
mod help;
mod logging;
mod platform;
mod provider;
mod registry;
mod signal;
mod supervisor;
mod sync;
mod task_record;
mod tui;
mod utils;
mod wait_mode;

use crate::cli_type::parse_cli_selector;
use crate::help::{print_command_help, print_general_help, print_quick_examples, print_version};
use crate::provider::network_detector::NetworkDetector;
use crate::registry::TaskRegistry;
use crate::sync::sync_config::save_network_status;
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Handle sync commands separately to avoid runtime conflicts
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if !args.is_empty() {
        if let Some(first_arg) = args[0].to_str() {
            match first_arg.to_lowercase().as_str() {
            // Help commands
            "--help" | "-h" | "help" => {
                if args.len() > 1 {
                    if let Some(second_arg) = args[1].to_str() {
                        if let Err(e) = print_command_help(second_arg) {
                            eprintln!("Failed to print help: {}", e);
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    if let Err(e) = print_general_help() {
                        eprintln!("Failed to print help: {}", e);
                        return ExitCode::from(1);
                    }
                }
                return ExitCode::from(0);
            }

            // Version command
            "--version" | "-v" => {
                if let Err(e) = print_version() {
                    eprintln!("Failed to print version: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::from(0);
            }

            // Quick examples
            "examples" | "demo" => {
                if let Err(e) = print_quick_examples() {
                    eprintln!("Failed to print examples: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::from(0);
            }

            // Provider commands - now launches TUI
            "provider" => {
                // Initialize color-eyre for better error handling
                color_eyre::install().unwrap_or_default();

                // Launch TUI with Provider Management screen
                use agentic_warden::tui::app::run_tui_app_with_screen;
                use agentic_warden::tui::ScreenType;
                if let Err(e) = run_tui_app_with_screen(Some(ScreenType::Provider)) {
                    eprintln!("TUI error: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::from(0);
            }

            // Status command - launches TUI
            "status" => {
                // Initialize color-eyre for better error handling
                color_eyre::install().unwrap_or_default();

                // Launch TUI with Sync Status screen
                use agentic_warden::tui::app::run_tui_app_with_screen;
                use agentic_warden::tui::ScreenType;
                if let Err(e) = run_tui_app_with_screen(Some(ScreenType::Status)) {
                    eprintln!("TUI error: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::from(0);
            }

            // Sync commands (but not "status" which is handled above)
            "push" | "pull" | "reset" | "list" => {
                // Check for help flag
                if args.len() > 1 {
                    if let Some(arg) = args[1].to_str() {
                        if arg == "--help" || arg == "-h" {
                            if let Err(e) = print_command_help(first_arg) {
                                eprintln!("Failed to print help: {}", e);
                                return ExitCode::from(1);
                            }
                            return ExitCode::from(0);
                        }
                    }
                }

                // Get configuration name (optional for push/pull, defaults to "default")
                let config_name = if args.len() > 1 {
                    args[1].to_str().map(|s| s.to_string())
                } else if first_arg == "push" || first_arg == "pull" {
                    // push and pull use "default" if no name provided
                    Some("default".to_string())
                } else {
                    None
                };

                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| {
                        eprintln!("Failed to create async runtime: {}", e);
                        1i32
                    })
                    .unwrap_or_else(|_| std::process::exit(1));

                match rt.block_on(async {
                    sync::sync_command::handle_sync_command(first_arg, config_name).await
                }) {
                    Ok(code) => return ExitCode::from((code & 0xFF) as u8),
                    Err(e) => {
                        eprintln!("Sync command failed: {}", e);
                        return ExitCode::from(1);
                    }
                }
            }
            _ => {
                // Continue to normal processing for other commands
            }
        }
        }
    }

    // Normal processing for other commands
    match std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create async runtime: {}", e))?;

        rt.block_on(async {
            // Start background network detection
        tokio::spawn(async {
            if let Err(e) = perform_background_network_detection().await {
                eprintln!("Warning: Background network detection failed: {}", e);
            }
        });

            run().await.map_err(|e| format!("Run failed: {}", e))
        })
    }) {
        Ok(result) => match result {
            Ok(code) => ExitCode::from((code & 0xFF) as u8),
            Err(err) => {
                eprintln!("{}", err);
                ExitCode::from(1)
            }
        },
        Err(_) => {
            eprintln!("A fatal error occurred");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<i32, String> {
    let mut args_iter = env::args_os();
    args_iter.next(); // skip program name
    let args: Vec<OsString> = args_iter.collect();

    // No arguments - launch Dashboard TUI
    if args.is_empty() {
        // Initialize color-eyre for better error handling
        color_eyre::install().map_err(|e| format!("Failed to install error handler: {}", e))?;

        // Start background network detection
        tokio::spawn(async {
            if let Err(e) = perform_background_network_detection().await {
                eprintln!("Warning: Background network detection failed: {}", e);
            }
        });

        // Launch Dashboard TUI
        use agentic_warden::tui::app::run_tui_app;
        run_tui_app().map_err(|e| format!("TUI error: {}", e))?;

        return Ok(0);
    }

    // Check for wait command
    if args.len() == 1
        && args[0]
            .to_str()
            .is_some_and(|cmd| cmd.eq_ignore_ascii_case("wait"))
    {
        wait_mode::run().map_err(|e| e.to_string())?;
        return Ok(0);
    }

    // Parse CLI type from first argument
    let first_arg = args[0]
        .to_str()
        .ok_or_else(|| "Invalid UTF-8 in arguments".to_string())?;

    let cli_selector = parse_cli_selector(first_arg).ok_or_else(|| {
        format!(
            "Unsupported agent type: '{}'. Supported types: claude, codex, gemini, all, or combinations like claude|gemini\n\nUse 'agentic-warden --help' for more information.",
            first_arg
        )
    })?;

    // Parse -p/--provider parameter and task prompt
    let mut provider: Option<String> = None;
    let mut prompt_parts: Vec<&str> = Vec::new();
    let mut i = 1; // skip cli_type

    while i < args.len() {
        if let Some(arg_str) = args[i].to_str() {
            match arg_str {
                "-p" | "--provider" => {
                    if i + 1 < args.len() {
                        provider = args[i + 1].to_str().map(|s| s.to_string());
                        i += 2;
                    } else {
                        return Err("Error: -p/--provider requires a value".to_string());
                    }
                }
                _ => {
                    prompt_parts.push(arg_str);
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }

    let task_prompt = prompt_parts.join(" ");
    let registry = TaskRegistry::connect().map_err(|e| e.to_string())?;

    // If it's a single CLI, use single CLI execution
    if cli_selector.types.len() == 1 {
        let cli_type = &cli_selector.types[0];

        // Check if it's interactive mode (no prompt provided)
        if task_prompt.is_empty() {
            // 交互模式：直接启动AI CLI
            let exit_code = supervisor::start_interactive_cli(&registry, cli_type, provider)
                .map_err(|e| e.to_string())?;
            Ok(exit_code)
        } else {
            // 任务模式：执行提示词任务
            let cli_args = cli_type.build_full_access_args(&task_prompt);
            let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

            let exit_code = supervisor::execute_cli(&registry, cli_type, &os_args, provider)
                .map_err(|e| e.to_string())?;
            Ok(exit_code)
        }
    } else {
        // 多个CLI模式 - 必须提供提示词（交互模式不支持多CLI）
        if task_prompt.is_empty() {
            return Err("Error: Interactive mode only supports single CLI. Please provide a task description for multiple CLI execution.".to_string());
        }

        // 多个CLI，使用批量执行
        println!(
            "Starting tasks for CLI(s): {}",
            cli_selector
                .types
                .iter()
                .map(|t| t.display_name())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let exit_codes =
            supervisor::execute_multiple_clis(&registry, &cli_selector, &task_prompt, provider)
                .map_err(|e| e.to_string())?;

        // 返回第一个失败的exit code，或者0如果全部成功
        let final_exit_code = exit_codes
            .iter()
            .find(|&&code| code != 0)
            .copied()
            .unwrap_or(0);
        Ok(final_exit_code)
    }
}

/// Perform background network detection to update cached network status (non-blocking)
async fn perform_background_network_detection() -> anyhow::Result<()> {
    let detector = NetworkDetector::new();
    let status = detector.detect().await?;

    // Save network status to sync configuration for future use
    if let Err(e) = save_network_status(status) {
        eprintln!("Warning: Failed to save network status: {}", e);
    }

    Ok(())
}
