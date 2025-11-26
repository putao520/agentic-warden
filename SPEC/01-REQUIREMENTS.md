# Requirements Specification - v0.1.0

## Version Information
- Current version: v0.1.0
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

**Technical Constraints**:
- Configuration file format: JSON with schema validation
- Environment variable injection must be transparent to AI CLI process
- Support OpenRouter, LiteLLM, and custom providers as minimum
- Provider configurations must support inheritance and overrides
- Sensitive data must be masked in UI output
- Compatibility validation required before provider injection

---

### REQ-003: Google Drive 配置记录和同步
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.0
**Related**: ARCH-003, DATA-003

**Description**:
Agentic-Warden MUST integrate with Google Drive for selective AI CLI configuration backup and restoration through `push` and `pull` commands, with intelligent file selection to avoid unnecessary data transfer.

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

**Configuration File Selection Strategy**:
- **Claude**: `CLAUDE.md`, `settings.json`, `agents/`, `skills/SKILL.md`, `hooks/`, `scripts/`, `commands/`, `.mcp.json`
- **Codex**: `auth.json`, `config.toml`, `agents.md`, `history.jsonl`
- **Gemini**: `google_accounts.json`, `oauth_creds.json`, `settings.json`, `gemini.md`
- **Exclude**: `.cache/`, `temp/`, `logs/`, `node_modules/`, binaries

**Claude Configuration Details**:
- `hooks/`: Claude Code hook handlers and configuration files (SessionEnd/PreCompact/Stop)
- `scripts/`: Execution scripts including ai-cli-runner.sh and custom workflow scripts
- `commands/`: Custom slash command definitions and configurations
- `.mcp.json`: MCP server configuration for Claude Code integration

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
- [x] Support timeout parameter (default: 12h)
- [x] Support verbose mode for detailed progress
- [x] `agentic-warden pwait <PID>` waits for specific process's completed shared tasks
- [x] Cross-process task completion detection

**Technical Constraints**:
- Use shared memory for cross-process communication
- Timeout format: 12h, 30m, 1d
- Default timeout: 12 hours

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
**Version**: v0.1.0
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

**Technical Constraints**:
- Environment injection MUST happen before process exec(), not after
- Provider validation MUST occur before process startup
- Process isolation MUST prevent provider cross-contamination
- Error handling MUST provide specific failure reasons
- Command syntax MUST be intuitive and consistent across AI CLI types

**Usage Examples**:
- `agentic-warden claude -p openrouter "Write a Python function"` - Use OpenRouter with Claude
- `agentic-warden codex "Debug this code"` - Use default provider with Codex
- `agentic-warden gemini,codex -p litellm "Compare algorithms"` - Multiple AI CLI with same provider

---

### REQ-009: 交互式 AI CLI 启动
**Status**: 🟢 Done
**Priority**: P1 (High)
**Version**: v0.1.1
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

**Technical Constraints**:
- Interactive mode MUST preserve all provider functionality and environment variable injection
- Process tracking MUST work for long-running interactive sessions without memory leaks
- Environment variable injection MUST happen before interactive CLI starts, not during
- Signal handling MUST be compatible with both Agentic-Warden process management and AI CLI signal handling
- Task completion detection MUST work when user exits interactive mode naturally (Ctrl+D, exit command, etc.)
- Interactive mode MUST NOT require additional prompts or confirmation dialogs after provider selection
- Provider compatibility validation MUST occur before process startup

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
- [x] Support industry-standard `.mcp.json` configuration file format
- [x] Provide configuration schema validation and migration support
- [x] Support per-MCP server configuration including command, args, description, category
- [x] Enable/disable individual MCP servers with runtime configuration updates
- [x] Support health check configuration per MCP server (interval, timeout)

#### 4.2 双模式向量数据库集成
- [x] **MCP路由模式**: MemVDB for intelligent MCP tool routing and discovery
- [x] **历史会话模式**: SahomeDB for Claude Code conversation history storage and search
- [x] Tool-level indexing: index MCP tools with descriptions and capabilities for routing
- [x] Method-level indexing: index individual methods with detailed schemas for precise routing
- [x] Conversation history: store and search Claude Code conversation history with semantic search
- [x] Maintain metadata associations between tools and methods
- [x] Support batch indexing and incremental updates
- [x] Provide semantic search capabilities with configurable similarity thresholds
- [x] Memory-only MCP index rebuilt on startup from .mcp.json configuration

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
- [x] 从.mcp.json发现的所有MCP工具
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
  - Pure memory operations, rebuilt from .mcp.json on startup
  - Thread-safe, multiple distance metrics supported
- **Ollama-rs**: Retained for LLM inference (tool selection decisions), not embeddings

#### Configuration Format (.mcp.json):
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
  - Pure memory mode, rebuilt on startup from .mcp.json configuration
  - Metadata: MCP name, tool name, category, capabilities, health status
  - **mcp_methods**: Method-level vectors with detailed schema embedding for precise routing
  - Real-time MCP tool discovery and intelligent routing decisions
  - Metadata: MCP name, method name, parameters, examples, availability
  - Thread-safe, zero dependencies, multiple distance metrics (cosine, euclidean, dot-product)

#### Environment Variables:
- `AGENTIC_WARDEN_LLM_ENDPOINT`: Internal LLM endpoint (default: http://localhost:11434)
- `AGENTIC_WARDEN_LLM_MODEL`: Internal LLM model (default: qwen2.5:7b)
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
- Existing configuration system for .mcp.json management

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
- [x] 可行时生成带输入参数定义的JS函数
- [x] JS函数内部调用注入的MCP工具(以`mcp`前缀暴露)
- [x] 代码验证失败时返回Err触发fallback
- [x] 验证通过后注册为单一动态JS编排工具到Registry
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
- [x] 实现执行时间限制(最大30秒)
- [x] 实现内存使用限制(最大256MB)
- [x] 实现调用栈深度限制(最大128层)
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
**Status**: 🟢 Done (Phase 1-3 ✅ Completed)
**Related**: ARCH-014
**Version**: v0.2.0

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
| REQ-003 | Google Drive 同步集成 | P1 | 🟢 Done | v0.1.0 | ARCH-003, DATA-003 | Initial commit |
| REQ-004 | 统一 TUI 体验 | P1 | 🟢 Done | v0.1.0 | ARCH-004, MODULE-001 | Initial commit |
| REQ-005 | Wait 模式跨进程等待 | P2 | 🟢 Done | v0.1.0 | ARCH-005, DATA-005, API-002 | Initial commit |
| REQ-006 | AI CLI 工具检测与状态管理 | P1 | 🟢 Done | v0.1.0 | ARCH-006, MODULE-002 | Initial commit |
| REQ-007 | MCP 服务器 | P1 | 🟢 Done | v0.1.0 | ARCH-007, API-003 | Initial commit |
| REQ-008 | 指定供应商模式 AI CLI 启动 | P0 | 🟢 Done | v0.1.0 | ARCH-002, ARCH-008, API-004 | Initial commit |
| REQ-009 | 交互式 AI CLI 启动 | P1 | 🟢 Done | v0.1.1 | ARCH-008, API-001 | Interactive mode implementation |
| REQ-010 | 内存集成与语义搜索 | P1 | 🟢 Done | v0.1.0 | ARCH-010, DATA-003, API-005 | Memory and search integration |
| REQ-011 | AI CLI 更新/安装管理 | P1 | 🟢 Done | v0.1.0 | ARCH-008, MODULE-002, API-004 | Update command implementation |
| REQ-012 | 智能MCP路由系统 | P0 | 🟢 Done | v0.2.0 | ARCH-012, DATA-012, API-012 | Intelligent routing system design |
| REQ-013 | OpenAI环境变量配置 | P0 | 🟢 Done | v5.1.1 | ARCH-013, API-013 | OpenAI environment variable configuration |

---

### REQ-013: OpenAI 环境变量配置
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
- [x] LLM configuration in `.mcp.json` MUST be ignored when environment variables are present
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

### 2025-11-19
- Added REQ-013: OpenAI环境变量配置
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
  - Industry-standard .mcp.json configuration support
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

<!-- 
更新记录 (2025-11-25):
- REQ-003 (Google Drive同步): 已从删除状态恢复
- REQ-010 (CC会话历史): 保持删除状态

说明: Google Drive同步是通用基础设施，与CC会话系统无技术依赖，
因此根据用户反馈恢复此功能。
-->
