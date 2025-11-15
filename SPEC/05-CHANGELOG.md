# Change Log - v0.x

## Version Information
- Current version: v0.1.0
- Latest planned: v0.3.0
- Start date: 2025-11-08
- Last updated: 2025-11-15

---

## v0.3.0 - 动态JS编排工具系统 (🟡 Planned)

### 🆕 New Features

#### REQ-013: 动态JS编排工具系统
**Status**: 🟡 Planned
**Priority**: High

**核心功能**:
- **DynamicToolRegistry**: 作为MCP工具定义的SSOT(Single Source of Truth)
  - 基础工具(永久): `intelligent_route`, `search_history`
  - 动态工具(TTL=600秒): JS编排工具 + 代理MCP工具
  - 自动清理过期工具(每60秒检查)
  - 最大100个动态工具限制

- **intelligent_route LLM优先路由** (带Fallback):
  - **LLM不存在**: 直接使用向量搜索模式(不尝试LLM)
  - **LLM存在**: 优先尝试LLM编排,失败则自动fallback到向量搜索

  - **LLM编排模式** (优先尝试):
    - LLM分析任务并规划执行步骤
    - 生成组合多个MCP工具的JS函数
    - 代码验证通过后注册为单一动态编排工具
    - 返回: "Use the 'xxx' tool to solve your problem"
    - 失败条件: LLM超时/无效响应/代码验证失败 → 触发fallback

  - **向量搜索模式** (Fallback保障):
    - 两层向量搜索(工具级+方法级)
    - 聚类算法筛选top-5候选
    - 批量注册为代理工具
    - 返回: "Found N tools. Choose which ones to use: ..."

- **Boa JS引擎集成**:
  - 安全沙箱环境(禁用eval, Function, import等危险API)
  - 执行限制: 30秒超时, 256MB内存, 128层调用栈
  - MCP函数注入(mcp前缀命名,如mcpGitStatus)
  - 运行时连接池(复用Boa实例)

- **LLM驱动的代码生成**:
  - 工作流规划: 分析任务可行性,规划步骤
  - JS代码生成: 生成`async function workflow(input){...}`
  - 多层验证: 语法检查 + 安全检查 + Dry-run测试

### 🏗️ Architecture Changes

#### ARCH-013: 动态JS编排工具系统架构
- **DynamicToolRegistry**作为核心注册表(SSOT)
- **LLM优先路由**: LLM存在时优先尝试编排,失败自动fallback到向量搜索
- **健壮性设计**: 任何LLM失败场景都有向量搜索兜底
- **MCP Protocol集成**: list_tools从Registry读取, tools/call路由到执行器
- **工具执行器**: JsExecutor(Boa) + ProxyExecutor(RMCP)

### 📦 New Dependencies

```toml
boa_engine = "0.17"         # JavaScript引擎
boa_gc = "0.17"             # Boa垃圾回收
swc_ecma_parser = "0.142"   # JS解析器(验证)
swc_ecma_ast = "0.110"      # AST分析
deadpool = "0.10"           # 运行时池管理
regex = "1.10"              # 安全检查(危险模式检测)
```

### ⚡ Performance Targets

| 操作 | 目标延迟 |
|-----|---------|
| Registry读取(list_tools) | < 50ms |
| LLM工作流规划 | < 3s |
| JS代码生成 | < 3s |
| 代码验证 | < 100ms |
| Boa初始化(从池获取) | < 50ms |
| MCP函数注入 | < 200ms |
| JS工具执行 | < 30s |
| 工具注册 | < 10ms |

### 🔒 Security Enhancements

- **JS沙箱隔离**: 禁用所有危险的JavaScript全局对象
- **代码验证**: 多层安全检查(语法+危险模式检测+执行测试)
- **资源限制**: 执行时间/内存/调用栈严格限制
- **运行时隔离**: 每次执行独立的Boa context

### 📝 Breaking Changes

无破坏性变更,完全向后兼容。

### 🐛 Known Limitations

- LLM代码生成质量依赖于Ollama模型能力
- JS工具仅在TTL(10分钟)内有效
- 最大支持100个并发动态工具

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