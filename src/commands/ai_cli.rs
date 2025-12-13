//! AI CLI 鍚姩鍛戒护澶勭悊閫昏緫
//!
//! 澶勭悊 codex銆乧laude銆乬emini 绛?AI CLI 鐨勫惎鍔ㄥ拰绠＄悊

use crate::cli_type::{parse_cli_selector_strict, CliType};
use crate::registry_factory::create_cli_registry;
use crate::supervisor;
use anyhow::{anyhow, Result};
use std::ffi::OsString;
use std::process::ExitCode;

/// AI CLI 鍚姩鍙傛暟
pub struct AiCliCommand {
    pub ai_types: Vec<CliType>,
    pub provider: Option<String>,
    pub prompt: String,
    pub cli_args: Vec<String>,
}

impl AiCliCommand {
    /// 鍒涘缓鏂扮殑 AI CLI 鍛戒护
    pub fn new(
        ai_types: Vec<CliType>,
        provider: Option<String>,
        prompt: String,
        cli_args: Vec<String>,
    ) -> Self {
        Self {
            ai_types,
            provider,
            prompt,
            cli_args,
        }
    }

    /// 鎵ц AI CLI 鍛戒护
    pub async fn execute(&self) -> Result<ExitCode> {
        let registry = create_cli_registry()?;

        // 妫€鏌ユ槸鍚︽槸浜や簰妯″紡锛堟棤鎻愮ず璇嶏級
        if self.prompt.is_empty() {
            // 浜や簰妯″紡浠呮敮鎸佸崟涓?CLI
            if self.ai_types.len() != 1 {
                return Err(anyhow!(
                    "Interactive mode only supports single CLI. Please provide a task description for multiple CLI execution."
                ));
            }

            let cli_type = &self.ai_types[0];
            let exit_code =
                supervisor::start_interactive_cli(
                    &registry,
                    cli_type,
                    self.provider.clone(),
                    &self.cli_args,
                )
                .await?;
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else {
            // 浠诲姟妯″紡
            if self.ai_types.len() == 1 {
                // 鍗曚釜 CLI 鎵ц
                let cli_type = &self.ai_types[0];
                let cli_args =
                    cli_type.build_full_access_args_with_cli(&self.prompt, &self.cli_args);
                let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

                let exit_code =
                    supervisor::execute_cli(&registry, cli_type, &os_args, self.provider.clone())
                        .await?;
                Ok(ExitCode::from((exit_code & 0xFF) as u8))
            } else {
                // 澶氫釜 CLI 鎵归噺鎵ц
                println!(
                    "Starting tasks for CLI(s): {}",
                    self.ai_types
                        .iter()
                        .map(|t| t.display_name())
                        .collect::<Vec<_>>()
                    .join(", ")
                );

                let cli_selector = crate::cli_type::CliSelector {
                    types: self.ai_types.clone(),
                };

                let exit_codes = supervisor::execute_multiple_clis(
                    &registry,
                    &cli_selector,
                    &self.prompt,
                    self.provider.clone(),
                    &self.cli_args,
                )
                .await?;

                // 杩斿洖绗竴涓け璐ョ殑 exit code锛屾垨鑰?0 濡傛灉鍏ㄩ儴鎴愬姛
                let final_exit_code = exit_codes
                    .iter()
                    .find(|&&code| code != 0)
                    .copied()
                    .unwrap_or(0);

                Ok(ExitCode::from((final_exit_code & 0xFF) as u8))
            }
        }
    }
}

/// 瑙ｆ瀽 AI 绫诲瀷瀛楃涓?
pub fn parse_ai_types(input: &str) -> Result<Vec<CliType>> {
    let selector = parse_cli_selector_strict(input).map_err(|err| anyhow!(err.to_string()))?;
    Ok(selector.types)
}
