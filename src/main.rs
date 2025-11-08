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

use crate::cli_type::parse_cli_selector_strict;
use crate::commands::ai_cli::AiCliCommand;
use crate::commands::{parse_external_as_ai_cli, Cli, Commands};
use crate::error::ErrorCategory;
use crate::help::{print_command_help, print_general_help, print_quick_examples};
use crate::provider::network_detector::NetworkDetector;
use crate::sync::sync_config::save_network_status;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match std::panic::catch_unwind(|| {
        let runtime = tokio::runtime::Runtime::new()
            .expect("Failed to create async runtime");
        runtime.block_on(main_impl())
    }) {
        Ok(Ok(code)) => code,
        Ok(Err(err)) => {
            eprintln!("{}", err);
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!("A fatal error occurred");
            ExitCode::from(1)
        }
    }
}

async fn main_impl() -> Result<ExitCode, String> {
    let command = Cli::parse_command();

    match command {
        Commands::Dashboard => launch_tui(None),
        Commands::Status => launch_tui(Some(crate::tui::ScreenType::Status)),
        Commands::Provider => launch_tui(Some(crate::tui::ScreenType::Provider)),
        Commands::Push { dirs } => {
            let directories = dirs
                .into_iter()
                .map(|dir| dir.to_string_lossy().to_string())
                .collect();
            launch_tui(Some(crate::tui::ScreenType::Push(directories)))
        }
        Commands::Pull => launch_tui(Some(crate::tui::ScreenType::Pull)),
        Commands::Reset => handle_sync_command("reset", None),
        Commands::List => handle_sync_command("list", None),
        Commands::Wait => {
            wait_mode::run().map_err(|e| e.to_string())?;
            Ok(ExitCode::from(0))
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
        Commands::External(tokens) => handle_external_command(tokens),
    }
}

fn launch_tui(initial_screen: Option<crate::tui::ScreenType>) -> Result<ExitCode, String> {
    color_eyre::install().map_err(|e| format!("Failed to install error handler: {}", e))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;

    runtime.block_on(async {
        tokio::spawn(async {
            if let Err(e) = perform_background_network_detection().await {
                eprintln!("Warning: Background network detection failed: {}", e);
            }
        });

        let tui_result = match initial_screen {
            Some(screen) => crate::tui::app::run_tui_app_with_screen(Some(screen)),
            None => crate::tui::app::run_tui_app(),
        };

        tui_result.map_err(|e| format!("TUI error: {}", e))?;

        Ok::<ExitCode, String>(ExitCode::from(0))
    })
}

fn handle_sync_command(command: &str, args: Option<Vec<PathBuf>>) -> Result<ExitCode, String> {
    let config_name = match (command, args) {
        ("push", Some(dirs)) => dirs
            .into_iter()
            .next()
            .map(|dir| dir.to_string_lossy().to_string())
            .or_else(|| Some("default".to_string())),
        ("pull", _) => Some("default".to_string()),
        _ => None,
    };

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;

    runtime.block_on(async {
        match sync::sync_command::handle_sync_command(command, config_name).await {
            Ok(code) => Ok(ExitCode::from((code & 0xFF) as u8)),
            Err(e) => Err(format!("Sync command failed: {}", e)),
        }
    })
}

fn handle_external_command(tokens: Vec<String>) -> Result<ExitCode, String> {
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

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;

    runtime.block_on(async { ai_command.execute().await.map_err(|e| e.to_string()) })
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

