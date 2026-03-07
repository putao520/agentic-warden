//! AI CLI 启动命令处理逻辑
//!
//! 处理 codex、claude、gemini 等 AI CLI 的启动和管理

use crate::cli_type::{parse_cli_selector_strict, CliType};
use crate::commands::cli_args::CliInvocation;
use crate::registry_factory::create_cli_registry;
use crate::supervisor;
use crate::task_prepare::{self, TaskParams};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::task_record::WorktreeInfo;

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

    /// 从 CliInvocation 创建 AiCliCommand
    ///
    /// 这是一个桥接方法，允许新的 CliInvocation 结构与现有代码兼容
    pub fn from_invocation(inv: CliInvocation) -> Self {
        // 对于 single CLI 类型，转换为 Vec
        let ai_types = vec![inv.cli_type];
        // prompt 就是 remaining_args joined with spaces
        let prompt = inv.remaining_args.join(" ");

        Self {
            ai_types,
            role: inv.aiw_args.role,
            provider: inv.aiw_args.provider,
            prompt,
            cli_args: inv.remaining_args,
            cwd: inv.aiw_args.cwd,
        }
    }

    /// 输出 worktree 信息到 stdout
    fn output_worktree_info(info: &WorktreeInfo) {
        println!();
        println!("=== AIW WORKTREE END ===");
        println!("Worktree: {}", info.path);
        println!("Branch: {}", info.branch);
        println!("Commit: {}", info.commit);
    }

    /// 执行 AI CLI 命令
    pub async fn execute(&self) -> Result<ExitCode> {
        let original_dir = self.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| ".".into())
        });

        // 检查是否是 git 仓库
        crate::worktree::check_git_repository(&original_dir)?;

        // Auto 类型不支持直接 CLI 执行，提前拒绝避免白建 worktree
        if self.ai_types.iter().any(|cli_type| matches!(cli_type, CliType::Auto)) {
            return Err(anyhow!(
                "Auto CLI type is only supported via `aiw auto`"
            ));
        }

        let registry = create_cli_registry()?;

        if self.prompt.is_empty() {
            // 交互模式只支持单个 CLI
            if self.ai_types.len() != 1 {
                return Err(anyhow!(
                    "Interactive mode only supports single CLI. Please provide a task description for multiple CLI execution."
                ));
            }

            // 交互模式：用 prepare_task 处理 worktree，但不构建参数（prompt 为空）
            let prepared = task_prepare::prepare_task(TaskParams {
                cli_type: self.ai_types[0].clone(),
                prompt: String::new(),
                role: None, // 交互模式不需要角色
                provider: self.provider.clone(),
                cli_args: self.cli_args.clone(),
                cwd: Some(original_dir),
                create_worktree: true,
            })?;

            if let Some(ref info) = prepared.worktree_info {
                eprintln!("Created worktree at: {}", info.path);
                eprintln!("Branch: {}, Commit: {}", info.branch, info.commit);
            }

            let exit_code = supervisor::start_interactive_cli(
                &registry,
                &prepared.cli_type,
                prepared.provider,
                &self.cli_args,
                prepared.cwd.clone(),
            ).await?;

            if let Some(ref info) = prepared.worktree_info {
                Self::output_worktree_info(info);
            }
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else if self.ai_types.len() == 1 {
            // 单 CLI 任务模式
            let prepared = task_prepare::prepare_task(TaskParams {
                cli_type: self.ai_types[0].clone(),
                prompt: self.prompt.clone(),
                role: self.role.clone(),
                provider: self.provider.clone(),
                cli_args: self.cli_args.clone(),
                cwd: Some(original_dir),
                create_worktree: true,
            })?;

            if let Some(ref info) = prepared.worktree_info {
                eprintln!("Created worktree at: {}", info.path);
                eprintln!("Branch: {}, Commit: {}", info.branch, info.commit);
            }

            let exit_code = supervisor::execute_cli(
                &registry,
                &prepared.cli_type,
                &prepared.args,
                prepared.provider,
                prepared.cwd.clone(),
            ).await?;

            if let Some(ref info) = prepared.worktree_info {
                Self::output_worktree_info(info);
            }
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else {
            // 多个 CLI 批量执行
            // 用 prepare_task 创建 worktree 和处理角色
            let prepared = task_prepare::prepare_task(TaskParams {
                cli_type: self.ai_types[0].clone(),
                prompt: self.prompt.clone(),
                role: self.role.clone(),
                provider: self.provider.clone(),
                cli_args: self.cli_args.clone(),
                cwd: Some(original_dir),
                create_worktree: true,
            })?;

            if let Some(ref info) = prepared.worktree_info {
                eprintln!("Created worktree at: {}", info.path);
                eprintln!("Branch: {}, Commit: {}", info.branch, info.commit);
            }

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
                &prepared.prompt,
                prepared.provider,
                &self.cli_args,
                prepared.cwd.clone(),
            ).await?;

            let final_exit_code = exit_codes
                .iter()
                .find(|&&code| code != 0)
                .copied()
                .unwrap_or(0);

            if let Some(ref info) = prepared.worktree_info {
                Self::output_worktree_info(info);
            }
            Ok(ExitCode::from((final_exit_code & 0xFF) as u8))
        }
    }

    /// 使用 CliInvocation 执行命令（新接口）
    ///
    /// 这个方法使用新的 CliInvocation 结构，提供更清晰的分层
    pub async fn execute_from_invocation(inv: CliInvocation) -> Result<ExitCode> {
        let original_dir = inv.aiw_args.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| ".".into())
        });

        // 检查是否是 git 仓库
        crate::worktree::check_git_repository(&original_dir)?;

        // Auto 类型不支持直接 CLI 执行，提前拒绝避免白建 worktree
        if inv.is_auto() {
            return Err(anyhow!(
                "Auto CLI type is only supported via `aiw auto`"
            ));
        }

        let registry = create_cli_registry()?;

        if inv.is_interactive() {
            // 交互模式：用 prepare_task 处理 worktree，但不构建参数（prompt 为空）
            let prepared = task_prepare::prepare_task(TaskParams {
                cli_type: inv.cli_type.clone(),
                prompt: String::new(),
                role: None, // 交互模式不需要角色
                provider: inv.aiw_args.provider.clone(),
                cli_args: inv.remaining_args.clone(),
                cwd: Some(original_dir),
                create_worktree: false,
            })?;

            if let Some(ref info) = prepared.worktree_info {
                eprintln!("Created worktree at: {}", info.path);
                eprintln!("Branch: {}, Commit: {}", info.branch, info.commit);
            }

            let exit_code = supervisor::start_interactive_cli(
                &registry,
                &prepared.cli_type,
                prepared.provider,
                &inv.remaining_args,
                prepared.cwd.clone(),
            ).await?;

            if let Some(ref info) = prepared.worktree_info {
                Self::output_worktree_info(info);
            }
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
        } else {
            // 非交互模式：prompt 就是 remaining_args joined with spaces
            let prompt = inv.remaining_args.join(" ");
            let prepared = task_prepare::prepare_task(TaskParams {
                cli_type: inv.cli_type.clone(),
                prompt: prompt.clone(),
                role: inv.aiw_args.role.clone(),
                provider: inv.aiw_args.provider.clone(),
                cli_args: inv.remaining_args.clone(),
                cwd: Some(original_dir),
                create_worktree: true,
            })?;

            if let Some(ref info) = prepared.worktree_info {
                eprintln!("Created worktree at: {}", info.path);
                eprintln!("Branch: {}, Commit: {}", info.branch, info.commit);
            }

            let exit_code = supervisor::execute_cli(
                &registry,
                &prepared.cli_type,
                &prepared.args,
                prepared.provider,
                prepared.cwd.clone(),
            ).await?;

            if let Some(ref info) = prepared.worktree_info {
                Self::output_worktree_info(info);
            }
            Ok(ExitCode::from((exit_code & 0xFF) as u8))
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
            path: "/tmp/aiw-worktree-a1b2c3d4".to_string(),
            branch: "main".to_string(),
            commit: "abc123def456".to_string(),
        };

        let output = format!(
            "\n=== AIW WORKTREE END ===\nWorktree: {}\nBranch: {}\nCommit: {}",
            info.path,
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

        let result = AiCliCommand::test_check_git_repository(&temp_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not a git repository"));
    }

    #[test]
    fn test_check_git_repository_valid_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        let result = AiCliCommand::test_check_git_repository(&temp_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_invocation_interactive() {
        use crate::commands::cli_args::{AiwArgs, CliInvocation};
        use crate::cli_type::CliType;

        let inv = CliInvocation {
            cli_type: CliType::Claude,
            aiw_args: AiwArgs {
                role: Some("senior".to_string()),
                provider: Some("anthropic".to_string()),
                cwd: None,
            },
            remaining_args: vec!["--flag".to_string()],
        };

        let cmd = AiCliCommand::from_invocation(inv);
        assert_eq!(cmd.ai_types, vec![CliType::Claude]);
        assert_eq!(cmd.role, Some("senior".to_string()));
        assert_eq!(cmd.provider, Some("anthropic".to_string()));
        // remaining_args joined with spaces becomes prompt
        assert_eq!(cmd.prompt, "--flag");
        assert_eq!(cmd.cli_args, vec!["--flag".to_string()]);
    }

    #[test]
    fn test_from_invocation_non_interactive() {
        use crate::commands::cli_args::{AiwArgs, CliInvocation};
        use crate::cli_type::CliType;

        let inv = CliInvocation {
            cli_type: CliType::Codex,
            aiw_args: AiwArgs {
                role: None,
                provider: None,
                cwd: Some(PathBuf::from("/test/path")),
            },
            remaining_args: vec!["hello".to_string(), "world".to_string()],
        };

        let cmd = AiCliCommand::from_invocation(inv);
        assert_eq!(cmd.ai_types, vec![CliType::Codex]);
        assert!(cmd.role.is_none());
        assert!(cmd.provider.is_none());
        assert_eq!(cmd.prompt, "hello world");
        assert_eq!(cmd.cwd, Some(PathBuf::from("/test/path")));
    }
}

// 暴露给测试的辅助方法
impl AiCliCommand {
    #[cfg(test)]
    pub fn test_generate_worktree_id() -> String {
        crate::worktree::generate_worktree_id()
    }

    #[cfg(test)]
    pub fn test_check_git_repository(path: &PathBuf) -> Result<()> {
        crate::worktree::check_git_repository(path)
    }
}
