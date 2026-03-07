use std::process::ExitCode;

use crate::commands::cli_args::CliInvocation;
use crate::error::{ConfigError, ExecutionError};
use crate::tui::screens::cli_order::run_cli_order_tui;

pub async fn handle_auto_command(args: &[String]) -> ExitCode {
    // 跳过第一个 "auto"
    let start = args
        .first()
        .filter(|value| value.eq_ignore_ascii_case("auto"))
        .map(|_| 1)
        .unwrap_or(0);

    let tokens = &args[start..];

    // 使用新的 CliInvocation 解析
    let inv = match CliInvocation::from_auto(tokens) {
        Ok(i) => i,
        Err(err) => {
            let (code, message) = format_auto_error(ExecutionError::ExecutionFailed { message: err });
            eprintln!("{}", message);
            return ExitCode::from(code);
        }
    };

    // auto 命令必须有提示词（remaining_args 不为空）
    if inv.is_interactive() {
        let (code, message) = format_auto_error(ExecutionError::EmptyPrompt);
        eprintln!("{}", message);
        return ExitCode::from(code);
    }

    // prompt 就是 remaining_args joined with spaces
    let prompt = inv.remaining_args.join(" ");

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
            role: inv.aiw_args.role,
            provider: inv.aiw_args.provider,
            cli_args: inv.remaining_args,
            cwd: inv.aiw_args.cwd,
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
