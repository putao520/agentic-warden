//! 统一数据模型定义
//!
//! 定义系统中使用的所有核心数据结构

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// AI CLI 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AiType {
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "claude")]
    Claude,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "all")]
    All,
}

/// 任务唯一标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        Self(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        )
    }
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// 等待启动
    Pending,
    /// 正在运行
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 被终止
    Terminated,
    /// 超时
    Timeout,
    /// 暂停
    Paused,
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

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// 任务唯一标识符
    pub id: TaskId,
    /// AI CLI 类型
    pub ai_type: AiType,
    /// 使用的 Provider
    pub provider: Option<String>,
    /// 提示词预览（前 50 个字符）
    pub prompt_preview: String,
    /// 完整提示词长度
    pub prompt_length: usize,
    /// 任务状态
    pub status: TaskStatus,
    /// 创建时间
    pub created_at: SystemTime,
    /// 开始时间
    pub started_at: Option<SystemTime>,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 父进程信息
    pub parent_process: ProcessInfo,
    /// 子进程 PID
    pub child_pid: Option<u32>,
    /// 工作目录
    pub working_directory: PathBuf,
    /// 环境变量快照
    pub environment: HashMap<String, String>,
    /// 命令行参数
    pub command_line: Vec<String>,
    /// 输出文件路径（如果有）
    pub output_file: Option<PathBuf>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 资源使用情况
    pub resource_usage: Option<ResourceUsage>,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU 使用率 (0-100)
    pub cpu_percent: f64,
    /// 内存使用量（字节）
    pub memory_usage: u64,
    /// 运行时间
    pub duration: std::time::Duration,
    /// 网络使用量（字节）
    pub network_bytes: Option<u64>,
    /// 磁盘 I/O（字节）
    pub disk_io: Option<u64>,
}

/// Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Provider 唯一标识符
    pub name: String,
    /// Provider 描述信息
    pub description: String,
    /// 兼容的 AI CLI 类型列表
    pub compatible_with: Vec<AiType>,
    /// 环境变量映射
    pub env: HashMap<String, String>,
    /// 是否为内置 Provider
    #[serde(default)]
    pub builtin: bool,
    /// 创建时间
    #[serde(default = "default_now")]
    pub created_at: DateTime<Utc>,
    /// 更新时间
    #[serde(default = "default_now")]
    pub updated_at: DateTime<Utc>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

/// Provider 配置文件
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// JSON Schema 版本
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Provider 映射表
    pub providers: HashMap<String, Provider>,
    /// 默认 Provider 名称
    pub default_provider: String,
    /// 配置文件版本
    #[serde(default = "default_config_version")]
    pub version: String,
    /// 配置文件格式版本
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    /// 配置设置
    #[serde(default)]
    pub settings: ProviderSettings,
}

fn default_config_version() -> String {
    "1.0.0".to_string()
}

fn default_format_version() -> u32 {
    1
}

/// Provider 设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    /// 是否自动刷新
    #[serde(default = "default_true")]
    pub auto_refresh: bool,
    /// 健康检查间隔（秒）
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: u64,
    /// 连接超时（秒）
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 启动时验证
    #[serde(default = "default_true")]
    pub validate_on_startup: bool,
}

fn default_true() -> bool {
    true
}

fn default_health_check_interval() -> u64 {
    300
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            health_check_interval: 300,
            connection_timeout: 30,
            max_retries: 3,
            validate_on_startup: true,
        }
    }
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