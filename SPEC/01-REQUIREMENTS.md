# Requirements Specification - v0.1.0

## Version Information
- Current version: v0.5.61
- Start date: 2025-11-08
- Based on: Initial development

## Historical Requirements
See:
- archive/HISTORY.md - Global history overview (initial version, no history)

---

## Functional Requirements

### REQ-001: AI CLI 进程树追踪
**Status**: 🟢 Done
**Priority**: P0 (Critical)
**Version**: v0.1.0
**Related**: ARCH-001, DATA-001

**Description**:
Agentic-Warden MUST provide intelligent process tree tracking to identify which AI CLI root process spawned the current process. This solves the problem of traditional tools attributing all processes to explorer.exe.

**Acceptance Criteria**:
- [x] Traverse process tree upward to identify the root AI CLI (codex/claude/gemini)
- [x] Group tasks by parent process
- [x] Isolate shared memory by AI CLI root process
- [x] Ensure different AI CLI tasks do not interfere with each other

**Technical Constraints**:
- MUST support Windows via winapi
- MUST support Linux and macOS via procfs
- MUST distinguish between codex, claude, and gemini processes

---

### REQ-002: 第三方 Provider 管理
**Status**: 🟢 Done
**Priority**: P0 (Critical)
**Version**: v0.1.0
**Related**: ARCH-002, DATA-002, API-001

**Description**:
Agentic-Warden MUST provide unified management of third-party API providers (OpenRouter, LiteLLM, etc.) through centralized configuration with transparent environment variable injection.

**Acceptance Criteria**:
- [x] Store provider configurations in `~/.aiw/provider.json` with JSON schema validation
- [x] Support multiple providers with environment variable injection (API keys, base URLs, org IDs)
- [x] Inject environment variables into AI CLI processes via `-p` parameter (transparent to user)
- [x] Set default provider in configuration with fallback to first available provider
- [x] Do NOT modify AI CLI's native configuration files (maintain separation of concerns)
- [x] Validate provider compatibility with AI CLI types before injection
- [x] Support built-in providers with read-only configurations
- [x] Provide TUI interface for provider management (add, edit, delete, test)
- [x] Implement provider health checking and network connectivity detection
- [x] Mask sensitive values (API keys) in display and logs
- [x] Support optional `scenario` field for provider usage description (v0.2.0)
- [x] Dynamic ENV injection via `get_all_env_vars()` auto-mapping token/base_url to standard env vars (v0.2.0)
- [x] Support `auto` provider for automatic random selection from compatible providers (v0.6.x)

**Technical Constraints**:
- Configuration file format: JSON with schema validation
- Environment variable injection must be transparent to AI CLI process
- Support OpenRouter, LiteLLM, and custom providers as minimum
- Provider configurations must support inheritance and overrides
- Sensitive data must be masked in UI output
- Compatibility validation required before provider injection
- `auto` is a reserved keyword: users MUST NOT create a provider named `auto` in provider.json

---

### REQ-004: 统一 TUI 体验
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.0
**Related**: ARCH-004, MODULE-001

**Description**:
Agentic-Warden MUST provide unified TUI experience using ratatui framework for all interactive interfaces.

**Acceptance Criteria**:
- [x] Dashboard: Display AI CLI status and task summary
- [x] Provider Management: Manage third-party API providers via TUI
- [x] Task Status: Display task status grouped by parent process
- [x] Push/Pull Progress: Real-time progress display for sync operations
- [x] Use ratatui as the single TUI framework

**Technical Constraints**:
- ratatui version: 0.24+
- crossterm version: 0.27+
- All TUI components must use unified design system

---

### REQ-005: Wait 模式跨进程等待
**Status**: 🟢 Done
**Priority**: P2 (Normal)
**Version**: v0.1.0
**Related**: ARCH-005, DATA-005, API-002

**Description**:
Agentic-Warden MUST provide `wait` command to wait for concurrent AI CLI tasks completion across processes, and `pwait` command to wait for specific process's shared tasks.

**Acceptance Criteria**:
- [x] `agentic-warden wait` waits for all concurrent AI CLI tasks
- [x] `agentic-warden pwait <PID>` waits for specific process's completed shared tasks
- [x] Cross-process task completion detection

**Technical Constraints**:
- Use shared memory for cross-process communication

---

### REQ-006: AI CLI 工具检测与状态管理
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.0
**Related**: ARCH-006, MODULE-002

**Description**:
Agentic-Warden MUST detect installed AI CLI tools (codex, claude, gemini) and provide status information through `status` command.

**Acceptance Criteria**:
- [x] Detect installed status of codex, claude, gemini
- [x] Retrieve version information for installed tools
- [x] Identify installation type (Native or NPM)
- [x] Display detection results in `agentic-warden status`
- [x] Provide installation hints for uninstalled tools

**Technical Constraints**:
- Use `which` command to detect tool availability
- Version retrieval via `--version` flag
- Installation type detection based on path (node_modules/npm = NPM)

---

### REQ-007: MCP (Model Context Protocol) 服务器
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.0 → v0.2.0
**Related**: ARCH-007, API-003

**Description**:
Agentic-Warden MUST provide MCP server to enable external AI assistants to access Agentic-Warden functionality.

**Acceptance Criteria**:
- [x] Support stdio transport protocol
- [x] Provide core task management tools:
  - `start_concurrent_tasks`: 并发启动多个AI CLI任务
  - `get_task_command`: 获取单个AI CLI任务的启动命令
- [x] Provide memory-related tools:
  - `search_history`: 查询历史对话（带session_id过滤，返回TODO items）
- [x] Compatible with Claude Code and other MCP clients

**Technical Constraints**:
- MCP Protocol v1.0
- Transport: stdio only
- Memory integration with Qdrant vector database
- Session-based TODO management

---

### REQ-008: 指定供应商模式 AI CLI 启动
**Status**: 🟢 Done
**Priority**: P0 (Critical)
**Version**: v0.5.23 (Enhanced with transparent parameter forwarding)
**Related**: ARCH-002, ARCH-008, API-004

**Description**:
Agentic-Warden MUST provide seamless AI CLI startup with dynamic provider selection through environment variable injection, enabling users to switch between different API providers without modifying AI CLI native configurations.

**Acceptance Criteria**:
- [x] Support `agentic-warden <ai_type> -p <provider> <prompt>` command syntax
- [x] Transparent environment variable injection before AI CLI process startup
- [x] Provider compatibility validation with AI CLI type before execution
- [x] Fallback to default provider when no provider specified
- [x] Support concurrent AI CLI processes with different providers
- [x] Maintain process isolation and namespace separation per provider
- [x] Handle provider configuration errors gracefully with clear error messages
- [x] Support both single AI CLI and multi-AI CLI execution modes
- [x] Preserve AI CLI native behavior while injecting provider configuration
- [x] Log provider usage for audit and debugging purposes
- [x] **Transparent parameter forwarding**: Parameters starting with `-` (excluding `-p/--provider`) are transparently forwarded to the AI CLI process without interpretation by Agentic-Warden

**Technical Constraints**:
- Environment injection MUST happen before process exec(), not after
- Provider validation MUST occur before process startup
- Process isolation MUST prevent provider cross-contamination
- Error handling MUST provide specific failure reasons
- Command syntax MUST be intuitive and consistent across AI CLI types
- **Parameter parsing**: `-p`/`--provider` parameters are consumed by Agentic-Warden for provider selection; all other `-` prefixed parameters are forwarded transparently to the target AI CLI
- **Parameter order**: Provider flags (`-p`/`--provider`) must be specified before other CLI parameters to ensure proper parsing

**Usage Examples**:
- `agentic-warden claude -p openrouter "Write a Python function"` - Use OpenRouter provider with Claude CLI
- `agentic-warden codex "Debug this code"` - Use default provider with Codex CLI
- `agentic-warden gemini,codex -p litellm "Compare algorithms"` - Multiple AI CLI with same provider
- `agentic-warden claude -p glm --model sonnet --debug api "Explain this code"` - Use GLM provider, forward `--model sonnet --debug api` to Claude CLI
- `agentic-warden claude -p glm --print --output-format json "Get structured response"` - Provider selection with multiple CLI parameters forwarded to Claude CLI
- `agentic-warden codex -p glm --temperature 0.7 --max-tokens 1000 "Generate text"` - Provider selection with CLI parameters forwarded to Codex CLI
- `agentic-warden claude -p auto "Analyze this code"` - Randomly select a compatible provider for Claude CLI
- `agentic-warden claude,codex -p auto "Multi-task"` - Each CLI independently selects a random compatible provider

**`auto` Provider Behavior**:
- When `-p auto` is specified, the system randomly selects one provider from those with `compatible_with` containing the target AI CLI type
- If no compatible providers exist, falls back to default mode (no provider environment injection, AI CLI uses native configuration)
- For multi-CLI execution (`claude,codex -p auto`), each CLI independently selects a random compatible provider
- The actually selected provider name is logged for debugging and audit purposes

**Key Implementation Notes**:
- AIW's `-p/--provider` flag selects the API provider configuration (e.g., glm, openrouter, auto)
- Provider configuration is injected via environment variables, NOT command-line arguments
- All other `-` prefixed parameters are transparently forwarded to the target AI CLI tool
- AI CLI tools maintain their native parameter semantics (e.g., Claude's `-p` for print mode is unrelated to AIW's provider selection)
- `auto` is a reserved provider name that triggers random selection logic

---

### REQ-009: 交互式 AI CLI 启动
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.5.23 (Enhanced with transparent parameter forwarding)
**Related**: ARCH-008, API-001

**Description**:
Agentic-Warden MUST support launching AI CLI tools in interactive mode when no task description is provided, while still supporting provider-specific environment variable injection for seamless switching between different API providers.

**Acceptance Criteria**:
- [x] Support `agentic-warden claude` - launch Claude CLI in interactive mode with default provider
- [x] Support `agentic-warden claude -p openrouter` - launch Claude CLI in interactive mode with OpenRouter provider
- [x] Support `agentic-warden codex --provider litellm` - support long format provider flag in interactive mode
- [x] Support `agentic-warden gemini,prompt -p custom` - multiple AI CLI in interactive mode with custom provider
- [x] Pass all environment variable injection to interactive AI CLI process before startup
- [x] Maintain process tree tracking and task registration for long-running interactive sessions
- [x] Support graceful signal handling (Ctrl+C) compatible with both Agentic-Warden and AI CLI processes
- [x] Detect interactive mode completion when user exits and mark task as completed in shared memory
- [x] Provide clear user feedback showing provider used in interactive mode
- [x] Handle provider validation errors gracefully before launching interactive CLI
- [x] **Interactive mode parameter forwarding**: Support transparent forwarding of CLI parameters (starting with `-`) in interactive mode for full AI CLI feature access

**Technical Constraints**:
- Interactive mode MUST preserve all provider functionality and environment variable injection
- Process tracking MUST work for long-running interactive sessions without memory leaks
- Environment variable injection MUST happen before interactive CLI starts, not during
- Signal handling MUST be compatible with both Agentic-Warden process management and AI CLI signal handling
- Task completion detection MUST work when user exits interactive mode naturally (Ctrl+D, exit command, etc.)
- Interactive mode MUST NOT require additional prompts or confirmation dialogs after provider selection
- Provider compatibility validation MUST occur before process startup
- **Parameter separation**: Provider selection parameters (`-p`/`--provider`) are processed by Agentic-Warden; all other CLI parameters are forwarded to maintain full AI CLI functionality
- **Parameter precedence**: Provider flags must be specified before other CLI parameters to ensure correct parsing and forwarding
- **Mode determination**: Interactive mode is activated when no prompt/task is provided; task mode when prompt is provided
- **Parameter forwarding**: In both modes, non-provider CLI parameters are forwarded to maintain full AI CLI functionality

**Usage Examples**:
```bash
# Basic interactive mode with default provider
agentic-warden claude
# Output: 🚀 Starting claude in interactive mode (provider: None)

# Interactive mode with specific provider
agentic-warden claude -p openrouter
# Output: 🚀 Starting claude in interactive mode (provider: Some("openrouter"))

# Multiple AI CLI in interactive mode with shared provider
agentic-warden claude,codex -p litellm
# Output: 🚀 Starting claude,codex in interactive mode (provider: Some("litellm"))

# Interactive mode with long format provider flag
agentic-warden gemini --provider custom-proxy
# Output: 🚀 Starting gemini in interactive mode (provider: Some("custom-proxy"))

# Interactive mode with transparent parameter forwarding
agentic-warden claude -p glm --model sonnet --debug api
# Output: 🚀 Starting claude in interactive mode (provider: Some("glm"))
# Forwards: --model sonnet --debug api to Claude CLI

# Interactive mode with multiple CLI parameters
agentic-warden claude -p glm --print --output-format json --allowed-tools Bash,Edit
# Output: 🚀 Starting claude in interactive mode (provider: Some("glm"))
# Forwards: --print --output-format json --allowed-tools Bash,Edit to Claude CLI
```

**Error Handling**:
- Missing provider name after `-p`/`--provider`: Clear error message with usage example
- Invalid provider name: Show available providers and suggest valid alternatives
- Provider compatibility issues: Explain which AI CLI types are supported
- Interactive CLI not found: Suggest installation or alternative CLI types

**Integration Notes**:
- Interactive mode leverages existing `AiCliCommand` infrastructure with empty prompt
- Provider injection works identically to task mode, ensuring consistency
- Process tree tracking continues throughout interactive session
- Task lifecycle follows same pattern: Running → Interactive → Completed

---

### REQ-011: AI CLI 更新/安装管理
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.0
**Related**: ARCH-008, MODULE-002, API-004

**Description**:
Agentic-Warden MUST provide `update` command to manage AI CLI tools (codex, claude, gemini). For each tool, if not installed, install the latest version; if already installed, update to the latest version.

**Acceptance Criteria**:
- [x] `agentic-warden update` updates all installed AI CLI tools to latest version
- [x] `agentic-warden update <tool>` updates/installs specific tool (codex/gemini/claude)
- [x] For codex/gemini: install latest version via npm if not installed
- [x] For codex/gemini: check latest version from npm registry and update if outdated
- [x] For claude: use `claude update` command (when installed)
- [x] For claude: show installation URL when not installed (https://console.anthropic.com/downloads)
- [x] Query npm registry API to get latest version for npm packages
- [x] Execute npm install -g <package>@latest for npm updates
- [x] Provide progress feedback during installation/update
- [x] Handle errors gracefully (network errors, permission issues, etc.)
- [x] Display before/after version information
- [x] Display summary report of all updates

**Technical Constraints**:
- **codex/gemini (NPM packages)**:
  - Use npm registry API: https://registry.npmjs.org/<package>/latest
  - @openai/codex (for codex) - ✅ verified working
  - @google/gemini-cli (for gemini) - ✅ verified working
  - Install/update via: `npm install -g <package>@latest`
  - Use urlencoding::encode for scoped packages

- **claude (Native package)**:
  - Not available on npm
  - Install from: https://console.anthropic.com/downloads
  - Update via: `claude update` command
  - Version detection via `claude --version`

- Common requirements:
  - MUST support proxy environments
  - MUST verify installation success after completion
  - Update process must not interrupt running AI CLI processes

**Error Handling**:
- MUST handle npm not found error (for codex/gemini)
- MUST handle `claude` command not found error
- MUST handle network connectivity issues
- MUST handle permission denied errors
- MUST handle package not found errors
- MUST provide clear error messages with resolution suggestions
- MUST distinguish between npm packages and native packages

---

### REQ-012: 智能MCP路由系统
**Status**: 🟢 Done
**Priority**: P0 (Critical)
**Version**: v0.2.0
**Related**: ARCH-012, DATA-012, API-012

**Description**:
Agentic-Warden MUST provide an intelligent MCP (Model Context Protocol) routing system that acts as a meta-MCP gateway with **dynamic tool registration architecture**. The system leverages Claude Code's automatic tool list refresh mechanism: before each tool use, Claude Code calls `list_tools` to get the current tool list. By maintaining an internal thread-safe tools registry (`DynamicToolManager`), we can dynamically expose only relevant tools to Claude Code, achieving **98% token reduction** (50k → 900 tokens) while maintaining full MCP ecosystem access.

**Acceptance Criteria**:

#### 4.1 MCP配置管理
- [x] Support industry-standard `mcp.json` configuration file format
- [x] Provide configuration schema validation and migration support
- [x] Support per-MCP server configuration including command, args, description, category
- [x] Enable/disable individual MCP servers with runtime configuration updates
- [x] Support health check configuration per MCP server (interval, timeout)
- [x] CLI semantics align with Claude Code/Cursor MCP configuration expectations (mcpServers map, command/args/env/description/category/enabled fields)

#### 4.2 双模式向量数据库集成
- [x] **MCP路由模式**: MemVDB for intelligent MCP tool routing and discovery
- [x] **历史会话模式**: SahomeDB for Claude Code conversation history storage and search
- [x] Tool-level indexing: index MCP tools with descriptions and capabilities for routing
- [x] Method-level indexing: index individual methods with detailed schemas for precise routing
- [x] Conversation history: store and search Claude Code conversation history with semantic search
- [x] Maintain metadata associations between tools and methods
- [x] Support batch indexing and incremental updates
- [x] Provide semantic search capabilities with configurable similarity thresholds
- [x] Memory-only MCP index rebuilt on startup from mcp.json configuration

#### 4.3 Claude Code工具刷新机制利用 [基于观察行为]

**信息来源**:
- 基于Claude Code CLI实际行为观察 (2025-11-14测试验证)
- 参考MCP Protocol Specification - `listChanged` capability定义
- **警告**: 此行为基于观察,非官方文档明确保证,未来版本可能变化

**观察到的Claude Code行为**:
- Claude Code在每次调用工具前会自动调用`list_tools`刷新工具列表
- 刷新触发条件: Server Capabilities声明`"listChanged": true`
- 刷新间隔: 实测 < 1s (可能随Claude Code版本变化)
- 刷新时机: 在每次执行工具调用前自动触发

**我们的设计利用**:
- [x] Leverage Claude Code's automatic `list_tools` refresh before each tool use
- [x] No need for notifications/tools/list_changed - Claude Code pulls updates naturally (Pull模式)
- [x] Maintain thread-safe global tools registry (DynamicToolManager)
- [x] Return base tools + dynamically registered tools in `list_tools` response
- [x] Tools become visible to Claude Code on next refresh (typically < 1s)
- [x] Zero client capability detection needed - works universally with MCP-compliant clients

#### 4.4 智能路由算法 (单层简化架构)

**工具选择决策** (选择最佳MCP工具):
- [x] Two-stage vector search: tool-level → method-level semantic search
- [x] LLM-powered decision with confidence scoring (when Ollama available)
- [x] Fallback to pure vector similarity when LLM unavailable
- [x] FastEmbed for local text embedding generation (AllMiniLML6V2, 384-dim)

**动态工具暴露流程**:
1. **用户请求** → `intelligent_route` tool called by Claude Code
2. **智能选择** → Vector search + LLM decision finds best MCP tool
3. **动态注册** → Register selected tool to DynamicToolManager (thread-safe global registry)
4. **自动刷新** → Claude Code calls `list_tools` before next action (< 1s)
5. **工具可见** → Claude Code sees new tool + full schema, calls it with accurate parameters
6. **代理执行** → Our MCP server proxies call to target MCP server
7. **清理优化** → Unregister unused tools to keep token count minimal

**关键优势**:
- ✅ **98% token reduction**: Only expose 2 base tools (Module 2: `search_history`, Module 3: `intelligent_route`) until needed
- ✅ **零延迟感知**: Claude Code's natural refresh cycle (< 1s) provides seamless UX
- ✅ **无需通知**: No notifications/tools/list_changed required - pull model works perfectly
- ✅ **准确参数**: Claude Code generates parameters with full context, not router guessing
- ✅ **通用兼容**: Works with any MCP client that implements `list_tools` (standard behavior)

#### 4.5 动态工具管理 (DynamicToolManager)
- [x] Thread-safe global registry for dynamically registered tools
- [x] Register tools on-demand when `intelligent_route` selects them
- [x] Track tool → MCP server mappings for proxy execution
- [x] Integrated with `list_tools` - returns base + dynamic tools
- [x] Maintain minimal base tool set (2 tools) to reduce token consumption
- [x] Auto-cleanup: Clear unused tools after configurable timeout
- [x] No notifications needed - Claude Code pulls updates via `list_tools`

#### 4.6 统一MCP接口 (跨模块工具暴露)

**基础工具 (始终可见,来自不同模块)**:

- [x] **intelligent_route** (Module 3: MCP Routing): 智能MCP工具选择和动态注册
  - [x] Accepts: `user_request` (用户需求描述), `session_id` (可选)
  - [x] Returns:
    - `selected_tool`: 选中的工具名称
    - `mcp_server`: 所属MCP服务器
    - `description`: 工具功能说明
    - `registered`: 是否已注册到动态工具列表
  - [x] Side effect: 将选中的工具注册到DynamicToolManager
  - [x] Next step: Claude Code refreshes tools, sees new tool, calls it directly

- [x] **search_history** (Module 2: CC Session Management): 会话历史语义搜索（带TODO上下文）
  - [x] Accepts: `query`, `session_id` (optional), `limit`
  - [x] Returns: Conversation records with embedded TODO items
  - [x] TODO extraction patterns: `- [ ]`, `TODO:`, `Action Items:`
  - [x] Each result includes: conversation context + associated TODO list
  - [x] 数据来源: SahomeDB持久化存储的Claude Code会话历史

**动态代理工具** (按需注册):
- [x] 从mcp.json发现的所有MCP工具
- [x] 通过`intelligent_route`选择后动态注册
- [x] 以原始工具名+schema暴露给Claude Code
- [x] 调用时代理到目标MCP服务器
- [x] 支持参数验证和错误处理
- [x] 执行后自动记录到会话历史

#### 4.7 RMCP客户端集成
- [x] Use rmcp library for dynamic MCP server connections
- [x] Maintain connection pool with health monitoring and auto-reconnection
- [x] Support concurrent MCP server operations with proper isolation
- [x] Provide tool schema discovery and caching from connected MCPs
- [x] Handle MCP server lifecycle (start, stop, restart, health checks)

#### 4.8 内部LLM集成
- [x] Integrate Ollama for internal LLM operations (tool selection decisions)
- [x] Support configurable LLM endpoint via environment variable
- [x] Support configurable LLM model via environment variable
- [x] Implement tool selection prompt engineering and response parsing
- [x] Provide clustering analysis and decision-making capabilities
- [x] Handle LLM fallback and error scenarios gracefully

**Technical Constraints**:

#### New Dependencies for REQ-012:
- **FastEmbed-rs**: Local text embedding generation, replacing Ollama for embeddings
  - Model: AllMiniLML6V2 (default), BGEBaseEN (for knowledge)
  - Zero network dependency, 10-50ms local generation
- **SahomeDB**: File-based vector database for conversation history storage
  - Persistent file storage, no external server required
  - Semantic search capabilities with configurable thresholds
- **MemVDB**: In-memory vector database for MCP tool routing
  - Pure memory operations, rebuilt from mcp.json on startup
  - Thread-safe, multiple distance metrics supported
- **Ollama-rs**: Retained for LLM inference (tool selection decisions), not embeddings

#### Configuration Format (mcp.json):
```json
{
  "mcpServers": {
    "git-server": {
      "command": "mcp-server-git",
      "args": ["--repository", "/workspace"],
      "env": {
        "GIT_REPO_PATH": "/workspace"
      }
    }
  }
}
```

#### Routing Configuration
The intelligent routing system uses hardcoded configuration constants for routing parameters:
- `DEFAULT_MAX_TOOLS_PER_REQUEST`: 10 - Maximum tools to consider per request
- `DEFAULT_CLUSTERING_THRESHOLD`: 0.7 - Vector similarity threshold for tool clustering
- `DEFAULT_RERANK_TOP_K`: 5 - Number of top candidates to rerank
- `DEFAULT_SIMILARITY_THRESHOLD`: 0.5 - Minimum similarity threshold for tool selection

#### 双模式向量数据库架构:
- **SahomeDB** (File-based Persistent): Claude Code conversation history storage
  - **conversation_history**: Store Claude Code conversation history with semantic search
  - Persistent file-based storage across service restarts
  - Metadata: session_id, timestamp, user, tools_used, conversation_context
  - Long-term memory for conversation retrieval and analysis
  - Semantic search for finding relevant past conversations

- **MemVDB** (In-Memory): MCP intelligent routing index
  - **mcp_tools**: Tool-level vectors with description embedding for routing discovery
  - Pure memory mode, rebuilt on startup from mcp.json configuration
  - Metadata: MCP name, tool name, category, capabilities, health status
  - **mcp_methods**: Method-level vectors with detailed schema embedding for precise routing
  - Real-time MCP tool discovery and intelligent routing decisions
  - Metadata: MCP name, method name, parameters, examples, availability
  - Thread-safe, zero dependencies, multiple distance metrics (cosine, euclidean, dot-product)

#### Environment Variables:
- `AGENTIC_WARDEN_LLM_ENDPOINT`: Internal LLM endpoint (default: http://localhost:11434)
- `AGENTIC_WARDEN_LLM_MODEL`: Internal LLM model (default: qwen3:1.7b)
- `AGENTIC_WARDEN_FASTEMBED_MODEL`: FastEmbed model (default: AllMiniLML6V2)

#### Algorithm Requirements:
- Vector search MUST use cosine similarity with configurable thresholds
- Clustering algorithm MUST support top-k selection and similarity grouping
- LLM decisions MUST include confidence scoring and fallback handling
- Route caching MUST respect TTL and invalidation strategies

#### Performance Requirements:
- Tool discovery: < 500ms for typical queries
- Method routing: < 1000ms end-to-end including LLM decisions
- MCP connection pool: Support 10+ concurrent connections
- Vector indexing: Batch operations for 1000+ items efficiently

**Usage Examples** (Claude Code Workflow):

```javascript
// Step 1: Claude Code calls intelligent_route to find the right tool
mcp_call("intelligent_route", {
  "user_request": "I want to check git status and commit all changes",
  "session_id": "session-abc123"
})

// Returns:
// {
//   "selected_tool": "git_status",
//   "mcp_server": "git-server",
//   "description": "Get current git repository status",
//   "registered": true  // Tool now in DynamicToolManager
// }

// Step 2: Claude Code automatically calls list_tools (< 1s later)
// Our list_tools returns: ["intelligent_route", "search_history", "git_status"]

// Step 3: Claude Code sees git_status tool with full schema, calls it directly
mcp_call("git_status", {
  "path": "."
})

// Step 4: Our server proxies to git-server MCP, returns result:
// "On branch main\nChanges not staged for commit:\n  modified:   src/main.rs\n"
```

**Key Insight**: Claude Code handles the tool refresh automatically. We just maintain the global tools registry, and Claude Code discovers new tools via its natural `list_tools` polling.

**Integration Points**:
- rmcp client library for MCP server connections
- Qdrant Server (HTTP API) for persistent historical data
- MemVDB for in-memory MCP routing index
- Ollama integration for internal LLM operations
- Existing memory module for embedding services
- Existing configuration system for mcp.json management

**Technical Dependencies**:
- `memvdb` = "0.1" # Fast, lightweight in-memory vector database
- `rmcp` = { version = "0.5", features = ["server", "transport-io", "macros"] }
- `ollama-rs` = "0.3.1" # For internal LLM communication
- Existing Qdrant HTTP integration (via reqwest)
- Existing embedding service (Ollama)

---

### REQ-013: 动态JS编排工具系统

**Priority**: High
**Status**: 🟢 Done
**Related**: REQ-012, ARCH-012
**Version**: v0.2.0

#### 背景和动机

intelligent_route当前只能选择单个MCP工具,对于复杂的多步骤任务需要用户多次调用工具。通过引入Boa JS引擎和LLM驱动的代码生成,我们可以动态创建组合多个MCP工具的编排工具,一次调用完成复杂工作流。

#### 核心功能需求

#### 5.1 DynamicToolRegistry (动态工具注册表)

**作为MCP工具定义的SSOT**:
- [x] 内部维护所有可被映射到MCP协议的工具定义
- [x] 支持两类工具:基础工具(永久) + 动态工具(带TTL)
- [x] 每次Claude Code调用`list_tools`时从Registry读取工具列表
- [x] 工具名称和schema在TTL内保持稳定不变
- [x] 提供线程安全的并发读写操作

**基础工具管理**:
- [x] 启动时初始化基础工具: `intelligent_route`, `search_history`
- [x] 基础工具永久存在,不受TTL影响

**动态工具管理**:
- [x] 支持注册两种动态工具类型:
  - JS编排工具 (`JsOrchestratedTool`)
  - 代理MCP工具 (`ProxiedMcpTool`)
- [x] 每个动态工具带有TTL = **600秒(10分钟)**
- [x] 自动清理过期工具(后台任务,每60秒检查一次)
- [x] 支持最大动态工具数限制(默认100个),超出时驱逐最旧工具
- [x] 记录工具注册时间、执行次数等元数据

#### 5.2 intelligent_route LLM优先路由 (带Fallback)

**路由决策逻辑**:
- [x] **LLM不存在** → 直接使用向量搜索模式(不尝试LLM,节省时间)
- [x] **LLM存在** → 优先尝试LLM编排,失败则fallback到向量搜索

**执行流程**:
```rust
match js_orchestrator {
    None => vector_mode(),           // LLM不存在,直接vector
    Some(orch) => {
        match try_llm_orchestrate() {
            Ok(result) => result,     // LLM成功
            Err(_) => vector_mode(),  // LLM失败,fallback
        }
    }
}
```

**LLM编排模式** (优先尝试):
- [x] LLM分析用户任务,规划所需步骤和MCP工具
- [x] 检查是否有合适的工具支持,不可行时返回Err触发fallback
- [x] **智能注册决策** (needs_orchestration字段):
  - `needs_orchestration = false`: 单工具直接透传,注册为代理工具(ProxiedMcpTool)
  - `needs_orchestration = true`: 需要JS编排,生成JS代码并注册为编排工具(JsOrchestratedTool)
- [x] **触发JS编排的条件** (任一满足):
  - 多步骤工作流(steps > 1)
  - 需要转换/过滤/聚合输出数据
  - 需要条件逻辑或循环
  - 需要组合多个工具的结果
- [x] **直接代理的条件** (全部满足):
  - 单步骤(steps = 1)
  - 输入参数直接透传到目标工具
  - 无输出处理需求
- [x] JS函数内部调用注入的MCP工具(以`mcp`前缀暴露)
- [x] 代码验证失败时返回Err触发fallback
- [x] 返回消息: "Use the 'xxx' tool to solve your problem"

**向量搜索模式** (Fallback保障):
- [x] 两层向量搜索(工具级 + 方法级)
- [x] 聚类算法筛选出top-N候选工具(默认5个)
- [x] 批量注册为代理工具到Registry(透传原始MCP定义)
- [x] 返回消息: "Found N relevant tools. Choose which ones to use: ..."

**Fallback触发条件**:
- LLM环境未配置 (`js_orchestrator = None`)
- LLM网络请求超时或失败
- LLM返回无效响应或代码
- JS代码验证失败(语法错误、安全检查未通过)
- LLM判断任务不可行

#### 5.3 Boa JS引擎集成

**安全沙箱**:
- [x] 使用Boa引擎提供安全的JS运行时环境
- [x] 禁用危险全局对象: `eval`, `Function`, `require`, `import`, `fetch`, `XMLHttpRequest`
- [x] 实现执行时间限制(最大10分钟)
- [x] 提供安全的`console.log`(仅用于调试)

**MCP函数注入**:
- [x] 将可用的MCP工具注入为JS异步函数
- [x] 函数命名规范: `mcp` + CamelCase (例: `git_status` → `mcpGitStatus`)
- [x] 注入函数实现异步调用RMCP Client Pool
- [x] 支持参数解析和结果转换(JSON ↔ JS Value)
- [x] 错误处理和异常传播

**运行时池管理**:
- [x] 使用连接池管理Boa运行时实例(复用,减少初始化开销)
- [x] 支持并发执行多个JS工具
- [x] 运行时隔离(每次执行独立的context)

#### 5.4 LLM驱动的代码生成

**工作流规划**:
- [x] LLM分析用户任务和可用MCP工具列表
- [x] 判断任务是否可行(is_feasible: true/false)
- [x] 规划执行步骤(steps: [{step, tool, description}, ...])
- [x] 确定所需输入参数(因为MCP只接收"做什么",不包含具体上下文)
- [x] 建议工具名称(snake_case)和描述

**JS代码生成**:
- [x] 根据规划生成完整的`async function workflow(input) {...}`
- [x] 生成的代码使用注入的MCP函数(mcp前缀)
- [x] 包含错误处理(try-catch)
- [x] 包含注释说明每个步骤
- [x] 返回结构化结果对象

**代码验证**:
- [x] 语法检查(使用Boa解析)
- [x] 安全性检查(检测危险模式: eval, new Function, __proto__等)
- [x] 检查只使用允许的MCP函数
- [x] Dry-run测试(使用mock数据执行一次)

#### 5.5 工具执行和生命周期

**JS工具执行**:
- [x] 从Registry获取工具定义
- [x] 从运行时池获取Boa实例
- [x] 注入所需的MCP函数
- [x] 执行JS代码,传入用户参数
- [x] 返回结果给Claude Code
- [x] 更新执行计数统计

**代理工具执行**:
- [x] 从Registry获取工具定义
- [x] 通过RMCP Client Pool代理到目标MCP服务器
- [x] 透传参数和结果
- [x] 更新执行计数统计

**TTL和清理**:
- [x] 动态工具TTL = **600秒(10分钟)**
- [x] 后台任务每60秒检查并清理过期工具
- [x] 超出最大工具数时驱逐最旧工具
- [x] 清理时记录日志

#### 技术约束

**新增依赖**:
```toml
boa_engine = "0.17"         # Rust实现的JavaScript引擎
boa_gc = "0.17"             # Boa垃圾回收
swc_ecma_parser = "0.142"   # 快速JS解析器(用于验证)
swc_ecma_ast = "0.110"      # AST分析
deadpool = "0.10"           # 异步对象池(Boa运行时池)
regex = "1.10"              # 危险模式检测
```

**性能目标**:
| 操作 | 目标延迟 | 说明 |
|-----|---------|------|
| LLM规划 | < 3s | Ollama本地推理 |
| JS代码生成 | < 3s | Ollama本地推理 |
| 代码验证 | < 100ms | 语法+安全检查 |
| Boa初始化 | < 50ms | 从池获取 |
| MCP函数注入 | < 200ms | 批量注册 |
| JS工具执行 | < 30s | 取决于MCP调用数 |
| 工具注册 | < 10ms | 写入Registry |
| list_tools响应 | < 50ms | 读取Registry |

**安全要求**:
- JS代码必须通过安全性检查
- 禁止使用危险的JavaScript特性
- 执行时间和内存限制
- 运行时隔离

**可靠性要求**:
- 工具注册失败不影响现有工具
- JS执行错误不导致服务崩溃
- 过期工具清理不阻塞主流程

---

### REQ-014: AI CLI任务生命周期管理和角色系统

**Priority**: High
**Status**: ✅ Done (Phase 1-4 Completed) [commit: a2cc2e2]
**Related**: ARCH-014
**Version**: v0.2.0 → v0.6.x (Phase 4 已完成)

#### 背景和动机

Claude Code通过MCP调用AI CLI工具时,缺乏对任务的完整生命周期管理能力。用户无法:
1. 查看当前正在运行的AI CLI任务
2. 通过MCP工具启动/停止后台任务
3. 获取任务执行日志
4. 加载预定义的AI角色配置

通过实现任务生命周期MCP工具和角色系统,Claude Code可以更灵活地管理AI CLI任务执行。

#### Phase 1: 角色系统 (✅ v0.2.0 已完成)

**1.1 角色文件管理**:
- [x] 角色配置存储在`~/.aiw/role/`目录
- [x] 使用Markdown格式(.md文件)
- [x] 文件格式: `<description>\n------------\n<content>`
- [x] Description: 角色简短描述(用于列表展示)
- [x] Content: 完整的角色提示词内容

**1.2 Role数据结构**:
- [x] `Role` struct: {name, description, content, file_path}
- [x] `RoleInfo` struct: {name, description, file_path} (轻量级,用于MCP返回)
- [x] `RoleManager`: 角色管理器,负责扫描和解析角色文件
- [x] `RoleError`: 自定义错误类型(NotFound, InvalidName, PathTraversal, FileTooLarge, InvalidEncoding, InvalidFormat, HomeDirectoryUnavailable, Io)

**1.3 安全约束**:
- [x] 路径穿越防护: 使用`fs::canonicalize()` + `starts_with()`验证
- [x] 文件大小限制: 最大1MB
- [x] UTF-8编码验证: 拒绝非UTF-8文件
- [x] 文件名验证: 阻止路径分隔符和遍历符号
- [x] 字符集限制: 角色名仅允许 `[A-Za-z0-9_-]` (字母、数字、下划线、连字符)
- [x] 分隔符验证: 必须包含12个短横线`------------`

**1.4 MCP工具**:
- [x] `list_roles`: 列出所有可用角色配置
  - 返回: `Vec<RoleInfo>`
  - 使用`RoleManager::list_all_roles()`
  - 自动过滤非.md文件
  - 按名称排序

**1.5 单元测试** (tests/roles_tests.rs):
- [x] 角色文件正确解析(带分隔符)
- [x] list_all_roles返回所有角色
- [x] 文件不存在时的错误处理
- [x] 路径穿越防护测试
- [x] 文件大小限制测试
- [x] 字符集限制验证测试 (仅允许`[A-Za-z0-9_-]`)

**1.6 实现文件**:
- [x] `src/roles/mod.rs` (282 lines): 核心角色管理模块
- [x] `src/mcp/mod.rs:347-356`: MCP工具`list_roles`集成
- [x] `src/lib.rs:25`: 模块导出
- [x] `tests/roles_tests.rs` (135 lines, 6 tests): 单元测试

#### Phase 2: 任务生命周期MCP工具 (✅ v0.2.0 已完成)

**2.1 start_task工具**:
- [x] 在后台启动AI CLI任务
- [x] 参数: {ai_type, task, provider?, role?}
- [x] 返回: {pid, log_file, status}
- [x] 使用`supervisor::execute_cli()`启动进程
- [x] 注册到MCP Registry (InProcessRegistry)
- [x] 角色注入: 支持role参数,格式`{role.content}\n\n---\n\n{task}`

**2.2 stop_task工具**:
- [x] 停止指定PID的任务
- [x] 参数: {pid}
- [x] 返回: {success, message}
- [x] 发送SIGTERM信号,超时后SIGKILL
- [x] 从Registry中标记完成

**2.3 list_tasks工具**:
- [x] 列出所有追踪的任务
- [x] 返回: `Vec<TaskInfo>` {pid, log_id, log_path, status, started_at}
- [x] 使用`MCP Registry::entries()`
- [x] 过滤已退出的僵尸任务(platform::process_alive)

**2.4 get_task_logs工具**:
- [x] 获取任务日志内容
- [x] 参数: {pid, tail_lines?}
- [x] 返回: {log_content, log_file}
- [x] 支持tail模式(最后N行)
- [x] 文件读取错误处理

#### Phase 3: 角色集成到任务启动 (✅ v0.2.0 已完成)

**3.1 start_task支持role参数**:
- [x] 可选参数`role`: 角色名称
- [x] 自动从`~/.aiw/role/`加载角色内容(RoleManager::get_role)
- [x] 将角色content注入到AI CLI的prompt中
- [x] 格式: `{role_content}\n\n---\n\n{user_task}`

**3.2 角色验证和错误处理**:
- [x] 角色不存在时返回错误
- [x] 角色文件格式错误时返回详细信息
- [x] 支持空role参数(不使用角色)

#### 技术约束

**依赖**:
```toml
walkdir = "2"          # 目录遍历 (已添加)
thiserror = "2"        # 错误处理 (已添加)
schemars = "0.8"       # MCP schema生成 (已有)
```

**现有模块复用**:
- `supervisor`模块(任务启动)
- `registry_factory`(MCP Registry)
- `task_record`(任务记录)

**错误处理**:
- 使用`RoleError`枚举覆盖所有角色相关错误
- 所有I/O操作包装在Result中
- MCP工具返回清晰的错误消息

**性能要求**:
- 角色列表查询: < 100ms (目录扫描+解析)
- 角色文件解析: < 10ms (单文件,1MB以内)
- 任务列表查询: < 10ms (从Registry读取)

#### 验收标准

**Phase 1 (✅ 已完成)**:
- [x] `cargo test --test roles_tests`全部通过 (6/6 tests)
- [x] MCP工具`list_roles`可在Claude Code中调用
- [x] 路径穿越攻击被正确阻止
- [x] 文件大小超限被正确拒绝
- [x] UTF-8编码验证正常工作
- [x] 字符集限制正常工作 (仅允许`[A-Za-z0-9_-]`)
- [x] 无TODO/FIXME/stub函数
- [x] 完整错误处理和边界检查

**Phase 2-3 (✅ v0.2.0 已完成)**:
- [x] `cargo test --test task_lifecycle_tests`全部通过 (5/5 tests)
- [x] MCP工具`start_task`可启动后台任务并返回PID
- [x] MCP工具`stop_task`可正确终止任务(SIGTERM→SIGKILL机制)
- [x] MCP工具`list_tasks`返回所有追踪任务(过滤僵尸进程)
- [x] MCP工具`get_task_logs`可读取任务日志(支持全文和tail模式)
- [x] `start_task`的role参数可正确加载角色
- [x] 角色content正确注入到AI CLI prompt (格式: `{role.content}\n\n---\n\n{task}`)
- [x] 角色验证错误返回清晰消息
- [x] 实现文件: `src/mcp/mod.rs:115-412` (4个MCP工具), `tests/task_lifecycle_tests.rs` (5个集成测试)

#### Phase 4: 多角色支持 (✅ v0.6.x 已完成) [commit: a2cc2e2]

**4.1 多角色参数语法**:
- [x] CLI支持逗号分隔的多角色参数: `-r common,security,testing`
- [x] 解析角色列表，去除空白字符
- [x] 角色名称验证复用现有字符集限制: `[A-Za-z0-9_-]`
- [x] 角色顺序保持用户指定顺序（影响组合顺序）
- [x] 角色去重（保持首次出现的顺序）

**4.2 多角色加载和组合**:
- [x] 遍历角色列表，逐个调用`RoleManager::get_role()`
- [x] 跳过无效/不存在的角色，记录警告日志
- [x] 组合所有有效角色的content，使用分隔符连接
- [x] 组合格式: `{role1.content}\n\n---\n\n{role2.content}\n\n---\n\n{task}`

**4.3 错误处理和兜底策略**:
- [x] 部分角色无效: 跳过无效角色，继续使用有效角色
- [x] **全部角色无效: 使用`common`角色作为兜底**
- [x] 兜底角色不存在: 返回原始prompt，不中断执行
- [x] 显示警告信息告知用户哪些角色被跳过

**4.4 MCP工具支持**:
- [x] `start_task`工具的`role`参数支持逗号分隔格式
- [x] 多角色解析和组合逻辑复用CLI实现
- [x] 角色去重和兜底策略与CLI一致

**4.5 单元测试**:
- [x] 多角色解析测试（逗号分隔、空白处理）- 9个测试用例
- [x] 部分角色无效的处理测试
- [x] 全部角色无效的兜底测试（使用common）
- [x] 角色组合格式测试
- [x] RoleManager::get_roles() 批量加载测试
- [x] get_builtin_roles() 内置角色批量加载测试

**实现文件**: `src/commands/ai_cli.rs`, `src/mcp/mod.rs`, `src/roles/mod.rs`, `src/roles/builtin.rs`

**Phase 4 技术约束**:

**角色组合格式**:
```
# 多角色注入格式（按用户指定顺序）
{role1.content}

---

{role2.content}

---

{role3.content}

---

{user_task}
```

**错误处理矩阵**:

| 场景 | 处理方式 | 用户提示 |
|------|----------|----------|
| 全部角色有效 | 组合所有角色 | 无 |
| 部分角色无效 | 跳过无效，使用有效 | ⚠️ 角色 'xxx' 不存在，已跳过 |
| 全部角色无效 | 使用`common`角色兜底 | ⚠️ 所有指定角色无效，使用默认角色 common |
| `common`角色也不存在 | 报错，不执行任务 | ❌ 默认角色 common 不存在，请检查角色配置 |

**性能要求**:
- 多角色解析: < 10ms
- 多角色加载(10个): < 100ms
- 角色组合: < 10ms

**Phase 4 验收标准**:
- [x] `cargo test` 9个多角色相关测试全部通过
- [x] CLI命令`aiw claude -r common,security "task"`正常工作
- [x] MCP工具`start_task`支持多角色参数
- [x] 全部角色无效时正确使用common兜底
- [x] 警告信息清晰明确

---

## Non-Functional Requirements

### NFR-001: 性能要求
**Type**: Performance
**Status**: 🟢 Done
**Version**: v0.1.0

**Description**:
Agentic-Warden MUST meet performance criteria for process tracking and task management.

**Metrics**:
- Process detection: < 100ms for 100 processes
- TUI rendering: < 16ms per frame (60 FPS)
- Shared memory access: < 1ms per operation
- Task status updates: Real-time (< 100ms delay)

**Acceptance Criteria**:
- [x] All performance tests pass under normal load
- [x] No UI lag during concurrent AI CLI operations
- [x] Shared memory operations do not block AI CLI processes

---

### NFR-002: 跨平台兼容性
**Type**: Compatibility
**Status**: 🟢 Done
**Version**: v0.1.0

**Requirements**:
- MUST support Windows 10+ (x86_64, ARM64)
- MUST support Linux (x86_64, ARM64)
- MUST support macOS 10.15+ (x86_64, ARM64)
- Rust version requirement: 1.70+

**Acceptance Criteria**:
- [x] All features work on Windows
- [x] All features work on Linux
- [x] All features work on macOS
- [x] Cross-platform CI tests pass

---

### NFR-003: 安全性
**Type**: Security
**Status**: 🟢 Done
**Version**: v0.1.0

**Requirements**:
- API tokens MUST be stored encrypted at rest
- OAuth tokens MUST use secure storage
- Process isolation MUST prevent unauthorized access
- Configuration files MUST have appropriate permissions (600)

**Acceptance Criteria**:
- [x] Tokens encrypted in storage
- [x] OAuth tokens stored securely
- [x] Shared memory access control implemented
- [x] Configuration files have restricted permissions

---

## User Stories

### US-001: 多AI用户统一管理
**Role**: As a developer using multiple AI CLI tools
**Goal**: I want to manage all AI CLIs from a single interface
**Value**: So that I can switch between AI tools seamlessly and track all tasks

**Linked Requirements**: REQ-001, REQ-002, REQ-003, REQ-004

**Acceptance Criteria**:
- Given I have codex, claude, and gemini installed
- When I run agentic-warden status
- Then I should see status of all three tools
- And I can launch any tool with provider configuration

---

## Requirements Traceability Matrix

| Requirement ID | Title | Priority | Status | Version | SPEC References | Git Commits |
|----------------|-------|----------|--------|---------|-----------------|-------------|
| REQ-001 | AI CLI 进程树追踪 | P0 | 🟢 Done | v0.1.0 | ARCH-001, DATA-001 | Initial commit |
| REQ-002 | 第三方 Provider 管理 | P0 | 🟢 Done | v0.1.0 | ARCH-002, DATA-002, API-001 | Initial commit |
| REQ-003 | Google Drive 同步集成 | P1 | ❌ Deprecated | v0.1.0 | ARCH-003, DATA-003 | Disabled since v0.5.19 |
| REQ-004 | 统一 TUI 体验 | P1 | 🟢 Done | v0.1.0 | ARCH-004, MODULE-001 | Initial commit |
| REQ-005 | Wait 模式跨进程等待 | P2 | 🟢 Done | v0.1.0 | ARCH-005, DATA-005, API-002 | Initial commit |
| REQ-006 | AI CLI 工具检测与状态管理 | P1 | 🟢 Done | v0.1.0 | ARCH-006, MODULE-002 | Initial commit |
| REQ-007 | MCP 服务器 | P1 | 🟢 Done | v0.1.0 | ARCH-007, API-003 | Initial commit |
| REQ-008 | 指定供应商模式 AI CLI 启动 | P0 | 🟢 Done | v0.1.0 | ARCH-002, ARCH-008, API-004 | Initial commit |
| REQ-009 | 交互式 AI CLI 启动 | P1 | 🟢 Done | v0.1.1 | ARCH-008, API-001 | Interactive mode implementation |
| REQ-010 | 内存集成与语义搜索 | P1 | ❌ Deprecated | v0.1.0 | ARCH-010, DATA-003, API-005 | 功能已删除 |
| REQ-011 | AI CLI 更新/安装管理 | P1 | 🟢 Done | v0.1.0 | ARCH-008, MODULE-002, API-004 | Update command implementation |
| REQ-012 | 智能MCP路由系统 | P0 | 🟢 Done | v0.2.0 | ARCH-012, DATA-012, API-012 | Intelligent routing system design |
| REQ-024 | OpenAI环境变量配置 | P0 | 🟢 Done | v5.1.1 | ARCH-013, API-013 | OpenAI environment variable configuration |
| REQ-014 | AI CLI任务生命周期管理和角色系统 | P1 | ✅ Done | v0.2.0→v0.6.x | ARCH-014 | Phase 1-4 全部完成 [commit: a2cc2e2] |
| REQ-015 | 简化的Google Drive OAuth授权流程 | P0 | ❌ Deprecated | v0.5.18 | ARCH-003, DATA-003 | Disabled since v0.5.19 |
| REQ-016 | MCP仓库CLI - 多源聚合搜索与安装 | P1 | ✅ Done | v0.6.0 | ARCH-016, API-016 | MCP Registry CLI implementation |
| REQ-022 | Auto 模式 CLI+Provider 组合轮转 | P1 | 🟢 Done | v0.5.48+ | REQ-021, ARCH-021, DATA-022, DATA-023 | CLI+Provider rotation in auto mode |
| REQ-023 | Git 仓库检查和 Worktree 管理 | P0 | 🟢 Done | v0.5.48+ | ARCH-023 | Git repository check and worktree management |
| REQ-018 | MCP Browse 环境变量快速跳过 | P1 | 🟡 Design | v0.6.1 | ARCH-018 | Skip optional env vars feature |
| REQ-019 | MCP Browse - 已安装MCP服务器查看 | P1 | 🟡 Design | v0.6.1 | ARCH-019, DATA-019 | View installed MCPs feature |
| REQ-020 | MCP Browse - 已安装MCP环境变量编辑 | P1 | 🟡 Design | v0.6.1 | ARCH-020, DATA-020 | Edit env vars for installed MCPs |
| REQ-017 | AIW插件市场系统 | P1 | 🟡 Partial Implementation | v0.7.0 | ARCH-017, DATA-017, API-017 | Plugin marketplace system |

---

### REQ-016: MCP仓库CLI - 多源聚合搜索与安装
**Status**: ✅ Done (手写代码)
**Priority**: P1 (High)
**Version**: v0.6.0
**Related**: ARCH-015, API-015

**Description**:
Agentic-Warden MUST provide CLI commands to search, install, and manage MCP servers from multiple registry sources (Official MCP Registry, Smithery.ai), with interactive selection and environment variable configuration to eliminate copy-paste friction.

**Acceptance Criteria**:

#### 6.1 多源聚合搜索
- [ ] 支持官方MCP Registry (`registry.modelcontextprotocol.io/v0.1`)
- [ ] 支持Smithery.ai Registry (`registry.smithery.ai`)
- [ ] 可扩展的Source trait设计，支持未来添加更多源
- [ ] 并行查询多源，合并去重结果
- [ ] 结果按相关度和源优先级排序
- [ ] `--source` 参数指定单一源查询

#### 6.2 交互式选择安装
- [ ] 搜索结果显示编号列表（#1, #2, #3...）
- [ ] 用户输入编号直接安装，无需复制名称
- [ ] 显示每个结果的来源标识（registry/smithery）
- [ ] 显示服务器描述和类型（npm/uvx/docker）
- [ ] 支持'q'退出选择

#### 6.3 环境变量交互式配置
- [ ] 安装时自动检测必需的环境变量
- [ ] 显示环境变量说明（description from registry）
- [ ] 交互式提示用户输入值
- [ ] 自动检测已存在的系统环境变量
- [ ] 支持`--env KEY=VALUE`参数直接传入
- [ ] 支持`--skip-env`跳过环境变量配置
- [ ] 配置文件中使用`${ENV_VAR}`引用格式

#### 6.4 服务器类型自动检测
- [ ] npm类型：生成`npx -y @scope/package`命令
- [ ] uvx类型：生成`uvx package-name`命令
- [ ] docker类型：生成`docker run image`命令
- [ ] remote类型：直接使用远程URL

#### 6.5 配置管理
- [ ] 安装后自动写入`~/.aiw/mcp_servers.yaml`
- [ ] 记录安装来源（source字段）
- [ ] `aiw mcp list`列出已安装服务器
- [ ] `aiw mcp remove <name>`移除服务器
- [ ] `aiw mcp info <name>`查看详细信息

#### 6.6 源配置（可选）
- [ ] `~/.aiw/registry.yaml`配置源优先级和认证
- [ ] Smithery API Key配置（可选，增强功能）
- [ ] 源启用/禁用控制

**CLI Commands**:
```bash
# 搜索（并行查询所有源）
aiw mcp search filesystem
aiw mcp search "git version control"
aiw mcp search filesystem --source registry

# 交互式安装
aiw mcp search filesystem
# → 显示编号列表
# → 用户输入编号
# → 提示环境变量
# → 完成安装

# 直接安装
aiw mcp install @anthropic/filesystem
aiw mcp install smithery:exa
aiw mcp install context7 --source smithery

# 带环境变量安装
aiw mcp install @anthropic/github --env GITHUB_TOKEN=$GITHUB_TOKEN

# 跳过环境变量
aiw mcp install @anthropic/exa --skip-env

# 管理
aiw mcp list
aiw mcp remove filesystem
aiw mcp info @anthropic/git
aiw mcp update  # 更新仓库索引缓存
```

**Technical Constraints**:

**新增依赖**:
```toml
dialoguer = "0.11"     # 交互式CLI提示
indicatif = "0.17"     # 进度条和spinner
```

**API端点**:
- Official Registry: `GET /v0.1/servers?search={query}&limit=96`
- Smithery: `GET /servers?q={query}&page=1&pageSize=10` (需Bearer Token)

**性能要求**:
- 搜索响应: < 3s（并行查询）
- 安装完成: < 10s（不含下载时间）
- 配置写入: < 100ms

**安全要求**:
- Smithery API Key安全存储（0600权限）
- 环境变量值在日志中脱敏
- 配置文件中敏感值使用引用格式

---

---

### REQ-017: AIW插件市场系统
**Status**: 🟡 Partial Implementation
**Priority**: P1 (High)
**Version**: v0.7.0
**Related**: ARCH-017, DATA-017, API-017

**Description**:
Agentic-Warden MUST provide a complete plugin marketplace system compatible with Claude Code plugin format,专注于MCP服务器的管理和安装。系统支持多个市场源（Claude Code官方市场、AIW官方市场），自动过滤只显示包含MCP服务器配置的插件，并从插件中提取MCP配置写入AIW的MCP配置文件。

**Background**:
AIW当前的MCP Registry CLI（REQ-016）从远程API搜索和安装MCP服务器。Claude Code提供了标准的插件市场格式（marketplace.json + plugin.json），包含了大量高质量的MCP服务器插件。本需求将MCP Registry CLI升级为完整的插件市场系统，复用Claude Code插件生态。

**Acceptance Criteria**:

#### 7.1 市场源管理
- 支持添加GitHub仓库作为市场源
- 支持添加本地路径作为市场源
- 支持添加远程URL作为市场源
- 默认包含Claude Code官方市场（anthropics/claude-plugins-official）
- 默认包含AIW官方市场（putao520/aiw-plugins）
- 列出所有已配置的市场源
- 移除指定的市场源
- 更新市场源索引（从GitHub拉取最新marketplace.json）

#### 7.2 插件发现和过滤
- 从所有市场源并行查询插件列表
- 读取marketplace.json获取插件清单
- 读取plugin.json获取插件元数据
- **严格MCP-only过滤**：只显示plugin.json中包含mcpServers字段的插件
- 忽略仅包含commands/agents/skills/hooks的插件
- 支持按名称、描述、分类、标签搜索插件
- 显示插件的完整元数据（名称、版本、描述、作者、类别）

#### 7.3 插件浏览
- 交互式TUI界面浏览所有可用插件
- 显示插件列表（名称、描述、来源市场）
- 支持模糊搜索插件
- 支持按类别筛选插件
- 支持按来源市场筛选插件
- 显示插件详细信息（版本、作者、仓库、许可证）
- 显示插件包含的MCP服务器列表

#### 7.4 插件安装
- 从指定市场源安装插件
- 克隆/下载插件到本地缓存目录
- 解析plugin.json和.mcp.json
- **提取MCP配置**：从插件中提取mcpServers配置
- **写入~/.aiw/mcp.json**：将提取的MCP配置合并到AIW的MCP配置文件
- 交互式环境变量配置
- 检测必需的环境变量
- 提示用户输入环境变量值
- 自动检测已存在的系统环境变量
- 支持--env参数直接传入环境变量
- 支持--skip-env跳过环境变量配置
- 配置文件中使用${ENV_VAR}引用格式
- 记录插件安装信息（名称、版本、市场源、安装时间）

#### 7.5 插件管理
- 列出所有已安装的插件
- 显示插件详细信息
- 启用/禁用已安装的插件
- 卸载插件（从mcp.json移除对应的MCP服务器配置）
- 更新插件（拉取最新版本并重新安装）

#### 7.6 配置文件格式（JSON）
- **~/.aiw/settings.json**：市场源和插件启用状态
- **~/.aiw/plugins.json**：已安装插件记录
- **~/.aiw/mcp.json**：MCP服务器配置（与Claude Code格式一致）

**CLI Commands**:
```bash
# 市场源管理
aiw plugin marketplace add <repo-url>
aiw plugin marketplace add putao520/aiw-plugins
aiw plugin marketplace list
aiw plugin marketplace remove <marketplace-name>
aiw plugin marketplace update

# 插件浏览和搜索
aiw plugin browse                       # 交互式TUI浏览
aiw plugin search <query>               # 搜索插件
aiw plugin search "git" --source aiw-official
aiw plugin search "filesystem" --category system

# 插件安装
aiw plugin install <plugin-name>@<marketplace>
aiw plugin install github-mcp@aiw-official
aiw plugin install github-mcp@aiw-official --env GITHUB_TOKEN=$GITHUB_TOKEN
aiw plugin install filesystem-mcp@claude-code-official --skip-env

# 插件管理
aiw plugin list                        # 列出已安装插件
aiw plugin info <plugin-name>           # 查看插件详情
aiw plugin remove <plugin-name>         # 卸载插件
aiw plugin enable <plugin-name>         # 启用插件
aiw plugin disable <plugin-name>        # 禁用插件
```

**Technical Constraints**:

**新增依赖**:
```toml
dialoguer = "0.11"     # 交互式CLI提示（已有）
indicatif = "0.17"     # 进度条和spinner（已有）
git2 = "0.18"          # Git仓库克隆
```

**Claude Code插件格式兼容性**:
- 支持读取.claude-plugin/marketplace.json
- 支持读取.claude-plugin/plugin.json
- 支持相对路径和GitHub仓库source格式
- 支持环境变量${CLAUDE_PLUGIN_ROOT}引用
- 严格验证plugin.json schema

**MCP配置提取规则**:
- 从plugin.json的mcpServers字段读取配置
- 支持内联MCP配置（mcpServers为对象）
- 支持外部MCP配置文件（mcpServers为文件路径字符串）
- 支持${ENV_VAR}环境变量引用格式
- 合并多个插件的MCP配置到同一个mcp.json
- 冲突处理：后安装的插件覆盖同名MCP服务器（警告用户）

**性能要求**:
- 市场源更新: < 5s（克隆GitHub仓库）
- 插件搜索: < 2s（扫描本地缓存）
- 插件安装: < 10s（不含克隆时间）
- 配置写入: < 100ms

**安全要求**:
- 私有仓库访问通过SSH认证
- 环境变量值在日志中脱敏
- 配置文件权限0600
- 插件缓存目录权限0700
- 验证plugin.json签名（未来）

**数据迁移**:
- 从mcp_servers.yaml迁移到mcp.json
- 自动检测并迁移旧配置文件
- 备份旧配置文件

---

<!--
更新记录 (2025-12-26):
- 添加REQ-017: AIW插件市场系统
- 状态: v0.7.0新功能设计
- 架构决策: 完全重构（统一Market系统）
- 命名空间: 独立plugin命令
- 配置格式: 迁移到JSON（与Claude Code一致）
- 过滤策略: 严格MCP-only

更新记录 (2025-12-09):
- 添加REQ-016: MCP仓库CLI - 多源聚合搜索与安装
- 状态: v0.6.0新功能规划

更新记录 (2025-11-25):
- REQ-003 (Google Drive同步): 已从删除状态恢复
- REQ-010 (CC会话历史): 保持删除状态

说明: Google Drive同步是通用基础设施，与CC会话系统无技术依赖，
因此根据用户反馈恢复此功能。
-->

---

### REQ-018: MCP Browse 环境变量快速跳过
**Status**: ✅ 已实现 (2025-12-26) [commit: 3a9f72a]
**Priority**: P1 (High)
**Version**: v0.6.1
**Related**: ARCH-018, REQ-016

**Description**:
Agentic-Warden MCP Browse TUI MUST support one-key skip for all remaining optional environment variables during interactive MCP installation. Users can quickly skip optional configuration steps without entering individual values.

**Acceptance Criteria**:
- [ ] In environment variable input dialog, display hint when optional variables remain
- [ ] Pressing 'a' or 'A' key skips all remaining optional environment variables
- [ ] Only works when current variable is optional (skip has no effect on required vars)
- [ ] Returns to MCP installation with remaining required variables still collected
- [ ] Behavior is intuitive and non-destructive (can still edit optional vars individually)
- [ ] Unit test: test_env_input_skip_all_optional passes

**Technical Constraints**:
- Integration with existing `EnvInputState` in `src/commands/mcp/registry/browse.rs`
- Extends enum-based event handling pattern (no major refactor)
- Display hint in env dialog footer when optional vars exist
- Call `skip_all_optional()` method on `EnvInputState`

---

### REQ-019: MCP Browse - 已安装MCP服务器查看
**Status**: ✅ 已实现 (2025-12-26) [commit: 3a9f72a]
**Priority**: P1 (High)
**Version**: v0.6.1
**Related**: ARCH-019, REQ-016, DATA-019

**Description**:
Agentic-Warden MCP Browse MUST support viewing, searching, and filtering installed MCP servers from `~/.aiw/mcp.json`. Users can browse their installed ecosystem without leaving the TUI.

**Acceptance Criteria**:
- [ ] New menu option "Installed MCPs" in Browse main screen (Press 'i')
- [ ] Display list of installed MCP servers with name, description, enabled status
- [ ] Search by name using '/' key (real-time filtering)
- [ ] Filter by status (enabled/disabled) using keyboard
- [ ] Show environment variable count per MCP
- [ ] Press Enter on MCP to view detailed configuration
- [ ] Details view shows: name, source, environment variables, command
- [ ] Return to list with ESC key
- [ ] Return to main Browse menu with ESC from list view
- [ ] Handle empty list gracefully (no installed MCPs)
- [ ] Integration test passes

**Data Structures**:
```rust
struct InstalledMcpListItem {
    name: String,
    description: String,
    enabled: bool,
    env_var_count: usize,
    source: String,
}

struct InstalledMcpScreen {
    items: Vec<InstalledMcpListItem>,
    selected_index: usize,
    search_query: String,
    filtered_items: Vec<usize>,
}
```

**Technical Constraints**:
- Read from `McpConfigManager::load()` existing implementation
- Reuse existing screen pattern from `DashboardScreen`, `StatusScreen`
- Implement `Screen` trait
- No new dependencies required
- File: `src/tui/screens/installed_mcp.rs` (new)

---

### REQ-020: MCP Browse - 已安装MCP环境变量编辑
**Status**: ✅ 已实现 (2025-12-26) [commit: 3a9f72a]
**Priority**: P1 (High)
**Version**: v0.6.1
**Related**: ARCH-020, REQ-019, DATA-020

**Description**:
Agentic-Warden MCP Browse MUST support editing and resetting environment variables for already-installed MCP servers. Users can modify MCP configurations without manual file editing.

**Acceptance Criteria**:
- [ ] From installed MCP list (REQ-019), press 'e' to enter edit mode
- [ ] Reuse existing `EnvInputState` component for variable input
- [ ] Preload existing variable values for editing
- [ ] Support updating variables to new values
- [ ] Support clearing optional variables
- [ ] Press 's' to save changes back to `~/.aiw/mcp.json`
- [ ] Press 'Esc' to cancel editing without saving
- [ ] Show confirmation before overwriting config
- [ ] Graceful error handling for file write failures
- [ ] Log changes for audit trail
- [ ] Integration test passes

**Edit Mode Workflow**:
```
InstalledMcpScreen
    ↓ (select MCP, press 'e')
EditEnvState (setup)
    ↓ (preload existing values)
EnvInputState (use existing component)
    ↓ (user modifies values)
Save confirmation (press 's')
    ↓ (persist to mcp.json)
Return to list (show success/error)
```

**Data Structures**:
```rust
struct EditEnvState {
    server_name: String,
    env_input: EnvInputState,
    original_values: HashMap<String, String>,
    modified: bool,
}

impl EditEnvState {
    fn apply_changes(&self) -> Result<()> {
        // Update McpConfigManager and save to mcp.json
    }
}
```

**Technical Constraints**:
- Reuse `EnvInputState` from REQ-018
- Use existing `McpConfigManager::update_server_env()` method
- Extend `InstalledMcpScreen` or create new `EditEnvScreen`
- Persist changes using `McpConfigManager::save()`
- Handle file locking for concurrent access
- File: modify `src/tui/screens/installed_mcp.rs`

---

### REQ-021: AI CLI 自动故障切换系统
**Status**: 🟢 Completed (v0.5.47)
**Priority**: P1 (High)
**Version**: v0.5.39+
**Related**: ARCH-021, DATA-021, API-021

**Description**:
Agentic-Warden MUST provide automatic failover mechanism for AI CLI execution. When a user executes with the virtual `auto` type, the system MUST attempt AI CLIs in the configured order, automatically switching to the next CLI upon failure, until one succeeds or all fail.

**核心特性**:
- 新增虚拟 AI CLI 类型 `CliType::Auto`
- 执行顺序由用户配置（在 `~/.aiw/config.json` 中）
- 基于退出码判断故障（exit_code == 0 为成功）
- 失败的 CLI 进入 30 秒冷却期
- 配置极简：只存储执行顺序数组

**Acceptance Criteria**:
- [x] 用户可通过 `aiw auto <prompt>` 命令执行自动故障切换模式
- [x] 执行顺序配置存储在 `~/.aiw/config.json` 的 `cli_execution_order` 字段
- [x] 配置数组必须包含所有 3 个 AI CLI：`["codex", "gemini", "claude"]`
- [x] 配置数组长度必须为 3，不得禁用任何 CLI
- [x] 系统按配置顺序依次尝试每个 CLI
- [x] 基于退出码判断成功（exit_code == 0）
- [x] 失败的 CLI 进入 30 秒冷却期，期间跳过
- [x] 所有 CLI 都失败或都在冷却期时，返回错误信息
- [x] 提供 TUI 界面调整执行顺序（仅支持移动位置和重置）
- [x] 配置验证拒绝无效的 CLI 名称或不完整的数组

**Technical Constraints**:
- **配置约束**:
  - 字段名称：`cli_execution_order`
  - 数据类型：字符串数组
  - 默认值：`["codex", "gemini", "claude"]`
  - 允许的值：`"codex"`, `"claude"`, `"gemini"`
  - 数组长度：必须为 3
- **故障检测约束**:
  - 基于退出码判断：exit_code == 0 表示成功
  - 失败判定：exit_code != 0
- **冷却机制约束**:
  - 冷却时长：30 秒（硬编码）
  - 冷却粒度：基于 `CliType`（不区分 Provider）
  - 冷却状态：内存存储（跨进程不共享）
- **执行逻辑约束**:
  - 每个 CLI 只尝试一次（无重试次数配置）
  - 按 `cli_execution_order` 数组顺序执行
  - 捕获每个 CLI 的退出码、标准输出、错误输出
  - 跳过冷却期内的 CLI

**使用场景**:
```
场景 1: 默认顺序执行
用户执行: aiw auto "Fix this bug"
执行顺序: codex → gemini → claude
结果: 第一个成功的 CLI 返回结果

场景 2: 自定义顺序
配置: {"cli_execution_order": ["claude", "gemini", "codex"]}
执行: claude → gemini → codex

场景 3: 调整顺序
用户执行: aiw config cli-order
操作: 在 TUI 中用 ↑/↓ 调整顺序
保存: 自动保存到 config.json
```

**性能要求**:
- LLM 判断响应时间: < 5 秒
- 配置文件读取: < 100ms
- TUI 界面响应: < 200ms

**错误处理**:
- 配置文件缺失: 使用默认顺序 `["codex", "gemini", "claude"]`
- 配置数组为空: 返回配置错误
- 配置包含无效 CLI 名称: 返回配置验证错误
- 配置数组长度不为 3: 返回配置验证错误
- 所有 CLI 执行失败: 返回最后一个 CLI 的错误信息
- LLM 服务不可用: 返回错误，提示用户启动 Ollama

**数据验证规则**:
```
cli_execution_order 验证:
├─ 必须是数组类型
├─ 数组长度必须等于 3
├─ 每个元素必须是字符串
├─ 元素值必须在 ["codex", "claude", "gemini"] 中
├─ 数组不得包含重复元素
└─ 不得包含其他值
```

**安全要求**:
- 配置文件权限: 0600（用户读写）
- LLM 判断不泄露敏感信息（如 API key）
- CLI 执行时的环境变量注入遵循现有安全规则

**CLI 命令**:
| 命令 | 参数 | 说明 |
|-----|------|------|
| `aiw auto <prompt>` | prompt: 任务描述 | 使用故障切换模式执行 |
| `aiw config cli-order` | - | 打开 TUI 管理执行顺序 |

**TUI 操作**:
| 按键 | 功能 |
|-----|------|
| `↑` / `↓` | 上下移动，调整执行优先级 |
| `r` | 重置为默认顺序 `["codex", "gemini", "claude"]` |
| `q` | 退出并自动保存 |

**未来扩展** (Out of Scope):
- 支持自定义 AI CLI 类型（当前硬编码 3 个）
- 支持每个 CLI 的重试次数配置
- 支持超时时间配置
- 支持模型选择配置

---

### REQ-022: Auto 模式 CLI+Provider 组合轮转
**Status**: 🟢 Done (v0.5.48)
**Priority**: P1 (High)
**Version**: v0.5.48+
**Related**: REQ-021, ARCH-021, DATA-022, DATA-023

**Description**:
扩展 REQ-021 的自动故障切换机制，支持同一 CLI 配置多个不同的 Provider 进行轮转。例如：claude+glm → claude+local → claude+official，而不仅仅是不同 CLI 之间轮转。

**核心特性**:
- 新增 `auto_execution_order` 配置字段（取代 `cli_execution_order`）
- 执行顺序为 CLI+Provider 组合数组
- 冷却机制粒度从 `CliType` 改为 `(CliType, Provider)` 组合
- 同一 CLI 可配置多个不同 Provider
- 不需要向后兼容旧配置格式

**Acceptance Criteria**:
- [ ] 新配置字段 `auto_execution_order` 存储 CLI+Provider 组合数组
- [ ] 每个组合为对象：`{"cli": "claude", "provider": "glm"}`
- [ ] 系统按数组顺序依次尝试每个 CLI+Provider 组合
- [ ] 冷却机制基于 (CLI, Provider) 组合，而非仅 CLI
- [ ] 同一 CLI 不同 Provider 可独立进入冷却期
- [ ] Provider 值 "auto" 表示使用 default_provider
- [ ] 移除旧的 `cli_execution_order` 字段支持
- [ ] TUI 界面支持管理 CLI+Provider 组合顺序

**Technical Constraints**:
- **配置约束**:
  - 字段名称：`auto_execution_order`
  - 数据类型：对象数组
  - 对象结构：`{"cli": string, "provider": string}`
  - cli 允许值：`"codex"`, `"claude"`, `"gemini"`
  - provider 允许值：providers.json 中定义的 provider 名称，或 `"auto"`
  - 数组长度：≥1，无上限
  - 允许同一 CLI 出现多次（配不同 Provider）
- **冷却机制约束**:
  - 冷却键：`(CliType, Provider)` 组合
  - 冷却时长：30 秒（不变）
  - 冷却状态：内存存储
- **执行逻辑约束**:
  - 按 `auto_execution_order` 数组顺序执行
  - 检查 (CLI, Provider) 组合是否在冷却期
  - 失败时标记 (CLI, Provider) 组合进入冷却期

**使用场景**:
```
场景 1: 同一 CLI 多 Provider 轮转
配置: [
  {"cli": "claude", "provider": "glm"},
  {"cli": "claude", "provider": "local"},
  {"cli": "claude", "provider": "official"}
]
执行: claude+glm → 失败 → claude+local → 失败 → claude+official → 成功

场景 2: 混合 CLI 和 Provider 轮转
配置: [
  {"cli": "claude", "provider": "glm"},
  {"cli": "codex", "provider": "auto"},
  {"cli": "gemini", "provider": "official"}
]
执行: claude+glm → codex+auto → gemini+official

场景 3: 冷却期跳过
状态: (claude, glm) 在冷却期
配置: [{"cli": "claude", "provider": "glm"}, {"cli": "claude", "provider": "local"}]
执行: 跳过 claude+glm → claude+local
```

**默认配置**:
```json
{
  "auto_execution_order": [
    {"cli": "codex", "provider": "auto"},
    {"cli": "gemini", "provider": "auto"},
    {"cli": "claude", "provider": "auto"}
  ]
}
```

**错误处理**:
- 配置字段缺失：使用默认配置
- 无效 CLI 名称：返回配置验证错误
- 无效 Provider 名称：返回配置验证错误（Provider 必须在 providers.json 中定义或为 "auto"）
- 空数组：返回配置验证错误
- 所有组合都失败或都在冷却期：返回错误信息

---

### REQ-023: Git 仓库检查和 Worktree 管理

**Status**: 🟢 Done (v0.5.48+)
**Priority**: P0 (Critical)
**Version**: v0.5.48+
**Related**: ARCH-023

**Description**:
在启动 AI CODE 任务前，aiw MUST 检查工作目录（由 `-C` 参数指定或当前目录）是否是 git 仓库。如果检查通过，aiw MUST 创建 git worktree 作为 AI CODE 的隔离工作目录。

**Acceptance Criteria**:
- [x] 执行 AI CLI 任务前检查工作目录是否是 git 仓库
- [x] 非 git 仓库时返回清晰的错误信息，包含 `git init` 命令提示
- [x] 检查逻辑在 `AiCliCommand::execute()` 方法开始时执行
- [x] 支持通过 `-C` 参数指定的工作目录检查
- [ ] 检查通过后，使用 git worktree add 创建临时工作目录
- [ ] Worktree 创建位置：`/tmp/aiw-worktree-<8位随机hex>`
- [ ] AI CODE 在 worktree 目录中执行任务
- [ ] 任务完成后在 stdout 输出 worktree 信息

**Technical Constraints**:
- 使用 git 库进行 git 仓库检测和 worktree 操作
- 检查和 worktree 创建 MUST 在 AI CLI 进程启动前完成
- 错误信息格式：`Error: Not a git repository. Please initialize git first:\n  cd <path> && git init`
- Worktree 命名格式：`aiw-worktree-<8位随机小写hex>`
- Worktree 创建失败 MUST 阻止 AI CLI 任务执行
- Worktree 已存在时 MUST 返回错误或使用现有 worktree

**错误处理**:
| 场景 | 错误消息 | 行为 |
|------|----------|------|
| 非 git 仓库 | 提示运行 `git init` | 返回错误码 1 |
| 权限不足 | 无法访问 git 仓库信息 | 返回错误码 1 |
| Worktree 创建失败 | 显示具体 git 错误 | 返回错误码 1 |
| Worktree 已存在 | 提示使用现有或手动清理 | 返回错误码 1 或使用现有 |
| 其他 git 错误 | 显示具体错误信息 | 返回错误码 1 |

**输出格式**:
- AI CODE 输出 → stdout（透传）
- aiw 调试信息 → stderr
- Worktree 信息 → stdout（任务完成后）
- 分隔符：`=== AIW WORKTREE END ===`

**使用流程**:
```bash
# 场景 1: 非 git 仓库
$ aiw codex -r common -C /tmp/not-a-repo "task"
Error: Not a git repository. Please initialize git first:
  cd /tmp/not-a-repo && git init

# 场景 2: git 仓库 - 创建 worktree 并执行
$ aiw codex -r common -C /path/to/repo "task"
Creating worktree: /tmp/aiw-worktree-a1b2c3d4
🚀 Starting codex with task: task ...
[AI CODE 输出...]
=== AIW WORKTREE END ===
Worktree: /tmp/aiw-worktree-a1b2c3d4
Branch: feature-branch
Commit: abc123def456
```

**职责划分**:

| 步骤 | 职责方 | 操作 | 输出 |
|------|--------|------|------|
| 1. Git 仓库检查 | aiw | 检查是否是 git 仓库 | - |
| 2. Worktree 创建 | aiw | 创建临时 worktree | worktree_path |
| 3. 指定工作目录 | aiw | 设置 cwd 为 worktree 路径 | - |
| 4. 任务执行 | AI CODE | 在 worktree 中执行任务 | 执行结果 |
| 5. 输出路径 | aiw | 在 stdout 输出 worktree 信息 | 格式化信息 |
| 6. 审查更改 | 主会话 | 查看代码变更 | - |
| 7. 决定合并 | 主会话 | 选择合并方式 | - |
| 8. 清理 worktree | 主会话 | 手动执行 git worktree remove | - |

**相关设计**:
- 架构设计：ARCH-023

---

### REQ-024: OpenAI 环境变量配置
**Status**: 🟢 Done
**Priority**: P0 (Critical)
**Version**: v5.1.1
**Related**: ARCH-013, API-013

**Description**:
Agentic-Warden MUST support OpenAI API configuration exclusively through environment variables, replacing configuration file-based LLM settings to improve security and containerized deployment compatibility.

**Acceptance Criteria**:
- [x] Support `OPENAI_ENDPOINT` environment variable for OpenAI API endpoint
- [x] Support `OPENAI_TOKEN` environment variable for OpenAI API authentication token
- [x] Support `OPENAI_MODEL` environment variable for default model selection
- [x] Environment variables MUST take precedence over any configuration file LLM settings
- [x] LLM configuration in `mcp.json` MUST be ignored when environment variables are present
- [x] Support containerized deployment with environment variable injection
- [x] Graceful fallback to default values when environment variables are not set
- [x] Security validation for token values and endpoint URLs

**Technical Constraints**:
- Environment variables MUST override configuration file LLM settings completely
- Default endpoint MUST be `https://api.openai.com/v1` when not specified
- Default model MUST be `gpt-4` when not specified
- Token validation MUST ensure non-empty string values
- Endpoint validation MUST ensure valid URL format
- Configuration loading MUST validate environment variables before file config
- Security warnings MUST be logged when tokens are detected in configuration files

**Environment Variables**:
```bash
# Required: OpenAI API token
OPENAI_TOKEN="sk-..."

# Optional: Custom OpenAI endpoint (defaults to https://api.openai.com/v1)
OPENAI_ENDPOINT="https://api.openai.com/v1"

# Optional: Default model (defaults to gpt-4)
OPENAI_MODEL="gpt-4"
```

## 已废弃需求 (Deprecated)

> **废弃原因**: Google Drive 云存储集成已禁用，push/pull 命令不可用。自 v0.5.19 起标记为 Disabled。

### REQ-003: Google Drive 配置记录和同步
**Status**: ❌ Disabled (v0.5.19+)
**Priority**: P1 (High)
**Version**: v0.1.0
**Related**: ARCH-003, DATA-003

**Description**:
~~Agentic-Warden MUST integrate with Google Drive for selective AI CLI configuration backup and restoration through `push` and `pull` commands, with intelligent file selection to avoid unnecessary data transfer.~~

**Disabled Reason**: Cloud storage integration disabled. Push/pull commands are not available in current version.

**Acceptance Criteria**:
- [x] Support OAuth 2.0 Device Flow (RFC 8628) for headless environments
- [x] Automatically detect environment (desktop/server/headless) and choose optimal auth method
- [x] Authorize only when executing push/pull commands (no background auth)
- [x] Implement selective configuration packing (exclude temp/cache/unnecessary files)
- [x] Push compressed configuration archives to Google Drive with metadata
- [x] Pull and extract configuration archives with conflict resolution
- [x] List remote configuration files with version information
- [x] Maintain sync state with hash-based change detection
- [x] Support incremental sync (only changed configurations)
- [x] Provide progress indicators for large configuration transfers
- [x] Handle network interruptions with automatic retry mechanism

**Technical Constraints**:
- Authorization MUST auto-trigger with push/pull commands
- Support concurrent local callback + manual input for better UX
- Store OAuth tokens securely with automatic refresh
- Configuration archives MUST be compressed (tar.gz format)
- File selection MUST exclude: temp files, cache, logs, binaries
- Hash validation MUST ensure data integrity
- Retry policy MUST use exponential backoff (max 3 attempts)
- Archive size MUST be optimized (< 5MB typical)

---

> **废弃原因**: Claude Code 会话历史集成功能已删除，相关 Hook 机制不再使用。

### REQ-010: Claude Code会话历史集成（Hook-Based）
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.2.0
**Related**: ARCH-010, DATA-010, API-010

**Description**:
Agentic-Warden MUST integrate with Claude Code hooks mechanism to automatically capture and index conversation history for semantic search via MCP tools.

**Architecture**:
Hook-driven design using Claude Code's `SessionEnd` and `PreCompact` hooks to trigger conversation history ingestion. MCP server automatically manages hooks installation/uninstallation.

**Acceptance Criteria**:
- [x] Implement `agentic-warden hooks handle` CLI command
- [x] Read hook input from stdin (session_id, transcript_path, hook_event_name)
- [x] Parse Claude Code JSONL transcript format
- [x] Extract and parse TODO items from conversation content
- [x] Generate embeddings using FastEmbed (AllMiniLML6V2)
- [x] Store conversations in SahomeDB vector database with TODO metadata
- [x] Provide MCP tool: `search_history` for semantic conversation search with TODO context
- [x] Return TODO items alongside conversation results (no separate get_session_todos tool)
- [x] Support session_id-based filtering
- [x] Handle incremental updates (avoid duplicates)
- [x] Auto-install hooks when MCP server starts
- [x] Auto-uninstall hooks when MCP server stops
- [x] RAII cleanup guard for signal/panic safety

**Hook Integration Flow**:
```
Claude Code Session End/PreCompact
    ↓ (trigger hook)
agentic-warden hooks handle
    ↓ (read stdin)
Hook Input JSON: {session_id, transcript_path, hook_event_name}
    ↓ (read JSONL file)
Parse Claude Code JSONL transcript
    ↓ (generate embeddings)
FastEmbed (local, no network)
    ↓ (save to vector DB)
SahomeDB: conversation_history collection
    ↓ (MCP query)
search_history MCP tool
```

**Claude Code Configuration** (~/.config/claude/hooks.json):
```json
{
  "SessionEnd": {
    "command": "agentic-warden hooks handle",
    "stdin": true
  },
  "PreCompact": {
    "command": "agentic-warden hooks handle",
    "stdin": true
  }
}
```

**Note**: This configuration is automatically managed by `agentic-warden mcp` (installs on start, uninstalls on stop). Manual configuration is not required.

**Hook Input Format** (stdin):
```json
{
  "session_id": "session-abc123",
  "transcript_path": "/home/user/.claude/sessions/2025-11-14.jsonl",
  "hook_event_name": "SessionEnd",
  "cwd": "/home/user/project",
  "permission_mode": "normal"
}
```

**Claude Code JSONL Format**:
```jsonl
{"session_id":"xxx","timestamp":"2025-11-14T10:30:00Z","message_id":"msg-001","role":"user","content":"Help me implement auth"}
{"session_id":"xxx","timestamp":"2025-11-14T10:30:05Z","message_id":"msg-002","role":"assistant","content":"I'll help..."}
```

**TODO Extraction**:
- Parse TODO markers from assistant messages: `- [ ]`, `TODO:`, `Action Items:`
- Extract task description, priority (if present), and context
- Store TODO items as structured metadata alongside conversation records
- `search_history` returns conversations with associated TODO items in response

**Technical Constraints**:
- Vector database: SahomeDB (file-based persistent storage)
- Embedding service: FastEmbed (AllMiniLML6V2, 384 dimensions, local generation)
- Session ID source: Hook stdin input (not parsed from JSONL)
- Semantic search: cosine similarity with configurable threshold
- Storage location: ~/.aiw/conversation_history.db
- Duplicate detection: Check existing session_id before insertion
- TODO extraction: Pattern matching on assistant messages (markdown checkboxes, TODO keywords)
- TODO metadata: Stored as JSON in conversation record metadata field

**Performance Requirements**:
- Hook processing: < 2s for typical session (~100 messages)
- Embedding generation: < 100ms for batch of 10 messages (FastEmbed local)
- Vector insertion: < 500ms for batch of 100 vectors
- MCP search_history: < 200ms for typical query
- Zero network dependency for embeddings

**Error Handling**:
- Hook must return exit code 0 on success
- Hook must return exit code 2 on critical errors (blocks Claude Code)
- Log errors to ~/.aiw/hooks.log
- Gracefully handle missing transcript files
- Skip already-processed sessions

---

> **废弃原因**: Google Drive OAuth public client 不再被 Google 支持，授权流程已禁用。自 v0.5.19 起标记为 Disabled。

### REQ-015: Google Drive OAuth授权流程
**Status**: ❌ Disabled (v0.5.19+)
**Priority**: P0 (Critical)
**Version**: v0.5.18
**Related**: ARCH-003, DATA-003

**Description**:
~~Agentic-Warden MUST provide Google Drive OAuth authorization flow that allows users to authenticate with their own Google Drive accounts using only the built-in public OAuth client. MUST comply with SPEC second iron law (no configuration files) and first iron law (no environment variables).~~

**Disabled Reason**: Google Drive OAuth public client is no longer supported by Google. Push/pull commands disabled pending future cloud storage solution.

**Acceptance Criteria**:
- [x] **Built-in Public Client**: MUST provide working public OAuth client ID for immediate usage
- [x] **No Application Registration**: Users MUST NOT need to register OAuth applications in Google Cloud Console
- [x] **CLI-Only Authorization**: MUST support headless CLI-based OAuth flow without requiring TUI interfaces
- [x] **Unique Authorization URLs**: MUST generate complete authorization URLs with embedded user code and device code parameters
- [x] **No Configuration Files**: MUST NOT support configuration file-based OAuth credentials (SPEC second iron law compliance)
- [x] **No Environment Variables**: MUST NOT support environment variable configuration (SPEC first iron law compliance)
- [x] **Built-in Client Only**: MUST use only the built-in public OAuth client
- [x] **Public Client Warnings**: MUST display clear warnings when using public client (rate limits, production limitations)
- [x] **English Log Messages**: All OAuth flow messages MUST be in English (no Chinese)
- [x] **Token Persistence**: MUST save and refresh OAuth tokens securely for repeated use

**Standard User Authorization Flow**:

1. **Direct Usage** (Immediate access):
   ```bash
   aiw push    # or aiw pull
   ```
   - System automatically uses built-in public OAuth client
   - Displays complete authorization URL with embedded parameters
   - User opens URL in browser and authenticates with their own Google account
   - User enters the displayed user code to complete authorization
   - OAuth tokens are saved locally for future use

2. **Subsequent Usage**:
   - After successful authorization, credentials are stored locally in `~/.aiw/auth.json`
   - No repeated authorization required for future operations
   - AIW directly operates on the user's own Google Drive
   - Built-in public client is used for all operations

**Technical Constraints**:
- **Public Client ID**: `77185425430.apps.googleusercontent.com` (built-in only)
- **Public Client Secret**: `GOCSPX-1r0aNJW8XY1Mqg4k5L_KzQDGH43` (built-in only)
- **Token Storage**: MUST store tokens in `~/.aiw/auth.json` with 0o600 permissions
- **Authorization URL**: MUST include both user_code and device_code as query parameters
- **Device Flow**: MUST implement RFC 8628 Device Authorization Flow
- **Error Handling**: MUST handle OAuth errors gracefully (invalid_client, access_denied, etc.)
- **Public Client Limitations**: MUST warn about rate limits and production limitations
- **No Configuration Files**: STRICTLY PROHIBITED by SPEC second iron law
- **No Environment Variables**: STRICTLY PROHIBITED by SPEC first iron law

**Public Client Warning Message**:
```
⚠️  Using public OAuth client. It may have limitations. For better results, create your own Google OAuth credentials in Google Cloud Console.
```

**Security Requirements**:
- OAuth tokens MUST be stored with 0o600 permissions (user read/write only)
- Configuration directory MUST have 0o700 permissions (user access only)
- MUST validate OAuth configuration before making API calls
- MUST clear sensitive data from memory when no longer needed
- MUST implement proper token refresh mechanism

**Why No Application Registration Required**:

1. **User's Own Drive Access**: The OAuth flow authorizes AIW to access the user's own Google Drive, following standard OAuth patterns where users grant applications access to their data
2. **Built-in Public Client**: The system provides a working public OAuth client that supports Google Drive API access for device flow authorization
3. **Immediate Usability**: Users can start using the system immediately without needing developers to register separate OAuth applications in Google Cloud Console

**Core Principle**:
Users authenticate AIW to access their own Google Drive accounts through standard OAuth 2.0 Device Flow. This is the normal OAuth pattern where users grant applications permission to access their personal data, rather than developers needing to register applications for each user.

**Configuration File Selection Strategy**:
- **Claude**: `CLAUDE.md`, `settings.json`, `agents/`, `skills/SKILL.md`, `hooks/`, `scripts/`, `commands/`, `mcp.json`
- **Codex**: `auth.json`, `config.toml`, `agents.md`, `history.jsonl`
- **Gemini**: `google_accounts.json`, `oauth_creds.json`, `settings.json`, `gemini.md`
- **Exclude**: `.cache/`, `temp/`, `logs/`, `node_modules/`, binaries

**Claude Configuration Details**:
- `hooks/`: Claude Code hook handlers and configuration files (SessionEnd/PreCompact/Stop)
- `scripts/`: Execution scripts including ai-cli-runner.sh and custom workflow scripts
- `commands/`: Custom slash command definitions and configurations
- `mcp.json`: MCP server configuration for Claude Code integration

---

### 2025-12-13
- Updated REQ-008 and REQ-009: 透明参数转发支持
- Status: Enhancement for v0.5.23 user experience improvement
- Added transparent parameter forwarding capability:
  - Parameters starting with `-` (excluding `-p/--provider`) are transparently forwarded to AI CLI
  - Provider flags must be specified before other CLI parameters
  - Maintains full AI CLI functionality while using Agentic-Warden provider management
  - Enhanced usage examples with parameter forwarding scenarios

### 2025-11-19
- Added REQ-024: OpenAI环境变量配置
- Status: Critical requirement for v5.1.1 security and containerization improvements
- Complete OpenAI environment variable configuration with:
  - OPENAI_ENDPOINT for API endpoint configuration
  - OPENAI_TOKEN for secure authentication (replaces config file tokens)
  - OPENAI_MODEL for default model selection
  - Environment variable precedence over configuration file settings
  - Security validation and container deployment support

### 2025-11-13
- Added REQ-012: 智能MCP路由系统
- Status: New requirement for v0.2.0 feature (P0 Critical)
- Complete intelligent routing system design with:
  - Industry-standard mcp.json configuration support
  - Dual Qdrant collections for tool/method indexing
  - RMCP client integration for dynamic MCP connections
  - Internal LLM-powered intelligent tool selection
  - Two-method interface for external AI integration
  - Clustering-based routing with semantic search
  - Health monitoring and performance optimization

### 2025-11-12
- Added REQ-009: 交互式 AI CLI 启动
- Status: New requirement for v0.1.1 feature
- Added REQ-010: AI CLI 更新/安装管理 (renumbered from REQ-009)
- Status: New requirement for v0.2.0 feature
- Re-numbered requirements to maintain sequential ordering

---
