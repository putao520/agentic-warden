use std::io::Write;
use std::process::ExitCode;

use crate::auto_mode::executor::AutoModeExecutor;
use crate::commands::parser::separate_provider_and_cli_args;
use crate::error::{ConfigError, ExecutionError};
use crate::tui::screens::cli_order::run_cli_order_tui;

pub async fn handle_auto_command(args: &[String]) -> ExitCode {
    let (prompt, provider) = match parse_prompt_and_provider(args) {
        Ok(result) => result,
        Err(err) => {
            let (code, message) = format_auto_error(err);
            eprintln!("{}", message);
            return ExitCode::from(code);
        }
    };

    match AutoModeExecutor::execute(&prompt, provider) {
        Ok(output) => {
            if !output.is_empty() {
                print!("{}", output);
                let _ = std::io::stdout().flush();
            }
            ExitCode::from(0)
        }
        Err(err) => {
            let (code, message) = format_auto_error(err);
            eprintln!("{}", message);
            ExitCode::from(code)
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

fn parse_prompt_and_provider(tokens: &[String]) -> Result<(String, Option<String>), ExecutionError> {
    // 跳过第一个 "auto"
    let start = tokens
        .first()
        .filter(|value| value.eq_ignore_ascii_case("auto"))
        .map(|_| 1)
        .unwrap_or(0);

    let tokens = &tokens[start..];

    // 使用与其他 AI CLI 命令相同的参数解析逻辑
    let separated = separate_provider_and_cli_args(tokens)
        .map_err(|msg| ExecutionError::ExecutionFailed { message: msg })?;

    // 忽略 role 和 cwd（auto 命令不使用这些参数）
    let _ = separated.role;
    let _ = separated.cwd;

    // 忽略 cli_args（auto 命令不透传 CLI 参数给底层 AI CLI）
    let _ = separated.cli_args;

    let prompt = separated.prompt.join(" ");
    if prompt.trim().is_empty() {
        return Err(ExecutionError::EmptyPrompt);
    }

    Ok((prompt, separated.provider))
}

fn parse_prompt(tokens: &[String]) -> Result<String, ExecutionError> {
    let (prompt, _) = parse_prompt_and_provider(tokens)?;
    Ok(prompt)
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
        (4, format!("Invalid cli_execution_order: {}", message))
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
