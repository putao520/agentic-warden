# SPEC: 可复用模块索引

**目的**: 防止代码重复，记录项目中所有可复用的核心模块和基础设施。

**警告**: ⚠️ **在实现新功能前，必须先查阅此文档，避免重复实现已有功能！**

---

## 1. AI CLI 执行基础设施 (`src/supervisor.rs`)

### 核心功能
统一的 AI CLI (Claude/Codex/Gemini) 进程执行、监控和输出处理。

### 关键API

#### 1.1 `execute_cli()` - 标准CLI执行（输出镜像到终端）
```rust
pub async fn execute_cli<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
) -> Result<i32, ProcessError>
```
**用途**:
- 交互式执行AI CLI工具
- 输出实时显示到stdout/stderr
- 记录日志到文件

**特性**:
- ✅ Provider配置注入
- ✅ 进程树追踪
- ✅ 信号处理（优雅关闭）
- ✅ 跨平台支持
- ✅ Registry注册管理

#### 1.2 `execute_cli_with_output()` - CLI执行（捕获输出）
```rust
pub async fn execute_cli_with_output<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    args: &[OsString],
    provider: Option<String>,
    timeout: std::time::Duration,
) -> Result<String, ProcessError>
```
**用途**:
- 非交互式执行（如代码生成）
- 捕获stdout到字符串返回
- 支持超时控制

**特性**:
- ✅ 所有 `execute_cli()` 的特性
- ✅ 超时控制（防止长时间hang）
- ✅ 输出捕获到内存

#### 1.3 `start_interactive_cli()` - 交互模式启动
```rust
pub async fn start_interactive_cli<S: TaskStorage>(
    registry: &Registry<S>,
    cli_type: &CliType,
    provider: Option<String>,
) -> Result<i32, ProcessError>
```
**用途**: 启动完全交互式AI CLI会话（stdin继承）

#### 1.4 `execute_multiple_clis()` - 批量执行
```rust
pub async fn execute_multiple_clis<S: TaskStorage>(
    registry: &Registry<S>,
    cli_selector: &crate::cli_type::CliSelector,
    task_prompt: &str,
    provider: Option<String>,
) -> Result<Vec<i32>, ProcessError>
```
**用途**: 顺序执行多个AI CLI（如 `claude|codex|gemini` 语法）

### 内部实现设计
```rust
// 策略模式避免代码重复
enum OutputStrategy {
    Mirror,                           // 镜像到stdout/stderr
    Capture(Arc<Mutex<Vec<u8>>>),    // 捕获到buffer
}

async fn execute_cli_internal<S: TaskStorage>(...) -> Result<(i32, Option<String>), ProcessError>
```

### 使用示例
```rust
// 代码生成场景（需要捕获输出）
let registry = create_cli_registry()?;
let cli_type = CliType::Claude;
let args = cli_type.build_full_access_args("Generate workflow code...");
let os_args: Vec<OsString> = args.into_iter().map(|s| s.into()).collect();

let output = supervisor::execute_cli_with_output(
    &registry,
    &cli_type,
    &os_args,
    Some("llmlite".to_string()),  // Provider可选
    Duration::from_secs(12 * 60 * 60),
).await?;

// 交互式场景（输出直接显示）
let exit_code = supervisor::execute_cli(
    &registry,
    &cli_type,
    &os_args,
    None,  // 使用default provider
).await?;
```

---

## 2. CLI类型管理 (`src/cli_type.rs`)

### 核心功能
AI CLI工具类型定义、命令参数构建、检测和选择。

### 关键API

#### 2.1 `CliType` 枚举
```rust
pub enum CliType {
    Claude,
    Codex,
    Gemini,
}
```
**方法**:
- `command_name()` → CLI命令名（claude/codex/gemini）
- `display_name()` → 显示名称
- `env_var_name()` → 环境变量名（CLAUDE_BIN/CODEX_BIN/GEMINI_BIN）
- `build_full_access_args(prompt)` → 非交互式完整权限参数
- `build_interactive_args()` → 交互模式参数

#### 2.2 `CliSelector` - 多CLI选择
```rust
pub struct CliSelector {
    pub types: Vec<CliType>,
}
```
**方法**:
- `all()` → 所有可用CLI
- `from_single(cli_type)` → 单个CLI
- `from_multiple(types)` → 多个CLI

#### 2.3 解析函数
```rust
pub fn parse_cli_type(arg: &str) -> Option<CliType>
pub fn parse_cli_selector(arg: &str) -> Option<CliSelector>
pub fn parse_cli_selector_strict(arg: &str) -> AgenticResult<CliSelector>
```

### 使用示例
```rust
// 构建命令参数
let cli_type = CliType::Claude;
let args = cli_type.build_full_access_args("Implement feature X");
// 返回: ["-p", "--dangerously-skip-permissions", "Implement feature X"]

// 解析CLI选择器
let selector = parse_cli_selector("claude|gemini").unwrap();
// selector.types = [CliType::Claude, CliType::Gemini]
```

---

## 3. Provider配置系统 (`src/provider/`)

### 核心功能
AI服务供应商配置管理（API密钥、端点、环境变量注入）。

### 关键API

#### 3.1 `ProviderManager` - 配置管理器
```rust
impl ProviderManager {
    pub fn new() -> Result<Self>
    pub fn get_provider(&self, name: &str) -> Result<ProviderConfig>
    pub fn get_default_provider(&self) -> Option<(String, ProviderConfig)>
    pub fn list_providers(&self) -> Vec<String>
}
```

#### 3.2 `ProviderConfig` - 供应商配置
```rust
pub struct ProviderConfig {
    pub env: HashMap<String, String>,  // 环境变量注入
    pub endpoint: Option<String>,
    pub model: Option<String>,
}
```

### 使用示例
```rust
let provider_manager = ProviderManager::new()?;

// 获取指定供应商
let llmlite = provider_manager.get_provider("llmlite")?;

// 获取默认供应商
let (name, config) = provider_manager.get_default_provider()
    .ok_or("No default provider")?;

// 注入环境变量到Command
for (key, value) in &config.env {
    command.env(key, value);
}
```

---

## 4. Registry系统 (`src/unified_registry.rs`)

### 核心功能
进程注册、追踪、清理和持久化。

### 关键API

#### 4.1 `Registry<S: TaskStorage>` - 统一注册表
```rust
impl<S: TaskStorage> Registry<S> {
    pub fn new(storage: S) -> Result<Self>
    pub fn register(&self, pid: u32, record: &TaskRecord) -> Result<()>
    pub fn mark_completed(&self, pid: u32, result: Option<String>, exit_code: Option<i32>, completed_at: DateTime<Utc>) -> Result<()>
    pub fn sweep_stale_entries<F1, F2>(&self, now: DateTime<Utc>, is_alive: F1, terminate: F2) -> Result<()>
}
```

#### 4.2 `create_cli_registry()` - 工厂函数
```rust
pub fn create_cli_registry() -> Result<Registry<impl TaskStorage>, AgenticWardenError>
```
**用途**: 创建带SQLite存储的Registry实例

### 使用示例
```rust
let registry = create_cli_registry()?;

// 注册进程
let record = TaskRecord::new(Utc::now(), pid.to_string(), log_path, parent_pid);
registry.register(pid, &record)?;

// 标记完成
registry.mark_completed(pid, Some("success".to_owned()), Some(0), Utc::now())?;

// 清理过期进程
registry.sweep_stale_entries(
    Utc::now(),
    platform::process_alive,
    |pid| { platform::terminate_process(pid); Ok(()) }
)?;
```

---

## 5. MCP连接池系统 (`src/mcp_routing/pool.rs`)

### 核心功能
MCP服务器连接管理、热重载、工具调用路由。

### 关键API

#### 5.1 `McpConnectionPool` - 连接池
```rust
impl McpConnectionPool {
    pub async fn new(config: Arc<McpConfig>) -> Result<Self>
    pub async fn call_tool(&self, server: &str, tool: &str, args: serde_json::Value) -> Result<serde_json::Value>
    pub async fn list_all_tools(&self) -> Result<Vec<ToolInfo>>
    pub async fn update_config(&self, new_config: Arc<McpConfig>)
}
```

#### 5.2 热重载特性
```rust
// 配置更新时自动：
// 1. 关闭已移除的服务器
// 2. 关闭已禁用的服务器
// 3. 重启配置变更的服务器
// 4. 保持配置未变的服务器运行
// 5. 新服务器延迟加载（首次调用时）
pub async fn update_config(&self, new_config: Arc<McpConfig>)
```

### 使用示例
```rust
// 创建连接池
let pool = McpConnectionPool::new(config).await?;

// 调用MCP工具
let result = pool.call_tool(
    "filesystem",
    "read_file",
    json!({"path": "/tmp/test.txt"})
).await?;

// 热重载配置
pool.update_config(new_config).await;
```

---

## 6. 代码生成抽象 (`src/mcp_routing/codegen.rs`)

### 核心功能
工作流规划和JS代码生成的统一接口，支持多后端（Ollama/AI CLI）。

### 关键API

#### 6.1 `WorkflowPlannerEngine` trait
```rust
#[async_trait]
pub trait WorkflowPlannerEngine: Send + Sync {
    async fn plan_workflow(&self, user_request: &str, available_tools: &[CandidateToolInfo]) -> Result<WorkflowPlan>;
    async fn generate_js_code(&self, plan: &WorkflowPlan) -> Result<String>;
}
```

#### 6.2 `CodeGeneratorFactory` - 工厂模式
```rust
impl CodeGeneratorFactory {
    pub fn from_env(default_endpoint: String, default_model: String) -> Result<Arc<dyn WorkflowPlannerEngine>>
}
```

**自动检测逻辑**:
- `OPENAI_TOKEN` 存在 → Ollama模式（本地LLM）
- 否则 → AI CLI模式（默认claude）

**环境变量**:
- `CLI_TYPE`: claude/codex/gemini（必选其一）
- `CLI_PROVIDER`: llmlite/openrouter/anthropic/等（可选）

#### 6.3 实现类

##### AiCliCodeGenerator
```rust
pub struct AiCliCodeGenerator {
    cli_type: CliType,
    provider: Option<String>,
    timeout: Duration,  // 固定12小时
}
```
**内部实现**: 完全复用 `supervisor::execute_cli_with_output()`

### 使用示例
```rust
// 自动检测并创建
let generator = CodeGeneratorFactory::from_env(
    "http://localhost:11434".to_string(),
    "qwen2.5:7b".to_string(),
)?;

// 规划工作流
let plan = generator.plan_workflow(
    "Generate a git status report",
    &available_tools,
).await?;

// 生成JS代码
let js_code = generator.generate_js_code(&plan).await?;
```

---

## 7. 配置热重载系统 (`src/mcp_routing/config_watcher.rs`)

### 核心功能
文件系统监听 + 自动配置重载 + 服务生命周期管理。

### 关键API

#### 7.1 `start_config_watcher()` - 启动监听
```rust
pub async fn start_config_watcher(
    connection_pool: Arc<McpConnectionPool>,
    config_path: PathBuf,
) -> Result<()>
```

**监听事件**:
- ✅ 文件修改（ModifyKind::Data）
- ✅ 文件创建（.mcp.json）
- ✅ 写入关闭（AccessKind::Close(Write)）

**重载策略**:
- 100ms去抖延迟
- 自动触发 `McpConnectionPool::update_config()`
- 错误不中断监听

### 使用示例
```rust
let pool = Arc::new(McpConnectionPool::new(config).await?);
let config_path = PathBuf::from("/path/to/.mcp.json");

// 启动后台监听（fire-and-forget）
start_config_watcher(pool.clone(), config_path).await?;

// 监听持续运行，文件变更时自动重载
```

---

## 8. JS运行时系统 (`src/mcp_routing/js_orchestrator/`)

### 核心功能
安全的JS代码执行环境，用于工作流编排。

### 关键API

#### 8.1 `BoaRuntime` - JS执行引擎
```rust
impl BoaRuntime {
    pub fn new(security_config: SecurityConfig) -> Self
    pub fn inject_mcp(&mut self, invoker: Arc<dyn McpToolInvoker>)
    pub fn run_workflow(&mut self, js_code: &str, input: serde_json::Value) -> Result<serde_json::Value>
}
```

#### 8.2 `BoaRuntimePool` - 运行时池
```rust
impl BoaRuntimePool {
    pub fn new(size: usize) -> Self
    pub async fn execute(&self, code: &str, input: serde_json::Value, invoker: Arc<dyn McpToolInvoker>) -> Result<serde_json::Value>
}
```

### 使用示例
```rust
let pool = BoaRuntimePool::new(4);  // 4个runtime实例

let result = pool.execute(
    &js_code,
    json!({"repo": "agentic-warden"}),
    mcp_invoker,
).await?;
```

---

## 9. 平台抽象层 (`src/platform/`)

### 核心功能
跨平台进程管理、信号处理、终止操作。

### 关键API

#### 9.1 进程管理
```rust
pub fn init_platform()
pub fn current_pid() -> u32
pub fn process_alive(pid: u32) -> bool
pub fn terminate_process(pid: u32)
```

#### 9.2 信号处理（Unix）
```rust
pub fn install(child_pid: u32) -> Result<SignalGuard, ProcessError>
```
**自动处理**: SIGTERM, SIGINT → 转发给子进程

### 使用示例
```rust
platform::init_platform();

let guard = signal::install(child_pid)?;
let status = child.wait().await?;
drop(guard);  // 自动清理信号处理
```

---

## 10. 内存子系统 (`src/memory/`)

### 核心功能
向量检索（MemVDB）+ 对话历史（SahomeDB）双模式内存。

### 关键API

#### 10.1 `MemoryConfig` - 配置
```rust
pub struct MemoryConfig {
    pub fastembed_model: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub sahome_db_path: PathBuf,
}
```

#### 10.2 `MemVDB` - 向量检索
```rust
impl MemVDB {
    pub fn new(model_name: &str) -> Result<Self>
    pub async fn index_tool(&mut self, server: &str, tool: &str, description: &str) -> Result<()>
    pub async fn search_tools(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>>
}
```

#### 10.3 `SahomeDB` - 对话历史
```rust
impl SahomeDB {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self>
    pub fn save_turn(&self, role: &str, content: &str) -> Result<()>
    pub fn load_history(&self, limit: usize) -> Result<Vec<ConversationTurn>>
}
```

---

## 反模式警告 ⚠️

### ❌ 不要做的事情

1. **不要重复实现CLI执行逻辑**
   ```rust
   // ❌ 错误示例
   let mut cmd = Command::new("claude");
   cmd.args(args);
   let output = cmd.output().await?;

   // ✅ 正确做法
   supervisor::execute_cli_with_output(&registry, &cli_type, &args, provider, timeout).await?
   ```

2. **不要手动管理进程生命周期**
   ```rust
   // ❌ 错误示例
   let mut child = command.spawn()?;
   let pid = child.id().unwrap();
   // ... 忘记注册到registry

   // ✅ 正确做法
   // 使用supervisor模块，自动处理registry注册、信号、清理
   ```

3. **不要绕过Provider系统**
   ```rust
   // ❌ 错误示例
   command.env("ANTHROPIC_API_KEY", "sk-xxx");

   // ✅ 正确做法
   let provider_manager = ProviderManager::new()?;
   let config = provider_manager.get_provider("llmlite")?;
   for (k, v) in &config.env {
       command.env(k, v);
   }
   ```

4. **不要创建重复的抽象层**
   ```rust
   // ❌ 错误示例：创建新的OutputStrategy enum
   enum MyOutputMode { Capture, Stream }

   // ✅ 正确做法：使用现有的supervisor::OutputStrategy（内部使用）
   ```

---

## 新功能开发检查清单 ✅

在实现新功能前，检查以下问题：

- [ ] 需要执行AI CLI吗？→ 使用 `supervisor::execute_cli*()`
- [ ] 需要进程管理吗？→ 使用 `Registry` + `platform`模块
- [ ] 需要供应商配置吗？→ 使用 `ProviderManager`
- [ ] 需要MCP工具调用吗？→ 使用 `McpConnectionPool`
- [ ] 需要代码生成吗？→ 使用 `CodeGeneratorFactory`
- [ ] 需要文件监听吗？→ 参考 `config_watcher.rs`
- [ ] 需要JS执行吗？→ 使用 `BoaRuntimePool`
- [ ] 需要向量检索吗？→ 使用 `MemVDB`
- [ ] 需要对话历史吗？→ 使用 `SahomeDB`

---

## 架构原则

### 单一职责
- `supervisor` → 进程执行
- `cli_type` → CLI定义
- `provider` → 配置管理
- `registry` → 进程追踪
- `mcp_routing` → 工具路由

### 策略模式
使用策略模式而非重复代码：
```rust
// supervisor.rs 内部设计
enum OutputStrategy { Mirror, Capture(...) }
async fn execute_cli_internal(..., output_strategy: OutputStrategy)
```

### 工厂模式
使用工厂创建复杂对象：
```rust
CodeGeneratorFactory::from_env()  // 自动检测环境
create_cli_registry()             // 创建带存储的Registry
```

### Trait抽象
定义可插拔接口：
```rust
trait WorkflowPlannerEngine { ... }  // Ollama/AI CLI都实现
trait TaskStorage { ... }            // SQLite/内存都实现
trait McpToolInvoker { ... }         // 真实MCP/Mock都实现
```

---

## 版本历史

- **v1.0** (2025-11-19): 初始版本，记录核心可复用模块
  - supervisor重构完成（消除重复代码）
  - AI CLI代码生成器集成

---

**维护说明**:
- 添加新的可复用模块时，更新此文档
- 发现重复代码时，重构并更新此文档
- 每月审查一次，确保文档与代码同步
