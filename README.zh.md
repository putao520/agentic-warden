# Agentic-Warden

<div align="center">

![版本](https://img.shields.io/badge/版本-5.0.1-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![许可证](https://img.shields.io/badge/许可证-MIT-green?style=flat-square)
![平台](https://img.shields.io/badge/平台-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=flat-square)

**通用 AI CLI 管理和协调平台**

</div>

## 什么是 Agentic-Warden？

Agentic-Warden 是一个命令行工具，旨在解决管理多个 AI CLI 助手的混乱局面。它为 Claude、Codex、Gemini 和其他 AI CLI 工具提供统一管理、进程跟踪和配置同步功能。

## 解决的核心问题

- **多 AI 管理**：在不同 AI CLI 工具之间切换，不丢失上下文
- **供应商灵活性**：使用任何 API 供应商（OpenRouter、LiteLLM 等），无需修改 AI CLI 配置
- **进程协调**：跟踪和管理同时运行的多个 AI CLI 进程
- **配置同步**：通过 Google Drive 在设备间备份和恢复 AI CLI 配置

## 核心功能

### 🔧 **AI CLI 管理**
- 从单一界面启动和管理 Claude、Codex、Gemini
- 交互模式：`agentic-warden claude`
- 任务模式：`agentic-warden claude "编写 Python 函数"`
- 多 CLI：`agentic-warden claude,codex "比较算法"`

### 🌐 **供应商管理**
- 无需配置更改即可切换 API 供应商：`agentic-warden claude -p openrouter`
- 支持 OpenRouter、LiteLLM、自定义端点
- 无缝供应商切换的环境变量注入

### 📊 **进程跟踪**
- 智能进程树识别
- 跨进程任务协调
- `wait` 命令：`agentic-warden wait` - 监控 CLI 和 MCP 注册表，所有任务完成才退出
- 进程隔离和命名空间管理

### 🧠 **内存与语义搜索**
- 与 Qdrant 向量数据库集成，实现语义对话搜索
- 与 Ollama 嵌入服务集成，实现文本向量化
- 基于 Qdrant 元数据的会话对话存储
- MCP 工具用于内存操作：
  - `search_history`：带 session_id 过滤的对话历史查询
  - `get_session_todos`：通过 session_id 查询未完成 TODO
- 自动 TODO 提取和会话关联

### ☁️ **Google Drive 集成**
- 备份配置：`agentic-warden push`
- 恢复配置：`agentic-warden pull`
- 选择性文件打包（无缓存/临时文件）
- 无头环境的 OAuth 2.0 设备流程

### 🛠️ **实用工具命令**
- `agentic-warden status` - 检查 AI CLI 工具和版本
- `agentic-warden update` - 更新/安装 AI CLI 工具
- `agentic-warden tui` - 启动终端界面
- `agentic-warden mcp` - 启动 MCP 服务器进行外部集成

## 安装

### 通过 Cargo（推荐）
```bash
cargo install agentic-warden
```

### 从源码
```bash
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo install --path .
```

## 快速开始

### 1. 检查 AI CLI 状态
```bash
agentic-warden status
```
显示哪些 AI CLI 工具已安装及其版本。

### 2. 使用默认供应商启动 AI CLI
```bash
# 交互模式
agentic-warden claude

# 任务模式
agentic-warden codex "调试这段 Rust 代码"
```

### 3. 切换供应商
```bash
# 使用 OpenRouter 配合 Claude
agentic-warden claude -p openrouter "编写 Python 脚本"

# 使用 LiteLLM 配合多个 AI CLI
agentic-warden claude,codex -p litellm "分析数据"
```

### 4. 管理后台任务
```bash
# 在后台启动多个 AI CLI 任务
agentic-warden codex "任务 1" &
agentic-warden gemini "任务 2" &
agentic-warden claude "任务 3" &

# 等待所有任务完成
agentic-warden wait
```

### 5. 内存与语义搜索
```bash
# 启动 MCP 服务器进行内存操作（后台运行）
agentic-warden mcp &

# 在 Claude Code 或其他 MCP 客户端中，搜索对话历史：
{
  "tool": "search_history",
  "arguments": {
    "query": "Python 编程最佳实践",
    "session_id": "session-123",
    "limit": 10
  }
}

# 查询特定会话的 TODO：
{
  "tool": "get_session_todos",
  "arguments": {
    "session_id": "session-123",
    "status": "pending"
  }
}
```

### 6. 同步配置
```bash
# 备份到 Google Drive
agentic-warden push

# 从 Google Drive 恢复
agentic-warden pull
```

## 供应商设置

### 添加自定义供应商
```bash
agentic-warden tui
# 导航到供应商管理
# 添加带有 API 密钥和端点的新供应商
```

### 示例供应商配置
```json
{
  "openrouter": {
    "name": "OpenRouter",
    "compatible_with": ["claude", "codex", "gemini"],
    "env": {
      "OPENAI_API_KEY": "sk-or-v1-...",
      "OPENAI_API_BASE": "https://openrouter.ai/api/v1"
    }
  }
}
```

## 使用场景

### 面向开发者
- **多模型开发**：在不同 AI 模型上测试相同提示
- **供应商测试**：比较不同 API 供应商的成本和质量
- **任务管理**：并发运行多个 AI 任务并跟踪进度

### 面向团队
- **配置同步**：在团队成员之间共享 AI CLI 配置
- **供应商管理**：集中式 API 密钥和端点管理
- **进程监控**：跟踪团队 AI 使用情况和任务进度

### 面向高级用户
- **批处理**：使用不同供应商运行多个 AI 任务
- **配置备份**：永不丢失您的 AI CLI 设置
- **高级工作流**：通过 MCP 服务器与其他工具集成

## 高级功能

### MCP（模型上下文协议）服务器
```bash
# 启动 MCP 服务器进行外部集成
agentic-warden mcp

# 与 Claude Code 或其他 MCP 客户端一起使用
# 提供工具：start_concurrent_tasks、get_task_command、search_history、get_session_todos
```

### 进程等待模式
```bash
# 等待所有 AI CLI 任务（同时监控 CLI 和 MCP 注册表）
agentic-warden wait

# 等待特定进程
agentic-warden pwait <PID>

# 带超时等待
agentic-warden wait --timeout 2h
```

### 更新管理
```bash
# 更新所有 AI CLI 工具
agentic-warden update

# 更新特定工具
agentic-warden update claude

# 如果不存在则安装
agentic-warden update gemini
```

## 系统要求

- **Rust**：1.70+ 用于从源码构建
- **操作系统**：Windows 10+、Linux、macOS 10.15+
- **AI CLI 工具**：Claude、Codex、Gemini（可选）
- **Node.js**：14+（用于 npm 包）

## 配置

配置文件存储在 `~/.agentic-warden/` 中：
- `provider.json` - 供应商配置
- `sync_state.json` - Google Drive 同步状态
- `oauth_tokens.json` - OAuth 令牌（加密）

### 内存配置
```yaml
# ~/.agentic-warden/providers.json（自动生成）
[memory]
ollama_url = "http://localhost:11434"
qdrant_url = "http://localhost:26333"
embedding_model = "qwen3-embedding:0.6b"
llm_model = "qwen3:8b"
```

## 支持

- **GitHub Issues**：[报告错误和请求功能](https://github.com/putao520/agentic-warden/issues)
- **文档**：[完整 SPEC 文档](./SPEC/)
- **讨论**：[社区讨论](https://github.com/putao520/agentic-warden/discussions)

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

---

**Agentic-Warden** - AI CLI 生态系统的统一控制。