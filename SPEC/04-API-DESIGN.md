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

#### Command: `agentic-warden push`
**Related**: REQ-003, ARCH-003

**Request Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| _none_ | | | Uses built-in selection of supported AI CLI config directories |

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

#### Command: `agentic-warden wait`
**Related**: REQ-005, ARCH-005

**Request Parameters**: _none_

**Success Response**: Completion report showing finished tasks and any still running items

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

### API-006: MCP Server Interface (Intelligent Routing)
**Version**: v0.1.0+
**Status**: 🟢 Implemented
**Related**: REQ-012, ARCH-012
**Protocol**: JSON-RPC 2.0 over stdio

**Note**: The MCP server implements intelligent routing to underlying MCP servers, not direct process management tools.

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
      "name": "intelligent_route",
      "description": "Intelligently route user requests to the best MCP tool (Module 3)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "user_request": {
            "type": "string",
            "description": "Natural language user request describing what to do"
          },
          "session_id": {
            "type": "string",
            "description": "Optional session ID for context"
          },
          "max_candidates": {
            "type": "integer",
            "description": "Maximum number of tool candidates to consider",
            "default": 5
          }
        },
        "required": ["user_request"]
      }
    },
    {
      "name": "get_method_schema",
      "description": "Get the JSON schema for a specific MCP method",
      "inputSchema": {
        "type": "object",
        "properties": {
          "mcp_server": {
            "type": "string",
            "description": "MCP server name"
          },
          "tool_name": {
            "type": "string",
            "description": "Tool/method name to query"
          }
        },
        "required": ["mcp_server", "tool_name"]
      }
    }
  ]
}
```

**`listChanged` Capability说明** (Pull模式 vs Push模式):

| 方面 | Pull模式 (我们使用) | Push模式 (不使用) |
|-----|------------------|-----------------|
| **Capability声明** | ✅ `"listChanged": true` | ✅ `"listChanged": true` |
| **工具列表更新方式** | ✅ 客户端主动调用`list_tools` | ❌ 服务端发送`notifications/tools/list_changed` |
| **更新触发** | 客户端行为(每次工具调用前) | 服务端推送事件 |
| **实现复杂度** | 简单(无需推送机制) | 复杂(需要通知系统) |
| **适用场景** | Claude Code等支持自动刷新的客户端 | 需要实时通知的场景 |

**关键区别**:
- ✅ **声明`listChanged`能力**: 告诉客户端"工具列表可能动态变化,请定期刷新"
- ✅ **Pull模式**: 客户端(如Claude Code)主动定期调用`list_tools`获取最新列表
- ❌ **无需Push**: 我们**不主动发送**`notifications/tools/list_changed`消息
- 📌 **Claude Code行为**: 每次调用工具前自动刷新(< 1s),无需我们推送通知

**数据流**:
```
Claude Code: 准备调用工具
  ↓ (自动触发)
Claude Code: 调用 list_tools
  ↓
MCP Server: 返回 [intelligent_route, search_history, ...动态注册的工具]
  ↓
Claude Code: 发现新工具,选择并调用
```

**参考**: 详见`SPEC/01-REQUIREMENTS.md § 4.3 Claude Code工具刷新机制利用`

#### Tool Call Examples

**intelligent_route** - Route request to appropriate MCP tool:
```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "method": "tools/call",
  "params": {
    "name": "intelligent_route",
    "arguments": {
      "user_request": "Check git status and commit all changes",
      "session_id": "session-123"
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
        "text": "{\"success\": true, \"message\": \"Successfully executed git operations\", \"selected_tool\": {\"mcp_server\": \"git-server\", \"tool_name\": \"git_status\"}, \"result\": {\"output\": \"On branch main\\nChanges not staged for commit:\\n  modified:   src/main.rs\"}}"
      }
    ]
  }
}
```

**get_method_schema** - Get tool schema:
```json
{
  "jsonrpc": "2.0",
  "id": "req-002",
  "method": "tools/call",
  "params": {
    "name": "get_method_schema",
    "arguments": {
      "mcp_server": "git-server",
      "tool_name": "git_commit"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": "req-002",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"name\": \"git_commit\", \"description\": \"Commit changes to git repository\", \"inputSchema\": {\"type\": \"object\", \"properties\": {\"message\": {\"type\": \"string\"}}, \"required\": [\"message\"]}}"
      }
    ]
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
- **File System**: Standard Unix permissions for system configuration
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

## Hooks API

### API-010: Claude Code Hooks Handler
**Version**: v0.2.0
**Status**: 🟢 Done
**Related**: REQ-010, ARCH-010

#### Command: `agentic-warden hooks handle`
**Description**: Handle Claude Code hook events from stdin, parse transcript, and index to vector database

**Input Source**: stdin (JSON)

**Hook Input Format**:
```json
{
  "session_id": "session-abc123",
  "transcript_path": "/home/user/.claude/sessions/2025-11-14.jsonl",
  "hook_event_name": "SessionEnd",
  "cwd": "/home/user/project",
  "permission_mode": "normal"
}
```

**Processing Flow**:
1. Read JSON from stdin
2. Extract `session_id` and `transcript_path`
3. Check if session already indexed (dedup)
4. Parse JSONL transcript file
5. Generate embeddings (FastEmbed batch)
6. Insert to SahomeDB with metadata
7. Print success message to stdout
8. Return exit code

**Success Response** (stdout):
```
✅ Indexed 127 messages for session session-abc123
```

**Exit Codes**:
- `0` - Success (conversation indexed)
- `1` - Non-critical error (logged, session skipped)
- `2` - Critical error (blocks Claude Code, stderr shown)

**Error Handling**:
```
Exit 1 scenarios (non-blocking):
- Session already indexed (idempotent)
- Transcript file not found (session may not exist yet)
- Empty transcript (no messages to index)

Exit 2 scenarios (blocking):
- Invalid JSON from stdin
- Vector database connection failure
- FastEmbed initialization error
```

**Log File**: `~/.aiw/hooks.log`

**Usage Examples**:
```bash
# Manually test hook (simulate Claude Code)
echo '{"session_id":"test-123","transcript_path":"~/.claude/sessions/test.jsonl","hook_event_name":"SessionEnd"}' | \
  agentic-warden hooks handle

# Check hook logs
tail -f ~/.aiw/hooks.log
```

---

## MCP Tools API

### API-010-MCP: search_history MCP Tool
**Version**: v0.2.0
**Status**: 🟢 Done
**Related**: REQ-010, ARCH-010

#### Tool Definition

**MCP Tool Name**: `search_history`

**Description**: Search Claude Code conversation history using semantic similarity

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query (will be embedded and compared)"
    },
    "session_id": {
      "type": "string",
      "description": "Optional: filter by specific session ID"
    },
    "limit": {
      "type": "integer",
      "default": 10,
      "description": "Maximum number of results to return"
    },
    "min_similarity": {
      "type": "number",
      "default": 0.7,
      "description": "Minimum cosine similarity threshold (0.0-1.0)"
    }
  },
  "required": ["query"]
}
```

**Output Format**:
```json
{
  "results": [
    {
      "session_id": "session-abc123",
      "role": "user",
      "content": "Can you help me implement authentication?",
      "timestamp": "2025-11-14T10:30:00Z",
      "similarity_score": 0.92
    }
  ],
  "total_results": 1,
  "query_time_ms": 145
}
```

---

## [v0] Intelligent MCP Routing APIs

### API-012: 智能MCP路由系统API设计

#### External MCP Interface

##### Public MCP Methods

###### intelligent_route
智能路由MCP工具调用的主要接口。

**Method Signature**:
```json
{
  "name": "intelligent_route",
  "description": "Intelligently route user requests to appropriate MCP tools using semantic analysis and LLM-powered decision making",
  "inputSchema": {
    "type": "object",
    "required": ["user_request"],
    "properties": {
      "user_request": {
        "type": "string",
        "description": "Natural language description of the task the user wants to accomplish"
      },
      "session_id": {
        "type": "string",
        "description": "Optional session identifier for context preservation"
      },
      "preferences": {
        "type": "object",
        "properties": {
          "preferred_categories": {
            "type": "array",
            "items": {"type": "string"}
          },
          "avoid_mcp_servers": {
            "type": "array",
            "items": {"type": "string"}
          }
        }
      }
    }
  }
}
```

**Response Schema**:
```json
{
  "type": "object",
  "properties": {
    "success": {
      "type": "boolean"
    },
    "result": {
      "type": ["object", "string", "number", "boolean", "array", "null"]
    },
    "confidence_score": {
      "type": "number",
      "minimum": 0.0,
      "maximum": 1.0
    },
    "routing_trace": {
      "type": "object",
      "properties": {
        "selected_tool": {"type": "string"},
        "mcp_server": {"type": "string"},
        "method_name": {"type": "string"},
        "execution_time_ms": {"type": "integer"}
      }
    },
    "alternatives": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "tool": {"type": "string"},
          "confidence": {"type": "number"},
          "reason": {"type": "string"}
        }
      }
    }
  }
}
```

**Error Responses**:
```json
{
  "success": false,
  "error": {
    "code": "NO_SUITABLE_TOOL_FOUND",
    "message": "No MCP tool matches the user request",
    "suggestions": [
      "Try rephrasing your request with more specific details",
      "Consider using get_method_schema to explore available tools"
    ]
  }
}
```

###### get_method_schema
获取特定MCP方法的详细schema信息。

**Method Signature**:
```json
{
  "name": "get_method_schema",
  "description": "Get detailed schema information for specific MCP tools and methods",
  "inputSchema": {
    "type": "object",
    "required": ["mcp_name"],
    "properties": {
      "mcp_name": {
        "type": "string",
        "description": "Name of the MCP server"
      },
      "method_name": {
        "type": "string",
        "description": "Optional specific method name (null returns all methods)"
      }
    }
  }
}
```

**Response Schema**:
```json
{
  "type": "object",
  "properties": {
    "mcp_server": {
      "type": "object",
      "properties": {
        "name": {"type": "string"},
        "description": {"type": "string"},
        "category": {"type": "string"},
        "health_status": {"type": "string"}
      }
    },
    "tools": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "tool_name": {"type": "string"},
          "description": {"type": "string"},
          "methods": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": {"type": "string"},
                "description": {"type": "string"},
                "parameters": {
                  "type": "object",
                  "properties": {}
                },
                "examples": {
                  "type": "array",
                  "items": {"type": "string"}
                }
              }
            }
          }
        }
      }
    }
  }
}
```

#### Internal Rust APIs

##### MCP Router Core API

```rust
// Main intelligent router interface
#[async_trait]
pub trait IntelligentRouter {
    async fn intelligent_route(
        &self,
        request: IntelligentRouteRequest,
    ) -> Result<IntelligentRouteResponse, RouterError>;

    async fn get_method_schema(
        &self,
        mcp_name: &str,
        method_name: Option<&str>,
    ) -> Result<MethodSchemaResponse, RouterError>;

    async fn health_check(&self) -> RouterHealthStatus;
}

// Vector search interface for MemVDB integration
#[async_trait]
pub trait VectorSearchEngine {
    async fn search_tools(
        &self,
        query: &str,
        limit: usize,
        threshold: f64,
    ) -> Result<Vec<ToolSearchResult>, SearchError>;

    async fn search_methods(
        &self,
        query: &str,
        tool_filter: Option<&str>,
        limit: usize,
        threshold: f64,
    ) -> Result<Vec<MethodSearchResult>, SearchError>;

    async fn index_tools(
        &self,
        tools: Vec<McpToolVector>,
    ) -> Result<(), IndexError>;

    async fn index_methods(
        &self,
        methods: Vec<McpMethodVector>,
    ) -> Result<(), IndexError>;
}
```

##### MCP Client Management API

```rust
// MCP connection pool management
#[async_trait]
pub trait McpConnectionPool {
    async fn get_connection(
        &self,
        server_name: &str,
    ) -> Result<McpClientConnection, PoolError>;

    async fn start_server(
        &self,
        config: &McpServerConfig,
    ) -> Result<ServerHandle, PoolError>;

    async fn stop_server(
        &self,
        server_name: &str,
    ) -> Result<(), PoolError>;

    async fn health_check_all(
        &self,
    ) -> Vec<HealthCheckResult>;

    async fn discover_tools(
        &self,
        server_name: &str,
    ) -> Result<Vec<DiscoveredTool>, DiscoveryError>;
}

// Individual MCP client interface
#[async_trait]
pub trait McpClientConnection {
    async fn call_method(
        &self,
        method_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError>;

    async fn get_tool_list(&self) -> Result<Vec<ToolInfo>, McpError>;

    async fn get_method_schema(
        &self,
        method_name: &str,
    ) -> Result<MethodSchema, McpError>;

    fn connection_info(&self) -> &McpServerConnection;
}
```

##### LLM Decision Engine API

```rust
// LLM-powered tool selection
#[async_trait]
pub trait LlmDecisionEngine {
    async fn analyze_and_select_tool(
        &self,
        request: LlmAnalysisRequest,
    ) -> Result<LlmAnalysisResponse, LlmError>;

    async fn analyze_request_intent(
        &self,
        user_query: &str,
    ) -> Result<RequestIntent, LlmError>;

    async fn cluster_similar_tools(
        &self,
        tools: Vec<CandidateTool>,
    ) -> Result<ClusteringAnalysis, LlmError>;
}

// Prompt engineering and response parsing
pub trait PromptEngine {
    fn generate_tool_selection_prompt(
        &self,
        user_query: &str,
        candidate_tools: &[CandidateTool],
        context: &RoutingContext,
    ) -> String;

    fn parse_llm_response(
        &self,
        response: &str,
    ) -> Result<LlmAnalysisResponse, ParseError>;
}
```

#### Configuration Management APIs

##### MCP Configuration Interface

```rust
// Configuration loading and validation
pub trait McpConfigManager {
    fn load_config(&self, path: &Path) -> Result<McpConfig, ConfigError>;

    fn validate_config(&self, config: &McpConfig) -> Result<(), ValidationError>;

    fn save_config(&self, config: &McpConfig, path: &Path) -> Result<(), ConfigError>;

    fn merge_configs(&self, base: &McpConfig, overlay: &McpConfig) -> McpConfig;

    fn get_schema(&self) -> serde_json::Value;
}

// Dynamic configuration updates
pub trait ConfigurationManager {
    async fn reload_config(&self) -> Result<(), ConfigError>;

    async fn add_mcp_server(&self, server: McpServerConfig) -> Result<(), ConfigError>;

    async fn remove_mcp_server(&self, server_name: &str) -> Result<(), ConfigError>;

    async fn update_mcp_server(&self, server_name: &str, config: McpServerConfig) -> Result<(), ConfigError>;

    fn get_current_config(&self) -> &McpConfig;
}
```

#### Error Handling and Status Codes

##### Router Error Taxonomy

```rust
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("No suitable tool found for request")]
    NoSuitableToolFound,

    #[error("MCP server connection failed: {server}")]
    McpConnectionFailed { server: String },

    #[error("LLM analysis failed: {reason}")]
    LlmAnalysisFailed { reason: String },

    #[error("Vector search error: {reason}")]
    VectorSearchError { reason: String },

    #[error("Configuration error: {reason}")]
    ConfigurationError { reason: String },

    #[error("Request timeout after {timeout_ms}ms")]
    RequestTimeout { timeout_ms: u64 },

    #[error("Insufficient confidence: {confidence}")]
    LowConfidence { confidence: f64 },
}
```

##### HTTP Status Mapping
| Router Error | HTTP Status | MCP Error Code |
|--------------|------------|---------------|
| NoSuitableToolFound | 404 Not Found | -32601 Method not found |
| McpConnectionFailed | 503 Service Unavailable | -32603 Internal error |
| LlmAnalysisFailed | 502 Bad Gateway | -32603 Internal error |
| VectorSearchError | 500 Internal Server Error | -32603 Internal error |
| ConfigurationError | 422 Unprocessable Entity | -32602 Invalid params |
| RequestTimeout | 408 Request Timeout | -32603 Internal error |

#### Performance Monitoring APIs

```rust
// Metrics collection interface
pub trait RouterMetrics {
    fn record_routing_request(&self, request: &RoutingMetrics);

    fn get_performance_stats(&self) -> RoutingStatistics;

    fn get_tool_usage_stats(&self) -> Vec<ToolUsageStats>;

    fn get_mcp_server_stats(&self) -> Vec<McpServerStats>;

    fn export_metrics(&self) -> serde_json::Value;
}

// Health check interface
pub trait HealthCheck {
    async fn check_router_health(&self) -> RouterHealthStatus;

    async fn check_dependencies_health(&self) -> DependencyHealthStatus;

    async fn detailed_health_report(&self) -> DetailedHealthReport;
}
```

---

### API-013: OpenAI Environment Variable Configuration

**Description**: OpenAI API configuration through environment variables with precedence over any LLM settings

**Related**: REQ-013, ARCH-013

#### Environment Variables
| Variable | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `OPENAI_ENDPOINT` | URL | ✗ | `https://api.openai.com/v1` | OpenAI API endpoint URL |
| `OPENAI_TOKEN` | string | ✓ | - | OpenAI API authentication token (sk-...) |
| `OPENAI_MODEL` | string | ✗ | `gpt-4` | Default OpenAI model to use |

#### Configuration Priority
1. **Environment Variables** (highest priority)
2. **Configuration File** (overridden when env vars present)
3. **Default Values** (fallback)

#### Usage Examples
```bash
# Basic OpenAI configuration
export OPENAI_TOKEN="sk-proj-..."
export OPENAI_MODEL="gpt-4-turbo"

# Custom endpoint (for compatible API)
export OPENAI_ENDPOINT="https://api.openai.com/v1"
export OPENAI_TOKEN="sk-..."
export OPENAI_MODEL="gpt-4"

# Container deployment
docker run -e OPENAI_TOKEN="sk-..." -e OPENAI_MODEL="gpt-3.5-turbo" agentic-warden
```

#### Security Considerations
- Token validation ensures non-empty string values starting with "sk-"
- Endpoint validation requires valid URL format
- Configuration file LLM settings are ignored when environment variables are detected
- Security warnings logged when tokens found in configuration files
- Environment variables take complete precedence over file-based settings

#### Validation Rules
```rust
// Token validation
fn validate_openai_token(token: &str) -> bool {
    !token.is_empty() && token.starts_with("sk-")
}

// Endpoint validation
fn validate_openai_endpoint(endpoint: &str) -> bool {
    endpoint.parse::<url::Url>().is_ok()
}
```

---

## Deprecated APIs

### [v0] Historical API Changes (Not applicable for v0)

*Note: This is the initial API specification version. Future deprecated APIs will be documented here when interface changes are made.*

**Deprecation Policy**:
- APIs will be deprecated for at least one minor version before removal
- Clear migration paths will be provided
- Backward compatibility maintained when possible
- Deprecation warnings in CLI output and logs
