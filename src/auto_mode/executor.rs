use std::ffi::OsString;
use std::future::Future;
use std::time::Instant;

use crate::auto_mode::{CliCooldownManager, config::ExecutionOrderConfig, ExecutionEntry, ExecutionResult};
use crate::cli_type::CliType;
use crate::error::ExecutionError;
use crate::registry_factory::create_cli_registry;
use crate::supervisor::{self, ProcessError};

pub struct AutoModeExecutor;

impl AutoModeExecutor {
    /// 执行 auto 模式（支持 CLI+Provider 组合轮转）
    pub fn execute(prompt: &str) -> Result<String, ExecutionError> {
        if prompt.trim().is_empty() {
            return Err(ExecutionError::EmptyPrompt);
        }

        let entries = ExecutionOrderConfig::get_execution_entries()?;
        let registry = create_cli_registry()
            .map_err(|err| ExecutionError::ExecutionFailed { message: err.to_string() })?;

        let cooldown = CliCooldownManager::global();
        let mut last_error = None;
        let mut skipped_count = 0;
        let total_count = entries.len();

        for entry in entries {
            let cli_type = match entry.to_cli_type() {
                Some(t) => t,
                None => {
                    println!("⚠ Invalid CLI type '{}', skipping...", entry.cli);
                    continue;
                }
            };

            // 检查 (CLI, Provider) 组合是否在冷却期
            if cooldown.is_in_cooldown(&cli_type, &entry.provider) {
                if let Some(remaining) = cooldown.remaining_cooldown_secs(&cli_type, &entry.provider) {
                    println!("⏸️ {} is in cooldown ({}s remaining), skipping...",
                             entry.display_name(), remaining);
                    skipped_count += 1;
                    continue;
                }
            }

            println!("✓ Trying {}...", entry.display_name());

            let result = Self::try_cli(&registry, cli_type.clone(), prompt, &entry.provider);

            // 简单判断：退出码为 0 表示成功
            if result.exit_code == 0 {
                println!("✓ {} succeeded", entry.display_name());
                return Ok(result.stdout);
            }

            // 标记 (CLI, Provider) 组合进入冷却期
            cooldown.mark_failure(&cli_type, &entry.provider);

            let failure_message = Self::summarize_failure(&result);
            println!("⚠ {} failed (exit code {}): {}", entry.display_name(), result.exit_code, failure_message);
            last_error = Some(failure_message);

            println!("  Trying next entry...");
        }

        // 如果所有组合都被跳过，返回错误
        if skipped_count == total_count {
            return Err(ExecutionError::AllFailed {
                message: "All CLI+Provider combinations are in cooldown period. Please wait 30 seconds and try again.".to_string(),
            });
        }

        Err(ExecutionError::AllFailed {
            message: last_error.unwrap_or_else(|| "All CLI+Provider combinations failed".to_string()),
        })
    }

    pub fn try_cli(
        registry: &crate::registry_factory::CliRegistry,
        cli_type: CliType,
        prompt: &str,
        provider: &str,
    ) -> ExecutionResult {
        let start = Instant::now();

        if matches!(cli_type, CliType::Auto) {
            return ExecutionResult {
                cli_type,
                provider: provider.to_string(),
                prompt: prompt.to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: "Auto CLI type cannot be executed directly".to_string(),
                duration: start.elapsed(),
            };
        }

        let cli_args = cli_type.build_full_access_args(prompt);
        let os_args: Vec<OsString> = cli_args.into_iter().map(OsString::from).collect();

        // 使用指定的 provider
        let provider_str = if provider == "auto" {
            None // 使用 default_provider
        } else {
            Some(provider.to_string())
        };

        let output = Self::run_async(supervisor::execute_cli_with_full_output(
            registry,
            &cli_type,
            &os_args,
            provider_str,
            std::time::Duration::MAX,  // 无超时限制
            None,
        ));

        match output {
            Ok((exit_code, captured)) => ExecutionResult {
                cli_type,
                provider: provider.to_string(),
                prompt: prompt.to_string(),
                exit_code,
                stdout: captured.stdout,
                stderr: captured.stderr,
                duration: start.elapsed(),
            },
            Err(err) => ExecutionResult {
                cli_type,
                provider: provider.to_string(),
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
