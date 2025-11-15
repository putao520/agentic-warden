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
- [x] Store provider configurations in `~/.agentic-warden/provider.json` with JSON schema validation
- [x] Support multiple providers with environment variable injection (API keys, base URLs, org IDs)
- [x] Inject environment variables into AI CLI processes via `-p` parameter (transparent to user)
- [x] Set default provider in configuration with fallback to first available provider
- [x] Do NOT modify AI CLI's native configuration files (maintain separation of concerns)
- [x] Validate provider compatibility with AI CLI types before injection
- [x] Support built-in providers with read-only configurations
- [x] Provide TUI interface for provider management (add, edit, delete, test)
- [x] Implement provider health checking and network connectivity detection
- [x] Mask sensitive values (API keys) in display and logs

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
- **Claude**: `CLAUDE.md`, `settings.json`, `agents/`, `skills/SKILL.md`
- **Codex**: `auth.json`, `config.toml`, `agents.md`, `history.jsonl`
- **Gemini**: `google_accounts.json`, `oauth_creds.json`, `settings.json`, `gemini.md`
- **Exclude**: `.cache/`, `temp/`, `logs/`, `node_modules/`, binaries

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
- Storage location: ~/.config/agentic-warden/conversation_history.db
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
- Log errors to ~/.config/agentic-warden/hooks.log
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

#### 4.3 Claude Code工具刷新机制利用
- [x] Leverage Claude Code's automatic `list_tools` refresh before each tool use
- [x] No need for notifications/tools/list_changed - Claude Code pulls updates naturally
- [x] Maintain thread-safe global tools registry (DynamicToolManager)
- [x] Return base tools + dynamically registered tools in `list_tools` response
- [x] Tools become visible to Claude Code on next refresh (typically < 1s)
- [x] Zero client capability detection needed - works universally

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
- ✅ **98% token reduction**: Only expose base tools (intelligent_route, search_history) until needed
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

#### 4.6 统一MCP接口 (2个基础工具 + 动态代理)

**基础工具 (始终可见)**:
- [x] **intelligent_route**: 智能MCP工具选择和动态注册
  - [x] Accepts: `user_request` (用户需求描述), `session_id` (可选)
  - [x] Returns:
    - `selected_tool`: 选中的工具名称
    - `mcp_server`: 所属MCP服务器
    - `description`: 工具功能说明
    - `registered`: 是否已注册到动态工具列表
  - [x] Side effect: 将选中的工具注册到DynamicToolManager
  - [x] Next step: Claude Code refreshes tools, sees new tool, calls it directly

- [x] **search_history**: 会话历史语义搜索（带TODO上下文）
  - [x] Accepts: `query`, `session_id` (optional), `limit`
  - [x] Returns: Conversation records with embedded TODO items
  - [x] TODO extraction patterns: `- [ ]`, `TODO:`, `Action Items:`
  - [x] Each result includes: conversation context + associated TODO list

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
  "version": "1.0",
  "mcp_servers": {
    "git-server": {
      "command": "uvx",
      "args": ["mcp-server-git"],
      "description": "Git version control operations",
      "category": "development",
      "enabled": true,
      "health_check": {
        "enabled": true,
        "interval": 60,
        "timeout": 10
      }
    }
  },
  "routing": {
    "max_tools_per_request": 10,
    "clustering_threshold": 0.7,
    "rerank_top_k": 5
  },
  "llm": {
    "endpoint": "http://localhost:11434",
    "model": "qwen2.5:7b",
    "timeout": 30
  }
}
```

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

---

## Change Log

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
