use std::process::ExitCode;

use crate::commands::parser::parse_cli_args;
use crate::error::{ConfigError, ExecutionError};
use crate::tui::screens::cli_order::run_cli_order_tui;

pub async fn handle_auto_command(args: &[String]) -> ExitCode {
    match parse_auto_args(args) {
        Ok(prompt) => {
            let registry = match crate::registry_factory::create_cli_registry() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to create registry: {}", e);
                    return ExitCode::from(2);
                }
            };

            let base = match crate::task_prepare::prepare_task_base(
                crate::task_prepare::TaskParams {
                    cli_type: crate::cli_type::CliType::Auto,
                    prompt,
                    role: None,
                    provider: None,
                    cli_args: vec![],
                    cwd: None,
                    create_worktree: false,
                },
            ) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::from(2);
                }
            };

            match crate::supervisor::execute_cli_with_failover(&registry, &base).await {
                Ok(exit_code) => ExitCode::from((exit_code & 0xFF) as u8),
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::from(2)
                }
            }
        }
        Err(err) => {
            let (code, message) = format_auto_error(err);
            eprintln!("{}", message);
            ExitCode::from(code)
        }
    }
}

/// 解析 auto 命令的参数，返回 prompt
/// 注意：provider 现在由 auto_execution_order 配置决定，不再从命令行读取
fn parse_auto_args(tokens: &[String]) -> Result<String, ExecutionError> {
    // 跳过第一个 "auto"
    let start = tokens
        .first()
        .filter(|value| value.eq_ignore_ascii_case("auto"))
        .map(|_| 1)
        .unwrap_or(0);

    let tokens = &tokens[start..];

    // 使用统一的参数解析逻辑
    let parsed = parse_cli_args(tokens)
        .map_err(|msg| ExecutionError::ExecutionFailed { message: msg })?;

    // auto 命令只关心 prompt，provider 由配置决定
    let prompt = parsed.prompt.join(" ");
    if prompt.trim().is_empty() {
        return Err(ExecutionError::EmptyPrompt);
    }

    Ok(prompt)
}

pub fn handle_cli_order_command() -> ExitCode {
    match run_cli_order_tui() {
        Ok(()) => ExitCode::from(0),
        Err(err) => {
            if let Some(config_err) = err.downcast_ref::<ConfigError>() {
                let (code, message) = format_cli_order_config_error(config_err);
                eprintln!("{}", message);
                ExitCode::from(code)
            } else {
                eprintln!("TUI error: {}", err);
                ExitCode::from(3)
            }
        }
    }
}

fn format_auto_error(err: ExecutionError) -> (u8, String) {
    match err {
        ExecutionError::Config(err) => format_auto_config_error(&err),
        ExecutionError::AllFailed { .. } => (2, err.to_string()),
        ExecutionError::Halt { .. } => (2, err.to_string()),
        ExecutionError::EmptyPrompt => (1, err.to_string()),
        ExecutionError::ExecutionFailed { .. } => (2, err.to_string()),
    }
}

fn format_auto_config_error(err: &ConfigError) -> (u8, String) {
    let message = err.to_string();
    if is_validation_error(err) {
        (4, format!("Invalid auto_execution_order: {}", message))
    } else {
        (1, format!("Check ~/.aiw/config.json: {}", message))
    }
}

fn format_cli_order_config_error(err: &ConfigError) -> (u8, String) {
    let message = err.to_string();
    if is_permission_error(err) {
        (2, message)
    } else {
        (1, format!("Check ~/.aiw/config.json: {}", message))
    }
}

fn is_validation_error(err: &ConfigError) -> bool {
    matches!(
        err,
        ConfigError::InvalidType
            | ConfigError::InvalidLength { .. }
            | ConfigError::InvalidElementType
            | ConfigError::InvalidCliType { .. }
            | ConfigError::DuplicateCliType
            | ConfigError::IncompleteSet
    )
}

fn is_permission_error(err: &ConfigError) -> bool {
    match err {
        ConfigError::Io { message } => message.to_lowercase().contains("permission denied"),
        _ => false,
    }
}
