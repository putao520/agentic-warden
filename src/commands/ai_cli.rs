//! AI CLI 启动命令处理逻辑
//!
//! 处理 codex、claude、gemini 等 AI CLI 的启动和管理

use crate::cli_type::{parse_cli_selector_strict, CliType};
use crate::registry_factory::create_cli_registry;
use crate::roles::{builtin::get_builtin_role, RoleManager, Role};
use crate::supervisor;
use anyhow::{anyhow, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

/// Worktree 信息（用于任务完成后输出）
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
}

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

    /// 检查指定路径是否是 git 仓库
    fn check_git_repository(work_dir: &std::path::PathBuf) -> Result<()> {
        // 使用 git2 检查是否是 git 仓库
        match git2::Repository::discover(work_dir) {
            Ok(_) => Ok(()),
            Err(e) if e.class() == git2::ErrorClass::Repository => {
                Err(anyhow!(
                    "Error: Not a git repository. Please initialize git first:\n  cd {} && git init",
                    work_dir.display()
                ))
            }
            Err(e) => {
                // 其他 git 错误也视为检查失败
                Err(anyhow!(
                    "Error: Unable to access git repository: {}",
                    e.message()
                ))
            }
        }
    }

    /// 生成 8 位随机小写 hex 字符串用于 worktree 命名
    fn generate_worktree_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        // 使用时间戳的低位生成 8 字节 hex
        format!("{:08x}", timestamp % 0x100000000)
    }

    /// 创建 git worktree
    /// 返回: (worktree_path, branch_name, commit_hash)
    fn create_worktree(repo_path: &PathBuf) -> Result<(PathBuf, String, String)> {
        let repo = git2::Repository::open(repo_path)
            .map_err(|e| anyhow!("Failed to open git repository: {}", e.message()))?;

        // 获取当前 HEAD 的 commit
        let head = repo.head()
            .map_err(|e| anyhow!("Failed to get HEAD: {}", e.message()))?;
        let commit = head.peel_to_commit()
            .map_err(|e| anyhow!("Failed to peel to commit: {}", e.message()))?;
        let commit_hash = commit.id().to_string();

        // 获取当前分支名
        let branch_name = head.shorthand()
            .unwrap_or("HEAD")
            .to_string();

        // 创建 worktree 目录：/tmp/aiw-worktree-<8位hex>
        let worktree_id = Self::generate_worktree_id();
        let worktree_path = std::path::PathBuf::from("/tmp")
            .join(format!("aiw-worktree-{}", worktree_id));

        // 检查 worktree 是否已存在
        if worktree_path.exists() {
            return Err(anyhow!(
                "Worktree directory already exists: {}. Please remove it manually.",
                worktree_path.display()
            ));
        }

        // 使用 git command 创建 worktree（git2 库的 worktree API 不可用）
        // 格式: git worktree add <path> <commit-ish>
        let status = std::process::Command::new("git")
            .args(["worktree", "add", "-b"])
            .arg(&format!("aiw-worktree-{}", worktree_id))
            .arg(&worktree_path)
            .arg(&commit_hash)
            .current_dir(repo_path)
            .output()
            .map_err(|e| anyhow!("Failed to execute git worktree command: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(anyhow!("Failed to create worktree: {}", stderr));
        }

        Ok((worktree_path, branch_name, commit_hash))
    }

    /// 输出 worktree 信息到 stdout
    fn output_worktree_info(info: &WorktreeInfo) {
        println!();
        println!("=== AIW WORKTREE END ===");
        println!("Worktree: {}", info.path.display());
        println!("Branch: {}", info.branch);
        println!("Commit: {}", info.commit);
    }

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
        // 确定工作目录
        let original_dir = self.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| ".".into())
        });

        // 检查是否是 git 仓库
        Self::check_git_repository(&original_dir)?;

        // 创建 worktree 作为隔离工作目录
        let (worktree_path, branch_name, commit_hash) = Self::create_worktree(&original_dir)?;

        // worktree 信息用于任务完成后输出
        let worktree_info = WorktreeInfo {
            path: worktree_path.clone(),
            branch: branch_name.clone(),
            commit: commit_hash.clone(),
        };

        eprintln!("Created worktree at: {}", worktree_path.display());
        eprintln!("Branch: {}, Commit: {}", branch_name, commit_hash);

        // 使用 worktree 作为工作目录
        let work_dir = Some(worktree_path);

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
                    work_dir.clone(),
                )
                .await?;

            // 交互模式完成后输出 worktree 信息
            Self::output_worktree_info(&worktree_info);
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
                    supervisor::execute_cli(&registry, cli_type, &os_args, self.provider.clone(), work_dir.clone())
                        .await?;

                // 任务完成后输出 worktree 信息
                Self::output_worktree_info(&worktree_info);
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
                    work_dir.clone(),
                )
                .await?;

                // 返回第一个失败的 exit code，或者 0 如果全部成功
                let final_exit_code = exit_codes
                    .iter()
                    .find(|&&code| code != 0)
                    .copied()
                    .unwrap_or(0);

                // 任务完成后输出 worktree 信息
                Self::output_worktree_info(&worktree_info);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_worktree_id_format() {
        // 多次调用验证格式一致性
        for _ in 0..10 {
            let id = AiCliCommand::test_generate_worktree_id();
            assert_eq!(id.len(), 8, "worktree_id should be 8 characters");
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()),
                    "worktree_id should be hex digits");
            assert!(id.chars().all(|c| !c.is_ascii_uppercase()),
                    "worktree_id should not contain uppercase letters");
        }
    }

    #[test]
    fn test_worktree_info_output_format() {
        let info = WorktreeInfo {
            path: PathBuf::from("/tmp/aiw-worktree-a1b2c3d4"),
            branch: "main".to_string(),
            commit: "abc123def456".to_string(),
        };

        // 模拟输出并验证格式
        let output = format!(
            "\n=== AIW WORKTREE END ===\nWorktree: {}\nBranch: {}\nCommit: {}",
            info.path.display(),
            info.branch,
            info.commit
        );

        assert!(output.contains("=== AIW WORKTREE END ==="));
        assert!(output.contains("/tmp/aiw-worktree-a1b2c3d4"));
        assert!(output.contains("Branch: main"));
        assert!(output.contains("Commit: abc123def456"));
    }

    #[test]
    fn test_check_git_repository_non_git_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 非 git 目录应该返回错误
        let result = AiCliCommand::test_check_git_repository(&temp_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not a git repository"));
    }

    #[test]
    fn test_check_git_repository_valid_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // 创建 git 仓库
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        // 有效的 git 仓库应该成功
        let result = AiCliCommand::test_check_git_repository(&temp_path);
        assert!(result.is_ok());
    }
}

// 暴露给测试的辅助方法
impl AiCliCommand {
    #[cfg(test)]
    pub fn test_generate_worktree_id() -> String {
        Self::generate_worktree_id()
    }

    #[cfg(test)]
    pub fn test_check_git_repository(path: &PathBuf) -> Result<()> {
        Self::check_git_repository(path)
    }
}
