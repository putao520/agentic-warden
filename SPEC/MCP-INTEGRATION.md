# MCP (Model Context Protocol) 集成设计

## 概述

Agentic-Warden 提供 MCP (Model Context Protocol) 服务器功能，允许外部 AI 助手（如 Claude Code）通过标准协议访问和管理 Agentic-Warden 的核心功能。

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
- **`start_ai_cli`**: 通过外部方式启动 AI CLI 并执行任务

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