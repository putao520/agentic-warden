use std::io::Write;
use std::process::ExitCode;

use crate::auto_mode::executor::AutoModeExecutor;
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

    let mut provider = None;
    let mut prompt_parts = Vec::new();
    let mut i = start;

    // 解析参数
    while i < tokens.len() {
        let token = &tokens[i];

        // 检查 -p 或 --provider 参数
        if token == "-p" || token == "--provider" {
            if i + 1 < tokens.len() {
                provider = Some(tokens[i + 1].clone());
                i += 2; // 跳过参数和值
                continue;
            } else {
                return Err(ExecutionError::ExecutionFailed {
                    message: "Missing value for -p/--provider parameter".to_string(),
                });
            }
        }

        // 其他参数作为 prompt 的一部分
        prompt_parts.push(token.clone());
        i += 1;
    }

    let prompt = prompt_parts.join(" ");
    if prompt.trim().is_empty() {
        return Err(ExecutionError::EmptyPrompt);
    }

    Ok((prompt, provider))
}

fn parse_prompt(tokens: &[String]) -> Result<String, ExecutionError> {
    let (prompt, _) = parse_prompt_and_provider(tokens)?;
    Ok(prompt)
}

fn format_auto_error(err: ExecutionError) -> (u8, String) {
    match err {
        ExecutionError::Config(err) => format_auto_config_error(&err),
        ExecutionError::Judge(err) => (3, err.to_string()),
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
