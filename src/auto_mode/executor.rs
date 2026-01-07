use std::ffi::OsString;
use std::future::Future;
use std::time::Instant;

use crate::auto_mode::{config::ExecutionOrderConfig, ExecutionResult};
use crate::cli_type::CliType;
use crate::error::ExecutionError;
use crate::registry_factory::create_cli_registry;
use crate::supervisor::{self, ProcessError};

pub struct AutoModeExecutor;

impl AutoModeExecutor {
    pub fn execute(prompt: &str, provider: Option<String>) -> Result<String, ExecutionError> {
        if prompt.trim().is_empty() {
            return Err(ExecutionError::EmptyPrompt);
        }

        let order = ExecutionOrderConfig::get_order()?;
        let registry = create_cli_registry()
            .map_err(|err| ExecutionError::ExecutionFailed { message: err.to_string() })?;

        let mut last_error = None;

        for cli_type in order {
            println!("✓ Trying {}...", cli_type.display_name());

            let result = Self::try_cli(&registry, cli_type.clone(), prompt, provider.clone());

            // 简单判断：退出码为 0 表示成功
            if result.exit_code == 0 {
                println!("✓ {} succeeded", cli_type.display_name());
                return Ok(result.stdout);
            }

            let failure_message = Self::summarize_failure(&result);
            println!("⚠ {} failed (exit code {}): {}", cli_type.display_name(), result.exit_code, failure_message);
            last_error = Some(failure_message);

            println!("  Trying next CLI...");
        }

        Err(ExecutionError::AllFailed {
            message: last_error.unwrap_or_else(|| "All AI CLIs failed".to_string()),
        })
    }

    pub fn try_cli(
        registry: &crate::registry_factory::CliRegistry,
        cli_type: CliType,
        prompt: &str,
        provider: Option<String>,
    ) -> ExecutionResult {
        let start = Instant::now();

        if matches!(cli_type, CliType::Auto) {
            return ExecutionResult {
                cli_type,
                prompt: prompt.to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: "Auto CLI type cannot be executed directly".to_string(),
                duration: start.elapsed(),
            };
        }

        let cli_args = cli_type.build_full_access_args(prompt);
        let os_args: Vec<OsString> = cli_args.into_iter().map(OsString::from).collect();

        // 不使用超时，让 AI CLI 自然执行
        // 使用指定的 provider，如果没有指定则使用 "auto"
        let provider = provider.unwrap_or_else(|| "auto".to_string());
        let output = Self::run_async(supervisor::execute_cli_with_full_output(
            registry,
            &cli_type,
            &os_args,
            Some(provider),
            std::time::Duration::MAX,  // 无超时限制
            None,
        ));

        match output {
            Ok((exit_code, captured)) => ExecutionResult {
                cli_type,
                prompt: prompt.to_string(),
                exit_code,
                stdout: captured.stdout,
                stderr: captured.stderr,
                duration: start.elapsed(),
            },
            Err(err) => ExecutionResult {
                cli_type,
                prompt: prompt.to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: err.to_string(),
                duration: start.elapsed(),
            },
        }
    }

    fn summarize_failure(result: &ExecutionResult) -> String {
        let stderr = result.stderr.trim();
        if !stderr.is_empty() {
            return stderr
                .lines()
                .next()
                .unwrap_or(stderr)
                .trim()
                .to_string();
        }

        let stdout = result.stdout.trim();
        if !stdout.is_empty() {
            return stdout
                .lines()
                .next()
                .unwrap_or(stdout)
                .trim()
                .to_string();
        }

        "Unknown error".to_string()
    }

    fn run_async<T, F>(future: F) -> Result<T, ProcessError>
    where
        F: Future<Output = Result<T, ProcessError>>,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
            Err(_) => {
                let runtime =
                    tokio::runtime::Runtime::new().map_err(|err| ProcessError::Other(err.to_string()))?;
                runtime.block_on(future)
            }
        }
    }
}
