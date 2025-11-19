# System Architecture Design - v0.1.0

## Version Information
- Current architecture version: v0.1.0
- Last updated: 2025-11-14
- Based on: Initial development

---

## [v0] Current Architecture Overview

### System Context Diagram

```mermaid
graph TB
    subgraph "User Environment"
        User[User]
        CLI[Terminal]
        TUI[TUI Interface]
    end

    subgraph "AI WARDEN - Three Core Modules"

        subgraph "1. External AI CLI Management"
            subgraph "1.1 供应商管理"
                ProviderMgr[Provider Manager]
                EnvInjector[Env Injector]
            end
            subgraph "1.2 AI CLI本地维护"
                CliDetector[CLI Detector]
                CliManager[CLI Manager]
            end
            ProcTracker[Process Tree Tracker]
            TaskRegistry[Task Registry]
        end

        subgraph "2. CC Session Management"
            MemoryMgr[Memory Manager]
            ConvHistory[Conversation History]
            SearchEngine[Semantic Search]
        end

        subgraph "3. MCP Proxy Routing"
            MCPRouter[MCP Router]
            ToolIndex[Tools Index]
            LLMEngine[LLM Decision Engine]
        end
    end

    subgraph "Supporting Services"
        SyncMgr[Sync Manager]
        TUIEngine[TUI Engine]
        CLIEngine[CLI Engine]
    end

    subgraph "External Services"
        AICLI1[codex CLI]
        AICLI2[claude CLI]
        AICLI3[gemini CLI]
        MCPsrv1[MCP Server 1]
        MCPsrv2[MCP Server 2]
        ClaudeCode[Claude Code User]
        GoogleDrive[Google Drive API]
    end

    subgraph "Storage"
        SharedMem[Shared Memory]
        ConfigFiles[Config Files]
        LocalFS[Local File System]
    end

    User --> CLI
    User --> TUI
    CLI --> Parser
    TUI --> Parser
    Parser --> Router
    Router --> Supervisor
    Router --> ProcTracker
    Router --> ProviderMgr
    Router --> SyncMgr
    Router --> MCPsrv

    Supervisor --> AICLI1
    Supervisor --> AICLI2
    Supervisor --> AICLI3

    ProcTracker --> SharedMem
    ProviderMgr --> ConfigFiles
    SyncMgr --> GoogleDrive
    Supervisor --> Registry
    Registry --> SharedMem
    SyncMgr --> LocalFS
```

### Three Core Modules Architecture

#### Module Independence and Relationships

AI WARDEN的三大核心模块在功能上相互独立，各自服务于不同的业务场景：

##### Module 1: External AI CLI Management
**服务对象**: AI CLI用户和开发者
**主要职责**:
- 管理外部AI CLI工具（Claude、Gemini、Codex等）的生命周期
- 提供商配置管理和环境变量注入
- 跨进程任务跟踪和状态监控
- 多AI CLI并发执行协调

**子模块1.1: 供应商管理 (Provider Management)**
**核心功能**:
- **供应商配置管理**: 支持多供应商配置和环境变量管理
- **多供应商支持**: OpenRouter、Anthropic、Google、DeepSeek等第三方API供应商
- **环境变量注入**: 动态注入`OPENAI_API_KEY`、`OPENAI_BASE_URL`等环境变量到AI CLI进程
- **兼容性验证**: 检查供应商与AI CLI类型的兼容性（如OpenRouter支持codex、claude、gemini）
- **默认供应商机制**: 支持设置全局默认供应商，可通过`-p`参数覆盖
- **健康检查**: 定期检查供应商连接状态（可选，间隔300秒）
- **敏感信息保护**: API Key等敏感值在日志和TUI中自动脱敏显示

**关键组件**:
- `ProviderManager`: 供应商管理核心逻辑
- `ProviderConfig`: 供应商配置结构定义
- `EnvInjector`: 环境变量注入器
- `EnvMapping`: AI CLI到环境变量的映射规则

**子模块1.2: AI CLI本地维护 (AI CLI Maintenance)**
**核心功能**:
- **自动检测**: 检测本地已安装的AI CLI工具（通过PATH查找和npm全局包检测）
- **版本管理**: 识别AI CLI版本（原生二进制 vs NPM包）
- **安装状态监控**: 实时检查AI CLI可执行文件的可用性
- **安装建议**: 对未安装的AI CLI提供安装命令提示（如`npm install -g @google/gemini-cli`）
- **更新检测**: 检查AI CLI是否有新版本可用（可选功能）
- **可执行路径定位**: 记录并缓存AI CLI的完整可执行路径
- **TUI状态展示**: 在TUI界面展示所有AI CLI的安装状态、版本、路径

**关键组件**:
- `CliToolDetector`: AI CLI检测和识别
- `CliType`: AI CLI类型枚举（Claude、Codex、Gemini）
- `CliManager`: AI CLI生命周期管理
- `StatusScreen`: TUI状态展示界面

**数据流**:
```
用户命令 → CLI解析 → [供应商管理: 加载配置+注入环境变量]
        → [AI CLI维护: 检测可执行文件] → AI CLI启动 → 进程监控 → 任务完成
```

##### Module 2: CC Session Management
**服务对象**: Claude Code用户
**主要职责**:
- 存储Claude Code的JSONL格式对话历史
- 基于向量嵌入的语义搜索
- session_id分组的会话管理
- 提供历史对话检索MCP工具

**数据流**:
```
Claude Code会话 → JSONL存储 → 向量化 → 语义索引 → 搜索检索 → 上下文返回
```

##### Module 3: MCP Proxy Routing
**服务对象**: AI助手（Claude/Gemini/Codex）
**主要职责**:
- MCP服务器工具的向量化索引
- 智能工具选择和路由决策
- 两阶段搜索算法（工具级→方法级）
- 统一的MCP接口对外提供服务

**数据流**:
```
AI助手请求 → 语义搜索 → 工具聚类 → LLM决策 → 工具执行 → 结果返回
```

#### MCP Server对外接口 (跨模块统一暴露)

AI WARDEN通过**单一MCP Server**对外暴露工具,但工具来自不同的功能模块:

| 工具名 | 所属模块 | 功能描述 | 数据存储 | 服务对象 |
|-------|---------|---------|---------|---------|
| `search_history` | Module 2: CC Session Management | 语义搜索Claude Code历史对话 | SahomeDB (持久化) | Claude Code用户 |
| `intelligent_route` | Module 3: MCP Proxy Routing | 智能选择和路由到最佳MCP工具 | MemVDB (内存) | AI助手 |
| 动态代理工具 | Module 3: MCP Proxy Routing | 按需注册,代理到外部MCP服务器 | - | AI助手 |

**架构模式**: Facade Pattern
- **统一入口**: 单一MCP Server进程对外提供服务
- **内部路由**: 根据工具名路由到对应模块的handler
- **模块解耦**: 各模块独立实现,通过接口协作
- **动态扩展**: Module 3支持运行时动态注册新工具

**MCP对外接口层架构**:

```mermaid
graph TB
    subgraph "External Client"
        ClaudeCode[Claude Code / AI Assistant]
    end

    subgraph "AI WARDEN - Unified MCP Interface"
        MCPServer[MCP Server<br/>统一对外接口<br/>Facade Pattern]
    end

    subgraph "Module 2: CC Session Management"
        SearchHistory[search_history<br/>Handler]
        SahomeDB[(SahomeDB<br/>Persistent Storage)]
    end

    subgraph "Module 3: MCP Proxy Routing"
        IntelligentRoute[intelligent_route<br/>Handler]
        DynamicProxy[Dynamic Proxy<br/>Tools Handler]
        MemVDB[(MemVDB<br/>In-Memory Index)]
        RMCPPool[RMCP Client Pool]
        ExternalMCP[External MCP<br/>Servers]
    end

    ClaudeCode -->|list_tools| MCPServer
    ClaudeCode -->|call: search_history| MCPServer
    ClaudeCode -->|call: intelligent_route| MCPServer
    ClaudeCode -->|call: dynamic_tool_xxx| MCPServer

    MCPServer -->|route Module 2| SearchHistory
    MCPServer -->|route Module 3| IntelligentRoute
    MCPServer -->|route Module 3| DynamicProxy

    SearchHistory --> SahomeDB
    IntelligentRoute --> MemVDB
    DynamicProxy --> RMCPPool
    RMCPPool --> ExternalMCP
```

**关键设计决策**:
1. **为什么单一MCP Server而非多MCP进程?**
   - 减少Claude Code配置复杂度(只需配置一个MCP服务器)
   - 统一管理连接和生命周期
   - 便于跨模块数据共享(如会话上下文)

2. **为什么跨模块暴露工具?**
   - 用户视角: 统一的MCP工具集,无需关心内部模块划分
   - 实现视角: 模块内聚,各自管理独立的数据和逻辑
   - 扩展性: 未来可无缝添加新模块的工具

#### Module Integration Points

虽然三大模块功能独立，但通过以下方式进行协作：

1. **进程管理模块**为其他模块提供进程隔离和资源管理
2. **会话管理模块**(Module 2)通过统一MCP接口向用户提供历史检索服务(`search_history`)
3. **路由模块**(Module 3)通过统一MCP接口为AI助手提供智能工具选择能力(`intelligent_route`及动态代理工具)

### Core Business Flows

#### 1. Provider-Based AI CLI Execution Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Parser
    participant ProviderMgr
    participant EnvInjector
    participant Supervisor
    participant AICLI
    participant Registry
    participant SharedMem

    User->>CLI: agentic-warden codex -p openrouter "task"
    CLI->>Parser: parse command
    Parser->>ProviderMgr: get_provider("openrouter")
    ProviderMgr->>ProviderMgr: validate_compatibility(codex)
    ProviderMgr-->>Parser: ProviderConfig

    Parser->>Supervisor: execute_cli(codex, provider_config)
    Supervisor->>EnvInjector: prepare_environment(provider_config)
    EnvInjector-->>Supervisor: env_vars HashMap

    Supervisor->>Supervisor: spawn_process(codex, env_vars)
    Supervisor->>AICLI: start with injected env
    Supervisor->>Registry: register_task(pid, task_info)
    Registry->>SharedMem: store_task_record

    Note over AICLI: AI CLI executes task
    AICLI-->>Supervisor: task_completion
    Supervisor->>Registry: mark_completed(pid, result)
    Registry->>SharedMem: update_status
```

#### 2. Google Drive Configuration Synchronization Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SyncMgr
    participant ConfigPacker
    participant OAuthClient
    participant DriveService
    participant LocalFS

    User->>CLI: agentic-warden push
    CLI->>SyncMgr: handle_push_command()

    SyncMgr->>ConfigPacker: pack_ai_configs()
    ConfigPacker->>ConfigPacker: scan_ai_cli_directories()
    Note over ConfigPacker: Selective file inclusion<br/>CLAUDE.md, settings.json, SKILL.md<br/>auth.json, config.toml, etc.
    ConfigPacker->>ConfigPacker: create_tar_gz_archive()
    ConfigPacker-->>SyncMgr: compressed_config.tar.gz

    SyncMgr->>OAuthClient: ensure_authenticated()
    OAuthClient->>OAuthClient: device_flow_oauth()
    OAuthClient-->>SyncMgr: access_token

    SyncMgr->>DriveService: upload_file(archive)
    DriveService->>DriveService: create_or_find_folder("agentic-warden")
    DriveService->>DriveService: upload_with_metadata()
    DriveService-->>SyncMgr: file_id

    SyncMgr->>LocalFS: update_sync_state(file_id, timestamp)
```

#### 3. Cross-Process Task Coordination Flow

```mermaid
sequenceDiagram
    participant Process1[AI CLI Process 1]
    participant Process2[AI CLI Process 2]
    participant SharedMem
    participant WaitCmd[wait command]
    participant Registry

    Note over Process1,Process2: Multiple AI CLI processes running concurrently

    Process1->>SharedMem: register_task(pid1, task1)
    Process2->>SharedMem: register_task(pid2, task2)

    Note over SharedMem: Tasks isolated by root process namespace

    WaitCmd->>Registry: wait_for_all_tasks()
    Registry->>SharedMem: monitor_all_namespaces()

    loop Task Monitoring
        SharedMem-->>Registry: task_status_updates
        Registry->>Registry: check_all_completed()
    end

    Process1->>SharedMem: mark_completed(pid1)
    Process2->>SharedMem: mark_completed(pid2)

    Registry->>Registry: all_tasks_completed()
    Registry-->>WaitCmd: return_completion_report
```

---

## Technology Stack

### [v0] Current Technology Stack
**Version**: v0.1.0+

#### Core Platform
- **Language**: Rust 2021 Edition (1.70+)
- **Async Runtime**: tokio 1.0+ (rt-multi-thread, macros, signal, process)
- **Error Handling**: thiserror 1.0+ + custom error categories

#### User Interface
- **TUI Framework**: ratatui 0.26+ (all-widgets, serde)
- **Terminal Control**: crossterm 0.28+ (event-stream)
- **Graphics**: plotters 0.3 + plotters-bitmap 0.3
- **Color Themes**: color-eyre 0.6

#### Data & Serialization
- **Serialization**: serde 1.0+ (derive) + serde_json 1.0+
- **Configuration**: config 0.14+ (toml, json, yaml) + confy 0.6
- **Binary Formats**: bincode 1.3 + rmp-serde 1.1 (MessagePack)

#### Inter-Process Communication
- **Shared Memory**: shared_memory 0.12 + shared_hashmap 0.1.2
- **Synchronization**: raw_sync 0.1.5 + parking_lot 0.12
- **Process Management**: tokio::process + platform-specific APIs

#### Platform Abstraction
- **Windows**: windows 0.54 (Win32 APIs) + sysinfo 0.32
- **Unix**: psutil 3.2 + nix 0.29 (signal, process) + libc 0.2

#### External Integrations
- **HTTP Client**: reqwest 0.12+ (json, multipart, cookies)
- **Google APIs**: yup-oauth2 8.3 + mime_guess 2.0
- **MCP Protocol**: rmcp 0.5 (server, transport-io, macros)
- **File Operations**: tar 0.4 + flate2 1.0 + walkdir 2.5

#### Development & Testing
- **CLI Parsing**: clap 4.4+ (derive)
- **Testing**: mockall 0.12 + tokio-test 0.4 + wiremock 0.6
- **Logging**: tracing 0.1 + tracing-subscriber 0.3

---

## Architecture Decision Records (ADR)

### ARCH-001: Shared Memory-Based Task Coordination
**Date**: 2025-11-08
**Status**: 🟢 Adopted
**Version**: v0.1.0
**Related Requirements**: REQ-001, REQ-005

#### Background
Agentic-Warden needs to coordinate tasks across multiple AI CLI processes while maintaining isolation and preventing interference. Traditional approaches like process polling or HTTP APIs would introduce latency and complexity.

#### Decision
Use shared memory with namespacing for cross-process task coordination and real-time status tracking.

#### Options Compared
| Approach | Pros | Cons | Performance |
|----------|------|------|-------------|
| **Shared Memory** | <1ms operations, real-time updates, OS-level isolation | Manual cleanup required, platform-specific APIs | **Excellent** (< 1ms) |
| HTTP API | Cross-platform, easy debugging | Network latency, requires service management | Good (10-50ms) |
| File-based IPC | Simple implementation | Slow I/O, file locking complexity | Poor (100+ms) |
| Message Queue | Reliable delivery, scalability | External dependency, complexity | Good (5-20ms) |

#### Rationale
- **Performance Critical**: Task status updates need to be real-time for `wait` command responsiveness
- **Process Isolation**: Shared memory provides OS-level isolation between different AI CLI root processes
- **No External Dependencies**: Self-contained solution doesn't require additional services
- **Cross-Platform**: All target platforms (Windows/Linux/macOS) support shared memory

#### Impact
- **Performance**: Sub-millisecond task status updates and coordination
- **Complexity**: Increased implementation complexity for shared memory management
- **Reliability**: Requires robust cleanup mechanisms to prevent memory leaks
- **Testing**: Complex to test due to concurrency and platform-specific behavior

#### Implementation Details
```rust
// Namespace isolation by AI CLI root process
pub const NAMESPACE_FORMAT: &str = "agentic-warden-{pid}_task";

// Shared memory layout
pub struct TaskRecord {
    pub pid: u32,
    pub root_parent_pid: u32,
    pub prompt: String,
    pub status: TaskStatus,
    pub start_time: DateTime<Utc>,
    pub provider: Option<String>,
}

// Access pattern
pub enum TaskStatus {
    Running,
    CompletedButUnread,
    Completed,
}
```

---

### ARCH-002: Environment Variable Injection for Provider Management
**Date**: 2025-11-08
**Status**: 🟢 Adopted
**Version**: v0.1.0
**Related Requirements**: REQ-002

#### Background
Users need to switch between different API providers (OpenRouter, LiteLLM, official APIs) without modifying AI CLI native configuration files. The solution must be transparent to AI CLI processes and support dynamic switching.

#### Decision
Use environment variable injection at process startup time to dynamically configure AI CLI processes with provider-specific settings.

#### Options Compared
| Approach | Pros | Cons | User Experience |
|----------|------|------|-----------------|
| **Env Variable Injection** | Transparent to AI CLI, no config file modification, dynamic switching | Process-level only, requires restart | **Excellent** (seamless) |
| Config File Patching | Persistent changes, works with restarts | Risk of corrupting native configs, conflicts | Poor (destructive) |
| Proxy Service | Language-agnostic, centralized control | Network dependency, latency | Good (transparent) |
| CLI Wrapper Scripts | Simple implementation | Maintenance overhead, platform differences | Fair (manual) |

#### Rationale
- **Non-Invasive**: Doesn't modify AI CLI native configuration files
- **Dynamic**: Support `-p` parameter for per-execution provider selection
- **Universal**: Works across different AI CLI tools (codex, claude, gemini)
- **Secure**: No need to store sensitive API keys in additional locations

#### Implementation Details
```rust
// Provider configuration structure (v0.2.0 enhanced)
pub struct Provider {
    pub token: Option<String>,           // Optional API token
    pub base_url: Option<String>,        // Optional base URL
    pub scenario: Option<String>,        // [v0.2.0] Usage scenario description
    pub env: HashMap<String, String>,    // Additional environment variables
}

impl Provider {
    // [v0.2.0] Dynamic ENV injection with auto-mapping
    pub fn get_all_env_vars(&self) -> HashMap<String, String> {
        let mut env = self.env.clone();

        // Auto-map token to standard env vars
        if let Some(token) = &self.token {
            env.entry("ANTHROPIC_API_KEY".to_string())
               .or_insert(token.clone());
        }

        // Auto-map base_url to standard env vars
        if let Some(base_url) = &self.base_url {
            env.entry("ANTHROPIC_BASE_URL".to_string())
               .or_insert(base_url.clone());
        }

        env
    }
}

// Injection process
impl EnvInjector {
    pub fn inject_to_command(cmd: &mut Command, provider: &Provider) {
        for (key, value) in provider.get_all_env_vars() {
            cmd.env(key, value);  // Direct environment variable setting
        }
    }
}
```

#### Impact
- **User Experience**: Seamless provider switching with `-p` flag
- **Security**: API keys remain in provider configuration, not scattered in environment
- **Compatibility**: Works with any CLI tool that respects environment variables
- **Maintenance**: No need to track AI CLI configuration file format changes

---

### ARCH-003: Google Drive Integration with OAuth 2.0 Device Flow
**Date**: 2025-11-08
**Status**: 🟢 Adopted
**Version**: v0.1.0
**Related Requirements**: REQ-003

#### Background
Users need to backup and restore AI CLI configurations across devices. The solution must work in headless environments and support automated workflows without requiring browser interactions.

#### Decision
Use Google Drive API with OAuth 2.0 Device Flow (RFC 8628) for secure, browser-optional authentication and file operations.

#### Options Compared
| Approach | Pros | Cons | Headless Support |
|----------|------|------|------------------|
| **OAuth Device Flow** | Works in headless, standard security, no browser required | Manual copy-paste of codes | **Excellent** |
| OAuth Implicit Flow | Simpler implementation | Requires browser, security concerns | Poor |
| Service Account Keys | Fully automated | Complex setup, key management overhead | Good |
| Local Storage Only | Simple, no external deps | No cross-device sync | Not Applicable |

#### Rationale
- **Headless Compatibility**: Device flow works in SSH sessions, CI/CD, and servers
- **Security**: Standard OAuth 2.0 without storing sensitive credentials
- **User Control**: Users explicitly authorize access to specific folders
- **Automatic**: Once authorized, subsequent operations are fully automated

#### Implementation Details
```rust
// OAuth Device Flow
pub struct OAuthClient {
    pub client_id: String,
    pub auth_scope: String,
}

pub async fn device_flow_auth() -> Result<TokenInfo> {
    // 1. Request device code
    let device_code = get_device_code().await?;

    // 2. Display user instructions
    println!("Please visit: {}", device_code.verification_url);
    println!("Enter code: {}", device_code.user_code);

    // 3. Poll for token completion
    let token = poll_for_token(device_code.device_code).await?;
    Ok(token)
}
```

#### Impact
- **Security**: Industry-standard OAuth with token refresh support
- **Usability**: One-time setup, automatic thereafter
- **Compatibility**: Works across all target platforms
- **Infrastructure**: No additional server requirements

---

### ARCH-004: Selective Configuration Packing Strategy
**Date**: 2025-11-08
**Status**: 🟢 Adopted
**Version**: v0.1.0
**Related Requirements**: REQ-003

#### Background
AI CLI configurations contain a mix of essential settings, temporary files, and user-specific data. Backup must include important configuration while excluding unnecessary files to reduce size and avoid conflicts.

#### Decision
Implement selective file packing that includes only essential configuration files and explicitly excludes temporary, cache, and user-specific content.

#### Options Compared
| Approach | Pros | Cons | Backup Size |
|----------|------|------|-------------|
| **Selective Packing** | Small backup size, no conflicts, targeted | Requires maintenance of file lists | **Optimal** (~1-5MB) |
| Full Directory Backup | Complete, simple implementation | Large size, conflict risk, includes junk | Poor (100MB+) |
| User Configuration Files | Simple, respects user choices | Incomplete backup, missing essential files | Unreliable |

#### Rationale
- **Efficiency**: Reduces backup size by excluding unnecessary files
- **Reliability**: Avoids conflicts from temporary files and caches
- **Portability**: Ensures backups don't contain machine-specific data
- **Maintainability**: Explicit file lists serve as documentation

#### Implementation Details
```rust
// File inclusion strategy
impl ConfigPacker {
    // Claude: CLAUDE.md, settings.json, agents/, skills/SKILL.md
    fn pack_claude_configs(&self) -> Result<Option<(usize, u64)>> {
        let files_to_pack = [
            ("CLAUDE.md", "Main memory file"),
            ("settings.json", "Main configuration"),
        ];
        // Pack agents/ directory and SKILL.md files selectively
    }

    // Codex: auth.json, config.toml, agents.md, history.jsonl
    // Gemini: google_accounts.json, settings.json, gemini.md
}
```

#### Impact
- **Performance**: Faster upload/download due to smaller archives
- **Reliability**: Reduced risk of conflicts from machine-specific files
- **Storage**: Efficient use of Google Drive quota
- **Maintenance**: Clear documentation of essential vs. non-essential files

---

## Module Structure

### [v0] Current Module Architecture

#### Three Core System Modules

AI WARDEN的核心系统由三大功能模块组成，每个模块负责独立的业务领域：

##### 1. External AI CLI Management Module
**Responsibility**: 外部AI CLI工具的启动、监控和管理
**Core Components**:
- Process Tracking (`src/core/process_tree.rs`) - AI CLI进程识别和隔离
- Provider Management (`src/provider/`) - 第三方API提供商配置
- Task Coordination (`src/storage/`, `src/registry/`) - 跨进程任务跟踪

**Key Functions**:
- AI CLI进程启动和生命周期管理
- 提供商配置管理和环境变量注入
- 任务状态监控和进程树跟踪
- 多AI CLI并发执行协调

##### 2. CC (Claude Code) Session Management Module
**Responsibility**: Claude Code用户会话历史的存储和语义搜索
**Core Components**:
- Conversation History Storage (`src/memory/history.rs`) - SahomeDB文件数据库
- Semantic Search (`src/memory/`) - 向量化会话检索
- MCP Tools - `search_history`, `get_session_todos`

**Key Features**:
- JSONL格式会话记录存储
- 基于session_id的会话分组管理
- 语义相似度搜索历史对话
- 工具使用记录和模式分析

##### 3. MCP Proxy Routing Module
**Responsibility**: 为AI助手提供智能MCP工具选择和路由服务
**Core Components**:
- MCP Tools Indexing (`src/mcp_routing/`) - MemVDB内存向量数据库
- Intelligent Routing (`src/mcp_routing/`) - LLM辅助工具选择决策
- RMCP Client Pool (`src/mcp_routing/`) - 动态MCP服务器连接管理

**Key Features**:
- 两阶段搜索：工具级→方法级精确匹配
- 智能聚类算法和相似度阈值配置
- 自动从MCP配置重建路由索引
- 统一的MCP接口对外提供服务

##### 4. Synchronization (`src/sync/`)
**Responsibility**: Google Drive integration and configuration backup/restore
**Dependencies**: Google Drive API, OAuth client, file system
**Key Components**:
- `GoogleDriveService` - Complete Drive API operations
- `ConfigPacker` - Selective configuration archive creation
- `OAuthClient` - OAuth 2.0 Device Flow implementation

##### 5. User Interface (`src/tui/`)
**Responsibility**: Terminal-based user interface with unified design system
**Dependencies**: ratatui, crossterm
**Key Components**:
- `DashboardScreen` - AI CLI status and task overview
- `ProviderManagementScreen` - Provider configuration interface
- `ProgressScreen` - Sync operation progress display

#### Integration Points

##### 1. Supervisor Integration (`src/supervisor.rs`)
**Central coordination hub that integrates:**
- Process tracking for AI CLI identification
- Provider management for environment injection
- Task registry for lifecycle management
- Signal handling for graceful termination

##### 2. MCP Server (`src/mcp.rs`)
**External integration point providing:**
- Process monitoring tools
- Task status queries
- Provider configuration access
- AI CLI launch capabilities

---

## Security Architecture

### [v0] Security Measures

#### Authentication & Authorization
- **OAuth 2.0**: Standard Google Drive authentication with Device Flow
- **Token Management**: Secure token storage with automatic refresh
- **Scope Limitation**: Minimal Google Drive scopes for configuration backup only

#### Data Protection
- **Configuration Files**: Restricted permissions (600) on provider configurations
- **Shared Memory**: Namespace isolation prevents cross-process data leakage
- **API Keys**: Stored encrypted at rest, never logged

#### Process Isolation
- **Namespace Separation**: Each AI CLI root process has isolated shared memory namespace
- **Privilege Separation**: No unnecessary privileges requested
- **Signal Handling**: Clean process termination without resource leaks

#### Network Security
- **HTTPS Only**: All external communications use TLS
- **Certificate Validation**: Proper certificate chain verification
- **Proxy Support**: Secure proxy configuration for enterprise environments

---

## Performance Architecture

### [v0] Performance Characteristics

#### Benchmarks & Metrics
- **Process Detection**: < 100ms for 100 processes
- **Task Registration**: < 1ms per operation (shared memory)
- **TUI Rendering**: < 16ms per frame (60 FPS)
- **Configuration Sync**: 1-5MB archives, 10-30s typical sync

#### Scalability Considerations
- **Concurrent Tasks**: Supports 50+ concurrent AI CLI processes
- **Memory Usage**: < 50MB baseline + shared memory for task tracking
- **Storage Growth**: Linear with active tasks, automatic cleanup
- **Network Usage**: Minimal, only during configuration sync operations

#### Optimization Strategies
- **Caching**: Process tree detection results cached for 5 seconds
- **Batching**: Shared memory operations batched to reduce syscalls
- **Lazy Loading**: TUI components loaded on-demand
- **Compression**: GZIP compression for configuration archives

---

## Deployment Architecture

### [v0] Deployment Model

#### Installation Methods
- **Cargo Install**: `cargo install agentic-warden` (crates.io)
- **Binary Release**: Pre-compiled binaries for Windows/Linux/macOS
- **Package Managers**: Homebrew, Scoop, AUR community packages

#### Configuration Management
- **User Directory**: `~/.aiw/` for persistent configuration
- **Runtime Directory**: System temp directory for temporary files
- **Shared Memory**: OS-managed shared memory segments
- **No System Dependencies**: Self-contained, no root privileges required

#### Upgrade Strategy
- **Backward Compatibility**: Configuration format versioning with migration
- **Graceful Migration**: Automatic backup during major version upgrades
- **Rollback Support**: Configuration archives for rollback scenarios

---

### ARCH-010: Claude Code会话历史Hook集成架构
**Date**: 2025-11-14
**Status**: 🟢 Done
**Version**: v0.2.0
**Related Requirements**: REQ-010

#### Background
Claude Code provides hooks mechanism for session lifecycle events. We need to capture conversation history automatically without manual CLI commands, enabling seamless semantic search via MCP tools.

#### Decision
Use Claude Code's `SessionEnd` and `PreCompact` hooks to trigger automatic conversation history ingestion into vector database.

#### Options Compared
| Approach | Pros | Cons | User Experience |
|----------|------|------|-----------------|
| **Hook-based (Selected)** | Automatic, zero user effort, real-time | Requires Claude Code setup | **Excellent** (invisible) |
| Manual CLI import | Simple, no dependencies | User must remember to run | Poor (friction) |
| File watcher | Automatic detection | Complex, resource-heavy | Good (automatic) |
| Periodic cron job | Scheduled, reliable | Delayed ingestion, overhead | Fair (not real-time) |

#### Rationale
- **Zero Friction**: Users configure hooks once, then forget about it
- **Real-Time**: Conversations indexed immediately after session ends
- **Native Integration**: Leverages Claude Code's official hook mechanism
- **Session Context**: Hook provides session_id directly from stdin
- **Idempotent**: Can re-run on same session without duplicates

#### Architecture Diagram

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant Hook as agentic-warden hooks handle
    participant Parser as JSONL Parser
    participant Embed as FastEmbed
    participant DB as SahomeDB

    Note over CC: User ends session or<br/>PreCompact triggered

    CC->>Hook: Execute hook (stdio)
    Note over CC,Hook: stdin: {session_id, transcript_path, hook_event_name}

    Hook->>Hook: Read stdin JSON
    Hook->>Hook: Extract session_id & transcript_path

    Hook->>Parser: Parse JSONL file
    Parser->>Parser: Read ~/.claude/sessions/xxx.jsonl
    Parser->>Parser: Parse each line<br/>(role, content, timestamp)
    Parser-->>Hook: Vec<ConversationMessage>

    Hook->>DB: Check if session_id exists
    DB-->>Hook: false (new session)

    Hook->>Embed: Generate embeddings(batch)
    Note over Embed: AllMiniLML6V2<br/>384-dim<br/>Local, no network
    Embed-->>Hook: Vec<Vec<f32>>

    Hook->>DB: Insert batch with metadata
    Note over DB: session_id (from stdin)<br/>timestamp, role, content

    DB-->>Hook: Success

    Hook->>CC: Exit code 0
    Note over Hook,CC: stdout: "Indexed N messages"
```

#### Component Design

```rust
// Hook input from Claude Code (stdin)
#[derive(Deserialize)]
struct ClaudeCodeHookInput {
    session_id: String,           // From hook stdin
    transcript_path: String,      // JSONL file path
    hook_event_name: String,      // "SessionEnd" | "PreCompact"
    cwd: Option<String>,
    permission_mode: Option<String>,
}

// JSONL parser for Claude Code format
pub struct ClaudeCodeTranscriptParser;

impl ClaudeCodeTranscriptParser {
    pub fn parse_file(path: &Path) -> Result<Vec<ConversationMessage>> {
        // Stream parse JSONL
        // Extract role, content, timestamp from each line
    }
}

// Hook handler orchestrator
pub struct HookHandler {
    parser: ClaudeCodeTranscriptParser,
    embedder: FastEmbedGenerator,
    store: ConversationHistoryStore,
}

impl HookHandler {
    pub async fn handle_from_stdin() -> Result<()> {
        // 1. Read stdin JSON
        let input: ClaudeCodeHookInput = serde_json::from_reader(std::io::stdin())?;

        // 2. Check if already processed
        if self.store.has_session(&input.session_id).await? {
            eprintln!("Session {} already indexed, skipping", input.session_id);
            return Ok(());
        }

        // 3. Parse JSONL transcript
        let messages = self.parser.parse_file(&input.transcript_path)?;

        // 4. Generate embeddings (batch of 10)
        let embeddings = self.embedder.generate_batch(
            messages.iter().map(|m| &m.content).collect()
        ).await?;

        // 5. Store with session_id from stdin
        for (msg, embedding) in messages.iter().zip(embeddings) {
            let record = ConversationRecord {
                id: Uuid::new_v4().to_string(),
                session_id: Some(input.session_id.clone()),  // From stdin!
                role: msg.role.clone(),
                content: msg.content.clone(),
                timestamp: msg.timestamp,
                tools_used: vec![],
            };
            self.store.append(record, embedding).await?;
        }

        println!("✅ Indexed {} messages for session {}", messages.len(), input.session_id);
        Ok(())
    }
}
```

#### Data Flow

| Stage | Component | Input | Output | Performance |
|-------|-----------|-------|--------|-------------|
| **Hook Trigger** | Claude Code | Session ends | Executes hook command | Instant |
| **Stdin Read** | Hook CLI | JSON from stdin | ClaudeCodeHookInput | < 1ms |
| **JSONL Parse** | Parser | transcript_path file | Vec<ConversationMessage> | 10ms per 1000 lines |
| **Dedup Check** | SahomeDB | session_id | boolean (exists?) | < 20ms |
| **Embedding** | FastEmbed | Batch of messages | Vec<Vec<f32>> (384-dim) | 10ms per 10 messages |
| **Vector Insert** | SahomeDB | Records + embeddings | Success | 5ms per 10 records |
| **Hook Exit** | Hook CLI | - | Exit code 0 | Instant |

**Total Time**: < 2s for typical session (100 messages)

#### Impact
- **User Experience**: Completely transparent, zero manual intervention
- **Data Freshness**: Conversations available for search immediately after session
- **Resource Usage**: Minimal (FastEmbed local, no network calls)
- **Reliability**: Idempotent design prevents duplicate entries
- **Integration**: Native Claude Code hooks, no polling or file watchers

---

## [v0] Intelligent MCP Routing Architecture

### ARCH-012: 智能MCP路由系统架构设计

#### System Context Integration

```mermaid
graph TB
    subgraph "External Users"
        MainAI[AI Assistant<br/>Claude/Gemini/Codex]
        ClaudeUser[Claude Code User]
    end

    subgraph "AI WARDEN - Module 3: MCP Proxy Routing"
        IntelligentRouter[Intelligent MCP Router]
        LLMEngine[LLM Decision Engine]
        FastEmbedRouter[FastEmbed - Router]
    end

    subgraph "AI WARDEN - Module 2: CC Session Management"
        ConvHistory[Conversation History Store]
        SearchEngine[Semantic Search Engine]
        FastEmbedCC[FastEmbed - CC]
    end

    subgraph "Vector Storage Layers"
        MemVDB[MemVDB In-Memory<br/>MCP Tools/Methods]
        SahomeDB[SahomeDB File Storage<br/>CC Conversation History]
    end

    subgraph "MCP Client Layer"
        RMCPClientPool[RMCP Client Pool]
        MCPServers[External MCP Servers]
    end

    subgraph "Configuration Layer"
        MCPConfig[MCP Configuration]
        ConfigValidator[Schema Validator]
    end

    %% MCP Routing Flow
    MainAI --> IntelligentRouter
    IntelligentRouter --> MemVDB
    IntelligentRouter --> LLMEngine
    IntelligentRouter --> FastEmbedRouter
    IntelligentRouter --> RMCPClientPool
    RMCPClientPool --> MCPServers
    FastEmbedRouter --> MemVDB

    %% CC Session Management Flow (Independent)
    ClaudeUser --> ConvHistory
    ClaudeUser --> SearchEngine
    ConvHistory --> SahomeDB
    SearchEngine --> FastEmbedCC
    FastEmbedCC --> SahomeDB

    %% Configuration
    MCPConfig --> ConfigValidator
    ConfigValidator --> IntelligentRouter
```

#### Module Independence

**重要说明**: CC会话管理模块与MCP路由模块在功能上完全独立：

- **CC会话管理**: 服务于Claude Code用户，存储和检索历史对话
- **MCP路由模块**: 服务于AI助手，智能选择和调用MCP工具
- **无直接依赖**: 两个模块使用独立的向量存储和嵌入服务
- **独立数据流**: 各自有不同的服务对象和数据用途

#### Component Architecture Details

##### 1. Intelligent MCP Router (Module 3 Core Component)
- **Purpose**: Meta-MCP gateway with dynamic tool registration architecture
- **Module 3提供的MCP工具**: `intelligent_route` (智能路由工具选择和动态注册)
- **Note**: `search_history`工具由Module 2提供,与此组件独立(详见前文"MCP Server对外接口"章节)
- **Key Mechanism**: Leverages Claude Code's automatic `list_tools` refresh (< 1s before each tool use)
- **Internal Components**:
  - Vector search engine (FastEmbed + MemVDB)
  - LLM decision engine (Ollama)
  - DynamicToolManager (thread-safe global tools registry)
  - RMCP client pool (proxy to target MCP servers)

##### 2. Dual-Mode Vector Database Layer
- **MemVDB (In-Memory)**:
  - Collections: `mcp_tools`, `mcp_methods`
  - Purpose: Fast MCP routing index, rebuilt on startup from .mcp.json
  - Features: Thread-safe, cosine similarity, batch operations
  - Lifecycle: Memory-only, destroyed on shutdown
  - Rebuild: Automatically reconstructed from MCP configuration

- **SahomeDB (File-based Persistent)**:
  - Collections: `conversation_history`
  - Purpose: Claude Code conversation history storage and semantic search
  - Features: Persistent file storage, zero external dependencies, semantic search
  - Integration: New conversation history management module
  - Data: Session metadata, conversation context, tool usage patterns

##### 3. RMCP Client Connection Pool
- **Purpose**: Dynamic MCP server lifecycle management
- **Features**: Health monitoring, auto-reconnection, concurrent operations
- **Isolation**: Proper process isolation and resource management
- **Discovery**: Automatic tool schema discovery and caching

##### 4. LLM Decision Engine
- **Purpose**: Intelligent tool/method selection using semantic understanding
- **Integration**: Ollama service with configurable endpoints
- **Models**: qwen2.5:7b (default), configurable via environment
- **Capabilities**: Clustering analysis, ambiguity handling, confidence scoring

#### Data Flow Architecture (Dynamic Tool Registration)

```mermaid
sequenceDiagram
    participant ClaudeCode as Claude Code
    participant Router as MCP Router
    participant MemVDB
    participant LLM as Ollama LLM
    participant DynMgr as DynamicToolManager
    participant RMCP as RMCP Client Pool
    participant TargetMCP as Target MCP Server

    Note over ClaudeCode: User: "Check git status"

    ClaudeCode->>Router: intelligent_route("check git status")
    Router->>MemVDB: semantic_search(tools)
    MemVDB-->>Router: candidates[git_status, git_log, ...]
    Router->>LLM: decide_best_tool(candidates, request)
    LLM-->>Router: {tool: "git_status", confidence: 0.95}
    Router->>DynMgr: register_tool("git_status", schema)
    DynMgr-->>Router: registered=true
    Router-->>ClaudeCode: {selected_tool: "git_status", registered: true}

    Note over ClaudeCode: Auto calls list_tools (< 1s)
    ClaudeCode->>Router: list_tools()
    Router->>DynMgr: get_all_tools()
    DynMgr-->>Router: [intelligent_route, search_history, git_status]
    Router-->>ClaudeCode: tools with full schemas

    Note over ClaudeCode: Sees git_status, calls it
    ClaudeCode->>Router: git_status({path: "."})
    Router->>DynMgr: lookup("git_status")
    DynMgr-->>Router: {mcp_server: "git-server"}
    Router->>RMCP: proxy_call("git-server", "git_status", params)
    RMCP->>TargetMCP: git_status({path: "."})
    TargetMCP-->>RMCP: "On branch main\n..."
    RMCP-->>Router: result
    Router-->>ClaudeCode: "On branch main\n..."
```

#### Technology Stack Integration

##### New Dependencies for ARCH-012:
- `fastembed` = "4.0.0" # Local text embedding generation
- `memvdb` = "0.1.1" # In-memory vector database for MCP routing (pure Rust, zero deps)
- `sahomedb` = "0.4.0" # File-based vector database for conversation history
- `rmcp` = { version = "0.5", features = ["client", "server", "transport-io", "transport-child-process", "macros"] } # MCP client functionality
- `ollama-rs` = "0.3.1" # LLM communication (retained for tool selection decisions)
- `ndarray` = { version = "0.15", features = ["serde"] } # Vector calculations (FastEmbed dependency)

##### Existing Component Integration:
- **Memory Module**: Refactored to use FastEmbed embeddings + SahomeDB for conversation history
- **Configuration System**: Extends .mcp.json validation and management
- **Process Supervisor**: Integrates with MCP server lifecycle management
- **Embedding Service**: Replaced Ollama-based embedding with FastEmbed local generation

#### Performance Architecture

##### Key Performance Targets:
- **Tool Discovery**: < 50ms for typical semantic queries (MemVDB + FastEmbed)
- **Method Routing**: < 200ms end-to-end including LLM decisions
- **Embedding Generation**: < 30ms local (FastEmbed vs 200-500ms network)
- **Vector Search**: < 10ms for MemVDB operations, < 150ms for SahomeDB
- **Conversation History Search**: < 200ms for semantic queries
- **MCP Connections**: Support 10+ concurrent client connections
- **Memory Usage**: < 50MB for MemVDB index, < 200MB for SahomeDB storage
- **Startup Time**: < 500ms for MemVDB index reconstruction from .mcp.json

##### Scalability Considerations:
- **Horizontal Scaling**: Multiple MCP server connections
- **Memory Management**: Efficient MemVDB data structures, automatic cleanup
- **Caching Strategy**: Route result caching with TTL-based invalidation
- **Load Balancing**: Connection pool distribution and health-based routing

---

### ARCH-013: 动态JS编排工具系统架构
**Date**: 2025-11-15
**Status**: 🟢 Adopted
**Version**: v0.2.0
**Related Requirements**: REQ-013, ARCH-012

#### Background

intelligent_route当前通过向量搜索选择单个MCP工具,对于复杂多步骤任务效率低下。通过引入Boa JS引擎和LLM驱动的代码生成,我们可以动态创建组合多个MCP工具的编排函数,一次调用完成复杂工作流。

#### Decision

使用DynamicToolRegistry作为MCP工具定义的SSOT,配合Boa JS引擎和LLM代码生成能力,实现intelligent_route的双模式路由(LLM编排 vs 向量搜索)。

#### Architecture Overview

```mermaid
graph TB
    subgraph "Claude Code Client"
        User[用户请求]
    end

    subgraph "MCP Protocol Layer"
        ListTools[list_tools<br/>读取Registry]
        ToolCall[tools/call<br/>查找&执行]
    end

    subgraph "DynamicToolRegistry (SSOT)"
        BaseTools[Base Tools<br/>永久]
        DynamicTools[Dynamic Tools<br/>TTL=600s]

        BaseTools --> IT[intelligent_route]
        BaseTools --> SH[search_history]

        DynamicTools --> JSTools[JS编排工具]
        DynamicTools --> ProxyTools[代理工具]
    end

    subgraph "intelligent_route LLM优先路由"
        Router{LLM<br/>环境?}
        TryLLM[尝试LLM编排]
        LLMSuccess{成功?}
        VectorFallback[Vector Fallback]
    end

    subgraph "LLM编排组件"
        LLMPlan[工作流规划器]
        LLMCodeGen[JS代码生成器]
        CodeValidator[代码验证器]
    end

    subgraph "向量搜索组件 (Fallback)"
        VectorSearch[向量搜索引擎]
        Cluster[聚类算法]
    end

    subgraph "Execution Layer"
        JSExec[JS执行器<br/>Boa Runtime Pool]
        ProxyExec[代理执行器<br/>RMCP Client Pool]
    end

    User --> ListTools
    User --> ToolCall

    ListTools --> DynamicTools
    ListTools --> BaseTools

    ToolCall --> IT
    IT --> Router

    Router -->|None<br/>直接fallback| VectorFallback
    Router -->|Some<br/>优先尝试| TryLLM

    TryLLM --> LLMPlan
    LLMPlan --> LLMCodeGen
    LLMCodeGen --> CodeValidator
    CodeValidator --> LLMSuccess

    LLMSuccess -->|成功| JSTools
    LLMSuccess -->|失败| VectorFallback

    VectorFallback --> VectorSearch
    VectorSearch --> Cluster
    Cluster --> ProxyTools

    JSTools --> JSExec
    ProxyTools --> ProxyExec
```

#### Core Components Design

##### 1. DynamicToolRegistry

**数据结构**:
```rust
pub struct DynamicToolRegistry {
    // 基础工具(启动时初始化,永久存在)
    base_tools: HashMap<String, BaseToolDefinition>,

    // 动态工具(运行时注册,带TTL)
    dynamic_tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,

    config: RegistryConfig,
}

pub struct RegistryConfig {
    default_ttl_seconds: u64,      // 默认TTL = 600秒(10分钟)
    max_dynamic_tools: usize,       // 最大100个动态工具
    cleanup_interval_seconds: u64,  // 清理间隔60秒
}

pub enum RegisteredTool {
    JsOrchestrated(JsOrchestratedTool),  // JS编排工具
    ProxiedMcp(ProxiedMcpTool),          // 代理MCP工具
}
```

**关键操作**:
- `register_js_tool()`: 注册JS编排工具
- `register_proxied_tools()`: 批量注册代理工具
- `get_all_tool_definitions()`: list_tools读取所有工具
- `get_tool()`: tools/call查找工具定义
- `cleanup_expired_tools()`: 后台清理过期工具

**TTL管理**:
```rust
// 后台清理任务
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        registry.cleanup_expired_tools().await;
    }
});
```

**关键架构设计 - base_tools vs dynamic_tools双层结构**:

**设计理念**:
- **base_tools (永久工具)**: 来自.mcp.json配置文件定义的MCP服务器工具,启动时通过warm_up()一次性扫描并永久驻留内存,无TTL限制
- **dynamic_tools (临时工具)**: 运行时LLM动态生成的JS编排工具,带TTL=600秒,最多100个,LRU驱逐策略

**数据结构优化**:
```rust
pub struct DynamicToolRegistry {
    // 永久工具 (来自.mcp.json)
    base_tools: HashMap<String, BaseToolDefinition>,
    base_snapshot: Arc<Vec<Tool>>,  // ✅ Arc共享,避免重复clone

    // 临时工具 (LLM运行时生成)
    dynamic_tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,  // ✅ TTL管理

    // list_tools缓存
    tool_cache: Arc<RwLock<Option<Arc<Vec<Tool>>>>>,  // ✅ Arc嵌套,零拷贝
}
```

**启动时构建流程 (仅一次)**:
```rust
// src/mcp_routing/mod.rs:100-105
pub async fn initialize() -> Result<Self> {
    // 1. 一次性warm_up所有MCP服务器
    let discovered = connection_pool.warm_up().await?;  // ✅ 扫描.mcp.json

    // 2. 构建向量索引 (MemVDB内存数据库)
    let embeddings = build_embeddings(&embedder, &discovered, config)?;
    index.rebuild(&embeddings.tools, &embeddings.methods)?;  // ✅ <500ms启动

    // 3. 填充永久工具注册表
    populate_registry(&tool_registry, discovered).await;  // ✅ base_tools固化

    // 4. 创建动态工具注册表(初始为空)
    let dynamic_registry = Arc::new(DynamicToolRegistry::new(Vec::new()));
}
```

**list_tools性能优化 (Arc共享)**:
```rust
// src/mcp_routing/registry.rs:331-352
pub async fn get_all_tool_definitions(&self) -> Arc<Vec<Tool>> {
    // 缓存命中: 直接返回Arc指针,零拷贝
    if let Some(cached) = self.tool_cache.read().await.clone() {
        return cached;  // ✅ Arc clone只复制指针,<1μs
    }

    // 缓存失效: 快速重建
    let mut snapshot = Vec::new();
    snapshot.extend(self.base_snapshot.iter().cloned());  // ✅ Arc浅拷贝,<1ms

    let map = self.dynamic_tools.read().await;
    for entry in map.values() {
        snapshot.push(entry.tool().clone());  // ✅ 只clone动态工具(≤100个)
    }

    let arc_snapshot = Arc::new(snapshot);
    *self.tool_cache.write().await = Some(arc_snapshot.clone());
    arc_snapshot  // ✅ 返回Arc,后续list_tools直接复用
}
```

**架构优势总结**:

| 维度 | base_tools | dynamic_tools | 性能影响 |
|------|-----------|---------------|---------|
| **来源** | .mcp.json配置文件 | LLM运行时生成 | - |
| **生命周期** | 启动时构建,永久存在 | TTL=600s,自动过期 | 避免重启重新扫描 |
| **数量限制** | 无限制(取决于MCP服务器数量) | 最多100个,LRU驱逐 | 内存可控 |
| **存储方式** | Arc<Vec<Tool>>共享 | RwLock<HashMap>隔离 | list_tools零拷贝 |
| **向量索引** | 启动时一次性构建 | 不索引(无需搜索) | 启动<500ms |
| **缓存失效开销** | Arc浅拷贝 | clone动态工具 | <1ms重建 |

**性能基准**:
- **启动时间**: warm_up + 向量化 + 索引构建 ≈ **500ms** (500个base_tools)
- **list_tools响应**: 缓存命中 < **1μs**, 缓存失效重建 < **1ms**
- **内存占用**: base_tools (~30MB) + dynamic_tools (~5MB) + 向量索引 (~30MB) ≈ **65MB**

**未来优化方向**:
- [ ] **批量Embedding生成**: 启动时对base_tools批量向量化,从500ms降至200ms (40x加速)
  ```rust
  // 当前: 逐个生成 (500工具 × 20ms = 10s)
  for tool in tools {
      let vector = embedder.embed(&doc)?;
  }

  // 优化: 批量生成 (FastEmbed原生支持)
  let docs: Vec<String> = tools.iter().map(|tool| format_doc(tool)).collect();
  let vectors = embedder.embed_batch(&docs)?;  // 200ms for 500
  ```
- [ ] **MemRoutingIndex单元测试**: 当前测试覆盖率0%,需补充边界测试(维度不匹配、空索引、相似度排序)

##### 2. intelligent_route LLM优先路由 (带Fallback)

**路由决策逻辑**:
```rust
impl IntelligentRouter {
    pub async fn intelligent_route(
        &self,
        request: IntelligentRouteRequest,
    ) -> Result<IntelligentRouteResponse> {
        // 前置检查
        if request.user_request.trim().is_empty() {
            return Ok(IntelligentRouteResponse { success: false, ... });
        }

        let embed = self.embedder.embed(&request.user_request)?;

        // LLM优先策略
        match &self.js_orchestrator {
            None => {
                // LLM不存在 - 直接用vector，不尝试
                eprintln!("🔍 LLM not configured, using vector search mode");
                self.vector_mode(&request, &embed).await
            }
            Some(orchestrator) => {
                // LLM存在 - 优先尝试，失败则fallback
                eprintln!("🤖 Trying LLM orchestration mode...");
                match self.try_orchestrate(orchestrator, &request, &embed).await {
                    Ok(response) => {
                        eprintln!("✅ LLM orchestration succeeded");
                        Ok(response)
                    }
                    Err(err) => {
                        // LLM失败 - fallback到vector
                        eprintln!("⚠️  LLM failed: {}, falling back to vector mode", err);
                        self.vector_mode(&request, &embed).await
                    }
                }
            }
        }
    }
}
```

**LLM编排模式流程** (优先尝试):
```
1. 获取候选MCP工具(通过向量搜索)
2. LLM规划工作流 → {is_feasible, steps, input_params}
3. 不可行? → 返回Err触发fallback
4. 可行? → LLM生成JS函数代码
5. 验证JS代码(语法+安全性)
6. 验证失败? → 返回Err触发fallback
7. 验证通过 → 注册到Registry为单一JS编排工具
8. 返回: "Use the 'xxx' tool to solve your problem"
```

**向量搜索模式流程** (Fallback保障):
```
1. 两层向量搜索(工具级+方法级)
2. 聚类算法筛选top-5候选
3. 批量注册到Registry为代理工具(透传schema)
4. 返回: "Found 5 tools. Choose which ones to use: ..."
```

**Fallback触发条件**:
- `js_orchestrator = None` (LLM未配置)
- LLM网络请求超时或失败
- LLM返回无效响应
- JS代码验证失败(语法错误、安全检查未通过)
- LLM判断任务不可行

##### 3. Boa JS Engine Integration

**安全沙箱配置**:
```rust
pub struct BoaEngineConfig {
    max_execution_time_ms: u64,      // 30秒超时
    max_memory_mb: usize,             // 256MB内存限制
    max_call_stack_depth: usize,      // 128层调用栈
    disabled_globals: Vec<String>,    // 禁用eval, Function, etc.
}

impl SecureBoaRuntime {
    fn disable_dangerous_globals(ctx: &mut Context) -> Result<()> {
        let dangerous = ["eval", "Function", "require", "import",
                        "fetch", "XMLHttpRequest", "WebSocket"];
        for api in dangerous {
            ctx.eval(&format!("delete globalThis.{}", api))?;
        }
        Ok(())
    }
}
```

**MCP函数注入**:
```rust
pub struct McpFunctionInjector {
    rmcp_pool: Arc<RmcpClientPool>,
}

impl McpFunctionInjector {
    /// 注入 MCP 工具为 JS 异步函数(带缓存)
    pub fn inject_all(
        &self,
        context: &mut Context,
        tools: &[InjectedMcpFunction],
        handle: Handle,
    ) -> Result<()> {
        for tool in tools {
            let name = format!("mcp{}", to_camel_case(&tool.name));
            // 已注入的函数直接跳过，避免重复注册
            if context.global_object().has_property(name.clone(), context)? {
                continue;
            }

            let invoker = Arc::clone(&self.rmcp_pool);
            let server = tool.server.clone();
            let method = tool.name.clone();

            let native = NativeFunction::from_async(move |args, ctx| {
                let request = args_to_json(args, ctx)?;
                let invoker = Arc::clone(&invoker);
                let server = server.clone();
                let method = method.clone();
                handle.spawn(async move {
                    invoker.call_tool(&server, &method, request).await
                })
            });

            context.register_global_property(name, native, Attribute::all())?;
        }
        Ok(())
    }
}
```

**运行时池**:
```rust
pub struct BoaRuntimePool {
    pool: deadpool::managed::Pool<BoaRuntimeManager>,
    config: BoaEngineConfig,
}

impl BoaRuntimePool {
    const MIN_WARM_INSTANCES: usize = 5;

    pub async fn acquire(&self) -> Result<PooledBoaRuntime> {
        let runtime = self.pool.get().await?;
        Ok(PooledBoaRuntime { runtime })
    }

    pub async fn prime_minimum_runtimes(&self) -> Result<()> {
        // 启动时预热5个实例，避免首次调用冷启动延迟
        let mut guards = Vec::with_capacity(Self::MIN_WARM_INSTANCES);
        for _ in 0..Self::MIN_WARM_INSTANCES {
            guards.push(self.pool.get().await?);
        }
        drop(guards);
        Ok(())
    }
}
```

##### 4. LLM-Driven Code Generation

**工作流规划Prompt**:
```rust
fn build_planning_prompt(user_request: &str, tools: &[McpToolInfo]) -> String {
    format!(r#"
## User Request: "{}"

## Available MCP Tools:
{}

## Task:
1. Analyze if request can be accomplished
2. If YES: Plan steps and required tools
3. If NO: Explain why

## Output JSON:
{{
  "is_feasible": true/false,
  "reason": "...",
  "steps": [{{"step": 1, "tool": "git_diff", "description": "..."}}],
  "required_input_params": [{{"name": "pr_id", "type": "number", "description": "..."}}],
  "tool_name_suggestion": "review_pr_workflow"
}}
    "#, user_request, format_tools(tools))
}
```

**JS代码生成Prompt**:
```rust
fn build_codegen_prompt(plan: &WorkflowPlan) -> String {
    format!(r#"
## Workflow Plan:
{}

## Generate async function workflow(input) {{...}}
- Use injected MCP functions: mcp{}()
- Access params via input.paramName
- Include try-catch error handling
- Return structured result

Output only JavaScript code.
    "#, serde_json::to_string_pretty(&plan.steps))
}
```

**代码验证**:
```rust
pub struct JsCodeValidator;

impl JsCodeValidator {
    pub fn validate(&self, code: &str) -> Result<()> {
        // 1. 语法检查(Boa解析)
        let _ = boa_engine::Context::default().eval(code)?;

        // 2. 危险模式检测
        let dangerous_patterns = [
            r"eval\s*\(", r"new\s+Function\s*\(",
            r"__proto__", r"constructor\.constructor",
        ];
        for pattern in dangerous_patterns {
            if regex::Regex::new(pattern)?.is_match(code) {
                return Err(anyhow!("Dangerous pattern: {}", pattern));
            }
        }

        Ok(())
    }
}
```

#### MCP Protocol Integration

**list_tools响应**:
```rust
impl McpServer {
    pub async fn handle_list_tools(&self) -> Result<ListToolsResponse> {
        // Registry内部缓存Arc<Vec<Tool>>，保证list_tools < 50ms
        let snapshot = self.registry.get_all_tool_definitions().await?;
        Ok(ListToolsResponse {
            tools: snapshot.as_ref().clone(),
        })
    }
}
```

**tools/call路由**:
```rust
impl McpServer {
    pub async fn handle_tool_call(&self, request: ToolCallRequest) -> Result<ToolCallResponse> {
        match request.name.as_str() {
            "intelligent_route" => self.intelligent_router.handle(request.arguments).await,
            "search_history" => self.search_history.handle(request.arguments).await,

            // 动态工具
            _ => {
                let tool = self.registry.get_tool(&request.name).await?;
                match tool {
                    RegisteredTool::JsOrchestrated(js) => {
                        // BoaRuntimePool + MCP注入器执行JS编排工具
                        let report = self.js_executor.execute(js, request.arguments).await?;
                        Ok(ToolCallResponse::from(report))
                    }
                    RegisteredTool::ProxiedMcp(proxy) =>
                        self.proxy_executor.execute(proxy, request.arguments).await,
                }
            }
        }
    }
}
```

#### Data Flow Example

**模式A完整流程**:
```
1. Claude Code: intelligent_route({user_request: "Review PR and generate report"})
   ↓
2. intelligent_route检测LLM环境存在 → 模式A
   ↓
3. LLM规划:
   {
     is_feasible: true,
     steps: [
       {step:1, tool:"git_diff", description:"Get PR changes"},
       {step:2, tool:"read_file", description:"Read changed files"},
       {step:3, tool:"write_file", description:"Write report"}
     ],
     required_input_params: [
       {name:"base_branch", type:"string"},
       {name:"pr_branch", type:"string"}
     ],
     tool_name_suggestion: "review_pr_and_report"
   }
   ↓
4. LLM生成JS:
   async function workflow(input) {
     const diff = await mcpGitDiff({base: input.base_branch, head: input.pr_branch});
     const files = await mcpReadFile({paths: diff.files});
     const report = generateMarkdown(diff, files);
     await mcpWriteFile({path: "REVIEW.md", content: report});
     return {success: true, report_path: "REVIEW.md"};
   }
   ↓
5. 验证JS代码(语法+安全性) ✓
   ↓
6. Registry.register_js_tool({
     name: "review_pr_and_report",
     input_schema: {...},
     js_code: "...",
     ttl_seconds: 600
   })   // 同时刷新list_tools缓存
   // Note: mcp_dependencies已废弃，统一通过mcp.call()接口调用
   ↓
7. 返回: {message: "Use the 'review_pr_and_report' tool to solve your problem"}
   ↓
8. Claude Code刷新list_tools (< 1s)
   ↓
9. 看到新工具: review_pr_and_report
   ↓
10. Claude Code调用: review_pr_and_report({base_branch: "main", pr_branch: "feat"})
    ↓
11. Registry.get_tool("review_pr_and_report") → JsOrchestratedTool
    ↓
12. JsExecutor.execute:
    - 获取Boa运行时
    - 注入MCP函数(mcpGitDiff, mcpReadFile, mcpWriteFile)
    - 执行JS脚本
    - JS内部调用MCP函数 → RMCP Pool → 外部MCP服务器
    - 返回结果
    ↓
13. 返回给Claude Code
```

#### Impact Analysis

**优势**:
- ✅ **单一入口**: intelligent_route统一处理,用户体验一致
- ✅ **自动降级**: 无LLM环境时回退到向量搜索模式
- ✅ **工作流复用**: 生成的JS工具可在TTL内重复使用
- ✅ **灵活扩展**: 轻松添加新的工具类型(只需实现RegisteredTool)
- ✅ **性能优化**: 运行时池复用,减少初始化开销

**挑战**:
- ⚠️ **LLM质量依赖**: 代码生成质量取决于LLM能力
- ⚠️ **调试复杂度**: JS执行错误需要友好的错误信息
- ⚠️ **安全风险**: 必须严格验证生成的JS代码
- ⚠️ **TTL管理**: 过期工具清理需要合理的策略

**风险缓解**:
- Dry-run测试: 生成代码后先用mock数据测试
- 多层验证: 语法检查 + 安全检查 + 执行测试
- 详细日志: 记录所有工具注册/执行/清理事件
- 降级机制: JS执行失败时提供清晰的错误信息

#### Technology Stack

**新增依赖**:
```toml
boa_engine = "0.17"         # JavaScript引擎
boa_gc = "0.17"             # 垃圾回收
swc_ecma_parser = "0.142"   # JS解析器(验证)
swc_ecma_ast = "0.110"      # AST分析
deadpool = "0.10"           # 运行时池
regex = "1.10"              # 安全检查
```

**性能目标**:
- Registry读取: < 50ms
- LLM规划: < 3s
- JS代码生成: < 3s
- 代码验证: < 100ms
- Boa初始化: < 50ms
- MCP注入: < 200ms
- JS执行: < 30s(取决于MCP调用)
- 工具注册: < 10ms

---

### ARCH-014: AI CLI角色系统和任务生命周期架构
**Date**: 2025-11-16
**Status**: 🟡 Partial (Phase 1 ✅ Adopted, Phase 2-3 ⏸️ Planned)
**Version**: v0.2.0 (Phase 1), v0.3.0 (Phase 2-3)
**Related Requirements**: REQ-014

#### Background

Claude Code通过MCP管理AI CLI任务时,缺少对角色配置和任务生命周期的统一管理能力。用户需要重复输入角色提示词,且无法通过MCP工具启动/停止/查询后台AI CLI任务。

#### Decision

**Phase 1 (✅ v0.2.0 已实现)**: 实现基于文件的角色管理系统,提供`list_roles` MCP工具。

**Phase 2-3 (⏸️ v0.3.0 计划)**: 实现任务生命周期MCP工具(start_task, stop_task, list_tasks, get_task_logs),并集成角色系统到任务启动流程。

#### Architecture Components

##### Phase 1: Role Management (✅ Implemented)

**1. Role Storage Layer**:
```
~/.aiw/role/
├── backend-developer.md
├── frontend-expert.md
└── qa-tester.md

File Format:
<description>
------------
<content>
```

**2. Role Module (`src/roles/mod.rs`)**:
```rust
pub struct Role {
    pub name: String,
    pub description: String,
    pub content: String,
    pub file_path: PathBuf,
}

pub struct RoleInfo { // Lightweight for MCP
    pub name: String,
    pub description: String,
    pub file_path: String,
}

pub struct RoleManager {
    base_dir: PathBuf, // Default: ~/.aiw/role/
}

impl RoleManager {
    pub fn list_all_roles() -> RoleResult<Vec<Role>>;
    pub fn get_role(name: &str) -> RoleResult<Role>;
}
```

**3. Security Design**:
- **Path Traversal Prevention**: `fs::canonicalize()` + `starts_with()` validation
- **File Size Limit**: 1MB maximum per role file
- **Encoding Validation**: UTF-8 only, reject invalid encodings
- **Name Validation**: Block path separators (`/`, `\`, `..`)
- **Delimiter Validation**: Require exactly 12 dashes `------------`

**4. MCP Integration**:
```rust
#[tool(
    name = "list_roles",
    description = "List all available AI CLI role configurations"
)]
async fn list_roles_tool() -> Result<Json<Vec<RoleInfo>>, String> {
    let manager = RoleManager::new()?;
    let roles = manager.list_all_roles()?;
    Ok(Json(roles.into_iter().map(|r| r.as_info()).collect()))
}
```

**5. Error Handling**:
```rust
pub enum RoleError {
    NotFound(String),
    InvalidName { message: String },
    PathTraversal { path: String },
    FileTooLarge { path: String, size: u64 },
    InvalidEncoding { path: String },
    InvalidFormat { path: String, details: String },
    HomeDirectoryUnavailable,
    Io { path: String, source: io::Error },
}
```

##### Phase 2: Task Lifecycle MCP Tools (⏸️ Planned)

**1. Task Launching**:
```rust
// MCP Tool: start_task
#[tool(name = "start_task")]
async fn start_task_tool(params: StartTaskParams) -> Result<TaskLaunchResult> {
    // 1. Load role content if role parameter provided
    let prompt = if let Some(role_name) = params.role {
        let role = RoleManager::new()?.get_role(&role_name)?;
        format!("{}\n\n---\n\n{}", role.content, params.task)
    } else {
        params.task
    };

    // 2. Launch AI CLI via supervisor
    let child = supervisor::execute_cli(params.ai_type, &prompt, params.provider).await?;

    // 3. Register to MCP Registry (InProcessRegistry)
    let registry = create_mcp_registry();
    registry.register(child.id(), &task_record)?;

    Ok(TaskLaunchResult { pid, log_file, status })
}

struct StartTaskParams {
    ai_type: String,        // "claude" | "codex" | "gemini"
    task: String,           // User task description
    provider: Option<String>, // Optional provider override
    role: Option<String>,   // Optional role name
}
```

**2. Task Control**:
```rust
// MCP Tool: stop_task
#[tool(name = "stop_task")]
async fn stop_task_tool(params: StopTaskParams) -> Result<StopTaskResult> {
    let registry = create_mcp_registry();

    // Send SIGTERM, wait 5s, then SIGKILL
    kill_process_gracefully(params.pid, Duration::from_secs(5))?;

    // Remove from registry
    registry.mark_completed(params.pid, Some("Stopped by user"), None, Utc::now())?;

    Ok(StopTaskResult { success: true, message: format!("Task {} stopped", params.pid) })
}
```

**3. Task Query**:
```rust
// MCP Tool: list_tasks
#[tool(name = "list_tasks")]
async fn list_tasks_tool() -> Result<Json<Vec<TaskInfo>>> {
    let registry = create_mcp_registry();
    let entries = registry.entries()?;

    // Filter out zombie processes
    let active_tasks: Vec<TaskInfo> = entries.into_iter()
        .filter(|e| is_process_alive(e.pid))
        .map(|e| TaskInfo {
            pid: e.pid,
            ai_type: e.record.ai_type.clone(),
            task: e.record.task.clone(),
            status: e.record.status,
            start_time: e.record.start_time,
            log_file: e.record.log_file.clone(),
        })
        .collect();

    Ok(Json(active_tasks))
}
```

**4. Log Access**:
```rust
// MCP Tool: get_task_logs
#[tool(name = "get_task_logs")]
async fn get_task_logs_tool(params: GetTaskLogsParams) -> Result<GetTaskLogsResult> {
    let registry = create_mcp_registry();
    let entry = registry.get(params.pid).ok_or("Task not found")?;

    // Security: verify log_file belongs to this process
    validate_log_file_ownership(&entry.record.log_file, params.pid)?;

    // Read log file (with optional tail)
    let content = if let Some(n) = params.tail_lines {
        read_last_n_lines(&entry.record.log_file, n)?
    } else {
        fs::read_to_string(&entry.record.log_file)?
    };

    Ok(GetTaskLogsResult {
        log_content: content,
        log_file: entry.record.log_file,
    })
}
```

#### Data Flow

**Phase 1 - Role Listing**:
```
Claude Code → list_roles MCP call
    → RoleManager::list_all_roles()
    → Scan ~/.aiw/role/*.md
    → Parse each file (validate, split on delimiter)
    → Return Vec<RoleInfo>
```

**Phase 2 - Task with Role**:
```
Claude Code → start_task(ai_type="codex", task="Fix bug", role="backend-developer")
    → RoleManager::get_role("backend-developer")
    → Load role content: "You are an expert backend developer..."
    → Compose prompt: "{role_content}\n\n---\n\n{task}"
    → supervisor::execute_cli("codex", composed_prompt, provider)
    → Register PID to MCP Registry
    → Return {pid, log_file, status}
```

#### Performance Considerations

**Phase 1 (Role System)**:
- Role list caching: 可选,初次扫描后缓存,TTL 60s
- File size limit: 1MB防止大文件解析性能问题
- 目录扫描优化: WalkDir非递归,仅扫描顶层.md文件

**Phase 2 (Task Lifecycle)**:
- Task list query: O(1)从Registry读取,< 10ms
- Log file access: 流式读取,支持tail模式避免读取整个文件
- Process kill: 异步SIGTERM → SIGKILL,不阻塞MCP响应

#### Security Measures

**Role System**:
- ✅ Path traversal: Canonicalize + prefix check
- ✅ File size: 1MB max
- ✅ Encoding: UTF-8 only
- ✅ Delimiter: Required `------------`

**Task Lifecycle**:
- ⏸️ PID validation: Verify PID belongs to current user
- ⏸️ Log file ownership: Validate log path before reading
- ⏸️ Resource limits: Limit concurrent task launches
- ⏸️ Signal permissions: Check user can signal PID

#### Testing Strategy

**Phase 1 (✅ Implemented)**:
- Unit tests: `tests/roles_tests.rs` (5 tests)
  - Role file parsing with delimiter
  - list_all_roles returns all roles
  - File not found error handling
  - Path traversal rejection
  - File size limit enforcement

**Phase 2 (⏸️ Planned)**:
- Integration tests: `tests/task_lifecycle_tests.rs`
  - start_task launches process and returns PID
  - stop_task terminates process gracefully
  - list_tasks returns active tasks
  - get_task_logs reads log files
  - Role integration: start_task with role parameter

#### Implementation Files

**Phase 1 (✅ v0.2.0)**:
- `src/roles/mod.rs` (269 lines): Core role management
- `src/mcp/mod.rs:347-356`: MCP tool `list_roles`
- `src/lib.rs:25`: Module export
- `tests/roles_tests.rs` (96 lines): Unit tests

**Phase 2-3 (⏸️ v0.3.0 planned)**:
- `src/mcp/mod.rs`: Add start_task, stop_task, list_tasks, get_task_logs tools
- `src/roles/integration.rs`: Role injection into task prompts (planned)
- `tests/task_lifecycle_tests.rs`: Integration tests (planned)

---

## Deprecated Architecture Solutions

### Historical Decisions (Not applicable for v0)

*Note: This is the initial architecture version. Future deprecated solutions will be documented here when architectural changes are made.*
