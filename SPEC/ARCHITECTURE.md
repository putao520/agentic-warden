# Agentic-Warden 架构设计

## 系统架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    Agentic-Warden                           │
├─────────────────┬─────────────────┬─────────────────────────┤
│   CLI Layer     │   TUI Layer     │   Core Engine           │
│                 │                 │                         │
│ • AI Commands   │ • Dashboard     │ • Process Tree Manager  │
│ • Provider Mgmt │ • Provider UI   │ • Google Drive Service  │
│ • Status Cmd    │ • Status UI     │ • Config Manager        │
│ • Push/Pull     │ • Progress UI   │ • Shared Memory         │
└─────────────────┴─────────────────┴─────────────────────────┘
                           │
                    ┌──────┴───────┐
                    │  Data Layer  │
                    │              │
                    │ • Config JSON│
                    │ • Auth JSON  │
                    │ • Shared Mem │
                    └──────────────┘
```

## 核心模块设计

### 1. CLI 命令模块 (CLI Layer)

#### 1.1 AI CLI 启动命令
```rust
// src/commands/
pub mod ai_cli {
    pub struct AiCliCommand {
        pub ai_type: AiType,        // codex, claude, gemini
        pub provider: Option<String>, // -p 参数
        pub prompt: String,         // 提示词
        pub multi_mode: bool,       // 多 AI 模式
    }

    pub enum AiType {
        Codex,
        Claude,
        Gemini,
        All, // all 命令
    }
}
```

#### 1.2 TUI 管理命令
```rust
// src/commands/
pub mod tui_commands {
    pub struct DashboardCommand;
    pub struct StatusCommand;
    pub struct PushCommand {
        pub dirs: Vec<PathBuf>,
    }
    pub struct PullCommand;
}
```

#### 1.3 命令解析和路由
- 使用 `clap` 进行命令行参数解析
- 统一的命令路由机制
- 环境变量注入逻辑

### 2. TUI 界面模块 (TUI Layer)

#### 2.1 应用状态管理
```rust
// src/tui/app.rs
pub struct App {
    pub current_screen: Screen,
    pub should_quit: bool,
    pub providers: Vec<Provider>,
    pub tasks: Vec<TaskInfo>,
    pub selected_index: usize,
}

pub enum Screen {
    Dashboard,
    ProviderList,
    ProviderEdit,
    Status,
    PushProgress,
    PullProgress,
}
```

#### 2.2 屏幕组件
```rust
// src/tui/screens/
pub mod dashboard {
    pub struct DashboardScreen;
    // 显示 AI CLI 状态 + 任务概要
}

pub mod provider {
    pub struct ProviderListScreen;
    pub struct ProviderEditScreen;
    // Provider 列表和编辑界面
}

pub mod status {
    pub struct StatusScreen;
    // 任务状态实时显示
}

pub mod progress {
    pub struct PushProgressScreen;
    pub struct PullProgressScreen;
    // Push/Pull 进度显示
}
```

#### 2.3 事件处理系统
```rust
// 使用 ratatui 内置的事件处理机制
// ratatui + crossterm 提供了完整的事件处理能力
// 无需单独实现事件循环，直接使用框架提供的方法

// 示例：事件处理集成在 TUI 应用中
impl App {
    pub fn handle_event(&mut self, event: crossterm::event::Event) -> Result<()> {
        match event {
            crossterm::event::Event::Key(key) => self.handle_key_event(key),
            crossterm::event::Event::Resize(_, _) => self.handle_resize(),
            // 其他事件处理...
        }
        Ok(())
    }
}
```

#### 2.4 UI 组件使用原则
```rust
// UI 组件使用原则：
// 1. 直接使用 ratatui 提供的组件，不创建自定义组件
// 2. 组合使用现有组件构建界面
// 3. 避免不必要的抽象层

// 可用的 ratatui 组件：
// - Paragraph: 文本显示，用于错误信息、说明等
// - Block: 边框和标题，用于分组
// - List: 列表显示，用于选项、菜单等
// - Gauge: 进度条，用于进度显示
// - Chart: 图表，用于数据可视化（如需要）
// - Table: 表格，用于结构化数据

// 示例：错误提示使用 Paragraph + Block
fn render_error(frame: &mut Frame, error: &str, area: Rect) {
    let error_text = Paragraph::new(error)
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title("错误")
        );
    frame.render_widget(error_text, area);
}

// 示例：弹出效果使用 Clear
fn render_popup(frame: &mut Frame, content: &str, area: Rect) {
    // 清除背景
    frame.render_widget(Clear, frame.size());

    // 渲染弹窗内容
    let popup = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(popup, area);
}
```

### 3. 核心引擎模块 (Core Engine)

#### 3.1 进程树管理器
```rust
// src/process_tree/
pub struct ProcessTreeManager {
    registry: ConnectedRegistry,
    task_tracker: TaskTracker,
}

impl ProcessTreeManager {
    pub fn find_root_process(&self) -> Option<ProcessInfo>;
    pub fn track_ai_cli_process(&mut self, cmd: &AiCliCommand) -> Result<TaskId>;
    pub fn get_all_tasks(&self) -> Vec<TaskInfo>;
    pub fn terminate_task(&mut self, task_id: TaskId) -> Result<()>;
}

pub struct TaskInfo {
    pub id: TaskId,
    pub parent_process: ProcessInfo,
    pub ai_type: AiType,
    pub prompt_preview: String,
    pub status: TaskStatus,
    pub start_time: SystemTime,
}

pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Terminated,
}
```

#### 3.2 Google Drive 服务
```rust
// src/sync/google_drive_service.rs
pub struct GoogleDriveService {
    oauth_client: OAuthClient,
    auth_config: AuthConfig,
}

impl GoogleDriveService {
    pub async fn ensure_authorized(&mut self) -> Result<bool>;
    pub async fn push_directories(&self, dirs: &[PathBuf]) -> Result<PushResult>;
    pub async fn pull_all(&self) -> Result<PullResult>;
    pub async fn refresh_token(&mut self) -> Result<()>;
}

// OOB 授权流程
pub struct OAuthClient {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl OAuthClient {
    pub async fn start_oob_flow(&self) -> Result<String>; // 返回授权URL
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<TokenInfo>;
    pub fn start_callback_server(&self) -> Result<()>;
}
```

#### 3.3 配置管理器
```rust
// src/provider/
pub struct ConfigManager {
    config_path: PathBuf,
    providers: HashMap<String, Provider>,
    default_provider: String,
}

impl ConfigManager {
    pub fn load(&mut self) -> Result<()>;
    pub fn save(&self) -> Result<()>;
    pub fn add_provider(&mut self, provider: Provider) -> Result<()>;
    pub fn remove_provider(&mut self, name: &str) -> Result<()>;
    pub fn set_default(&mut self, name: &str) -> Result<()>;
    pub fn get_provider(&self, name: &str) -> Option<&Provider>;
    pub fn get_env_vars(&self, provider_name: &str) -> HashMap<String, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub description: String,
    pub compatible_with: Vec<AiType>,
    pub env: HashMap<String, String>,
}
```

#### 3.4 共享内存管理
```rust
// src/shared_map.rs
pub struct SharedMemoryMap {
    instance_id: usize,
    map: SharedMap,
}

impl SharedMemoryMap {
    pub fn new() -> Result<Self>;
    pub fn register_task(&mut self, task: TaskInfo) -> Result<()>;
    pub fn update_task_status(&mut self, task_id: TaskId, status: TaskStatus) -> Result<()>;
    pub fn get_tasks_by_parent(&self, parent_pid: Pid) -> Vec<TaskInfo>;
    pub fn scan_all_instances(&self) -> Vec<TaskInfo>;
}
```

## 数据流架构

### 1. AI CLI 启动流程
```
用户命令 → CLI 解析 → Provider 选择 → 环境变量注入 → 子进程启动 → 进程树注册
```

### 2. TUI 管理流程
```
TUI 启动 → 事件循环 → 屏幕渲染 → 用户交互 → 状态更新 → 界面刷新
```

### 3. Google Drive 集成流程
```
Push/Pull 命令 → 授权检查 → OOB 流程(如需要) → 文件操作 → 进度显示
```

## 模块依赖关系

```
cli_commands
    ↓
tui_app ← core_engine
    ↓           ↓
event_handler ← shared_memory
    ↓           ↓
screens ←→ process_tree_manager
    ↓           ↓
widgets ←→ google_drive_service
             ↓
         config_manager
```

## 关键设计决策

### 1. 架构分层
- **CLI Layer**: 命令行接口和参数解析
- **TUI Layer**: 用户界面和交互逻辑
- **Core Engine**: 核心业务逻辑和服务

### 2. 异步设计
- Google Drive 操作使用 `async/await`
- TUI 事件处理保持响应性
- 进程监控使用后台线程

### 3. 错误处理
- 统一的 `Result<T>` 错误处理
- 用户友好的错误信息
- 优雅的降级处理

### 4. 配置管理
- JSON 格式配置文件
- 热重载支持
- 默认值和验证

## 性能考虑

### 1. 内存使用
- 共享内存用于进程间通信
- 避免不必要的数据复制
- 定期清理完成的任务

### 2. 响应性
- TUI 刷新率 2 秒
- 异步操作避免阻塞
- 事件驱动架构

### 3. 扩展性
- 插件化的 Provider 系统
- 可配置的扫描范围
- 模块化的屏幕组件