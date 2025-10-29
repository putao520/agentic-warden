mod cli_manager;
mod cli_type;
mod config;
mod help;
mod logging;
mod platform;
mod process_tree;
mod registry;
mod shared_map;
mod signal;
mod supervisor;
mod sync;
mod task_record;
mod wait_mode;

use crate::cli_manager::CliManager;
use crate::cli_type::parse_cli_selector;
use crate::help::{print_command_help, print_general_help, print_quick_examples, print_version};
use crate::registry::TaskRegistry;
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Handle sync commands separately to avoid runtime conflicts
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if !args.is_empty()
        && let Some(first_arg) = args[0].to_str()
    {
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

            // Sync commands
            "push" | "pull" | "status" | "reset" | "list" => {
                // Handle sync commands directly
                let directories = if args.len() > 1 {
                    Some(
                        args[1..]
                            .iter()
                            .filter_map(|arg| arg.to_str())
                            .map(|s| s.to_string())
                            .collect(),
                    )
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
                    sync::sync_command::handle_sync_command(first_arg, directories).await
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

    // Normal processing for other commands
    match std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create async runtime: {}", e))?;

        rt.block_on(async { run().await.map_err(|e| format!("Run failed: {}", e)) })
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

    // No arguments - run CLI management flow
    if args.is_empty() {
        // Initialize color-eyre for better error handling
        color_eyre::install().map_err(|e| format!("Failed to install error handler: {}", e))?;

        let cli_manager =
            CliManager::new().map_err(|e| format!("Failed to create CLI manager: {}", e))?;

        cli_manager
            .run_management_flow()
            .await
            .map_err(|e| format!("CLI management flow failed: {}", e))?;

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

    // 检查是否是任务提示词模式
    if args.len() < 2 {
        return Err(format!(
            "Usage: agentic-warden {} \"<task description>\"\nExample: agentic-warden {} \"写一段Rust代码\"",
            first_arg, first_arg
        ));
    }

    let registry = TaskRegistry::connect().map_err(|e| e.to_string())?;

    // 将剩余参数合并为任务提示词
    let task_prompt = args[1..]
        .iter()
        .filter_map(|arg| arg.to_str())
        .collect::<Vec<_>>()
        .join(" ");

    // 如果是单个CLI，使用单CLI执行
    if cli_selector.types.len() == 1 {
        let cli_type = &cli_selector.types[0];
        let cli_args = cli_type.build_full_access_args(&task_prompt);
        let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

        let exit_code =
            supervisor::execute_cli(&registry, cli_type, &os_args).map_err(|e| e.to_string())?;
        Ok(exit_code)
    } else {
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

        let exit_codes = supervisor::execute_multiple_clis(&registry, &cli_selector, &task_prompt)
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
