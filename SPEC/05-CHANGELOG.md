# Change Log - v0.1.x

## Version Information
- Current version: v0.1.0
- Start date: 2025-11-08
- Last updated: 2025-11-14

---

## v0.1.0 - Initial Release (2025-11-14)

### 🆕 New Features

#### REQ-010: 内存集成与语义搜索
- **Vector Database**: Integrated Qdrant for semantic conversation search
- **Embedding Service**: Integrated Ollama for text vectorization with configurable models
- **Session Management**: Session-based conversation storage in Qdrant metadata
- **MCP Memory Tools**: Added two new MCP tools:
  - `search_history`: Query conversation history with session_id filtering
  - `get_session_todos`: Query incomplete TODOs by session_id
- **TODO Management**: Session-associated TODO system with status tracking

#### Wait Mode Enhancement
- **Cross-Process Waiting**: Enhanced wait command to monitor both CLI and MCP registries
- **Comprehensive Monitoring**: Wait only exits when all tasks in both registries are completed
- **Unified Reporting**: Combined task completion reporting from both registries

### 🔧 Major Refactoring

#### Code Quality Improvements
- **Duplicate Code Cleanup**: Removed 536KB+ duplicate code across modules
- **Common Module System**: Created unified `src/common/` module (758 lines):
  - `utils.rs`: Shared utility functions
  - `constants.rs`: Centralized constants
  - `messages.rs`: Type-safe message system
  - `data_structures.rs`: Common data structures
  - `screen_base.rs`: Screen trait base implementations
- **MCP Simplification**: Reduced from 7 tools to 4 focused tools, eliminating redundant functionality

#### Architecture Improvements
- **TUI Component Factory**: Unified component creation for consistent UI
- **Sync Service Layer**: Abstracted sync operations with trait-based design
- **Memory Module**: Complete integration of gmemory functionality with clean API

### 📊 Technical Specifications

#### MCP Tools (v0.2.0)
| Tool | Description | Status |
|------|-------------|--------|
| `start_concurrent_tasks` | Concurrent AI CLI task management | ✅ |
| `get_task_command` | Single AI CLI task command generation | ✅ |
| `search_history` | Semantic conversation history search | 🆕 |
| `get_session_todos` | Session-based TODO management | 🆕 |

#### Memory Configuration
```yaml
# ~/.agentic-warden/providers.json (auto-generated)
[memory]
ollama_url = "http://localhost:11434"
qdrant_url = "http://localhost:6333"
embedding_model = "qwen3-embedding:0.6b"
llm_model = "qwen3:8b"
```

### 🧪 Testing & Quality
- **All Tests Pass**: 205 tests across all modules
- **Memory Integration**: 5 new memory-specific tests
- **MCP Functionality**: Comprehensive MCP tool testing
- **Performance**: Optimized shared memory and vector operations

### 📝 Documentation Updates
- **SPEC-01-REQUIREMENTS.md**: Updated to reflect memory integration and MCP changes
- **README.md**: Updated to include new memory features
- **API Documentation**: Comprehensive MCP tool documentation

### 🐛 Bug Fixes
- **MCP API Compatibility**: Fixed rmcp library integration issues
- **Memory Metadata**: Corrected session_id storage in Qdrant metadata
- **Wait Mode**: Fixed cross-process task monitoring
- **Compilation Issues**: Resolved all post-refactoring compilation errors

---

##  Historical Notes

### Development Philosophy
- **SPEC-Driven Development**: All features documented in SPEC before implementation
- **Simplified Design**: Removed complex features like regionalization and recommendation engines
- **Type Safety**: Extensive use of Rust's type system for error prevention
- **Performance Optimization**: Shared memory and efficient process tracking