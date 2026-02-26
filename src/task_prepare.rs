//! 共享任务准备层
//!
//! CLI (`AiCliCommand::execute`) 和 MCP (`start_task`) 共用的核心逻辑：
//! 角色处理、Auto CLI 解析、worktree 创建、参数构建。

use crate::cli_type::CliType;
use crate::roles::{builtin::get_builtin_role, Role, RoleManager};
use crate::task_record::WorktreeInfo;
use std::ffi::OsString;
use std::path::PathBuf;

/// 任务准备输入参数
pub struct TaskParams {
    pub cli_type: CliType,
    pub prompt: String,
    pub role: Option<String>,
    pub provider: Option<String>,
    pub cli_args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub create_worktree: bool,
}

/// 公共准备结果（角色+worktree），不含 CLI 特定信息
pub struct PreparedTaskBase {
    pub prompt: String,
    pub cwd: Option<PathBuf>,
    pub worktree_info: Option<WorktreeInfo>,
    pub cli_args: Vec<String>,
    pub user_provider: Option<String>,
}

/// 准备完成的任务，可直接交给 supervisor 执行
pub struct PreparedTask {
    pub cli_type: CliType,
    pub prompt: String,
    pub args: Vec<OsString>,
    pub provider: Option<String>,
    pub cwd: Option<PathBuf>,
    pub worktree_info: Option<WorktreeInfo>,
}

/// 统一的任务准备函数
///
/// 按顺序执行：Auto CLI 解析 → 角色处理 → worktree 创建 → 参数构建
pub fn prepare_task(params: TaskParams) -> anyhow::Result<PreparedTask> {
    // 1. Auto CLI 解析
    let (cli_type, resolved_provider) = if matches!(params.cli_type, CliType::Auto) {
        let (resolved, provider) = crate::auto_mode::resolve_first_available_cli()?;
        (resolved, Some(provider))
    } else {
        (params.cli_type.clone(), None)
    };

    // 2. 公共准备（角色 + worktree）
    let base = prepare_task_base(params)?;

    // 3. Provider: resolved_provider（Auto 解析出的）优先于用户指定的
    let provider = resolved_provider
        .or(base.user_provider.clone())
        .unwrap_or_default();

    // 4. 构建最终任务
    Ok(finalize_for_entry(&base, cli_type, provider))
}

/// 只做角色处理 + worktree 创建，不解析 Auto CLI，不构建 CLI 参数
///
/// 用于故障切换场景：先做公共准备，再对每个 CLI+Provider 组合调用 `finalize_for_entry`
pub fn prepare_task_base(params: TaskParams) -> anyhow::Result<PreparedTaskBase> {
    // 角色处理 → 富化 prompt
    let prompt = apply_role(params.role.as_deref(), &params.prompt)?;

    // Worktree 创建（条件性）
    let (cwd, worktree_info) = if params.create_worktree {
        let work_dir = params.cwd.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| ".".into())
        });
        crate::worktree::check_git_repository(&work_dir)?;
        let (wt_path, branch, commit) = crate::worktree::create_worktree(&work_dir)?;
        let info = WorktreeInfo {
            path: wt_path.display().to_string(),
            branch,
            commit,
        };
        (Some(wt_path), Some(info))
    } else {
        (params.cwd, None)
    };

    Ok(PreparedTaskBase {
        prompt,
        cwd,
        worktree_info,
        cli_args: params.cli_args,
        user_provider: params.provider,
    })
}

/// 基于公共准备结果 + 具体 CLI+Provider 构建最终 PreparedTask
pub fn finalize_for_entry(base: &PreparedTaskBase, cli_type: CliType, provider: String) -> PreparedTask {
    let args = cli_type.build_full_access_args_with_cli(&base.prompt, &base.cli_args);
    let os_args: Vec<OsString> = args.into_iter().map(OsString::from).collect();

    PreparedTask {
        cli_type,
        prompt: base.prompt.clone(),
        args: os_args,
        provider: Some(provider),
        cwd: base.cwd.clone(),
        worktree_info: base.worktree_info.clone(),
    }
}

// --- 统一的角色处理函数 ---

const DEFAULT_ROLE: &str = "common";

/// 检测用户首选语言（基于系统 locale）
fn detect_language() -> String {
    if let Some(locale) = sys_locale::get_locale() {
        if locale.starts_with("zh") {
            return "zh-CN".to_string();
        }
    }
    "en".to_string()
}

/// 解析逗号分隔的角色字符串（去重，保持顺序）
fn parse_role_names(role_str: &str) -> Vec<&str> {
    let mut seen = std::collections::HashSet::new();
    role_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(*s))
        .collect()
}

/// 加载单个角色（优先用户自定义，其次内置）
fn load_single_role(name: &str, lang: &str) -> Option<Role> {
    if let Ok(manager) = RoleManager::new() {
        if let Ok(role) = manager.get_role(name) {
            return Some(role);
        }
    }
    if let Ok(role) = get_builtin_role(name, lang) {
        return Some(role);
    }
    None
}

/// 加载多个角色，返回 (有效角色列表, 无效角色名列表)
fn load_roles(names: &[&str], lang: &str) -> (Vec<Role>, Vec<String>) {
    let mut valid_roles = Vec::new();
    let mut invalid_names = Vec::new();
    for name in names {
        if let Some(role) = load_single_role(name, lang) {
            valid_roles.push(role);
        } else {
            invalid_names.push(name.to_string());
        }
    }
    (valid_roles, invalid_names)
}

/// 组合多个角色内容与用户 prompt
fn combine_role_contents(roles: &[Role], prompt: &str) -> String {
    if roles.is_empty() {
        return prompt.to_string();
    }
    let role_contents: Vec<&str> = roles.iter().map(|r| r.content.as_str()).collect();
    let combined = role_contents.join("\n\n---\n\n");
    format!("{}\n\n---\n\n{}", combined, prompt)
}

/// 应用角色到 prompt（支持多角色，逗号分隔）
fn apply_role(role_str: Option<&str>, prompt: &str) -> anyhow::Result<String> {
    let role_str = match role_str {
        Some(s) => s,
        None => return Ok(prompt.to_string()),
    };

    let role_names = parse_role_names(role_str);
    if role_names.is_empty() {
        return Ok(prompt.to_string());
    }

    let lang = detect_language();
    let (valid_roles, invalid_names) = load_roles(&role_names, &lang);

    for name in &invalid_names {
        eprintln!("Warning: Role '{}' not found, skipping.", name);
    }

    if valid_roles.is_empty() {
        eprintln!(
            "Warning: All specified roles not found, falling back to '{}' role.",
            DEFAULT_ROLE
        );
        if let Some(fallback) = load_single_role(DEFAULT_ROLE, &lang) {
            return Ok(combine_role_contents(&[fallback], prompt));
        }
        eprintln!("Warning: Default role '{}' also not available.", DEFAULT_ROLE);
        return Ok(prompt.to_string());
    }

    Ok(combine_role_contents(&valid_roles, prompt))
}
