use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::cli_type::CliType;

pub mod config;
pub mod executor;

pub const DEFAULT_EXECUTION_ORDER: [&str; 3] = ["codex", "gemini", "claude"];
pub const COOLDOWN_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub cli_type: CliType,
    pub prompt: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

/// CLI 冷却状态管理（基于内存）
pub struct CliCooldownManager {
    /// 记录每个 CLI 的最后故障时间
    last_failure_times: Mutex<HashMap<CliType, Instant>>,
}

impl CliCooldownManager {
    pub fn new() -> Self {
        Self {
            last_failure_times: Mutex::new(HashMap::new()),
        }
    }

    /// 记录 CLI 故障
    pub fn mark_failure(&self, cli_type: &CliType) {
        let mut times = self.last_failure_times.lock().unwrap();
        times.insert(cli_type.clone(), Instant::now());
    }

    /// 检查 CLI 是否在冷却期
    pub fn is_in_cooldown(&self, cli_type: &CliType) -> bool {
        let times = self.last_failure_times.lock().unwrap();
        if let Some(last_failure) = times.get(cli_type) {
            let elapsed = last_failure.elapsed();
            if elapsed < COOLDOWN_DURATION {
                return true;
            } else {
                // 冷却期已过，移除记录
                // 注意：这里在函数结束时自动释放锁，所以需要重新获取锁来修改
                drop(times);
                let mut times = self.last_failure_times.lock().unwrap();
                times.remove(cli_type);
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

    /// 获取 CLI 剩余冷却时间（秒）- 接受引用以支持 for 循环
    pub fn remaining_cooldown_secs_ref(&self, cli_type: &CliType) -> Option<u64> {
        let times = self.last_failure_times.lock().unwrap();
        times.get(cli_type).and_then(|last_failure| {
            let elapsed = last_failure.elapsed();
            if elapsed < COOLDOWN_DURATION {
                Some(COOLDOWN_DURATION.as_secs() - elapsed.as_secs())
            } else {
                None
            }
        })
    }

    /// 获取 CLI 剩余冷却时间（秒）
    pub fn remaining_cooldown_secs(&self, cli_type: &CliType) -> Option<u64> {
        self.remaining_cooldown_secs_ref(cli_type)
    }
}

/// 全局冷却管理器
static COOLDOWN_MANAGER: std::sync::OnceLock<CliCooldownManager> = std::sync::OnceLock::new();

impl CliCooldownManager {
    pub fn global() -> &'static CliCooldownManager {
        COOLDOWN_MANAGER.get_or_init(|| CliCooldownManager::new())
    }
}

