# Requirements Specification - v0.x.x

## Version Information
- Current version: v0.1.0-dev
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
**Version**: v0.1.0
**Related**: ARCH-007, API-003

**Description**:
Agentic-Warden MUST provide MCP server to enable external AI assistants to access Agentic-Warden functionality.

**Acceptance Criteria**:
- [x] Support stdio transport protocol
- [x] Provide process management tools: monitor_processes, get_process_tree, terminate_process
- [x] Provide provider status tool: get_provider_status
- [x] Provide AI CLI launch tool: start_ai_cli
- [x] Compatible with Claude Code and other MCP clients

**Technical Constraints**:
- MCP Protocol v1.0
- Transport: stdio only
- Support stdio for better integration

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
- [ ] Support `agentic-warden claude` - launch Claude CLI in interactive mode with default provider
- [ ] Support `agentic-warden claude -p openrouter` - launch Claude CLI in interactive mode with OpenRouter provider
- [ ] Support `agentic-warden codex --provider litellm` - support long format provider flag in interactive mode
- [ ] Support `agentic-warden gemini,prompt -p custom` - multiple AI CLI in interactive mode with custom provider
- [ ] Pass all environment variable injection to interactive AI CLI process before startup
- [ ] Maintain process tree tracking and task registration for long-running interactive sessions
- [ ] Support graceful signal handling (Ctrl+C) compatible with both Agentic-Warden and AI CLI processes
- [ ] Detect interactive mode completion when user exits and mark task as completed in shared memory
- [ ] Provide clear user feedback showing provider used in interactive mode
- [ ] Handle provider validation errors gracefully before launching interactive CLI

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

### REQ-010: AI CLI 更新/安装管理
**Status**: 🔴 To Do
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
| REQ-010 | AI CLI 更新/安装管理 | P1 | 🔴 To Do | v0.1.0 | ARCH-008, MODULE-002, API-004 | - |

---

## Change Log

### 2025-11-12
- Added REQ-009: 交互式 AI CLI 启动
- Status: New requirement for v0.1.1 feature
- Added REQ-010: AI CLI 更新/安装管理 (renumbered from REQ-009)
- Status: New requirement for v0.2.0 feature
- Re-numbered requirements to maintain sequential ordering
