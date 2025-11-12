# API Design - v0.x.x

## Version Information
- API version: v0
- Base URL: N/A (CLI application)
- Last updated: 2025-11-12
- Based on: Initial development (v0.1.0)

---

## [v0] API Specifications

### General Specifications

#### Command Line Interface (CLI) API
- **Protocol**: Command-line arguments and flags
- **Authentication**: Local filesystem permissions
- **Transport**: Direct binary execution
- **Error Format**: Human-readable stderr messages + exit codes

#### Model Context Protocol (MCP) API
- **Protocol**: JSON-RPC 2.0 over stdio
- **Authentication**: Local process execution
- **Transport**: Standard input/output streams
- **Error Format**: JSON-RPC error objects

#### Internal Rust API
- **Protocol**: Rust function calls and traits
- **Authentication**: Process-local access
- **Transport**: Direct method invocation
- **Error Handling**: Result<T, AgenticWardenError>

---

## CLI Endpoints

### API-001: AI CLI Execution Commands
**Version**: v0.1.0+
**Status**: 🟢 Implemented
**Related**: REQ-008, ARCH-002

#### Command: `agentic-warden <ai_type> [options] <prompt>`

**Request Parameters**:
| Parameter | Type | Required | Validation | Description |
|-----------|------|----------|------------|-------------|
| `ai_type` | enum | ✓ | codex|claude|gemini|AI CLI tool to execute |
| `-p, --provider` | string | ✗ | Valid provider name | Third-party API provider |
| `prompt` | string | ✓ | Non-empty | Task description for AI |
| `--help` | flag | ✗ | - | Show command help |

**Usage Examples**:
```bash
# Basic execution with default provider
agentic-warden claude "Write a Python function to sort a list"

# Provider-specific execution
agentic-warden codex -p openrouter "Debug this Rust code"

# Multiple AI CLI execution
agentic-warden gemini,codex -p litellm "Compare sorting algorithms"

# Interactive mode (no prompt)
agentic-warden claude
```

**Success Response**: `ExitCode 0`
- AI CLI process executes successfully
- Task registered in shared memory
- Process tree tracking established

**Error Responses**:
- `ExitCode 1` - Invalid arguments or provider configuration
- `ExitCode 2` - AI CLI not found or execution failed
- `ExitCode 3` - Provider validation failed

---

### API-002: Management Commands

#### Command: `agentic-warden status [options]`
**Related**: REQ-006, ARCH-006

**Request Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `--tui` | flag | ✗ | Launch TUI status interface |

**Success Response**: Human-readable status report
```
AI CLI Status:
✓ claude v1.0.0 (Native) - /usr/local/bin/claude
✓ codex v0.2.1 (NPM) - ~/.npm-global/bin/codex
✗ gemini - Not installed (npm install -g @google/gemini-cli)

Active Tasks: 3 running
Default Provider: openrouter
```

#### Command: `agentic-warden provider`
**Related**: REQ-002, ARCH-002

**Description**: Launch TUI interface for provider management

#### Command: `agentic-warden dashboard`
**Related**: REQ-004, ARCH-004

**Description**: Launch main TUI dashboard

---

### API-003: Synchronization Commands

#### Command: `agentic-warden push [directories...]`
**Related**: REQ-003, ARCH-003

**Request Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `directories` | path[] | ✗ | Directories to push (default: all) |

**Success Response**: Progress indication and completion status
```
✓ Packing configurations (3 directories)
✓ Authenticating with Google Drive
✓ Uploading: claude-config-20251112.tar.gz (1.2MB)
✓ Uploading: codex-config-20251112.tar.gz (856KB)
✓ Upload complete: 2 files, 2.1MB
```

#### Command: `agentic-warden pull`
**Related**: REQ-003, ARCH-003

**Description**: Pull and restore configurations from Google Drive

#### Command: `agentic-warden list`
**Related**: REQ-003, ARCH-003

**Description**: List available configuration archives

#### Command: `agentic-warden reset`
**Related**: REQ-003, ARCH-003

**Description**: Reset synchronization state

---

### API-004: Wait Mode Commands

#### Command: `agentic-warden wait [options]`
**Related**: REQ-005, ARCH-005

**Request Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `--timeout` | duration | ✗ | 12h | Maximum wait time |
| `--verbose` | flag | ✗ | false | Detailed progress output |

**Success Response**: Completion report
```
Waiting for AI CLI tasks... (timeout: 12h)
✓ claude-1234: Completed (15s)
✓ codex-5678: Completed (32s)
✓ gemini-9012: Failed (timeout after 2m)

Summary: 2 completed, 1 failed, 0 running
```

#### Command: `agentic-warden pwait <pid>`
**Related**: REQ-005, ARCH-005

**Request Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pid` | integer | ✓ | Process ID to wait for |

---

### API-005: Utility Commands

#### Command: `agentic-warden update [tool]`
**Related**: REQ-009, ARCH-008

**Request Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `tool` | string | ✗ | Specific tool to update (claude|codex|gemini) |

#### Command: `agentic-warden examples`
**Description**: Show usage examples

#### Command: `agentic-warden help [topic]`
**Description**: Show help for specific topic

---

## MCP (Model Context Protocol) API

### API-006: MCP Server Interface
**Version**: v0.1.0+
**Status**: 🟢 Implemented
**Related**: REQ-007, ARCH-007
**Protocol**: JSON-RPC 2.0 over stdio

#### Server Capabilities
```json
{
  "protocolVersion": "2024-11-05",
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  },
  "tools": [
    {
      "name": "monitor_processes",
      "description": "Monitor AI CLI processes",
      "inputSchema": {
        "type": "object",
        "properties": {
          "filter": {
            "type": "string",
            "description": "Filter by AI CLI type (optional)"
          }
        }
      }
    },
    {
      "name": "get_process_tree",
      "description": "Get process tree information",
      "inputSchema": {
        "type": "object",
        "properties": {
          "pid": {
            "type": "integer",
            "description": "Process ID to analyze"
          }
        },
        "required": ["pid"]
      }
    },
    {
      "name": "terminate_process",
      "description": "Terminate AI CLI process",
      "inputSchema": {
        "type": "object",
        "properties": {
          "pid": {
            "type": "integer",
            "description": "Process ID to terminate"
          },
          "graceful": {
            "type": "boolean",
            "description": "Attempt graceful termination",
            "default": true
          }
        },
        "required": ["pid"]
      }
    },
    {
      "name": "get_provider_status",
      "description": "Get provider configuration status",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider": {
            "type": "string",
            "description": "Specific provider name (optional)"
          }
        }
      }
    },
    {
      "name": "start_ai_cli",
      "description": "Start AI CLI with prompt",
      "inputSchema": {
        "type": "object",
        "properties": {
          "ai_type": {
            "type": "string",
            "enum": ["codex", "claude", "gemini"],
            "description": "AI CLI type"
          },
          "prompt": {
            "type": "string",
            "description": "Task prompt"
          },
          "provider": {
            "type": "string",
            "description": "Provider name (optional)"
          }
        },
        "required": ["ai_type", "prompt"]
      }
    }
  ]
}
```

#### Tool Call Examples

**monitor_processes**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "method": "tools/call",
  "params": {
    "name": "monitor_processes",
    "arguments": {
      "filter": "claude"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 2 running claude processes:\n- PID 1234: claude ask \"Write code\" (Provider: openrouter)\n- PID 5678: claude --help (Interactive mode)"
      }
    ]
  }
}
```

**start_ai_cli**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-002",
  "method": "tools/call",
  "params": {
    "name": "start_ai_cli",
    "arguments": {
      "ai_type": "gemini",
      "prompt": "Explain machine learning",
      "provider": "litellm"
    }
  }
}
```

---

## Internal Rust API

### API-007: Provider Management Interface

#### `ProviderManager` Trait
```rust
#[async_trait]
pub trait ProviderManager: Send + Sync {
    /// Get provider configuration by name
    async fn get_provider(&self, name: &str) -> AgenticResult<Provider>;

    /// Get default provider configuration
    async fn get_default_provider(&self) -> AgenticResult<Provider>;

    /// Validate provider compatibility with AI CLI type
    fn validate_compatibility(&self, provider: &Provider, ai_type: &AiType) -> AgenticResult<()>;

    /// List all available providers
    async fn list_providers(&self) -> AgenticResult<Vec<String>>;

    /// Test provider connectivity
    async fn test_provider(&self, name: &str) -> AgenticResult<bool>;
}
```

#### `EnvInjector` Interface
```rust
pub struct EnvInjector;

impl EnvInjector {
    /// Inject environment variables into Command
    pub fn inject_to_command(&self, cmd: &mut Command, env_vars: &HashMap<String, String>);

    /// Mask sensitive values for display
    pub fn mask_sensitive_value(&self, key: &str, value: &str) -> String;

    /// Validate environment variable format
    pub fn validate_env_vars(&self, env_vars: &HashMap<String, String>) -> AgenticResult<()>;
}
```

---

### API-008: Task Registry Interface

#### `TaskRegistry` Trait
```rust
#[async_trait]
pub trait TaskRegistry: Send + Sync {
    /// Register a new task
    async fn register(&self, pid: u32, record: &TaskRecord) -> AgenticResult<()>;

    /// Mark task as completed
    async fn mark_completed(&self, pid: u32, result: Option<String>) -> AgenticResult<()>;

    /// Get all task entries
    async fn entries(&self) -> AgenticResult<Vec<RegistryEntry>>;

    /// Get tasks for specific namespace
    async fn get_namespace_tasks(&self, root_pid: u32) -> AgenticResult<Vec<TaskRecord>>;

    /// Clean up stale entries
    async fn sweep_stale_entries(&self, now: DateTime<Utc>) -> AgenticResult<usize>;

    /// Wait for all tasks to complete
    async fn wait_for_all(&self, timeout: Option<Duration>) -> AgenticResult<Vec<TaskRecord>>;
}
```

---

### API-009: Process Tree Interface

#### `ProcessTreeTracker` Trait
```rust
pub trait ProcessTreeTracker: Send + Sync {
    /// Get process tree information for PID
    fn get_process_tree_info(&self, pid: u32) -> AgenticResult<ProcessTreeInfo>;

    /// Detect AI CLI type for process
    fn detect_ai_cli_type(&self, pid: u32) -> AgenticResult<Option<AiType>>;

    /// Find AI CLI root process
    fn find_ai_cli_root(&self, pid: u32) -> AgenticResult<Option<u32>>;

    /// Build process chain from PID to root
    fn build_process_chain(&self, pid: u32) -> AgenticResult<Vec<u32>>;

    /// Cache process tree information
    fn cache_result(&self, pid: u32, info: ProcessTreeInfo);

    /// Clear expired cache entries
    fn clear_expired_cache(&self);
}
```

---

### API-010: Synchronization Interface

#### `GoogleDriveService` Interface
```rust
#[async_trait]
pub trait GoogleDriveService: Send + Sync {
    /// Authenticate with OAuth 2.0
    async fn authenticate(&mut self) -> AgenticResult<()>;

    /// Upload configuration archive
    async fn upload_archive(&mut self, archive_path: &Path) -> AgenticResult<DriveFile>;

    /// Download configuration archive
    async fn download_archive(&mut self, file_id: &str, output_path: &Path) -> AgenticResult<()>;

    /// List available archives
    async fn list_archives(&mut self) -> AgenticResult<Vec<DriveFile>>;

    /// Create or find folder
    async fn create_or_find_folder(&mut self, name: &str, parent_id: Option<&str>) -> AgenticResult<String>;

    /// Delete archive
    async fn delete_archive(&mut self, file_id: &str) -> AgenticResult<()>;
}
```

---

## Error Code Definitions

### [v0] CLI Exit Code Table

| Exit Code | Category | Description | Recovery Action |
|-----------|----------|-------------|-----------------|
| 0 | Success | Operation completed successfully | N/A |
| 1 | User Error | Invalid arguments, configuration errors | Check command syntax and config |
| 2 | System Error | AI CLI not found, execution failed | Install missing AI CLI tools |
| 3 | Provider Error | Provider validation or connection failed | Check provider configuration |
| 4 | Network Error | Network connectivity issues | Check internet connection |
| 5 | Permission Error | File permissions, access denied | Check file/directory permissions |
| 6 | Sync Error | Google Drive synchronization failed | Re-authenticate or retry later |
| 7 | Memory Error | Shared memory allocation failed | Restart application |
| 8 | Timeout Error | Operation timed out | Increase timeout or retry |
| 130 | Interrupt | User interrupted (Ctrl+C) | N/A |

### [v0] MCP Error Code Table

| Error Code | JSON-RPC Code | Description | Handling Recommendation |
|------------|---------------|-------------|------------------------|
| MCP-001 | -32600 | Invalid request format | Validate JSON-RPC structure |
| MCP-002 | -32601 | Method not found | Check tool name spelling |
| MCP-003 | -32602 | Invalid parameters | Validate parameter types |
| MCP-004 | -32603 | Internal error | Check application logs |
| MCP-005 | -32000 | Process not found | Verify PID is valid |
| MCP-006 | -32001 | Provider not found | Check provider name |
| MCP-007 | -32002 | Permission denied | Check process ownership |
| MCP-008 | -32003 | Operation timeout | Retry with longer timeout |

### [v0] Internal Error Categories

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgenticWardenError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Process error: {message}")]
    Process { message: String },

    #[error("Shared memory error: {message}")]
    SharedMemory { message: String },

    #[error("Synchronization error: {message}")]
    Synchronization { message: String },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## Authentication & Authorization

### [v0] Local Authentication
- **File System**: Standard Unix permissions for configuration files
- **Process**: Parent-child process relationship validation
- **Shared Memory**: Namespace isolation by root PID

### [v0] External Authentication
- **Google Drive**: OAuth 2.0 Device Flow with scoped permissions
- **Scope**: `https://www.googleapis.com/auth/drive.file`
- **Token Storage**: Encrypted local file with automatic refresh
- **Security**: No token logging, masked display values

---

## Rate Limiting Strategy

| Operation Type | Rate Limit Rules | Over-limit Response |
|-----------------|-----------------|--------------------|
| CLI Commands | No limit (local execution) | N/A |
| Provider Testing | 5 requests/minute per provider | "Rate limited, try again later" |
| Google Drive API | Standard Google Drive quotas | HTTP 429 with backoff |
| MCP Tool Calls | 100 calls/minute per session | JSON-RPC error with retry hint |
| Shared Memory Ops | 1000 operations/second per process | Brief pause to prevent overload |

---

## API Change Log

### CHANGE-001: Initial API Specification
**Date**: 2025-11-12
**Version**: v0.1.0
**Type**: Add API
**Related**: All REQ-XXX

**Change Description**:
- Defined complete CLI command interface with parameter validation
- Specified MCP server protocol with 5 core tools
- Documented internal Rust API interfaces for major components
- Established error code taxonomy and handling strategies

**Impact Scope**:
- All external interfaces (CLI, MCP) fully specified
- Internal API contracts defined for implementation
- Error handling standardized across all components
- Authentication and security model documented

---

## Deprecated APIs

### [v0] Historical API Changes (Not applicable for v0)

*Note: This is the initial API specification version. Future deprecated APIs will be documented here when interface changes are made.*

**Deprecation Policy**:
- APIs will be deprecated for at least one minor version before removal
- Clear migration paths will be provided
- Backward compatibility maintained when possible
- Deprecation warnings in CLI output and logs