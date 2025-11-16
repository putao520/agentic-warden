# Requirements-Architecture-Implementation Traceability Matrix

**Version**: v0.1.0
**Last Updated**: 2025-11-12
**Purpose**: Complete traceability from requirements through architecture to implementation

---

## Traceability Overview

This matrix provides complete traceability from:
- **Requirements** (REQ-XXX) → What needs to be built
- **Architecture** (ARCH-XXX) → How it's designed
- **Data Structures** (DATA-XXX) → What data models are used
- **API Design** (API-XXX) → How interfaces work
- **Implementation** → Actual code files and tests

---

## Complete Traceability Matrix

### REQ-001: AI CLI 进程树追踪

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-001 | AI CLI 进程树追踪 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-001 |
| **Architecture** | ARCH-001 | Shared Memory Task Coordination | 🟢 Done | SPEC/02-ARCHITECTURE.md#ARCH-001 |
| **Data** | DATA-005 | Process Tree Information | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-005 |
| **API** | API-006 | MCP Process Tools | 🟢 Done | SPEC/04-API-DESIGN.md#API-006 |
| **Implementation** | - | Process Tree Tracker | 🟢 Done | `src/core/process_tree.rs` |
| **Implementation** | - | Shared Memory Storage | 🟢 Done | `src/storage/shared_map.rs` |
| **Implementation** | - | Process Detection | 🟢 Done | `src/platform/windows.rs`, `src/platform/unix.rs` |
| **Tests** | - | Process Tree Tests | 🟢 Done | `tests/integration/config_integration.rs` |

**Key Insights**:
- Uses shared memory for sub-millisecond cross-process coordination
- Platform-specific process detection (winapi vs procfs)
- Namespace isolation by AI CLI root process
- MCP exposes process monitoring tools for external integration

---

### REQ-002: 第三方 Provider 管理

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-002 | 第三方 Provider 管理 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-002 |
| **Architecture** | ARCH-002 | Environment Variable Injection | 🟢 Done | SPEC/02-ARCHITECTURE.md#ARCH-002 |
| **Data** | DATA-001 | Provider Configuration | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-001 |
| **API** | API-007 | Provider Management Interface | 🟢 Done | SPEC/04-API-DESIGN.md#API-007 |
| **API** | API-006 | MCP Provider Status Tool | 🟢 Done | SPEC/04-API-DESIGN.md#API-006 |
| **Implementation** | - | Provider Manager | 🟢 Done | `src/provider/manager.rs` |
| **Implementation** | - | Environment Injector | 🟢 Done | `src/provider/env_injector.rs` |
| **Implementation** | - | Provider Config | 🟢 Done | `src/provider/config.rs` |
| **Implementation** | - | Network Detector | 🟢 Done | `src/provider/network_detector.rs` |
| **Implementation** | - | TUI Provider Management | 🟢 Done | `src/tui/screens/provider_management.rs` |
| **Tests** | - | Provider Configuration Tests | 🟢 Done | `tests/provider_config.rs` |

**Key Insights**:
- Transparent environment variable injection without modifying AI CLI configs
- JSON schema validation for provider configurations
- Network detection and health checking for providers
- TUI interface for provider management
- MCP integration exposes provider status to external tools

---

### REQ-003: Google Drive 配置记录和同步

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-003 | Google Drive 配置记录和同步 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-003 |
| **Architecture** | ARCH-003 | Google Drive Integration with OAuth | 🟢 Done | SPEC/02-ARCHITECTURE.md#ARCH-003 |
| **Data** | DATA-002 | Synchronization Configuration | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-002 |
| **Data** | DATA-003 | OAuth Token Information | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-003 |
| **Data** | DATA-006 | Google Drive File Metadata | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-006 |
| **API** | API-003 | Synchronization Commands | 🟢 Done | SPEC/04-API-DESIGN.md#API-003 |
| **API** | API-010 | Synchronization Interface | 🟢 Done | SPEC/04-API-DESIGN.md#API-010 |
| **Implementation** | - | Google Drive Service | 🟢 Done | `src/sync/google_drive_service.rs` |
| **Implementation** | - | OAuth Client | 🟢 Done | `src/sync/oauth_client.rs` |
| **Implementation** | - | Config Packer | 🟢 Done | `src/sync/config_packer.rs` |
| **Implementation** | - | Sync Command Handler | 🟢 Done | `src/sync/sync_command.rs` |
| **Implementation** | - | Directory Hasher | 🟢 Done | `src/sync/directory_hasher.rs` |
| **Implementation** | - | TUI Progress Screens | 🟢 Done | `src/tui/screens/mod_simple.rs` |

**Key Insights**:
- OAuth 2.0 Device Flow for headless environments
- Selective configuration packing (1-5MB typical archives)
- Hash-based change detection for incremental sync
- Progress indication for large transfers
- Automatic retry with exponential backoff

---

### REQ-004: 统一 TUI 体验

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-004 | 统一 TUI 体验 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-004 |
| **Architecture** | ARCH-004 | TUI Framework Integration | 🟢 Done | SPEC/02-ARCHITECTURE.md#TUI-Components |
| **API** | API-002 | Management Commands | 🟢 Done | SPEC/04-API-DESIGN.md#API-002 |
| **Implementation** | - | Main TUI App | 🟢 Done | `src/tui/app.rs` |
| **Implementation** | - | Dashboard Screen | 🟢 Done | `src/tui/screens/dashboard.rs` |
| **Implementation** | - | Status Screen | 🟢 Done | `src/tui/screens/mod_simple.rs` |
| **Implementation** | - | Provider Edit Screens | 🟢 Done | `src/tui/screens/provider_edit.rs` |
| **Implementation** | - | Render Helpers | 🟢 Done | `src/tui/screens/render_helpers.rs` |
| **Tests** | - | TUI Navigation Tests | 🟢 Done | `tests/integration/tui_navigation.rs` |

**Key Insights**:
- Single TUI framework: ratatui 0.26+ (unified design system)
- Multiple screens: Dashboard, Status, Provider Management, Progress
- Event-driven architecture with crossterm for input handling
- Consistent component library and design patterns

---

### REQ-005: Wait 模式跨进程等待

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-005 | Wait 模式跨进程等待 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-005 |
| **Architecture** | ARCH-001 | Shared Memory Task Coordination | 🟢 Done | SPEC/02-ARCHITECTURE.md#ARCH-001 |
| **Data** | DATA-004 | Task Registry Record | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-004 |
| **API** | API-004 | Wait Mode Commands | 🟢 Done | SPEC/04-API-DESIGN.md#API-004 |
| **API** | API-008 | Task Registry Interface | 🟢 Done | SPEC/04-API-DESIGN.md#API-008 |
| **Implementation** | - | Wait Mode Implementation | 🟢 Done | `src/wait_mode.rs` |
| **Implementation** | - | Process-Specific Wait | 🟢 Done | `src/pwait_mode.rs` |
| **Implementation** | - | Task Registry | 🟢 Done | `src/registry.rs` |
| **Implementation** | - | Unified Registry | 🟢 Done | `src/unified_registry.rs` |
| **Implementation** | - | Task Record Management | 🟢 Done | `src/task_record.rs` |

**Key Insights**:
- Shared memory enables real-time task status monitoring
- Supports both global wait (`wait`) and process-specific wait (`pwait`)
- Timeout handling with human-readable duration formats
- Verbose mode for detailed progress tracking

---

### REQ-006: AI CLI 工具检测与状态管理

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-006 | AI CLI 工具检测与状态管理 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-006 |
| **Architecture** | ARCH-006 | CLI Tool Detection | 🟢 Done | SPEC/02-ARCHITECTURE.md#Core-Modules |
| **API** | API-002 | Status Command | 🟢 Done | SPEC/04-API-DESIGN.md#API-002 |
| **Implementation** | - | CLI Type Detection | 🟢 Done | `src/cli_type.rs` |
| **Implementation** | - | CLI Detection Logic | 🟢 Done | `src/commands/cli_detection.rs` |
| **Implementation** | - | Version Management | 🟢 Done | `src/utils/version.rs` |
| **Implementation** | - | External Command Handling | 🟢 Done | `src/commands/mod.rs` |

**Key Insights**:
- Cross-platform detection using `which` (Unix) and `where` (Windows)
- Distinguishes NPM packages vs native installations
- Version detection via `--version` flag
- Provides installation hints for missing tools

---

### REQ-007: MCP (Model Context Protocol) 服务器

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-007 | MCP 服务器 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-007 |
| **Architecture** | ARCH-007 | External Integration Point | 🟢 Done | SPEC/02-ARCHITECTURE.md#Integration-Points |
| **Data** | DATA-007 | MCP Protocol Messages | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-007 |
| **API** | API-006 | MCP Server Interface | 🟢 Done | SPEC/04-API-DESIGN.md#API-006 |
| **Implementation** | - | MCP Server | 🟢 Done | `src/mcp.rs` |
| **Implementation** | - | MCP Tools Implementation | 🟢 Done | `src/mcp/tools.rs` |
| **Implementation** | - | Command Router for MCP | 🟢 Done | `src/commands/parser.rs` |
| **Tests** | - | MCP Integration Tests | 🟡 In Progress | `tests/integration/mcp_tests.rs` |

**Key Insights**:
- JSON-RPC 2.0 over stdio transport
- 5 core tools: process management, provider status, AI CLI launch
- Enables external AI assistants to access Agentic-Warden functionality
- Stdio transport for easy integration

---

### REQ-008: 指定供应商模式 AI CLI 启动

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-008 | 指定供应商模式 AI CLI 启动 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-008 |
| **Architecture** | ARCH-002 | Environment Variable Injection | 🟢 Done | SPEC/02-ARCHITECTURE.md#ARCH-002 |
| **Architecture** | ARCH-008 | Process Supervisor | 🟢 Done | SPEC/02-ARCHITECTURE.md#Core-Modules |
| **Data** | DATA-001 | Provider Configuration | 🟢 Done | SPEC/03-DATA-STRUCTURE.md#DATA-001 |
| **API** | API-001 | AI CLI Execution Commands | 🟢 Done | SPEC/04-API-DESIGN.md#API-001 |
| **Implementation** | - | AI CLI Command Handler | 🟢 Done | `src/commands/ai_cli.rs` |
| **Implementation** | - | Process Supervisor | 🟢 Done | `src/supervisor.rs` |
| **Implementation** | - | External Command Parser | 🟢 Done | `src/commands/parser.rs` |
| **Implementation** | - | Main CLI Router | 🟢 Done | `src/main.rs` |

**Key Insights**:
- Seamless `aiw <ai_type> -p <provider> <prompt>` syntax
- Environment injection happens before process exec()
- Provider compatibility validation before startup
- Supports both single and multi-AI CLI execution
- Process isolation prevents provider cross-contamination

---

### REQ-009: 交互式 AI CLI 启动

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-009 | 交互式 AI CLI 启动 | 🟢 Done | SPEC/01-REQUIREMENTS.md#REQ-009 |
| **Architecture** | ARCH-008 | Process Supervisor | 🟢 Done | SPEC/02-ARCHITECTURE.md#Core-Modules |
| **API** | API-001 | AI CLI Execution Commands | 🟢 Done | SPEC/04-API-DESIGN.md#API-001 |
| **Implementation** | - | External AI CLI Parser | 🟢 Done | `src/commands/parser.rs` |
| **Implementation** | - | Main CLI Handler | 🟢 Done | `src/main.rs` |

**Key Insights**:
- ✅ **Implemented**: Interactive mode launched with empty prompt for AI CLI
- Supports both single and multiple AI CLI with shared provider
- Leverages existing `AiCliCommand` infrastructure with empty prompt
- Provider environment injection works identically to task mode
- Smart detection automatically distinguishes between interactive and task modes

**Usage Examples**:
- `aiw claude` → Interactive mode with default provider ✅
- `aiw claude -p openrouter` → Interactive mode with OpenRouter ✅
- `aiw claude,gemini -p litellm` → Multiple AI CLI interactive mode ✅

### REQ-010: AI CLI 更新/安装管理

| Layer | ID | Component | Status | Implementation Files |
|-------|----|-----------|---------|----------------------|
| **Requirement** | REQ-010 | AI CLI 更新/安装管理 | 🔴 To Do | SPEC/01-REQUIREMENTS.md#REQ-010 |
| **Architecture** | ARCH-008 | Update Management | 🟡 Planned | SPEC/02-ARCHITECTURE.md#Update-Management |
| **API** | API-005 | Update Command | 🟡 Planned | SPEC/04-API-DESIGN.md#API-005 |
| **Implementation** | - | Update Implementation | 🔴 To Do | `src/commands/update.rs` |
| **Implementation** | - | NPM Registry Client | 🔴 To Do | `src/utils/npm_client.rs` |
| **Implementation** | - | Version Comparison | 🔴 To Do | `src/utils/version_compare.rs |

**Key Insights**:
- **Not Yet Implemented**: This requirement is planned for v0.2.0
- Will support NPM package management for codex/gemini
- Will use native `claude update` for Claude CLI
- Automatic version detection and installation

---

## Cross-Cutting Concerns Traceability

### Security Architecture
| Requirements | Architecture | Implementation |
|--------------|-------------|----------------|
| REQ-002, REQ-003 | Token encryption, process isolation | `src/provider/token_validator.rs`, shared memory namespaces |
| REQ-003 | OAuth 2.0 Device Flow | `src/sync/oauth_client.rs` |
| REQ-001 | Process isolation by namespace | `src/core/process_tree.rs` |

### Performance Requirements
| Requirements | Architecture | Implementation |
|--------------|-------------|----------------|
| NFR-001 | Shared memory coordination | `src/storage/shared_map.rs` (< 1ms operations) |
| NFR-001 | Process tree caching | `src/core/process_tree.rs` (5-second cache) |
| REQ-003 | Archive compression | `src/sync/compressor.rs` (1-5MB archives) |

### Error Handling
| Requirements | Architecture | Implementation |
|--------------|-------------|----------------|
| All requirements | Centralized error types | `src/error.rs` |
| API requirements | Exit code taxonomy | SPEC/04-API-DESIGN.md#Error-Codes |
| Sync requirements | Network retry logic | `src/sync/google_drive_service.rs` |

---

## Implementation Status Summary

### Completed Features (🟢)
- **Core Infrastructure**: Process tracking, shared memory, provider management
- **User Interface**: Complete TUI with all screens
- **External Integration**: MCP server with 5 tools
- **Synchronization**: Full Google Drive integration with OAuth
- **CLI Interface**: All core commands implemented

### In Progress (🟡)
- **Testing**: Some integration tests still being expanded
- **Documentation**: API documentation completion

### Not Yet Implemented (🔴)
- **Update Management**: AI CLI update/installation (REQ-009)
- **Advanced Features**: Some edge cases and error scenarios

---

## Quality Assurance Traceability

### Test Coverage Mapping
| Component | Unit Tests | Integration Tests | E2E Tests |
|-----------|------------|-------------------|-----------|
| Provider Management | `tests/provider_config.rs` | `tests/integration/config_integration.rs` | Manual |
| Process Tracking | Internal tests | `tests/integration/cli_end_to_end.rs` | Manual |
| TUI Interface | `tests/integration/tui_navigation.rs` | Manual | Manual |
| Synchronization | `tests/integration/config_integration.rs` | Manual | Manual |
| MCP Server | `tests/integration/mcp_tests.rs` | Manual | Manual |

### Validation Checklist
- ✅ All requirements have corresponding architecture decisions
- ✅ All architecture decisions are implemented in code
- ✅ All data structures have validation rules
- ✅ All APIs have error handling defined
- ✅ All critical paths have test coverage
- ⚠️ Some edge cases need additional testing
- 🔴 REQ-009 implementation pending

---

## Usage Examples Traceability

### Complete User Workflow
```
1. User runs: aiw claude -p openrouter "Write code"
   REQ-008 → API-001 → supervisor.rs → env_injector.rs

2. Process tracking begins automatically
   REQ-001 → ARCH-001 → process_tree.rs → shared_map.rs

3. User checks status: aiw status
   REQ-006 → API-002 → cli_detection.rs

4. User syncs config: aiw push
   REQ-003 → API-003 → config_packer.rs → google_drive_service.rs

5. User waits for completion: aiw wait
   REQ-005 → API-004 → wait_mode.rs → registry.rs
```

Each command maps through the complete traceability chain from requirement to implementation.

---

**Conclusion**: This traceability matrix ensures that every requirement is backed by architectural decisions, data structures, API designs, and concrete implementations. The system maintains complete traceability from user needs down to code execution.