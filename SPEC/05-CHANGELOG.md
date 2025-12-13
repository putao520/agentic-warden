# Change Log - v0.x

## Version Information
- Current version: v5.3.0
- Latest planned: v0.3.0
- Start date: 2025-11-08
- Last updated: 2025-11-19

---

## v5.3.0 - MCP服务器管理CLI命令 (🟢 Released, 2025-11-19)

### 🆕 New Features

**MCP Management CLI Commands**:
实现完整的MCP服务器管理CLI命令集，简化MCP配置管理工作流程。

**7个核心命令**:
```bash
aiw mcp list                              # 列出所有MCP服务器
aiw mcp add <name> <command> [args...]    # 添加MCP服务器
aiw mcp remove <name> [-y]                # 移除MCP服务器
aiw mcp get <name>                        # 查看服务器详细配置
aiw mcp enable <name>                     # 启用服务器
aiw mcp disable <name>                    # 禁用服务器
aiw mcp edit                              # 用编辑器打开mcp.json
```

**功能特性**:
- ✅ 友好的彩色终端输出
- ✅ 表格格式展示服务器列表
- ✅ YAML格式输出配置详情
- ✅ 环境变量自动脱敏（API keys等）
- ✅ 交互式确认提示（remove命令）
- ✅ JSON格式验证和自动恢复（edit命令）
- ✅ $EDITOR环境变量支持
- ✅ 100% Claude Code配置兼容

**使用示例**:
```bash
# 添加filesystem服务器
aiw mcp add filesystem npx --description "Filesystem operations" --category system \
  -- -y @modelcontextprotocol/server-filesystem /home/user

# 列出所有服务器
aiw mcp list

# 临时禁用某个服务器
aiw mcp disable brave-search

# 直接编辑配置文件
aiw mcp edit
```

**Configuration Hot Reload**:
实现MCP配置文件的自动监听和热重载，无需重启进程即可应用配置更改。

**核心功能**:
- ✅ 使用 `notify` 库监听 `~/.aiw/mcp.json` 文件变化
- ✅ 智能生命周期管理:
  - 🗑️ 自动关闭被删除的MCP服务器
  - ⏸️ 自动关闭被禁用的MCP服务器
  - 🔄 自动重启配置改变的服务器(command/args/env)
  - ✨ 保持未变化的服务器继续运行
  - 🆕 懒加载新增的服务器(首次调用时启动)
- ✅ 利用RMCP的 `kill_on_drop` 自动清理子进程
- ✅ 100ms延迟确保文件写入完成
- ✅ 支持编辑器原子写入(vim等)
- ✅ CLI命令(add/remove/enable/disable)更改即时生效

**技术实现**:
- 文件监听在独立线程运行，通过mpsc channel与tokio runtime通信
- 配置包装在 `Arc<RwLock<Arc<McpConfig>>>` 实现线程安全的热更新
- 对比新旧配置智能决定哪些服务需要重启
- 无需重启 `aiw mcp serve` 或任何进程

### 📁 Implementation

**新增模块**:
- `src/commands/mcp/mod.rs` - MCP命令入口和路由
- `src/commands/mcp/config_editor.rs` - 配置文件编辑器工具类
- `src/commands/mcp/list.rs` - 列表展示命令
- `src/commands/mcp/add.rs` - 添加服务器命令
- `src/commands/mcp/remove.rs` - 移除服务器命令
- `src/commands/mcp/get.rs` - 查看配置命令
- `src/commands/mcp/enable_disable.rs` - 启用/禁用命令
- `src/commands/mcp/edit.rs` - 编辑器集成命令
- `src/mcp_routing/config_watcher.rs` - 配置文件热重载监听器

**修改模块**:
- `src/mcp_routing/pool.rs` - 添加配置热更新和服务生命周期管理
- `src/mcp/mod.rs` - 集成配置监听器启动

**新增依赖**:
```toml
colored = "2.1"          # 彩色终端输出
prettytable-rs = "0.10"  # 表格格式化
dialoguer = "0.11"       # 交互式提示
which = "6.0"            # 命令查找
notify = "8.2"           # 文件系统事件监听
```

### 🎯 Design Principles

- **简单实用** - 只做配置管理，不做包注册表
- **单一级别** - 只操作 `~/.aiw/mcp.json`
- **Claude Code兼容** - 配置格式100%兼容
- **用户友好** - 丰富的彩色输出和清晰的错误提示

### 📖 Documentation

- `docs/MCP_CLI_SIMPLE_DESIGN.md` - 简化版MCP CLI设计文档
- README.md更新 - 添加MCP管理命令说明

### ✅ Testing

- [x] McpConfigEditor单元测试通过
- [x] 所有命令手动测试通过
- [x] JSON验证和错误恢复测试通过
- [x] 环境变量脱敏功能测试通过

**Commits**:
- `2b7f399`: feat: 实现MCP服务器管理CLI命令
- `b0bb8d6`: feat: 实现MCP配置文件热重载和服务生命周期管理

---

## v5.2.0 - 配置路径统一与Claude Code兼容性增强 (🟢 Released, 2025-11-19)

### 🔧 Configuration & Compatibility

**Configuration Path Unification**:
- 统一所有持久化配置路径使用 `~/.aiw/` 目录
- 移除对 `~/.agentic-warden/` 和 `~/.config/agentic-warden/` 的支持
- 运行时数据保持在系统临时目录 `/tmp/.aiw/` (Linux/macOS) 或 `%TEMP%\.aiw\` (Windows)
- 配置文件路径标准化：
  - `~/.aiw/mcp.json` - MCP服务器配置(全局唯一)
  - `~/.aiw/provider.json` - Provider配置
  - `~/.aiw/auth.json` - 认证信息
  - `~/.aiw/config.json` - 主配置文件
  - `/tmp/.aiw/aiw.log` - 日志文件(运行时)

**Claude Code 100% Compatibility**:
- MCP配置完全兼容Claude Code格式
- 仅支持全局配置文件 `~/.aiw/mcp.json`(移除项目级mcp.json支持)
- 新增可选字段支持：
  - `description`: MCP服务器描述
  - `category`: 服务器分类(system, development, search等)
  - `enabled`: 启用/禁用开关(默认true)
  - `healthCheck`: 健康检查配置(enabled, interval, timeout)
- 自动过滤已禁用的服务器(`enabled: false`)

**AI CLI Process Detection Enhancement**:
- 添加 `claude-code` 二进制名称检测支持
- 保持专注于AI CLI检测(claude, claude-code, codex, gemini)
- 移除过度工程化的功能(环境变量AIW_CLI_TYPE、python/bash/zsh检测等)
- 仅为AI CLI工具检测node/npm/npx进程

**Configuration Management**:
- 移除未使用的环境变量 `AGENTIC_WARDEN_MCP_CONFIG`
- LLM配置完全通过环境变量管理(OPENAI_TOKEN, OPENAI_ENDPOINT, OPENAI_MODEL)
- 环境变量优先级高于provider.json配置

### 📁 File Changes

**Modified Files**:
- `src/core/process_tree.rs` - AI CLI检测逻辑简化和claude-code支持
- `src/utils/config_paths.rs` - 配置路径从.agentic-warden迁移到.aiw
- `src/mcp_routing/config.rs` - MCP配置Claude Code兼容性增强
- `src/mcp_routing/pool.rs` - 添加enabled字段过滤
- `src/sync/sync_config.rs` - 路径更新
- `src/sync/sync_config_manager.rs` - 路径更新
- `src/mcp/mod.rs` - 路径更新
- `mcp.json.example` - 更新为Claude Code兼容格式

**Commits**:
- `b889314`: refactor: 优化MCP配置管理,100%兼容Claude Code
- `9e4dcdb`: fix: 修复AI CLI进程识别和配置路径问题
- `cc0fa40`: Revert "enhance: 改进AI CLI进程检测逻辑"

### 🎯 Design Principles

**简化原则**:
- 只维护AI CLI，不管理通用解释器
- 全局配置优先，移除多层级配置支持
- 与Claude Code等工具保持100%配置兼容

**零门槛配置**:
- 统一配置目录结构
- 标准化文件路径
- 自动创建必要目录

### 🔜 Planned Features (v5.3.0)

**MCP Management CLI Commands** (设计阶段):
基于MCPM和Claude Code的最佳实践，计划实现：

**核心命令**:
- `aiw mcp list` - 列出所有MCP服务器及状态
- `aiw mcp add <name> <command> [args...]` - 添加MCP服务器
- `aiw mcp remove <name>` - 移除MCP服务器
- `aiw mcp get <name>` - 获取服务器详细配置
- `aiw mcp edit <name>` - 编辑服务器配置

**状态控制**:
- `aiw mcp enable <name>` - 启用服务器
- `aiw mcp disable <name>` - 禁用服务器
- `aiw mcp restart <name>` - 重启服务器连接

**健康检查**:
- `aiw mcp test <name>` - 测试服务器连接
- `aiw mcp health [name]` - 检查健康状态
- `aiw mcp tools <name>` - 列出服务器提供的工具

**高级功能**:
- `aiw mcp validate` - 验证mcp.json配置
- `aiw mcp export` - 导出配置
- `aiw mcp import <file>` - 导入配置

**Package Registry Strategy** (研究阶段):
- 评估Smithery.ai集成可能性
- 考虑GitHub MCP Registry作为数据源
- 可选功能：`aiw mcp search <query>` 和 `aiw mcp install <package>`

### 📖 Breaking Changes

**配置路径迁移** (需要用户手动操作):
```bash
# 如果存在旧配置，需要手动迁移
mv ~/.agentic-warden ~/.aiw
# 或
mv ~/.config/agentic-warden ~/.aiw
```

**MCP配置文件位置变更**:
- 旧: `~/.config/agentic-warden/mcp.json` 或项目目录 `mcp.json`
- 新: `~/.aiw/mcp.json` (仅全局配置)

---

## v5.1.1 - 二进制命名修复 (🟢 Released, 2025-11-16)

### 🐛 Bug Fixes

**Binary Configuration Fix**:
- 添加显式 `[[bin]]` 配置节到 Cargo.toml
- 确保编译产物统一为 `aiw` 二进制文件(之前会同时生成 `agentic-warden`)
- 清理 README 和 SPEC 文档中所有命令行示例,统一使用 `aiw` 命令
- 保留配置目录路径 `~/.config/agentic-warden/` 不变

**Commits**:
- `4ccb776`: fix: 明确指定二进制文件名为aiw
- `c985560`: docs: 统一二进制命令名称为aiw

---

## v0.3.0 - Future Enhancements (🟡 Planned)

### 🚀 Planned Features

*详细功能规划待定，可能包括：性能优化、新的AI CLI支持、更多MCP工具等*

---

## v0.2.0 - 动态JS编排工具系统 (🟢 Released, 2025-11-16)

### 🆕 New Features

#### REQ-013: 动态JS编排工具系统
**Status**: 🟢 Done
**Priority**: High
**Released**: v0.2.0

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

#### REQ-014: AI CLI角色系统 (Phase 1)
**Status**: 🟢 Done (Phase 1), ⏸️ Pending (Phase 2-3)
**Priority**: High
**Released**: v0.2.0 (Phase 1)

**Phase 1: Role Management System** (✅ Completed):
- **Role File Storage**: Markdown-based role configurations in `~/.aiw/role/` directory
  - File format: `<description>\n------------\n<content>`
  - Description: Short summary for role listings
  - Content: Full role prompt content

- **Role Management Module** (`src/roles/mod.rs`, 269 lines):
  - `Role` struct: {name, description, content, file_path}
  - `RoleInfo` struct: {name, description, file_path} (lightweight for MCP)
  - `RoleManager`: Scan and parse role files from disk
  - `RoleError`: Comprehensive error types (8 variants)

- **Security Features**:
  - ✅ Path traversal防护: `fs::canonicalize()` + `starts_with()` validation
  - ✅ File size limit: 1MB maximum per role file
  - ✅ UTF-8 encoding validation: Reject invalid encodings
  - ✅ Name validation: Block path separators and traversal attempts
  - ✅ Delimiter validation: Require 12-dash `------------` separator

- **MCP Tool Integration**:
  - `list_roles` MCP tool: List all available role configurations
  - Returns `Vec<RoleInfo>` with role metadata
  - Auto-filters non-.md files
  - Sorts by name

- **Comprehensive Testing** (`tests/roles_tests.rs`, 96 lines):
  - ✅ Role file parsing with delimiter
  - ✅ list_all_roles returns all roles
  - ✅ File not found error handling
  - ✅ Path traversal rejection
  - ✅ File size limit enforcement
  - **Test Results**: 5/5 tests passing

**Phase 2-3: Task Lifecycle MCP Tools** (✅ Completed, v0.2.0):
- ✅ `start_task` (src/mcp/mod.rs:230-283): Launch AI CLI tasks with optional role parameter
  - Role injection: `{role.content}\n\n---\n\n{task}` format
  - Async task spawning with PID tracking
  - Returns TaskLaunchInfo {pid, ai_type, task, log_file, status}
- ✅ `stop_task` (src/mcp/mod.rs:326-377): Stop running tasks by PID
  - Graceful termination: SIGTERM → wait 5s → SIGKILL
  - Registry cleanup via mark_completed
- ✅ `list_tasks` (src/mcp/mod.rs:311-322): List all tracked MCP tasks
  - Filters zombie processes using `platform::process_alive()`
  - Returns Vec<TaskInfo> {pid, log_id, log_path, status, started_at}
- ✅ `get_task_logs` (src/mcp/mod.rs:386-412): Retrieve task log contents
  - Full log mode and tail mode (last N lines)
  - File read with error handling
- ✅ Integration tests (tests/task_lifecycle_tests.rs, 5/5 passing):
  - start_task_launches_and_returns_pid
  - start_task_injects_role_prompt
  - list_tasks_returns_running_tasks
  - stop_task_terminates_process
  - get_task_logs_supports_full_and_tail_modes

**REQ-013 Schema修正机制增强** (✅ Completed, v0.2.0):
- ✅ 移除mcp_dependencies依赖跟踪(统一使用mcp.call接口)
- ✅ prompts.rs: Schema修正Prompt构建器
- ✅ schema_validator.rs: Schema结构验证器(SchemaValidationResult)
- ✅ schema_corrector.rs: 双层修正机制
  - SchemaCorrector: 静态分析修正(regex推断参数)
  - IterativeSchemaFixer: LLM迭代修正循环(最多3次)
- ✅ injector.rs: 统一mcp对象注入(mcp.call(server, tool, args))
- ✅ decision.rs: 新增chat_completion方法支持Schema LLM修正

#### Provider Scenario Optimization
**Status**: 🟢 Done
**Priority**: Medium
**Released**: v0.2.0

**核心改进**:
- **Provider Configuration Enhancement** (src/provider/config.rs:38-41):
  - Added optional `scenario` field to Provider struct for usage description
  - Backward compatible with existing configs (Option<String>)
  - Updated `Provider::summary()` to display scenario information
  - Dynamic ENV injection via `get_all_env_vars()` (lines 182-202):
    - Auto-maps `token` field to `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`
    - Auto-maps `base_url` field to `ANTHROPIC_BASE_URL` or `OPENAI_BASE_URL`
    - Merges user-defined `env` fields with auto-generated mappings
    - Enables flexible provider configuration without hardcoding env var names

- **MCP Tool Documentation Improvement** (src/mcp/mod.rs:121-126):
  - Enhanced StartTaskParams provider field with detailed JsonSchema descriptions
  - All providers and their scenarios are user-defined in ~/.agentic-warden/providers.json
  - Users can add `scenario` field when configuring providers to help AI choose the right one
  - Example: `{"scenario": "Best for production workloads with official API"}`

- **Testing** (src/provider/config.rs:271-290):
  - Added `test_provider_with_scenario` test verifying scenario display
  - Added `test_provider_backward_compatibility` test ensuring old configs work
  - All 5 provider config tests passing

#### MCP工具重构 - 移除冗余工具
**Status**: 🟢 Done
**Priority**: Medium
**Released**: v0.2.0
**Issue**: #12

**核心改进**:
- **移除3个冗余MCP工具** (src/mcp/mod.rs):
  - `get_method_schema`: Schema已包含在动态工具定义中,无需单独获取
  - `execute_tool`: 主LLM在intelligent_route返回后直接调用工具,不需要中间执行层
  - `list_roles`: 应该是CLI命令而非MCP工具

- **CLI命令替代** (src/commands/parser.rs, src/main.rs):
  - 新增`agentic-warden roles list`命令替代list_roles MCP工具
  - 实现RolesAction enum和handle_roles_command处理器
  - 友好的用户界面输出(显示角色名、描述、文件路径)

- **验证结果**:
  - ✅ cargo build成功 (14.40s)
  - ✅ 所有129个单元测试通过 (2.00s)
  - ✅ 移除相关结构体和imports清理完毕
  - ✅ `agentic-warden roles list`命令工作正常

### 🏗️ Architecture Changes

#### ARCH-013: 动态JS编排工具系统架构 (🟢 Adopted)
- **DynamicToolRegistry**作为核心注册表(SSOT)
- **LLM优先路由**: LLM存在时优先尝试编排,失败自动fallback到向量搜索
- **健壮性设计**: 任何LLM失败场景都有向量搜索兜底
- **MCP Protocol集成**: list_tools从Registry读取, tools/call路由到执行器
- **工具执行器**: JsExecutor(Boa) + ProxyExecutor(RMCP)

#### ARCH-001: Module 1架构补充 - 供应商管理与AI CLI维护
- **子模块1.1: 供应商管理 (Provider Management)**
  - 多供应商支持配置(OpenRouter、Anthropic、Google等)
  - 环境变量动态注入机制
  - 兼容性验证和健康检查
  - API Key自动脱敏保护

- **子模块1.2: AI CLI本地维护 (AI CLI Maintenance)**
  - 自动检测和版本识别
  - 安装状态监控和建议
  - 可执行路径缓存
  - TUI状态展示界面

#### ARCH-014: AI CLI角色系统架构 (🟡 Partial - Phase 1 ✅ Adopted)
- **Role Storage Layer**: Markdown files in `~/.aiw/role/` with 12-dash delimiter
- **Role Management Module**: `src/roles/mod.rs` with secure file parsing
- **Security Design**:
  - Path traversal prevention: `fs::canonicalize()` + prefix validation
  - File size limit: 1MB per role file
  - UTF-8 encoding enforcement
  - Name validation: Block path separators
- **MCP Integration**: `list_roles` tool returns `Vec<RoleInfo>`
- **Error Handling**: Custom `RoleError` enum with 8 error types
- **Testing**: Unit tests in `tests/roles_tests.rs` (5/5 passing)
- **Phase 2-3 (Planned)**: Task lifecycle MCP tools integration

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