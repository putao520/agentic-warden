# Agentic-Warden 数据模型定义

## 配置数据模型

### 1. Provider 配置模型

#### 1.1 Provider 结构
```rust
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
}

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

fn default_now() -> DateTime<Utc> {
    Utc::now()
}
```

#### 1.2 Provider 配置文件模型
```rust
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
}

fn default_config_version() -> String {
    "1.0.0".to_string()
}

fn default_format_version() -> u32 {
    1
}
```

### 2. Google Drive 认证模型

#### 2.1 OAuth Token 信息
```rust
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
```

#### 2.2 认证配置
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Google OAuth 客户端 ID
    pub client_id: String,

    /// Google OAuth 客户端密钥
    pub client_secret: String,

    /// 重定向 URI
    pub redirect_uri: String,

    /// OAuth 作用域
    pub scopes: Vec<String>,

    /// OOB 流程端口
    #[serde(default = "default_oob_port")]
    pub oob_port: u16,

    /// 授权超时时间（秒）
    #[serde(default = "default_auth_timeout")]
    pub auth_timeout: u64,
}

fn default_oob_port() -> u16 {
    8080
}

fn default_auth_timeout() -> u64 {
    300 // 5 分钟
}
```

## 进程树数据模型

### 1. 进程树结构模型

#### 1.1 ProcessTreeInfo - 进程树信息
```rust
use serde::{Deserialize, Serialize};

/// 进程树信息，包含完整的进程链和AI CLI根进程信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTreeInfo {
    /// 进程链：从当前进程到根进程的完整PID序列
    /// [current_pid, parent_pid, grandparent_pid, ..., root_pid]
    pub process_chain: Vec<u32>,

    /// AI CLI根进程PID（第一个匹配的AI CLI进程）
    /// 如果没有找到AI CLI，则为传统根父进程
    pub root_parent_pid: Option<u32>,

    /// 进程树深度（从当前进程到根进程的层级数）
    pub depth: usize,

    /// 是否找到AI CLI根进程
    pub has_ai_cli_root: bool,

    /// AI CLI根进程类型（claude/codex/gemini）
    pub ai_cli_type: Option<String>,
}

impl ProcessTreeInfo {
    /// 创建新的进程树信息
    pub fn new(process_chain: Vec<u32>) -> Self;

    /// 获取AI CLI根进程PID
    pub fn get_ai_cli_root(&self) -> Option<u32>;

    /// 检查是否包含指定PID
    pub fn contains_process(&self, pid: u32) -> bool;

    /// 获取当前进程到AI CLI根进程的子链
    pub fn get_chain_to_ai_cli_root(&self) -> Vec<u32>;
}
```

#### 1.2 AiCliProcessInfo - AI CLI进程信息
```rust
/// AI CLI进程的详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCliProcessInfo {
    /// 进程PID
    pub pid: u32,

    /// AI CLI类型
    pub ai_type: String,

    /// 进程名称
    pub process_name: String,

    /// 命令行参数
    pub command_line: String,

    /// 是否为NPM包形式
    pub is_npm_package: bool,

    /// 检测到的时间戳
    pub detected_at: DateTime<Utc>,

    /// 进程路径
    pub executable_path: Option<PathBuf>,
}

impl AiCliProcessInfo {
    /// 创建新的AI CLI进程信息
    pub fn new(pid: u32, ai_type: String) -> Self;

    /// 检查是否为有效的AI CLI进程
    pub fn is_valid_ai_cli(&self) -> bool;

    /// 获取进程描述
    pub fn get_description(&self) -> String;
}
```

### 2. 任务信息模型

#### 1.1 任务基本信息（TaskRecord - 实用主义设计）
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Running,
    CompletedButUnread,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// 任务开始时间
    pub started_at: DateTime<Utc>,

    /// 日志ID（唯一标识）
    pub log_id: String,

    /// 日志文件路径
    pub log_path: String,

    /// 管理器进程ID（可选）
    #[serde(default)]
    pub manager_pid: Option<u32>,

    /// 清理原因（可选）
    #[serde(default)]
    pub cleanup_reason: Option<String>,

    /// 任务状态
    #[serde(default)]
    pub status: TaskStatus,

    /// 执行结果（可选）
    #[serde(default)]
    pub result: Option<String>,

    /// 完成时间（可选）
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,

    /// 退出码（可选）
    #[serde(default)]
    pub exit_code: Option<i32>,

    // 进程树跟踪相关字段
    /// 进程链
    #[serde(default)]
    pub process_chain: Vec<u32>,

    /// 根父进程ID
    #[serde(default)]
    pub root_parent_pid: Option<u32>,

    /// 进程树深度
    #[serde(default)]
    pub process_tree_depth: usize,

    /// 完整进程树信息（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_tree: Option<ProcessTreeInfo>,

    /// AI CLI进程信息（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_cli_process: Option<AiCliProcessInfo>,
}

// 注意：TaskRecord 设计原则
// 1. 简洁实用 - 只包含必要的字段
// 2. 避免过度设计 - 不包含 ResourceUsage 等复杂统计
// 3. 关注核心功能 - 专注于任务跟踪和状态管理
```

#### 1.2 进程信息
```rust
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
```

#### 1.3 TaskStatus定义说明

**注意**: TaskStatus枚举定义在本文档第240-250行的TaskRecord结构体中。
系统采用简洁的2状态设计（Running, CompletedButUnread），符合"简洁实用"的设计原则。

**已移除的过度设计**:
- ResourceUsage 结构体 - 资源监控属于过度设计，可使用外部工具如htop、top
- 多余的任务状态（Pending, Failed, Terminated, Timeout, Paused）- 不符合简洁实用原则

### 2. 共享内存模型

#### 2.1 实例注册信息
```rust
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
```

#### 2.2 共享数据结构
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SharedData {
    /// 注册的实例信息
    pub instances: HashMap<usize, InstanceRegistry>,

    /// 所有任务信息
    pub tasks: HashMap<String, TaskRecord>, // 使用 log_id 作为 key

    /// 最后更新时间
    pub last_updated: SystemTime,
}

// 注意：移除了 GlobalStatistics 和 AiTypeStatistics 结构体
// 原因：统计功能属于非核心需求，过度增加了复杂性
// 如果需要统计功能，可以通过查询 TaskRecord 列表动态计算

## 设计原则说明

### 不包含的功能（设计决策）

1. **统计和分析功能**
   - 不提供全局统计信息（如总任务数、成功率等）
   - 不提供 AI 使用统计
   - 不提供性能分析
   - 原因：这些功能增加了复杂性，但对核心任务管理没有帮助

2. **任务历史记录**
   - 不持久化保存已完成的任务历史
   - 不提供任务执行历史查询
   - 不保存任务执行结果
   - 原因：任务完成后即清理，避免积累历史数据

3. **资源使用监控**
   - 不监控 CPU、内存使用情况
   - 不记录网络使用量
   - 不跟踪资源消耗
   - 原因：这是任务管理器，不是监控系统

### 核心功能聚焦

本项目专注于：
- **实时任务管理** - 跟踪当前运行的任务
- **进程树管理** - 管理相关进程
- **配置同步** - 管理 AI 工具配置
- **Provider 管理** - 管理服务提供商

避免功能膨胀，保持简洁实用的设计。

### 配置同步策略（v0.1.0）

#### 命名配置管理
用户可以为配置集合命名，实现多环境管理：
- **开发环境**: `agentic-warden push dev`
- **生产环境**: `agentic-warden push prod`
- **测试环境**: `agentic-warden push test`
- **默认配置**: `agentic-warden push` (使用 "default" 作为配置名)

#### Push 操作
1. **命令格式**:
   - `agentic-warden push [CONFIG_NAME]` - CONFIG_NAME 可选，默认为 "default"
   - `agentic-warden push` - 等同于 `agentic-warden push default`
2. **处理流程**:
   - 扫描配置目录，仅包含指定文件类型
   - 检查 Google Drive 是否已存在同名配置
   - 如果存在，提示用户确认是否覆盖
   - 上传到 Google Drive: `.agentic-warden/<CONFIG_NAME>.zip`
3. **覆盖保护机制**:
   - 系统在上传前自动检查云端是否已存在同名配置
   - 如果发现重复，显示确认提示：
     ```
     ⚠️  Configuration 'xxx' already exists in Google Drive.
     Do you want to overwrite it?
       [Y] Yes, overwrite
       [N] No, cancel
     Your choice [Y/N]:
     ```
   - 用户选择 'N' 或 'No' 则取消上传操作
   - 用户选择 'Y' 或 'Yes' 则继续上传并覆盖
4. **包含的文件类型**:

**重要说明**: 所有的 AI CLI 根目录（~/.claude、~/.codex、~/.gemini）都是可选的。
- 如果某个目录不存在，则跳过该目录的扫描
- 如果三个目录都不存在，则显示提示信息并退出推送操作

**Claude (~/.claude)** (目录和所有文件可选):
- `CLAUDE.md` - 主记忆文件（可选）
- `settings.json` - 主配置文件（可选）
- `agents/` - 自定义 agent 配置目录（可选）
  - `agents/.claude/settings.local.json` - agent 级配置（可选）
  - `agents/COLLABORATION_EXAMPLES.md` - agent 协作示例（可选）
  - `agents/*` - 其他 agent 相关文件（可选）
- `skills/` - 技能目录（可选）
  - `skills/*/*/SKILL.md` - 所有技能定义文件（可选）

**Codex (~/.codex)** (目录和所有文件可选):
- `auth.json` - 认证配置文件（可选）
- `config.toml` - 主配置文件（可选）
- `version.json` - 版本信息文件（可选）
- `agents.md` - 主记忆文件（可选）
- `history.jsonl` - 命令历史（可选）

**Gemini (~/.gemini)** (目录和所有文件可选):
- `google_accounts.json` - Google 账户配置（可选）
- `oauth_creds.json` - OAuth 凭证（可选）
- `settings.json` - 主配置文件（可选）
- `gemini.md` - 主记忆文件（可选）
- `tmp/` - 临时文件目录（可选）

4. **排除的文件类型**:
   - 缓存文件（.cache, __pycache__等）
   - 临时文件（tmp/, *.tmp, *.temp）
   - 日志文件（*.log, log/）
   - 会话文件（sessions/, history.jsonl）
   - 二进制文件（.exe, .dll, .so）
   - 大文件（>10MB）
   - TODO 文件（todos/*.json）
   - 项目历史（file-history/, projects/）
   - 调试文件（debug/）
   - IDE 相关文件（ide/）
   - Shell 快照（shell-snapshots/）

5. **文件大小限制**: 单个配置集合最大 50MB
6. **示例**:
   - `agentic-warden push` → 上传为 `default.zip`
   - `agentic-warden push my-config` → 上传为 `my-config.zip`
   - `agentic-warden push work` → 上传为 `work.zip`
   - `agentic-warden pull` → 下载 `default.zip`
   - `agentic-warden pull dev` → 下载 `dev.zip`

#### Pull 操作
1. **命令格式**:
   - `agentic-warden pull [CONFIG_NAME]` - CONFIG_NAME 可选，默认为 "default"
   - `agentic-warden pull` - 等同于 `agentic-warden pull default`
2. **处理流程**:
   - 下载 `.agentic-warden/<CONFIG_NAME>.zip`
   - 解压到对应配置目录
3. **覆盖模式**: 总是覆盖本地文件
4. **默认配置**: 如果不提供名称，使用 "default"

#### 设计原则
- **命名管理**: 通过名称区分不同配置集合
- **简单覆盖**: 不保留版本，总是使用最新
- **用户控制**: 用户决定何时保存和恢复配置
```

## TUI 数据模型

### 1. 应用状态模型

#### 1.1 主应用状态
```rust
#[derive(Debug)]
pub struct AppState {
    /// 当前屏幕
    pub current_screen: Screen,

    /// 是否应该退出
    pub should_quit: bool,

    /// Provider 列表
    pub providers: Vec<Provider>,

    /// 当前选中的 Provider
    pub selected_provider: Option<String>,

    /// 任务列表
    pub tasks: Vec<TaskInfo>,

    /// 当前选中的任务索引
    pub selected_task_index: Option<usize>,

    /// 状态消息
    pub status_message: Option<String>,

    /// 错误消息
    pub error_message: Option<String>,

    /// 加载状态
    pub loading: bool,

    /// 最后刷新时间
    pub last_refresh: SystemTime,

    /// 自动刷新间隔（秒）
    pub auto_refresh_interval: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    ProviderList,
    ProviderEdit(ProviderEditState),
    Status,
    PushProgress(PushProgressState),
    PullProgress(PullProgressState),
    AuthDialog(AuthDialogState),
}
```

#### 1.2 编辑状态模型
```rust
#[derive(Debug, Clone)]
pub struct ProviderEditState {
    /// 编辑的 Provider
    pub provider: Provider,

    /// 当前编辑的字段
    pub editing_field: EditingField,

    /// 输入缓冲区
    pub input_buffer: String,

    /// 光标位置
    pub cursor_position: usize,

    /// 是否为新建 Provider
    pub is_new: bool,

    /// 验证错误
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum EditingField {
    Name,
    Description,
    CompatibleWith,
    EnvVarKey(usize),
    EnvVarValue(usize),
    NewEnvVarKey,
    NewEnvVarValue,
}
```

#### 1.3 进度状态模型
```rust
#[derive(Debug, Clone)]
pub struct PushProgressState {
    /// 总文件数
    pub total_files: usize,

    /// 已完成文件数
    pub completed_files: usize,

    /// 当前处理的文件
    pub current_file: Option<PathBuf>,

    /// 总字节数
    pub total_bytes: u64,

    /// 已传输字节数
    pub transferred_bytes: u64,

    /// 开始时间
    pub start_time: SystemTime,

    /// 当前操作
    pub current_operation: String,

    /// 错误信息
    pub error: Option<String>,

    /// 是否完成
    pub is_completed: bool,
}

#[derive(Debug, Clone)]
pub struct PullProgressState {
    /// 总文件数
    pub total_files: usize,

    /// 已下载文件数
    pub downloaded_files: usize,

    /// 当前下载的文件
    pub current_file: Option<String>,

    /// 总字节数
    pub total_bytes: u64,

    /// 已下载字节数
    pub downloaded_bytes: u64,

    /// 开始时间
    pub start_time: SystemTime,

    /// 当前操作
    pub current_operation: String,

    /// 错误信息
    pub error: Option<String>,

    /// 是否完成
    pub is_completed: bool,
}
```

#### 1.4 授权对话框状态
```rust
#[derive(Debug, Clone)]
pub struct AuthDialogState {
    /// 授权 URL
    pub auth_url: String,

    /// 当前授权状态
    pub auth_status: AuthStatus,

    /// 用户代码（如果需要）
    pub user_code: Option<String>,

    /// 倒计时（秒）
    pub countdown: Option<u64>,

    /// 错误信息
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    /// 等待用户访问授权 URL
    WaitingForUser,

    /// 等待用户输入授权码
    WaitingForCode,

    /// 正在交换令牌
    ExchangingToken,

    /// 授权成功
    Authorized,

    /// 授权失败
    Failed(String),

    /// 用户取消
    Cancelled,
}
```

## 文件系统数据模型

### 1. 配置文件路径
```rust
pub struct ConfigPaths {
    /// 主配置目录
    pub config_dir: PathBuf,

    /// Provider 配置文件
    pub provider_config: PathBuf,

    /// 认证信息文件
    pub auth_file: PathBuf,

    /// 日志文件
    pub log_file: PathBuf,

    /// 临时文件目录
    pub temp_dir: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = home_dir.join(".agentic-warden");

        Ok(Self {
            provider_config: config_dir.join("provider.json"),
            auth_file: config_dir.join("auth.json"),
            log_file: config_dir.join("agentic-warden.log"),
            temp_dir: config_dir.join("temp"),
            config_dir,
        })
    }
}
```

### 2. 日志数据模型
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// 日志级别
    pub level: LogLevel,

    /// 模块名称
    pub module: String,

    /// 消息内容
    pub message: String,

    /// 关联的任务 ID（如果有）
    pub task_id: Option<TaskId>,

    /// 额外的键值对数据
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
```