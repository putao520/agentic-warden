# Agentic-Warden 模块划分和职责

## 模块架构概览

```
src/
├── main.rs                    # 程序入口点
├── lib.rs                     # 库入口
├── cli_manager.rs             # CLI 工具检测和管理
├── cli_type.rs                # CLI 类型定义
├── commands/                  # CLI 命令处理
│   ├── mod.rs
│   ├── ai_cli.rs              # AI CLI 启动命令
│   ├── tui_commands.rs        # TUI 管理命令
│   └── parser.rs              # 命令行解析（使用 clap）
├── config.rs                  # 配置管理
├── core/                      # 核心业务逻辑
│   ├── mod.rs
│   ├── models.rs              # 数据模型
│   ├── process_tree.rs        # 进程树管理
│   └── shared_map.rs          # 共享内存
├── error.rs                   # 错误定义
├── help.rs                    # 帮助系统
├── logging.rs                 # 日志系统
├── platform/                  # 平台特定代码
│   ├── mod.rs
│   ├── unix.rs
│   └── windows.rs
├── provider/                  # Provider 管理
│   ├── mod.rs
│   ├── config.rs              # Provider 配置
│   ├── env_injector.rs        # 环境变量注入
│   ├── env_mapping.rs         # 环境变量映射
│   ├── error.rs               # Provider 错误
│   ├── manager.rs             # Provider 管理器
│   ├── network_detector.rs    # 网络检测
│   ├── recommendation_engine.rs # 推荐引擎
│   └── token_validator.rs     # Token 验证
├── registry.rs                # 任务注册表（替代 task_tracker）
├── signal.rs                  # 信号处理
├── supervisor.rs              # 进程监督器
├── sync/                      # Google Drive 同步
│   ├── mod.rs
│   ├── compressor.rs          # 压缩器
│   ├── config_packer.rs       # 配置打包
│   ├── config_sync_manager.rs # 配置同步管理
│   ├── directory_hasher.rs    # 目录哈希
│   ├── error.rs               # 同步错误
│   ├── google_drive_service.rs # Google Drive 服务
│   ├── device_flow_client.rs  # Device Flow 客户端
│   ├── smart_device_flow.rs   # 智能 Device Flow
│   ├── sync_command.rs        # 同步命令
│   └── sync_config.rs         # 同步配置
├── task_record.rs             # 任务记录（使用 TaskRecord）
├── tui/                       # TUI 界面系统
│   ├── mod.rs                 # TUI 模块入口
│   ├── app.rs                 # 应用主程序
│   ├── app_state.rs           # 应用状态
│   └── screens/               # TUI 屏幕
│       ├── mod.rs
│       ├── mod_simple.rs      # 简单模块
│       ├── dashboard.rs       # Dashboard 界面
│       ├── oauth.rs           # OAuth 界面
│       ├── provider.rs        # Provider 管理
│       ├── provider_add_wizard.rs # Provider 添加向导
│       ├── provider_edit.rs   # Provider 编辑
│       ├── provider_management.rs # Provider 管理
│       ├── pull.rs            # Pull 界面
│       ├── push.rs            # Push 界面
│       └── status.rs          # 状态界面
├── utils/                     # 工具模块
│   ├── mod.rs
│   ├── config_paths.rs        # 配置路径
│   ├── logger.rs              # 日志工具
│   └── version.rs             # 版本信息
├── wait_mode.rs               # 等待模式

# 注意：
# 1. 移除了 tui/event.rs - 使用 ratatui 内置事件处理
# 2. 移除了 tui/widgets/ - 直接使用 ratatui 组件
# 3. registry.rs 替代了 task_tracker.rs
# 4. cli_manager.rs 实现了 AI CLI 检测功能
# 5. task_record.rs 使用简洁的 TaskRecord 结构
```

## 核心模块职责

### 1. commands/ - CLI 命令处理

#### 1.1 commands/mod.rs
**职责**: CLI 命令模块的统一入口和导出
```rust
pub mod ai_cli;
pub mod tui_commands;
pub mod parser;

pub use ai_cli::*;
pub use tui_commands::*;
pub use parser::*;
```

#### 1.2 commands/ai_cli.rs
**职责**: AI CLI 启动命令的处理逻辑
- 多 AI 语法解析（`codex|claude|gemini`, `all`）
- Provider 参数处理（`-p`, `--provider`）
- 环境变量注入
- 子进程启动和监控
- 进程树注册

**主要函数**:
```rust
pub async fn execute_ai_cli_command(
    ai_types: Vec<AiType>,
    provider: Option<String>,
    prompt: String,
) -> Result<Vec<TaskId>>

fn parse_multi_ai_syntax(input: &str) -> Result<Vec<AiType>>

fn inject_provider_env(provider_name: &str) -> Result<HashMap<String, String>>

fn spawn_ai_cli_process(ai_type: AiType, provider: &str, prompt: &str) -> Result<Process>
```

#### 1.3 commands/tui_commands.rs
**职责**: TUI 管理命令的处理逻辑
- Dashboard 启动
- Status 命令处理
- Push/Pull 命令处理
- 授权检测和触发

**主要函数**:
```rust
pub async fn execute_dashboard_command() -> Result<()>
pub async fn execute_status_command() -> Result<()>
pub async fn execute_push_command(dirs: Vec<PathBuf>) -> Result<()>
pub async fn execute_pull_command() -> Result<()>
```

#### 1.4 commands/parser.rs
**职责**: 命令行参数解析和路由
- 使用 `clap` 定义命令行接口
- 参数验证和转换
- 子命令路由

**主要结构**:
```rust
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start AI CLI
    #[command(name = "codex|claude|gemini|all")]
    AiCli {
        #[arg(short = 'p', long = "provider")]
        provider: Option<String>,
        prompt: String,
    },

    /// Show dashboard (default)
    Dashboard,

    /// Show task status
    Status,

    /// Push to Google Drive
    Push {
        #[arg(default_value = ".")]
        dirs: Vec<PathBuf>,
    },

    /// Pull from Google Drive
    Pull,
}
```

### 2. tui/ - TUI 界面系统

#### 2.1 tui/mod.rs
**职责**: TUI 模块的统一入口和导出
- TUI 框架初始化
- 主要组件导出
- 事件循环入口

**主要函数**:
```rust
pub async fn run_tui_app(initial_screen: Screen) -> Result<()>
pub fn init_tui() -> Result<()>
pub fn restore_terminal() -> Result<()>
```

#### 2.2 tui/app.rs
**职责**: TUI 应用状态管理
- 应用状态维护
- 屏幕切换逻辑
- 事件分发处理
- 数据更新和刷新

**主要结构**:
```rust
pub struct App {
    pub current_screen: Screen,
    pub should_quit: bool,
    pub state: AppState,
}

impl App {
    pub fn new() -> Self;
    pub fn run(&mut self) -> Result<()>;
    pub fn handle_event(&mut self, event: Event) -> Result<()>;
    pub fn update(&mut self) -> Result<()>;
    pub fn draw(&mut self, frame: &mut Frame) -> Result<()>;
}
```

#### 2.3 tui/screens/ 各屏幕模块
**职责**: 具体 TUI 屏幕的实现
- 屏幕状态管理
- 用户交互处理
- 界面渲染逻辑

**模块分工**:
- `dashboard.rs`: 主界面，显示 AI 状态和任务概要
- `provider.rs`: Provider 列表管理界面
- `provider_edit.rs`: Provider 编辑界面
- `status.rs`: 任务状态监控界面
- `push.rs`: Push 进度显示界面
- `pull.rs`: Pull 进度显示界面
- `auth_dialog.rs`: Google Drive 授权对话框

# 注意：关于 TUI 事件处理和组件
# - 事件处理：直接使用 ratatui + crossterm 内置的事件处理机制
# - UI 组件：直接使用 ratatui 提供的组件，不单独封装
# - 这种设计更简洁，减少了不必要的抽象层

### 3. core/ - 核心业务逻辑

#### 3.1 core/process_tree.rs
**职责**: 智能进程树管理和AI CLI归属追踪

**核心功能**:
- **AI CLI根进程识别**: 向上遍历进程树，找到启动当前进程的AI CLI
- **智能进程检测**: 识别Native和NPM包形式的AI CLI进程
- **进程归属管理**: 基于AI CLI根进程进行任务分组和隔离
- **性能优化**: 使用缓存避免重复的进程树遍历
- **跨平台支持**: 优化Windows下的进程查找，避免explorer.exe问题

**设计原则**: 使用简洁的独立函数，遵循KISS原则，避免不必要的Manager结构

**主要函数和结构**:
```rust
/// 进程树信息，包含完整的进程链
pub struct ProcessTreeInfo {
    pub process_chain: Vec<u32>,      // 从当前到根的PID链
    pub root_parent_pid: Option<u32>, // AI CLI根进程PID
    pub depth: usize,                 // 进程树深度
    pub ai_cli_process: Option<AiCliProcessInfo>, // AI CLI进程信息
}

/// 获取AI CLI根进程（带全局缓存）
pub fn get_root_parent_pid_cached() -> Result<u32>;

/// 查找最近的AI CLI进程作为根进程
pub fn find_ai_cli_root_parent(pid: u32) -> Result<u32>;

/// 获取完整进程树信息
pub fn get_process_tree(pid: u32) -> Result<ProcessTreeInfo>;

/// 检查两个进程是否属于同一AI CLI根进程
pub fn same_root_parent(pid1: u32, pid2: u32) -> Result<bool>;

/// 获取进程名称（跨平台）
pub fn get_process_name(pid: u32) -> Option<String>;
```

#### 3.2 core/task_tracker.rs
**职责**: 任务跟踪和状态管理
- 任务生命周期管理
- 状态更新和通知
- 资源使用监控
- 任务历史记录

**主要结构**:
```rust
pub struct TaskTracker {
    tasks: HashMap<TaskId, TaskInfo>,
    active_processes: HashMap<u32, TaskId>, // PID -> TaskId
}

impl TaskTracker {
    pub fn new() -> Self;
    pub fn create_task(&mut self, task_info: TaskInfo) -> TaskId;
    pub fn update_task_status(&mut self, task_id: TaskId, status: TaskStatus) -> Result<()>;
    pub fn get_task(&self, task_id: TaskId) -> Option<&TaskInfo>;
    pub fn get_tasks_by_parent(&self, parent_pid: u32) -> Vec<&TaskInfo>;
    pub fn terminate_task(&mut self, task_id: TaskId) -> Result<()>;
}
```

#### 3.3 core/shared_map.rs
**职责**: 共享内存管理
- 跨实例数据共享
- 实例注册和发现
- 数据同步和一致性
- 内存映射管理

**主要结构**:
```rust
pub struct SharedMemoryMap {
    instance_id: usize,
    map: SharedMap,
}

impl SharedMemoryMap {
    pub fn new() -> Result<Self>;
    pub fn register_instance(&mut self, instance: InstanceRegistry) -> Result<()>;
    pub fn register_task(&mut self, task: TaskInfo) -> Result<()>;
    pub fn update_task_status(&mut self, task_id: TaskId, status: TaskStatus) -> Result<()>;
    pub fn scan_all_instances(&self) -> Vec<TaskInfo>;
    pub fn cleanup_dead_instances(&mut self) -> Result<()>;
}
```

#### 3.4 registry.rs (任务注册表)
**职责**: 任务注册和管理（替代 task_tracker）
- 任务生命周期管理
- 共享内存中的任务注册
- 跨实例任务发现
- 任务清理和状态更新

### 4. provider/ - Provider 管理

#### 4.1 provider/mod.rs
**职责**: Provider 模块的统一入口
- Provider 管理器导出
- 配置验证
- 默认 Provider 定义

#### 4.2 provider/config.rs
**职责**: Provider 配置管理
- 配置文件读写
- 配置验证和校验
- 默认配置生成
- 配置迁移和升级

**主要结构**:
```rust
pub struct ConfigManager {
    config_path: PathBuf,
    config: ProviderConfig,
}

impl ConfigManager {
    pub fn load() -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn add_provider(&mut self, provider: Provider) -> Result<()>;
    pub fn remove_provider(&mut self, name: &str) -> Result<()>;
    pub fn set_default(&mut self, name: &str) -> Result<()>;
    pub fn validate_provider(&self, provider: &Provider) -> Result<()>;
    pub fn get_builtin_providers() -> Vec<Provider>;
}
```

#### 4.3 cli_manager.rs (AI CLI 管理)
**职责**: AI CLI 工具检测和管理（替代 detector.rs）
- 系统中已安装 AI CLI 的检测
- 版本信息获取
- 兼容性检查
- 安装类型识别（Native/NPM）

**主要结构**:
```rust
pub struct CliTool {
    pub name: String,
    pub command: String,
    pub npm_package: String,
    pub description: String,
    pub installed: bool,
    pub version: Option<String>,
    pub install_type: Option<InstallType>,
    pub install_path: Option<PathBuf>,
}

pub struct CliToolDetector {
    tools: Vec<CliTool>,
}
```

#### 4.4 token_validator.rs
**职责**: Token 验证和管理
- API Token 有效性检查
- Token 过期时间管理
- 刷新 Token 机制
- 加密存储验证

### 5. sync/ - Google Drive 同步

#### 5.1 sync/mod.rs
**职责**: 同步模块统一入口
- 同步服务导出
- 错误处理定义
- 配置管理

#### 5.2 sync/google_drive_service.rs
**职责**: Google Drive API 集成
- 文件上传下载
- 文件夹结构管理
- 冲突解决
- 增量同步

**主要结构**:
```rust
pub struct GoogleDriveService {
    client: DriveClient,
    auth_client: DeviceFlowClient,
    config: SyncConfig,
}

impl GoogleDriveService {
    pub async fn new(auth_client: DeviceFlowClient) -> Result<Self>;
    pub async fn ensure_authorized(&mut self) -> Result<bool>;
    pub async fn push_directories(&self, dirs: &[PathBuf]) -> Result<PushResult>;
    pub async fn pull_all(&self) -> Result<PullResult>;
    pub async fn list_files(&self) -> Result<Vec<DriveFile>>;
    pub async fn delete_file(&self, file_id: &str) -> Result<()>;
}
```

#### 5.3 sync/device_flow_client.rs
**职责**: Google OAuth 2.0 Device Flow 处理 (RFC 8628)
- Device Flow 授权流程
- 设备代码请求
- 授权状态轮询
- Token 管理和刷新
- 授权状态管理

#### 5.4 sync/smart_device_flow.rs
**职责**: 智能 Device Flow 封装
- TUI 状态管理
- 后台轮询处理
- 错误重试机制
- Token 缓存和刷新
- 授权缓存管理

#### 5.5 sync/sync_command.rs
**职责**: 同步命令执行
- Push/Pull 命令实现
- 进度跟踪和报告
- 错误处理和重试
- 冲突解决策略

#### 5.6 sync/network_detector.rs
**职责**: 网络环境检测
- 网络连通性检查
- 代理设置检测
- 防火墙限制识别
- 最佳连接方式选择

### 6. utils/ - 工具模块

#### 6.1 utils/mod.rs
**职责**: 工具模块统一入口

#### 6.2 utils/config_paths.rs
**职责**: 配置路径管理
- 跨平台路径处理
- 配置目录创建
- 路径验证和修复

#### 6.3 utils/logger.rs
**职责**: 日志系统
- 结构化日志记录
- 日志级别控制
- 文件和控制台输出
- 日志轮转和清理

#### 6.4 utils/version.rs
**职责**: 版本信息管理
- 版本号解析和比较
- 构建信息
- 更新检查（如果需要）

## 模块依赖关系图

```
main.rs
    ↓
commands/
    ↓ (调用)
tui/ ←→ core/
    ↓       ↓
provider/ ← core/
    ↓       ↓
sync/ ←→ utils/
    ↓       ↓
core/ ←→ utils/
```

## 模块间通信协议

### 1. 事件驱动通信
- 使用 `tokio::sync::mpsc` 进行异步通信
- 定义统一的事件类型和错误处理
- 模块间通过事件解耦

### 2. 共享状态管理
- 使用 `Arc<Mutex<T>>` 共享可变状态
- 使用 `tokio::sync::RwLock` 进行读写锁
- 避免死锁和竞态条件

### 3. 配置和状态持久化
- 统一的配置文件格式
- 原子性写入操作
- 配置变更通知机制

## 测试模块划分

### 1. 单元测试 (src/*/tests.rs)
- 每个模块包含对应的单元测试
- 测试覆盖率目标：≥90%
- 重点测试核心业务逻辑

### 2. 集成测试 (tests/integration.rs)
- 模块间集成测试
- 端到端功能测试
- 错误场景测试

### 3. 性能测试 (tests/performance.rs)
- 大量任务处理性能
- 内存使用优化
- 并发安全性测试