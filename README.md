# Agentic-Warden

<div align="center">

![Version](https://img.shields.io/badge/version-5.0.2-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)
![MCP](https://img.shields.io/badge/MCP-2024--11--05-purple?style=flat-square)

**Intelligent MCP Router with 98% Token Optimization**

</div>

Agentic-Warden is an intelligent MCP (Model Context Protocol) routing system that dramatically reduces token consumption through dynamic tool registration. Instead of exposing 100+ tools (50,000 tokens) to your AI, it intelligently exposes only 4 base tools (~900 tokens) and registers specific tools on-demand as needed.

## 🎯 What It Does

### Core Problem Solved
When using multiple MCP servers (filesystem, git, web search, etc.), Claude Code must load **all tools** from **all servers** into context. With 100+ tools averaging 500 tokens each, this consumes ~50,000 tokens before you even start your conversation.

### Agentic-Warden Solution
1. **Intelligent Routing**: Routes user requests to the best MCP tool using LLM ReAct or vector search
2. **Dynamic Registration**: Registers tools on-demand only when needed (98% token reduction)
3. **Dual-Mode Architecture**: Auto-detects client capabilities and adapts (Dynamic vs Two-phase negotiation)
4. **Conversation History**: Integrates Claude Code session history for context-aware routing

### Token Optimization Results
```
Before:  100+ tools × ~500 tokens = ~50,000 tokens
After:   4 base tools × ~200 tokens = ~900 tokens
Savings: 98% token reduction
```

## 🚀 Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/putao520/agentic-warden.git
cd agentic-warden
cargo build --release

# Install binary
cargo install --path .
```

### Basic Usage

#### Mode 1: MCP Server Mode (Recommended)

Add Agentic-Warden as an MCP server to Claude Code:

```bash
# Start the MCP server (stdio mode)
agentic-warden mcp
```

In your Claude Code MCP configuration (`~/.config/claude-code/mcp.json`):

```json
{
  "mcpServers": {
    "agentic-warden": {
      "command": "agentic-warden",
      "args": ["mcp"]
    }
  }
}
```

Once configured, you'll have access to these tools in Claude Code:

- `intelligent_route` - Automatically route requests to best tool (auto-registers on-demand)
- `search_history` - Search conversation history using semantic similarity
- `get_method_schema` - Get JSON schema for any MCP tool
- `execute_tool` - Execute tools in two-phase negotiation mode (fallback)

#### Mode 2: CLI Mode (Direct Routing)

```bash
# Route a user request to best tool
agentic-warden route "list files in /tmp directory"

# Search conversation history
agentic-warden search "how did I configure git yesterday?"

# Get tool schema
agentic-warden schema filesystem read_file
```

## ⚙️ Configuration

### MCP Server Configuration

Create `.mcp.json` in your project root or `~/.config/agentic-warden/.mcp.json`:

```json
{
  "version": "1.0",
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user"],
      "description": "Filesystem operations (read, write, search files)",
      "category": "system",
      "enabled": true,
      "healthCheck": {
        "enabled": true,
        "interval": 60,
        "timeout": 10
      }
    },
    "git": {
      "command": "uvx",
      "args": ["mcp-server-git", "--repository", "/home/user/project"],
      "description": "Git version control operations",
      "category": "development",
      "enabled": true
    },
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "description": "Web search using Brave Search API",
      "category": "search",
      "enabled": false,
      "env": {
        "BRAVE_API_KEY": "your-api-key-here"
      }
    }
  },
  "routing": {
    "max_tools_per_request": 10,
    "clustering_threshold": 0.7,
    "rerank_top_k": 5,
    "similarity_threshold": 0.6
  },
  "llm": {
    "endpoint": "http://localhost:11434",
    "model": "qwen2.5:7b",
    "timeout": 30
  }
}
```

See [`.mcp.json.example`](./.mcp.json.example) for a complete configuration example.

### Configuration Fields

#### MCP Servers
- `command` - Executable command (npx, uvx, node, python, etc.)
- `args` - Command-line arguments
- `description` - Tool description for routing decisions
- `category` - Tool category (system, development, search, etc.)
- `enabled` - Enable/disable server (default: true)
- `env` - Environment variables (API keys, etc.)
- `healthCheck` - Health monitoring configuration

#### Routing Settings
- `max_tools_per_request` - Maximum tools to consider (default: 10)
- `clustering_threshold` - Similarity threshold for grouping (default: 0.7)
- `rerank_top_k` - Top-K tools for reranking (default: 5)
- `similarity_threshold` - Minimum similarity score (default: 0.6)

#### LLM Settings (Optional)
Configure for **LLM ReAct mode** (primary decision engine). If not configured, falls back to **Vector Search mode**.

- `endpoint` - LLM API endpoint (OpenAI-compatible)
- `model` - Model name
- `timeout` - Request timeout in seconds

## 🏗️ Architecture

### Dual-Layer Design

#### Decision Layer (Chooses Best Tool)
1. **LLM ReAct** (Primary) - Uses LLM reasoning to analyze request and select optimal tool
2. **Vector Search** (Fallback) - Uses semantic similarity when LLM unavailable
3. **Auto Mode** (Default) - Automatically selects based on LLM endpoint availability

#### Execution Layer (How to Provide Tool to AI)
1. **Dynamic Registration** (Primary) - Registers tool on-demand via `tools/list_changed` notification (98% token savings)
2. **Two-Phase Negotiation** (Fallback) - Returns tool suggestion for AI to review and confirm

The system **auto-detects client capabilities** at startup and adapts accordingly.

### How Dynamic Registration Works

```
┌─────────────────┐
│   Claude Code   │ Initially sees only 4 base tools (~900 tokens)
└────────┬────────┘
         │ "list files in /tmp"
         ▼
┌─────────────────┐
│intelligent_route│ Routes to filesystem::list_directory
└────────┬────────┘
         │
         ├─> Fetches tool schema from filesystem server
         ├─> Registers list_directory dynamically
         ├─> Sends tools/list_changed notification
         └─> Returns schema + rationale

┌─────────────────┐
│   Claude Code   │ Now sees list_directory in tools list
└────────┬────────┘ Calls it directly with accurate parameters
         │
         ▼
┌─────────────────┐
│ list_directory  │ Proxied to real filesystem MCP server
└─────────────────┘ Returns results
```

### Client Capability Detection

On initialization, Agentic-Warden **tests** if the client supports dynamic tools:

```rust
// Send test notification with 500ms timeout
send_notification(ToolListChangedNotification)

✅ Success → Dynamic Registration Mode
⚠️  Timeout/Error → Two-Phase Negotiation Mode
```

No hardcoded client lists - pure test-based detection.

## 🔧 Claude Code Integration

### Automatic Hooks Setup

Agentic-Warden automatically manages Claude Code hooks for conversation history capture:

```bash
~/.config/claude-code/hooks/
├── SessionEnd.sh           # Captures conversation on exit
└── PreCompact.sh          # Captures before context compaction
```

These hooks send conversation data to Agentic-Warden's history database for semantic search.

### Conversation History Search

```bash
# In Claude Code, use the search_history tool
"Search for conversations about git configuration"

# Returns semantically similar past conversations
# with relevance scores and full context
```

## 📊 Features in Detail

### ✅ Intelligent Routing
- LLM ReAct reasoning for complex requests
- Vector semantic search for fast lookups
- Multi-stage candidate selection and reranking
- Conversation context integration

### ✅ Dynamic Tool Registration
- On-demand tool exposure (98% token reduction)
- Automatic `tools/list_changed` notifications
- Tool schema proxying and validation
- Seamless tool calling via proxy

### ✅ Client Capability Detection
- Test-based dynamic tools support detection
- Automatic mode adaptation
- Graceful fallback to two-phase negotiation
- No client version dependencies

### ✅ Conversation History
- Semantic search with FastEmbed (AllMiniLML6V2)
- SahomeDB file-based vector storage
- Claude Code hooks integration
- Session and compaction event capture

### ✅ Health Monitoring
- Per-server health checks
- Configurable intervals and timeouts
- Automatic server reconnection
- Status reporting

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test dynamic_tools
cargo test capability_detector
cargo test routing_integration

# Run with coverage
cargo tarpaulin --out Html
```

### Test Coverage

- **Unit Tests**: DynamicToolManager, ClientCapabilities, routing logic
- **Integration Tests**: End-to-end routing workflows, dynamic registration flow
- **Mock Tests**: MCP client simulation for both Dynamic and Query modes

See [`tests/`](./tests/) directory for test implementation details.

## 📖 Documentation

- **Requirements**: [SPEC/01-REQUIREMENTS.md](./SPEC/01-REQUIREMENTS.md)
- **Architecture**: [SPEC/02-ARCHITECTURE.md](./SPEC/02-ARCHITECTURE.md)
- **API Reference**: [SPEC/03-API.md](./SPEC/03-API.md)
- **Audit Report**: [AUDIT_REPORT.md](./AUDIT_REPORT.md)

## 🛠️ Development

### Project Structure

```
agentic-warden/
├── src/
│   ├── mcp/
│   │   ├── mod.rs                    # MCP server implementation
│   │   ├── capability_detector.rs    # Client capability detection
│   │   └── dynamic_tools.rs          # Dynamic tool registration
│   ├── mcp_routing/
│   │   ├── mod.rs                    # Intelligent routing logic
│   │   ├── models.rs                 # Request/response models
│   │   ├── decision/                 # LLM ReAct + Vector search
│   │   └── execution/                # Tool execution
│   ├── memory/
│   │   └── mod.rs                    # Conversation history store
│   └── main.rs
├── tests/                            # Integration tests
├── .mcp.json.example                 # Example configuration
└── SPEC/                             # Technical specifications
```

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run with logging
RUST_LOG=debug cargo run -- mcp
```

## 🤝 Contributing

Contributions welcome! Please:

1. Check [GitHub Issues](https://github.com/putao520/agentic-warden/issues)
2. Follow existing code style
3. Add tests for new features
4. Update documentation as needed

## 📜 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **MCP Protocol**: [modelcontextprotocol.io](https://modelcontextprotocol.io)
- **rmcp**: Rust MCP SDK by [pkolaczk](https://github.com/pkolaczk/rmcp)
- **FastEmbed**: Fast embedding generation
- **Claude Code**: Anthropic's official CLI for Claude

---

**Agentic-Warden** - Intelligent MCP Routing for Token-Efficient AI Assistants
