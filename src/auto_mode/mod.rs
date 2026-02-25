use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::cli_type::CliType;

pub mod config;
pub mod executor;

pub const DEFAULT_EXECUTION_ORDER: [&str; 3] = ["codex", "gemini", "claude"];
pub const COOLDOWN_DURATION: Duration = Duration::from_secs(30);

/// CLI+Provider 执行组合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEntry {
    pub cli: String,
    pub provider: String,
}

impl ExecutionEntry {
    pub fn new(cli: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            cli: cli.into(),
            provider: provider.into(),
        }
    }

    /// 转换为 CliType
    pub fn to_cli_type(&self) -> Option<CliType> {
        match self.cli.to_lowercase().as_str() {
            "codex" => Some(CliType::Codex),
            "claude" => Some(CliType::Claude),
            "gemini" => Some(CliType::Gemini),
            _ => None,
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> String {
        format!("{}+{}", self.cli, self.provider)
    }
}

/// 默认执行顺序（最小后备，仅当配置文件缺失时使用）
///
/// 实际配置应存储在 `~/.aiw/config.json` 的 `auto_execution_order` 字段中
pub fn default_execution_entries() -> Vec<ExecutionEntry> {
    vec![
        // 仅保留一个最小后备，完整配置应在 config.json 中
        ExecutionEntry::new("codex", "auto"),
    ]
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub cli_type: CliType,
    pub provider: String,
    pub prompt: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

/// 冷却键：(CliType, Provider) 组合
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CooldownKey {
    pub cli_type: CliType,
    pub provider: String,
}

impl CooldownKey {
    pub fn new(cli_type: CliType, provider: impl Into<String>) -> Self {
        Self {
            cli_type,
            provider: provider.into(),
        }
    }

    pub fn display_name(&self) -> String {
        format!("{}+{}", self.cli_type.display_name(), self.provider)
    }
}

/// CLI+Provider 冷却状态管理（基于内存）
pub struct CliCooldownManager {
    /// 记录每个 (CLI, Provider) 组合的最后故障时间
    last_failure_times: Mutex<HashMap<CooldownKey, Instant>>,
}

impl CliCooldownManager {
    pub fn new() -> Self {
        Self {
            last_failure_times: Mutex::new(HashMap::new()),
        }
    }

    /// 记录 CLI+Provider 组合故障
    pub fn mark_failure(&self, cli_type: &CliType, provider: &str) {
        let key = CooldownKey::new(cli_type.clone(), provider);
        let mut times = self.last_failure_times.lock().unwrap();
        times.insert(key, Instant::now());
    }

    /// 检查 CLI+Provider 组合是否在冷却期
    pub fn is_in_cooldown(&self, cli_type: &CliType, provider: &str) -> bool {
        let key = CooldownKey::new(cli_type.clone(), provider);
        let times = self.last_failure_times.lock().unwrap();
        if let Some(last_failure) = times.get(&key) {
            let elapsed = last_failure.elapsed();
            if elapsed < COOLDOWN_DURATION {
                return true;
            } else {
                // 冷却期已过，移除记录
                drop(times);
                let mut times = self.last_failure_times.lock().unwrap();
                times.remove(&key);
                return false;
            }
        }
        false
    }

    /// 清除所有冷却状态（用于测试或重置）
    pub fn clear_all(&self) {
        let mut times = self.last_failure_times.lock().unwrap();
        times.clear();
    }

    /// 获取 CLI+Provider 组合剩余冷却时间（秒）
    pub fn remaining_cooldown_secs(&self, cli_type: &CliType, provider: &str) -> Option<u64> {
        let key = CooldownKey::new(cli_type.clone(), provider);
        let times = self.last_failure_times.lock().unwrap();
        times.get(&key).and_then(|last_failure| {
            let elapsed = last_failure.elapsed();
            if elapsed < COOLDOWN_DURATION {
                Some(COOLDOWN_DURATION.as_secs() - elapsed.as_secs())
            } else {
                None
            }
        })
    }
}

/// 解析 Auto 类型为第一个可用的具体 CLI 类型
///
/// 按 auto_execution_order 配置顺序检测，返回第一个在 PATH 中可用的 (CliType, provider)。
/// 用于在创建 worktree 等重操作之前快速确定实际 CLI。
pub fn resolve_first_available_cli() -> Result<(CliType, String), crate::error::ExecutionError> {
    let entries = config::ExecutionOrderConfig::get_execution_entries()?;
    for entry in &entries {
        if let Some(cli_type) = entry.to_cli_type() {
            if which::which(cli_type.command_name()).is_ok() {
                return Ok((cli_type, entry.provider.clone()));
            }
        }
    }
    Err(crate::error::ExecutionError::AllFailed {
        message: "No available AI CLI found in auto_execution_order".to_string(),
    })
}

/// 全局冷却管理器
static COOLDOWN_MANAGER: std::sync::OnceLock<CliCooldownManager> = std::sync::OnceLock::new();

impl CliCooldownManager {
    pub fn global() -> &'static CliCooldownManager {
        COOLDOWN_MANAGER.get_or_init(|| CliCooldownManager::new())
    }
}

