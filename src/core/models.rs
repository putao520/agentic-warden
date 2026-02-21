//! 统一数据模型定义
//!
//! 定义系统中使用的所有核心数据结构

#![allow(dead_code)] // 数据模型定义，部分结构和函数是公共API

use crate::error::{AgenticResult, AgenticWardenError};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

/// 任务唯一标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(u64);

impl TaskId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| {
                    // Fallback: Use a pseudo-random value if system time is before UNIX_EPOCH
                    // This should never happen on properly configured systems
                    use std::time::Duration;
                    Duration::from_nanos(std::process::id() as u64)
                })
                .as_nanos() as u64,
        )
    }
}

/// 进程树信息，包含完整进程链与AI CLI元数据
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessTreeInfo {
    /// 进程链：[current_pid, parent_pid, grandparent_pid, ..., root_pid]
    #[serde(default)]
    pub process_chain: Vec<u32>,
    /// AI CLI根进程PID（如果没有找到则为传统根父进程）
    #[serde(default)]
    pub root_parent_pid: Option<u32>,
    /// 进程树深度
    #[serde(default, alias = "process_tree_depth")]
    pub depth: usize,
    /// 是否找到AI CLI根进程
    #[serde(default)]
    pub has_ai_cli_root: bool,
    /// AI CLI类型
    #[serde(default)]
    pub ai_cli_type: Option<String>,
    /// 可选的AI CLI进程信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_cli_process: Option<AiCliProcessInfo>,
}

impl ProcessTreeInfo {
    /// 创建新的进程树信息
    pub fn new(process_chain: Vec<u32>) -> Self {
        let depth = process_chain.len();
        let root_parent_pid = process_chain.last().copied();
        Self {
            process_chain,
            root_parent_pid,
            depth,
            has_ai_cli_root: false,
            ai_cli_type: None,
            ai_cli_process: None,
        }
    }

    /// 附加 AI CLI 元数据
    pub fn with_ai_cli_process(mut self, ai_cli_process: Option<AiCliProcessInfo>) -> Self {
        if let Some(info) = ai_cli_process {
            self.root_parent_pid = Some(info.pid);
            self.ai_cli_type = Some(info.ai_type.clone());
            self.has_ai_cli_root = true;
            self.ai_cli_process = Some(info);
        }
        self
    }

    /// 获取AI CLI根进程PID
    pub fn get_ai_cli_root(&self) -> Option<u32> {
        if self.has_ai_cli_root {
            self.ai_cli_process
                .as_ref()
                .map(|info| info.pid)
                .or(self.root_parent_pid)
        } else {
            self.root_parent_pid
        }
    }

    /// 检查进程链中是否包含指定PID
    pub fn contains_process(&self, pid: u32) -> bool {
        self.process_chain.contains(&pid)
    }

    /// 获取当前进程到AI CLI根进程的子链
    pub fn get_chain_to_ai_cli_root(&self) -> Vec<u32> {
        if let Some(root_pid) = self.get_ai_cli_root() {
            if let Some(pos) = self.process_chain.iter().position(|pid| *pid == root_pid) {
                return self.process_chain[..=pos].to_vec();
            }
        }
        self.process_chain.clone()
    }

    /// 校验数据完整性
    pub fn validate(&self) -> AgenticResult<()> {
        if self.process_chain.is_empty() {
            return Err(validation_error(
                "process_tree.process_chain",
                "process chain cannot be empty",
            ));
        }

        if self.depth != self.process_chain.len() {
            return Err(validation_error(
                "process_tree.depth",
                format!(
                    "depth ({}) must equal process_chain length ({})",
                    self.depth,
                    self.process_chain.len()
                ),
            ));
        }

        let mut seen = HashSet::new();
        for pid in &self.process_chain {
            if !seen.insert(pid) {
                return Err(validation_error(
                    "process_tree.process_chain",
                    format!("duplicate pid {} detected", pid),
                ));
            }
        }

        if self.has_ai_cli_root {
            if self.ai_cli_type.is_none() {
                return Err(validation_error(
                    "process_tree.ai_cli_type",
                    "ai_cli_type required when has_ai_cli_root=true",
                ));
            }
            if self.ai_cli_process.is_none() {
                return Err(validation_error(
                    "process_tree.ai_cli_process",
                    "ai_cli_process required when has_ai_cli_root=true",
                ));
            }
        }

        Ok(())
    }
}

/// AI CLI进程的详细信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiCliProcessInfo {
    /// 进程PID
    pub pid: u32,
    /// AI CLI类型
    pub ai_type: String,
    /// 进程名称
    #[serde(default)]
    pub process_name: String,
    /// 命令行
    #[serde(default)]
    pub command_line: String,
    /// 是否为NPM包形式
    pub is_npm_package: bool,
    /// 检测时间
    pub detected_at: DateTime<Utc>,
    /// 可选的可执行路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<PathBuf>,
}

impl AiCliProcessInfo {
    /// 创建新的AI CLI进程信息
    pub fn new(pid: u32, ai_type: impl Into<String>) -> Self {
        Self {
            pid,
            ai_type: ai_type.into(),
            process_name: String::new(),
            command_line: String::new(),
            is_npm_package: false,
            detected_at: Utc::now(),
            executable_path: None,
        }
    }

    pub fn with_process_name(mut self, name: impl Into<String>) -> Self {
        self.process_name = name.into();
        self
    }

    pub fn with_command_line(mut self, command_line: impl Into<String>) -> Self {
        self.command_line = command_line.into();
        self
    }

    pub fn with_is_npm_package(mut self, is_npm_package: bool) -> Self {
        self.is_npm_package = is_npm_package;
        self
    }

    pub fn with_executable_path(mut self, path: Option<PathBuf>) -> Self {
        self.executable_path = path;
        self
    }

    /// 检查是否为有效的AI CLI进程
    pub fn is_valid_ai_cli(&self) -> bool {
        self.pid > 0 && !self.ai_type.is_empty() && !self.process_name.is_empty()
    }

    /// 获取进程描述
    pub fn get_description(&self) -> String {
        let mut description = format!("{} (pid {})", self.ai_type, self.pid);
        if !self.process_name.is_empty() {
            description.push_str(&format!(" via {}", self.process_name));
        }
        if self.is_npm_package {
            description.push_str(" [npm]");
        }
        description
    }

    /// 校验AI CLI进程信息
    pub fn validate(&self) -> AgenticResult<()> {
        if self.pid == 0 {
            return Err(validation_error(
                "ai_cli_process.pid",
                "pid must be a non-zero value",
            ));
        }
        if self.ai_type.trim().is_empty() {
            return Err(validation_error(
                "ai_cli_process.ai_type",
                "ai_type cannot be empty",
            ));
        }
        if self.process_name.trim().is_empty() {
            return Err(validation_error(
                "ai_cli_process.process_name",
                "process_name cannot be empty",
            ));
        }
        Ok(())
    }
}

/// 进程信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// 进程 ID
    pub pid: u32,
    /// 父进程 ID
    pub ppid: u32,
    /// 进程名称
    pub name: String,
    /// 进程路径
    pub path: Option<PathBuf>,
    /// 命令行
    pub command_line: String,
    /// 启动时间
    pub start_time: SystemTime,
    /// 用户 ID
    pub user_id: Option<u32>,
    /// 是否为根进程
    pub is_root: bool,
    /// 进程树深度
    pub depth: u32,
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}


/// OAuth Token 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: u64,
    /// 实际过期时间戳
    #[serde(default = "calculate_expiry")]
    pub expiry_time: DateTime<Utc>,
    /// 令牌获取时间
    #[serde(default = "default_now")]
    pub obtained_at: DateTime<Utc>,
    /// 令牌范围
    pub scope: Option<String>,
}

fn calculate_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::seconds(3600) // 默认 1 小时
}

/// 实例注册信息
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceRegistry {
    /// 实例 ID
    pub instance_id: usize,
    /// 实例启动时间
    pub start_time: SystemTime,
    /// 主进程 PID
    pub main_pid: u32,
    /// 用户名
    pub username: String,
    /// 主机名
    pub hostname: String,
    /// 工作目录
    pub working_directory: PathBuf,
    /// agentic-warden 版本
    pub version: String,
    /// 最后心跳时间
    pub last_heartbeat: SystemTime,
    /// 任务计数
    pub task_count: usize,
    /// 活跃任务数
    pub active_task_count: usize,
}

fn validation_error(field: &str, message: impl Into<String>) -> AgenticWardenError {
    AgenticWardenError::Validation {
        message: message.into(),
        field: Some(field.to_string()),
        value: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_tree_info_roundtrip_includes_ai_cli_metadata() {
        let ai_info = AiCliProcessInfo::new(42, "claude")
            .with_process_name("claude-cli")
            .with_command_line("claude ask --debug")
            .with_is_npm_package(false);
        let tree = ProcessTreeInfo::new(vec![4242, 1337, 42]).with_ai_cli_process(Some(ai_info));
        tree.validate().expect("tree should be valid");

        let serialized = serde_json::to_string(&tree).expect("serialize tree");
        let restored: ProcessTreeInfo =
            serde_json::from_str(&serialized).expect("deserialize tree");

        assert_eq!(restored.depth, 3);
        assert!(restored.has_ai_cli_root);
        assert_eq!(restored.get_ai_cli_root(), Some(42));
        assert!(restored.ai_cli_process.is_some());
    }

    #[test]
    fn process_tree_info_accepts_legacy_depth_field() {
        let json = r#"{
            "process_chain": [100, 50],
            "process_tree_depth": 2,
            "root_parent_pid": 50
        }"#;

        let tree: ProcessTreeInfo =
            serde_json::from_str(json).expect("legacy depth should deserialize");
        assert_eq!(tree.depth, 2);
        tree.validate().expect("tree should remain valid");
    }

    #[test]
    fn ai_cli_process_requires_non_empty_name() {
        let ai = AiCliProcessInfo::new(1, "codex").with_process_name("codex-cli");
        assert!(ai.validate().is_ok());

        let invalid = AiCliProcessInfo::new(0, "").with_process_name("");
        assert!(invalid.validate().is_err());
    }
}
