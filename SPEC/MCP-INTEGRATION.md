# MCP (Model Context Protocol) 集成设计

## 概述

Agentic-Warden 提供 MCP (Model Context Protocol) 服务器功能，允许外部 AI 助手（如 Claude Code）通过标准协议访问和管理 Agentic-Warden 的核心功能。

## 架构原则

### 🏗️ 任务存储隔离

**关键设计**: MCP启动的任务与CLI启动的任务使用**完全独立的存储**：

- **MCP任务**: 使用 `InProcessRegistry` (进程内HashMap) - 只在MCP Server进程内可见
- **CLI任务**: 使用 `SharedMemoryStorage` (跨进程共享内存) - 多个进程可见

**等待命令**:
- **`pwait`**: 等待MCP任务（从InProcessRegistry读取）
- **`wait`**: 等待CLI任务（从SharedMemoryStorage读取）

⚠️ **重要**: 不要混用！MCP任务必须用`pwait`等待，不能用`wait`（它们读取不同的存储）。

### 🏗️ MCP 模块的两种模式

**关键设计决策**: MCP 根据任务数量采用不同的启动策略。

#### 模式1：普通任务（单个任务）
- MCP 返回标准 task JSON 结构
- 包含 tool、command、timeout_ms 等字段
- Claude Code 解析并通过 Bash 工具执行
- 适用于：单个任务，需要实时反馈

**工作流程**:
```
Claude Code (MCP客户端)
    ↓ 请求单个任务
MCP Server
    ↓ 返回 task JSON: {"tool": "bash", "command": "agent codex 'task'", ...}
Claude Code
    ↓ 解析 task JSON
    ↓ 使用 Bash 工具执行（前台，12h超时）
supervisor::execute_cli()
    ↓ 处理所有复杂逻辑
    ├─ Provider 配置加载
    ├─ 环境变量注入
    ├─ 进程启动
    ├─ TaskRegistry 注册
    └─ 日志管理
```

#### 模式2：并发任务（多个任务）
- MCP 直接启动所有 AI CLI 进程（后台）
- 返回标准 task JSON 结构（包含 **pwait** 命令）
- Claude Code 解析并执行 **pwait** 任务等待所有MCP任务完成
- 适用于：多个任务并发执行
- **存储**: 所有任务注册到 InProcessRegistry

**工作流程**:
```
Claude Code (MCP客户端)
    ↓ 提交多个任务
MCP Server
    ↓ 逐个调用 supervisor::execute_cli() 后台启动
    ├─ 任务1 → supervisor::execute_cli(&mcp_registry) (spawn)
    ├─ 任务2 → supervisor::execute_cli(&mcp_registry) (spawn)
    └─ 任务3 → supervisor::execute_cli(&mcp_registry) (spawn)
    ↓ 所有任务注册到 InProcessRegistry
    ↓ 返回 task JSON: {"tool": "bash", "command": "agentic-warden pwait", ...}
Claude Code
    ↓ 解析 task JSON
    ↓ 使用 Bash 工具执行 pwait 命令（前台，12h超时）
pwait_mode
    ↓ 通过 InProcessRegistry 追踪所有MCP任务
    └─ 等待所有任务完成，返回报告
```

**代码复用原则**:
- 无论哪种模式，都使用 `supervisor::execute_cli()` 启动进程
- 不在 MCP 中重复实现 Provider 配置、环境变量注入、TaskRegistry 注册等逻辑
- 统一流程：通过 supervisor 处理所有执行细节

## 设计目标

### 🎯 核心目标
- **外部集成**: 为外部 AI 助手提供 Agentic-Warden 功能的编程接口
- **标准协议**: 使用 MCP 标准协议，确保与各种 AI 助手的兼容性
- **安全访问**: 通过进程安全检查，只允许操作 AI CLI 相关进程
- **实时查询**: 提供实时的进程状态和配置信息查询

### 📋 功能范围

#### AI CLI 启动工具（核心功能）
- **`start_concurrent_tasks`**: 并发启动多个 AI CLI 任务
  - 直接在 MCP Server 中启动所有进程（后台）
  - 返回: 标准 task JSON 结构，包含 **pwait** 命令
  - task 结构: `{"tool": "bash", "command": "agentic-warden pwait", "timeout_ms": 43200000}`
  - 通过 supervisor::execute_cli(&mcp_registry) 复用现有实现
  - 每个任务自动注册到 **InProcessRegistry** (进程内独享)
- **`get_task_command`**: 获取单个 AI CLI 的启动任务
  - 返回: 标准 task JSON 结构
  - task 结构: `{"tool": "bash", "command": "agent <ai_type> 'task'", "timeout_ms": 43200000}`
  - 不直接启动进程，由 Claude Code 解析并通过 Bash 工具执行

### 🆕 并发模式工作流程

#### 场景：Claude Code 需要并发执行多个AI任务

**步骤1: Claude Code 通过 MCP 提交任务**
```json
{
  "method": "call_tool",
  "params": {
    "name": "start_concurrent_tasks",
    "arguments": {
      "tasks": [
        {"ai_type": "codex", "provider": "openrouter", "task": "分析代码A"},
        {"ai_type": "gemini", "task": "生成文档B"},
        {"ai_type": "codex", "task": "重构模块C"}
      ]
    }
  }
}
```

**步骤2: Agentic-Warden 直接启动所有AI CLI**
- MCP Server 逐个调用 `supervisor::execute_cli(&mcp_registry)` 启动进程
- 每个任务在后台运行（tokio::spawn）
- supervisor 自动将任务注册到 **InProcessRegistry** (进程内独享)
- 立即返回响应（不等待任务完成）

**步骤3: 返回 task JSON**
```json
{
  "success": true,
  "started_count": 3,
  "task": {
    "description": "Wait for 3 concurrent MCP tasks to complete",
    "tool": "bash",
    "command": "agentic-warden pwait",
    "timeout_ms": 43200000
  },
  "message": "Started 3 concurrent MCP tasks in background (InProcessRegistry). Execute the 'task' using Bash tool with 'pwait' command.",
  "note": "MCP tasks use InProcessRegistry (process-local). Use 'pwait' to wait for MCP tasks, not 'wait' (which is for cross-process CLI tasks)."
}
```

**步骤4: Claude Code 执行等待任务**
- Claude Code 解析 `task` 字段
- 使用 Bash 工具执行 `command` (**pwait**, 不是wait!)
- 应用 `timeout_ms` 超时设置（Bash工具的超时，而非pwait命令参数）

**步骤5: 等待所有MCP任务完成并返回报告**
- `agentic-warden pwait` 命令通过 **InProcessRegistry** 追踪所有MCP任务
- 阻塞等待，直到所有任务完成
- 返回详细的执行报告

⚠️ **注意**:
- MCP任务存储在 InProcessRegistry（进程内）
- CLI任务存储在 SharedMemoryStorage（跨进程）
- **不要混用**: MCP任务必须用 `pwait` 等待，CLI任务用 `wait` 等待

### 🔧 普通模式工作流程

#### 场景：Claude Code 只需执行单个AI任务

**步骤1: Claude Code 通过 MCP 获取启动命令**
```json
{
  "method": "call_tool",
  "params": {
    "name": "get_task_command",
    "arguments": {
      "ai_type": "codex",
      "task": "Fix the bug in main.rs",
      "provider": "openrouter"
    }
  }
}
```

**步骤2: Agentic-Warden 返回 task JSON**
```json
{
  "success": true,
  "task": {
    "description": "Execute codex task with provider openrouter: Fix the bug in main.rs",
    "tool": "bash",
    "command": "agent codex -p openrouter 'Fix the bug in main.rs'",
    "timeout_ms": 43200000
  },
  "ai_type": "codex",
  "provider": "openrouter",
  "message": "Execute the 'task' using Bash tool with 12h timeout"
}
```

**步骤3: Claude Code 执行任务**
- Claude Code 解析 `task` 字段
- 使用 Bash 工具执行 `command`
- 应用 `timeout_ms` 超时设置
- 获得实时反馈

## 技术实现

### 协议层
- **协议版本**: MCP v1.0
- **传输协议**: stdio (标准输入/输出)
- **数据格式**: JSON-RPC 2.0

### 服务器架构
```
┌─────────────────────────────────────┐
│          External AI Assistant      │
│            (Claude Code)             │
└─────────────────┬───────────────────┘
                  │ stdio
                  ▼
┌─────────────────────────────────────┐
│        Agentic-Warden MCP Server     │
│                                     │
│  ┌─────────────┐  ┌──────────────┐  │
│  │ JSON-RPC    │  │  Tool Router │  │
│  │  Handler    │  │              │  │
│  └─────────────┘  └──────────────┘  │
│         │               │           │
│         ▼               ▼           │
│  ┌─────────────┐  ┌──────────────┐  │
│  │ Core Logic  │  │  Provider    │  │
│  │             │  │  Manager     │  │
│  └─────────────┘  └──────────────┘  │
└─────────────────────────────────────┘
```

### 核心组件

#### MCP 服务器实现要求

**使用 rmcp 库**：
- **依赖**: `rmcp = { version = "0.5", features = ["server", "transport-io"] }`
- **原因**: rmcp 是 Rust 的标准 MCP 实现库，提供完整的 JSON-RPC 2.0 支持
- **禁止**: 不允许自己实现 JSON-RPC 协议，必须使用 rmcp 库

#### MCP 服务器 (`AgenticWardenMcpServer`)

**实现方式**：
```rust
use rmcp::prelude::*;

#[derive(Server)]
pub struct AgenticWardenMcpServer {
    // 服务器状态
}

impl AgenticWardenMcpServer {
    /// 启动 MCP 服务器
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // 使用 rmcp 的标准 stdio 传输
        self.serve_stdio().await?;
        Ok(())
    }
}
```

- **职责**: 处理 MCP 请求和响应
- **功能**: 使用 rmcp 提供的 JSON-RPC 解析、工具路由、错误处理
- **传输**: 使用 rmcp 的 stdio 异步 I/O

#### 工具接口
每个工具使用 rmcp 的 `#[tool]` 宏定义：

```rust
#[tool(description = "Generate commands for multiple AI CLI tasks to run concurrently")]
pub async fn start_concurrent_tasks(
    &self,
    params: Parameters<StartConcurrentTasksParams>
) -> String {
    // 工具实现
}

#[tool(description = "Get command to start a single AI CLI task")]
pub async fn get_task_command(
    &self,
    params: Parameters<GetTaskCommandParams>
) -> String {
    // 工具实现
}
```

**rmcp 工具定义要求**：
- 使用 `#[tool]` 宏标注工具函数
- 使用参数结构体定义输入（通过 `Parameters<T>` 包装）
- 返回类型为简单的 `String`（JSON格式）

## 使用场景

### Claude Code 集成
```bash
# 添加 MCP 服务器到 Claude Code
claude --mcp-add agentic-warden "agentic-warden mcp server"

# 在 Claude Code 中使用
# "并发启动3个AI任务：codex分析代码、claude写文档、gemini生成测试"
# "启动一个codex任务处理bug修复"
```

### 开发者工具集成
- IDE 插件可以通过 MCP 访问 Agentic-Warden 的任务启动功能
- 自动化脚本可以通过 MCP 调用并发任务执行
- CI/CD 流程可以集成 AI CLI 任务调度

## 安全考虑

### AI类型验证
- **AI CLI 验证**: 只允许 claude、codex、gemini 三种AI类型
- **参数验证**: 严格验证所有输入参数
- **命令转义**: 正确处理任务描述中的特殊字符

### 配置安全
- **Provider 配置**: 通过 ProviderManager 统一管理
- **环境变量**: 由 supervisor 模块安全注入
- **访问控制**: 通过 MCP 客户端身份验证

## 命令行接口

### MCP 服务器管理
```bash
# 启动 MCP 服务器
agentic-warden mcp server [--transport stdio] [--log-level info]

# 测试 MCP 配置
agentic-warden mcp test

# 查看 MCP 服务器状态
agentic-warden mcp status
```

### 配置选项
- **`--transport`**: 传输协议（当前支持 stdio）
- **`--log-level`**: 日志级别（debug, info, warn, error）

## 错误处理

### MCP 错误响应
遵循 MCP 错误规范：
- **InvalidParams**: 参数验证失败
- **InternalError**: 内部服务器错误
- **ToolError**: 工具执行错误

### 错误示例
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid parameters: Process not found",
    "data": { "pid": 12345 }
  }
}
```

## 性能优化

### 异步处理
- **非阻塞 I/O**: 使用 tokio 异步运行时
- **并发支持**: 支持多个并发 MCP 请求
- **资源管理**: 及时释放不再使用的资源

### 缓存策略
- **进程信息缓存**: 短时间内缓存进程状态信息
- **配置缓存**: Provider 配置信息本地缓存

## 测试策略

### 单元测试
- **工具逻辑测试**: 每个工具的核心逻辑验证
- **参数验证测试**: 输入参数的各种边界情况
- **错误处理测试**: 异常情况的错误响应

### 集成测试
- **MCP 协议测试**: 标准 MCP 消息交换验证
- **端到端测试**: 完整的客户端-服务器交互流程
- **并发测试**: 多个并发请求的处理能力

### 兼容性测试
- **Claude Code**: 与 Claude Code 的完整兼容性验证
- **其他客户端**: 与其他 MCP 客户端的基本兼容性

## 未来扩展

### 任务管理增强
- **任务查询**: 查询当前运行的MCP任务状态
- **任务取消**: 取消运行中的任务
- **任务历史**: 查看已完成任务的历史记录

### 并发控制
- **并发限制**: 限制同时运行的任务数量
- **优先级队列**: 支持任务优先级调度
- **资源管理**: 基于系统资源的智能调度

## 相关文档

- [API 设计规范](./API.md)
- [进程树追踪设计](../core/process_tree.md)
- [Provider 管理设计](../provider/manager.md)
- [部署指南](./DEPLOYMENT.md)