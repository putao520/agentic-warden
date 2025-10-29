# AGENTIC WARDEN - 技术规范文档 v0.3.0

## 项目概述

**项目名称**: agentic-warden  
**项目类型**: Rust CLI 工具 (AI Agent 管理器)  
**版本**: v0.3.0  
**许可证**: MIT  

## 核心功能

### 1. 多AI Agent统一管理系统

agentic-warden 是一个统一的AI Agent管理器，支持：
- **Claude CLI** - Anthropic的Claude代码助手
- **Codex CLI** - OpenAI的Codex代码生成工具  
- **Gemini CLI** - Google的Gemini AI助手

### 2. 统一任务提示词接口

采用简化的任务提示词模式，用户无需了解各Agent的具体参数：

```bash
# 单个Agent调用
agentic-warden claude "写一段Rust代码"
agentic-warden codex "解释这个算法"
agentic-warden gemini "review this code"

# 批量Agent调用
agentic-warden all "review this rust code"
agentic-warden "claude|gemini" "explain this algorithm"
agentic-warden "codex|claude|gemini" "write documentation"
```

### 3. 进程树管理和任务追踪

- **共享内存任务注册表**: 4MiB共享内存空间，实时任务状态追踪
- **跨平台进程树管理**: Windows、Linux、macOS全平台支持
- **任务日志系统**: 每个任务独立日志文件，完整输出记录
- **等待模式**: `codex-warden wait` 监控和清理完成的任务

### 4. 配置同步系统

- **Google Drive集成**: 配置文件云端备份和同步
- **OAuth 2.0认证**: 安全的Google API访问
- **跨平台压缩**: TAR.GZ、ZIP、7Z格式支持
- **增量同步**: MD5哈希变化检测，仅同步变更文件

## 技术架构

### 模块结构

```
src/
├── main.rs                    # 主程序入口和CLI路由
├── cli_type.rs               # CLI类型解析和多CLI支持
├── supervisor.rs             # CLI进程管理和任务执行
├── registry.rs               # 共享内存任务注册表
├── process_tree.rs           # 跨平台进程树检测
├── wait_mode.rs              # 任务监控和清理
├── sync/                     # 配置同步模块
│   ├── sync_config_manager.rs
│   ├── google_drive_client.rs
│   ├── directory_hasher.rs
│   ├── config_packer.rs
│   └── compressor.rs
├── platform/                 # 平台特定代码
│   ├── windows.rs
│   └── unix.rs
└── [支持模块...]
```

### 核心数据结构

#### CLI选择器
```rust
pub struct CliSelector {
    pub types: Vec<CliType>,
}

impl CliSelector {
    pub fn all() -> Self;                    // 所有3个AI
    pub fn from_single(cli_type: CliType) -> Self;  // 单个AI
    pub fn from_multiple(types: Vec<CliType>) -> Self;  // 多个AI组合
}
```

#### 任务记录
```rust
pub struct TaskRecord {
    pub started_at: DateTime<Utc>,
    pub log_id: String,
    pub log_path: String,
    pub manager_pid: Option<u32>,
    pub status: TaskStatus,
    pub process_chain: Vec<u32>,
    // ... 其他字段
}
```

#### 配置同步
```rust
pub struct SyncConfig {
    pub config: SyncConfigConfig,
    pub state: SyncConfigState,
}

pub struct SyncConfigConfig {
    pub directories: Vec<String>,
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: u32,
}
```

## CLI命令格式

### Agent任务执行

#### 语法格式
```bash
agentic-warden <AGENT_SELECTOR> "<TASK_DESCRIPTION>"
```

#### Agent选择器
- **单个Agent**: `claude`, `codex`, `gemini`
- **全部Agent**: `all`
- **组合Agent**: `"agent1|agent2|agent3"` (需要引号)

#### 示例
```bash
# 单个Agent任务
agentic-warden claude "写一个快速排序算法的Rust实现"
agentic-warden codex "生成Python数据可视化代码"
agentic-warden gemini "解释机器学习中的梯度下降"

# 批量Agent任务
agentic-warden all "review this code and suggest improvements"
agentic-warden "claude|gemini" "compare these two approaches"
agentic-warden "codex|claude" "write comprehensive documentation"
```

### 配置同步命令

```bash
# 推送配置到云端
agentic-warden push [directory...]

# 从云端拉取配置
agentic-warden pull [directory...]

# 查看同步状态
agentic-warden status

# 重置同步状态
agentic-warden reset

# 列出可同步目录
agentic-warden list
```

### 任务管理命令

```bash
# 等待模式 - 监控任务完成
agentic-warden wait

# CLI管理界面(无参数时)
agentic-warden
```

## 固化的Agent参数

为确保一致性和安全性，各AI Agent使用固定的完整权限参数：

### Claude CLI
```bash
claude -p --dangerously-skip-permissions "<prompt>"
```
- `-p`: 非交互式打印模式
- `--dangerously-skip-permissions`: 跳过权限检查，完整文件访问

### Codex CLI  
```bash
codex exec --dangerously-bypass-approvals-and-sandbox "<prompt>"
```
- `exec`: 非交互式执行模式
- `--dangerously-bypass-approvals-and-sandbox`: 绕过审批和沙箱，完整系统访问

### Gemini CLI
```bash
gemini --approval-mode yolo "<prompt>"
```
- `--approval-mode yolo`: 自动批准所有操作，完整工具访问

## 配置文件

### 认证配置 (`~/.agentic-warden/auth.json`)
```json
{
  "client_id": "your-google-client-id.apps.googleusercontent.com",
  "client_secret": "your-google-client-secret",
  "access_token": "ya29.a0AaHxxxxxxxxxxx",
  "refresh_token": "ya29.c0Ab-xxxxxxxxxxx",
  "expires_at": "2024-12-31T23:59:59Z"
}
```

### 同步配置 (`~/.agentic-warden/sync.json`)
```json
{
  "config": {
    "directories": ["~/.claude", "~/.codex", "~/.gemini"],
    "auto_sync_enabled": false,
    "sync_interval_minutes": 60
  },
  "state": {
    "directories": {
      "~/.claude": {
        "hash": "d41d8cd98f00b204e9800998ecf8427e",
        "last_sync": "2024-01-01T12:00:00Z"
      }
    },
    "last_sync": "2024-01-01T12:00:00Z"
  }
}
```

## 性能要求

### 响应时间
- **CLI启动时间**: < 2秒
- **任务注册延迟**: < 100ms  
- **配置检测时间**: < 5秒 (大型配置目录)
- **云端同步延迟**: < 30秒 (标准网络)

### 资源使用
- **共享内存大小**: 4 MiB (codex-task命名空间)
- **最大并发任务**: 50个
- **内存占用**: < 50MB (正常运行)
- **日志文件大小**: 动态，基于进程输出

### 可扩展性
- **支持的Agent工具**: 可扩展至新的AI Agent工具
- **配置目录**: 支持任意数量的配置目录
- **用户账户**: 支持多用户配置文件
- **平台支持**: Windows、Linux、macOS

## 安全要求

### 数据保护
- **凭证加密**: OAuth客户端密钥和访问令牌加密存储
- **文件权限**: 配置文件权限设置为600 (用户读写)
- **传输安全**: 所有云端通信使用HTTPS
- **数据隔离**: 认证数据与配置数据分离存储

### 访问控制
- **最小权限原则**: 只请求必要的OAuth权限范围
- **用户确认**: 重要操作需要用户确认
- **数据清理**: 提供完整的凭证删除功能
- **透明处理**: 明确告知用户数据使用范围

## 错误处理

### 错误类型
```rust
pub enum ProcessError {
    Io(#[from] io::Error),
    Registry(#[from] RegistryError),
    ProcessTree(#[from] ProcessTreeError),
    CliNotFound(String),
}
```

### 错误处理原则
- **敏感信息保护**: 错误日志中不包含敏感信息
- **优雅降级**: 网络错误时提供离线模式
- **重试机制**: 网络操作支持自动重试
- **用户友好**: 提供清晰的错误信息和解决建议

## 开发工具链

### 核心依赖
```toml
[dependencies]
# 异步运行时
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }

# 序列化和配置
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }

# 共享内存和进程管理
shared_memory = "0.12"
shared_hashmap = "0.1.2"

# 平台特定依赖
[target.'cfg(unix)'.dependencies]
psutil = "3.2"
libc = "0.2"

[target.'cfg(windows)'.dependencies]
sysinfo = "0.32"
windows = { version = "0.54", features = [...] }

# 云端集成
reqwest = { version = "0.11", features = ["json", "multipart"] }
```

### 开发标准
- **Rust Edition**: 2024
- **构建系统**: Cargo
- **测试框架**: 内置cargo test
- **代码格式**: cargo fmt
- **代码检查**: cargo clippy (deny warnings)
- **文档生成**: cargo doc

## 版本历史

### v0.3.0 (当前版本)
- ✅ 新增多CLI统一接口支持
- ✅ 简化任务提示词调用方式
- ✅ 删除传统透传模式
- ✅ 改进用户体验和错误处理

### v0.2.1
- ✅ 完整的配置同步系统
- ✅ OAuth 2.0认证
- ✅ 跨平台压缩抽象层
- ✅ 修复编译警告

### v0.2.0
- ✅ 进程树管理系统
- ✅ 共享内存任务注册表
- ✅ 等待模式
- ✅ 跨平台支持

## 未来规划

### v0.4.0 (计划中)
- 配置热重载功能
- 性能监控和指标收集
- 高级CLI管理界面
- 插件系统支持

### v1.0.0 (长期目标)
- 团队协作功能
- 企业级安全特性
- 更多AI CLI工具集成
- 分布式配置同步

---

**总结**: agentic-warden v0.3.0 是一个功能完整、架构清晰的AI Agent管理工具，提供统一的多Agent调用接口、强大的进程管理能力和可靠的配置同步功能。项目采用现代Rust实践，具备优秀的性能、安全性和可维护性。