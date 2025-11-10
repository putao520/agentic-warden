# MCP (Model Context Protocol) 集成设计

## 概述

Agentic-Warden 提供 MCP (Model Context Protocol) 服务器功能，允许外部 AI 助手（如 Claude Code）通过标准协议访问和管理 Agentic-Warden 的核心功能。

## 架构原则

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
- 返回标准 task JSON 结构（包含 wait 命令）
- Claude Code 解析并执行 wait 任务等待所有任务完成
- 适用于：多个任务并发执行

**工作流程**:
```
Claude Code (MCP客户端)
    ↓ 提交多个任务
MCP Server
    ↓ 逐个调用 supervisor::execute_cli() 后台启动
    ├─ 任务1 → supervisor::execute_cli() (spawn)
    ├─ 任务2 → supervisor::execute_cli() (spawn)
    └─ 任务3 → supervisor::execute_cli() (spawn)
    ↓ 返回 task JSON: {"tool": "bash", "command": "agentic-warden wait", ...}
Claude Code
    ↓ 解析 task JSON
    ↓ 使用 Bash 工具执行 wait 命令（前台，12h超时）
wait_mode
    ↓ 通过 TaskRegistry 追踪所有任务
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
- **实时监控**: 提供实时的进程状态和配置信息查询

### 📋 功能范围

#### 进程管理工具
- **`monitor_processes`**: 监控所有 AI CLI 进程状态
- **`get_process_tree`**: 获取详细的进程树信息
- **`terminate_process`**: 安全终止 AI CLI 进程

#### 配置管理工具
- **`get_provider_status`**: 获取 Provider 配置和状态信息

#### AI CLI 启动工具
- **`start_concurrent_tasks`**: 并发启动多个 AI CLI 任务
  - 直接在 MCP Server 中启动所有进程（后台）
  - 返回: 标准 task JSON 结构，包含 wait 命令
  - task 结构: `{"tool": "bash", "command": "agentic-warden wait --timeout 12h", "timeout_ms": 43200000}`
  - 通过 supervisor::execute_cli() 复用现有实现
  - 每个任务自动注册到 TaskRegistry
- **`get_task_command`**: 获取单个 AI CLI 的启动任务
  - 返回: 标准 task JSON 结构
  - task 结构: `{"tool": "bash", "command": "agent <ai_type> 'task'", "timeout_ms": 43200000}`
  - 不直接启动进程，由 Claude Code 解析并通过 Bash 工具执行
- **`start_ai_cli`**: (已废弃) 直接启动并等待单个 AI CLI 完成

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
- MCP Server 逐个调用 `supervisor::execute_cli()` 启动进程
- 每个任务在后台运行（tokio::spawn）
- supervisor 自动将任务注册到 TaskRegistry
- 立即返回响应（不等待任务完成）

**步骤3: 返回 task JSON**
```json
{
  "success": true,
  "started_count": 3,
  "task": {
    "description": "Wait for 3 concurrent AI CLI tasks to complete",
    "tool": "bash",
    "command": "agentic-warden wait --timeout 12h",
    "timeout_ms": 43200000
  },
  "message": "Started 3 concurrent AI CLI tasks in background. Execute the 'task' using Bash tool."
}
```

**步骤4: Claude Code 执行等待任务**
- Claude Code 解析 `task` 字段
- 使用 Bash 工具执行 `command`
- 应用 `timeout_ms` 超时设置

**步骤5: 等待所有任务完成并返回报告**
- `agentic-warden wait` 命令通过 TaskRegistry 追踪所有任务
- 阻塞等待，直到所有任务完成
- 返回详细的执行报告

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

#### MCP 服务器 (`AgenticWardenMcpServer`)
- **职责**: 处理 MCP 请求和响应
- **功能**: JSON-RPC 请求解析、工具路由、错误处理
- **传输**: stdio 异步 I/O

#### 工具接口
每个工具都遵循 MCP 工具规范：

```rust
#[tool(description = "Tool description")]
pub async fn tool_name(
    &self,
    #[arg] params: ToolParams,
) -> Result<CallToolResult, McpError>
```

#### 进程监控模块
- **进程识别**: 智能 AI CLI 进程识别算法
- **状态查询**: 跨平台进程状态获取
- **安全检查**: 防止误操作非 AI CLI 进程

## 使用场景

### Claude Code 集成
```bash
# 添加 MCP 服务器到 Claude Code
claude --mcp-add agentic-warden "agentic-warden mcp server"

# 在 Claude Code 中使用
# "监控当前运行的 AI CLI 进程"
# "获取所有 claude 进程的进程树"
# "安全终止空闲的 codex 进程"
```

### 开发者工具集成
- IDE 插件可以通过 MCP 访问 Agentic-Warden 功能
- 自动化脚本可以通过 MCP 调用进程管理
- CI/CD 流程可以集成 AI CLI 状态检查

## 安全考虑

### 进程安全
- **AI CLI 识别**: 只允许操作 claude、codex、gemini 相关进程
- **权限检查**: 操作前验证进程归属
- **安全终止**: 优先使用 SIGTERM，强制终止使用 SIGKILL

### 配置安全
- **只读访问**: Provider 配置只允许读取，不允许修改
- **敏感信息**: 不暴露 API 密钥等敏感配置
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

### 新工具支持
- **任务管理**: 创建、查询、删除 AI CLI 任务
- **性能监控**: CPU、内存使用情况查询
- **日志管理**: AI CLI 进程日志访问

### 高级传输
- **HTTP/WebSocket**: 支持 Web 传输协议
- **IPC**: 进程间通信支持
- **网络**: 远程 MCP 服务器支持

### 配置管理
- **动态配置**: 运行时修改部分配置
- **配置版本**: 配置变更历史和回滚
- **权限细分**: 更细粒度的访问控制

## 相关文档

- [API 设计规范](./API.md)
- [进程监控设计](../core/process_tree.md)
- [Provider 管理设计](../provider/manager.md)
- [部署指南](./DEPLOYMENT.md)