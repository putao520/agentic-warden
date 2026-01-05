//! AI CLI 启动命令处理逻辑
//!
//! 处理 codex、claude、gemini 等 AI CLI 的启动和管理

use crate::cli_type::{parse_cli_selector_strict, CliType};
use crate::registry_factory::create_cli_registry;
use crate::roles::{builtin::get_builtin_role, RoleManager, Role};
use crate::supervisor;
use anyhow::{anyhow, Result};
use std::ffi::OsString;
use std::process::ExitCode;

/// AI CLI 启动参数
pub struct AiCliCommand {
    pub ai_types: Vec<CliType>,
    pub role: Option<String>,
    pub provider: Option<String>,
    pub prompt: String,
    pub cli_args: Vec<String>,
    pub cwd: Option<std::path::PathBuf>,
}

impl AiCliCommand {
    /// 创建新的 AI CLI 命令
    pub fn new(
        ai_types: Vec<CliType>,
        role: Option<String>,
        provider: Option<String>,
        prompt: String,
        cli_args: Vec<String>,
        cwd: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            ai_types,
            role,
            provider,
            prompt,
            cli_args,
            cwd,
        }
    }

    /// 获取用户首选语言
    /// 使用系统 locale 自动检测语言
    fn get_preferred_language() -> String {
        // Get system locale string
        if let Some(locale) = sys_locale::get_locale() {
            // Check if locale starts with "zh" (Chinese)
            // Only Chinese locales use Chinese, all others use English
            if locale.starts_with("zh") {
                return "zh-CN".to_string();
            }
        }
        // Default to English for all non-Chinese locales (en_*, ja_*, ko_*, de_*, etc.)
        "en".to_string()
    }

    /// Default fallback role when specified role is not found
    const DEFAULT_ROLE: &'static str = "common";

    /// 解析逗号分隔的角色字符串（去重，保持顺序）
    fn parse_role_names(role_str: &str) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        role_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter(|s| seen.insert(*s))  // 去重：只保留首次出现
            .collect()
    }

    /// 加载单个角色（优先用户自定义，其次内置）
    fn load_single_role(name: &str, lang: &str) -> Option<Role> {
        // Try user-defined roles first (allows overriding built-in roles)
        if let Ok(manager) = RoleManager::new() {
            if let Ok(role) = manager.get_role(name) {
                return Some(role);
            }
        }
        // Fall back to built-in roles
        if let Ok(role) = get_builtin_role(name, lang) {
            return Some(role);
        }
        None
    }

    /// 加载多个角色
    /// Returns: (valid_roles, invalid_names)
    fn load_roles(names: &[&str], lang: &str) -> (Vec<Role>, Vec<String>) {
        let mut valid_roles = Vec::new();
        let mut invalid_names = Vec::new();

        for name in names {
            if let Some(role) = Self::load_single_role(name, lang) {
                valid_roles.push(role);
            } else {
                invalid_names.push(name.to_string());
            }
        }

        (valid_roles, invalid_names)
    }

    /// 组合多个角色内容
    fn combine_role_contents(roles: &[Role], prompt: &str) -> String {
        if roles.is_empty() {
            return prompt.to_string();
        }

        let role_contents: Vec<&str> = roles.iter().map(|r| r.content.as_str()).collect();
        let combined = role_contents.join("\n\n---\n\n");
        format!("{}\n\n---\n\n{}", combined, prompt)
    }

    /// 应用角色到提示词（支持多角色）
    fn apply_role(&self, prompt: &str) -> Result<String> {
        if let Some(role_str) = &self.role {
            let lang = Self::get_preferred_language();
            let role_names = Self::parse_role_names(role_str);

            if role_names.is_empty() {
                return Ok(prompt.to_string());
            }

            let (valid_roles, invalid_names) = Self::load_roles(&role_names, &lang);

            // Warn about invalid roles
            for name in &invalid_names {
                eprintln!("Warning: Role '{}' not found, skipping.", name);
            }

            // If all roles are invalid, fallback to default
            if valid_roles.is_empty() {
                eprintln!(
                    "Warning: All specified roles not found, falling back to '{}' role.",
                    Self::DEFAULT_ROLE
                );
                if let Some(fallback) = Self::load_single_role(Self::DEFAULT_ROLE, &lang) {
                    return Ok(Self::combine_role_contents(&[fallback], prompt));
                }
                // Even fallback failed
                eprintln!("Warning: Default role '{}' also not available.", Self::DEFAULT_ROLE);
                return Ok(prompt.to_string());
            }

            return Ok(Self::combine_role_contents(&valid_roles, prompt));
        }

        Ok(prompt.to_string())
    }

    /// 执行 AI CLI 命令
    pub async fn execute(&self) -> Result<ExitCode> {
        let registry = create_cli_registry()?;

        if self.ai_types.iter().any(|cli_type| matches!(cli_type, CliType::Auto)) {
            return Err(anyhow!(
                "Auto CLI type is only supported via `aiw auto`"
            ));
        }

        // 应用角色到提示词
        let final_prompt = if !self.prompt.is_empty() {
            self.apply_role(&self.prompt)?
        } else {
            self.prompt.clone()
        };

        // 检查是否是交互模式（无提示词）
        if final_prompt.is_empty() {
            // 交互模式只支持单个 CLI
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
                    self.cwd.clone(),
                )
                .await?;
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else {
            // 任务模式
            if self.ai_types.len() == 1 {
                // 单个 CLI 执行
                let cli_type = &self.ai_types[0];
                let cli_args =
                    cli_type.build_full_access_args_with_cli(&final_prompt, &self.cli_args);
                let os_args: Vec<OsString> = cli_args.into_iter().map(|s| s.into()).collect();

                let exit_code =
                    supervisor::execute_cli(&registry, cli_type, &os_args, self.provider.clone(), self.cwd.clone())
                        .await?;
                Ok(ExitCode::from((exit_code & 0xFF) as u8))
            } else {
                // 多个 CLI 批量执行
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
                    &final_prompt,
                    self.provider.clone(),
                    &self.cli_args,
                    self.cwd.clone(),
                )
                .await?;

                // 返回第一个失败的 exit code，或者 0 如果全部成功
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

/// 解析 AI 类型字符串
pub fn parse_ai_types(input: &str) -> Result<Vec<CliType>> {
    let selector = parse_cli_selector_strict(input).map_err(|err| anyhow!(err.to_string()))?;
    Ok(selector.types)
}
