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

#### 3.1 智能进程树管理器

**核心价值**: 自动识别启动当前进程的AI CLI根进程，提供精确的进程归属追踪

##### 进程树追踪算法

```rust
/// 查找最近的AI CLI进程作为根进程
/// 如果没有找到AI CLI，则回退到传统的根父进程
pub fn find_ai_cli_root_parent(pid: u32) -> Result<u32, ProcessTreeError> {
    // 1. 获取完整进程链：当前进程 → 所有父进程 → 系统根进程
    let process_tree = get_process_tree(pid)?;

    // 2. 向上遍历进程链，跳过当前进程
    for &process_pid in process_tree.process_chain.iter().skip(1) {
        // 3. 检查是否为AI CLI进程
        if let Some(process_name) = get_process_name(process_pid) {
            if is_ai_cli_process(&process_name) {
                return Ok(process_pid); // 返回第一个找到的AI CLI
            }
        }
    }

    // 4. 如果没找到AI CLI，返回传统根父进程
    Ok(process_tree.root_parent_pid.unwrap())
}
```

##### AI CLI 识别规则

**支持的AI CLI类型**:
- **Native进程**: `claude`, `claude-cli`, `anthropic-claude`
- **Native进程**: `codex`, `codex-cli`, `openai-codex`
- **Native进程**: `gemini`, `gemini-cli`, `google-gemini`
- **NPM包**: `@anthropic-ai/claude-cli`, `codex-cli`, `@google/generative-ai-cli`

**智能检测逻辑**:
```rust
fn is_ai_cli_process(process_name: &str) -> bool {
    // 1. 精确匹配Native进程
    match process_name.to_lowercase().as_str() {
        "claude" | "claude-cli" | "anthropic-claude" => return true,
        "codex" | "codex-cli" | "openai-codex" => return true,
        "gemini" | "gemini-cli" | "google-gemini" => return true,
        _ => {}
    }

    // 2. 部分匹配（排除混淆进程如claude-desktop）
    if process_name.to_lowercase().contains("claude")
        && !process_name.to_lowercase().contains("claude-desktop") {
        return true;
    }

    // 3. NPM进程检测 + 命令行参数分析
    if is_npm_node_process(process_name) {
        return analyze_command_line_for_ai_cli(process_pid);
    }

    false
}
```

##### 核心设计

**进程树管理** (src/core/process_tree.rs):
采用简洁的独立函数设计，避免不必要的Manager结构（KISS原则）

```rust
/// 获取AI CLI根进程（带全局缓存）
pub fn get_root_parent_pid_cached() -> Result<u32>;

/// 查找最近的AI CLI进程作为根进程
pub fn find_ai_cli_root_parent(pid: u32) -> Result<u32>;

/// 检查两个进程是否属于同一AI CLI根进程
pub fn same_root_parent(pid1: u32, pid2: u32) -> Result<bool>;

/// 获取完整进程树信息
pub fn get_process_tree(pid: u32) -> Result<ProcessTreeInfo>;
```

**任务模型** (src/task_record.rs):
统一使用TaskRecord作为唯一的任务数据结构

```rust
pub enum TaskStatus {
    Running,
    CompletedButUnread,
}

pub struct TaskRecord {
    pub started_at: DateTime<Utc>,
    pub log_id: String,
    pub log_path: String,
    pub status: TaskStatus,
    pub exit_code: Option<i32>,
    pub process_chain: Vec<u32>,
    pub root_parent_pid: Option<u32>,
    // ... 其他字段见src/task_record.rs
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
用户命令 → CLI 解析 → Provider 选择 → 环境变量注入 → 子进程启动 → 智能进程树追踪 → 任务注册
```

**详细进程树追踪流程**:
```
1. agentic-warden 启动 → 获取当前进程PID
2. 向上遍历进程树 → 查找最近的AI CLI进程
3. AI CLI类型识别 → claude/codex/gemini + NPM包检测
4. 根进程缓存 → 优化性能，避免重复计算
5. 任务归属标记 → 将新任务关联到AI CLI根进程
6. 共享内存隔离 → 按AI CLI根进程分组管理
```

### 1.1 进程归属示例
```
示例场景：用户在Claude CLI中运行agentic-warden codex "prompt"

进程树：
explorer.exe (PID: 1234)
└── cmd.exe (PID: 2345)
    └── claude.exe (PID: 3456) ← AI CLI根进程
        └── agentic-warden.exe (PID: 4567)
            └── codex.exe (PID: 5678) ← 被追踪的任务

追踪结果：
- agentic-warden进程向上找到claude.exe作为AI CLI根进程
- codex任务被标记为属于claude.exe会话
- TUI界面显示："Claude CLI 会话中的任务"
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