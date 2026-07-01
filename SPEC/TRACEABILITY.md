# AIW (Agentic-Warden) — Requirements Traceability Matrix

**Version**: v0.5.99
**Last Updated**: 2026-03-19
**Document Status**: Active

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

**Status**: 🟢 Done
**SPEC References**: ARCH-001, DATA-005, API-006

**Implementation Files**:
- `src/core/process_tree.rs` — 进程树向上遍历，识别 AI CLI 根进程
- `src/core/shared_map.rs` — 共享内存映射，跨进程任务协调
- `src/core/models.rs` — 核心数据模型
- `src/platform/unix.rs` — Unix 平台进程检测 (procfs)
- `src/platform/windows.rs` — Windows 平台进程检测 (winapi)
- `src/supervisor.rs` — 进程监控与执行
- `src/storage.rs` — 任务持久化存储
- `src/registry.rs` — 任务注册表
- `src/unified_registry.rs` — 统一注册表
- `src/registry_factory.rs` — 注册表工厂
- `src/task_record.rs` — 任务记录模型

**Test Files**:
- `tests/integration/config_integration.rs` — 配置集成测试
- `tests/integration/shared_memory_isolation.rs` — 共享内存隔离测试
- `tests/task_lifecycle_tests.rs` — 任务生命周期测试

**Notes**: 使用共享内存实现亚毫秒级跨进程协调，按 AI CLI 根进程进行命名空间隔离。

---

### REQ-002: 第三方 Provider 管理

**Status**: 🟢 Done
**SPEC References**: ARCH-002, DATA-001, API-007

**Implementation Files**:
- `src/provider/manager.rs` — Provider 生命周期管理
- `src/provider/config.rs` — Provider 配置读写 (JSON schema 验证)
- `src/provider/env_injector.rs` — 环境变量透明注入
- `src/provider/env_mapping.rs` — 环境变量映射规则
- `src/provider/error.rs` — Provider 领域错误类型
- `src/tui/screens/provider.rs` — TUI Provider 管理界面

**Test Files**:
- `tests/unit/provider_config.rs` — Provider 配置单元测试
- `tests/integration/third_party_provider_test.rs` — 第三方 Provider 集成测试

**Notes**: 透明环境变量注入，无需修改 AI CLI 配置文件。

---

### REQ-003: Google Drive 配置记录和同步

**Status**: ❌ Deprecated
**SPEC References**: ARCH-003, DATA-002, DATA-003, DATA-006, API-003, API-010

**Implementation Files**:
- `src/sync/google_drive_service.rs` — Google Drive 服务 (disabled)
- `src/sync/oauth_client.rs` — OAuth 客户端 (disabled)
- `src/sync/config_packer.rs` — 配置打包器 (disabled)
- `src/sync/sync_command.rs` — 同步命令处理 (disabled)
- `src/sync/directory_hasher.rs` — 目录哈希 (disabled)
- `src/sync/config_sync_manager.rs` — 配置同步管理器 (disabled)
- `src/sync/smart_oauth.rs` — 智能 OAuth (disabled)
- `src/sync/sync_config.rs` — 同步配置 (disabled)
- `src/sync/sync_config_manager.rs` — 同步配置管理器 (disabled)
- `src/sync/error.rs` — 同步错误类型 (disabled)

**Notes**: Google Drive 云存储集成已禁用 (v0.5.19+)，push/pull 命令不可用。

---

### REQ-004: 统一 TUI 体验

**Status**: 🟢 Done
**SPEC References**: ARCH-004, API-002

**Implementation Files**:
- `src/tui/app.rs` — TUI 应用主循环 (ratatui + crossterm)
- `src/tui/app_state.rs` — TUI 状态管理
- `src/tui/data_binding.rs` — 数据绑定层
- `src/tui/screens/dashboard.rs` — 仪表盘界面
- `src/tui/screens/provider.rs` — Provider 管理界面
- `src/tui/screens/status.rs` — 状态界面
- `src/tui/screens/installed_mcp.rs` — 已安装 MCP 列表界面
- `src/tui/screens/cli_order.rs` — CLI 排序界面
- `src/tui/screens/render_helpers.rs` — 渲染辅助函数
- `src/tui/screens/mod_simple.rs` — 简化屏幕模块
- `src/tui/components/component_factory.rs` — 组件工厂
- `src/tui/components/layout_builder.rs` — 布局构建器
- `src/tui/components/style_manager.rs` — 样式管理器
- `src/common/screen_base.rs` — 屏幕基类

**Test Files**:
- `tests/integration/tui_navigation.rs` — TUI 导航集成测试
- `tests/scrolling_display_test.rs` — 滚动显示测试

**Notes**: 统一设计系统，ratatui 0.26+ 框架，事件驱动架构。

---

### REQ-005: Wait 模式跨进程等待

**Status**: 🟢 Done
**SPEC References**: ARCH-001, DATA-004, API-004, API-008

**Implementation Files**:
- `src/wait_mode.rs` — Wait 模式实现 (全局等待)
- `src/pwait_mode.rs` — PWait 模式实现 (按 PID 等待)
- `src/registry.rs` — 任务注册表
- `src/unified_registry.rs` — 统一注册表
- `src/task_record.rs` — 任务记录管理

**Test Files**:
- `tests/integration/pwait_command.rs` — PWait 命令集成测试
- `tests/integration/pwait_pid_parameter.rs` — PWait PID 参数测试

**Notes**: 支持全局等待 (`wait`) 和进程特定等待 (`pwait`)，超时处理使用人类可读时长格式。

---

### REQ-006: AI CLI 工具检测与状态管理

**Status**: 🟢 Done
**SPEC References**: ARCH-006, API-002

**Implementation Files**:
- `src/cli_manager.rs` — AI CLI 检测/安装/更新管理
- `src/cli_type.rs` — AI CLI 类型定义与参数构建
- `src/utils/version.rs` — 版本管理工具

**Test Files**:
- `tests/unit/status_command.rs` — 状态命令单元测试

**Notes**: 跨平台检测 (which/where)，区分 NPM 包与原生安装，版本检测。

---

### REQ-007: MCP (Model Context Protocol) 服务器

**Status**: 🟢 Done
**SPEC References**: ARCH-007, DATA-007, API-006

**Implementation Files**:
- `src/mcp/mod.rs` — MCP 服务器入口 (AgenticWardenMcpServer)
- `src/mcp/capability_detector.rs` — MCP 能力检测
- `src/mcp/js_executor.rs` — JS 工具执行器
- `src/mcp/table_format.rs` — 表格格式化输出

**Test Files**:
- `tests/real_mcp_integration_test.rs` — MCP 真实集成测试
- `tests/real_downstream_mcp_test.rs` — 下游 MCP 测试
- `tests/tool_count_test.rs` — 工具计数测试

**Notes**: JSON-RPC 2.0 over stdio 传输，rmcp 0.16 协议实现。

---

### REQ-008: 指定供应商模式 AI CLI 启动

**Status**: 🟢 Done
**SPEC References**: ARCH-002, ARCH-008, DATA-001, API-001

**Implementation Files**:
- `src/commands/ai_cli.rs` — AI CLI 命令执行与参数转发
- `src/commands/parser.rs` — CLI 参数解析 (clap)
- `src/commands/cli_args.rs` — CLI 参数定义
- `src/cli_type.rs` — AI CLI 类型定义与参数构建
- `src/task_prepare.rs` — 任务准备共享层
- `src/supervisor.rs` — 进程监控与执行
- `src/provider/env_injector.rs` — 环境变量透明注入

**Test Files**:
- `tests/unit/cli_parser.rs` — CLI 解析器单元测试
- `tests/unit/parameter_forwarding.rs` — 参数转发单元测试
- `tests/integration/cli_end_to_end.rs` — CLI 端到端集成测试
- `tests/e2e_cli_workflow.rs` — E2E CLI 工作流测试

**Notes**: `aiw <ai_type> -p <provider> <prompt>` 语法，环境注入在 exec() 前完成，透明参数转发。

---

### REQ-009: 交互式 AI CLI 启动

**Status**: 🟢 Done
**SPEC References**: ARCH-008, API-001

**Implementation Files**:
- `src/commands/ai_cli.rs` — AI CLI 命令处理 (空 prompt 触发交互模式)
- `src/commands/parser.rs` — CLI 参数解析

**Test Files**:
- `tests/unit/cli_parser.rs` — CLI 解析器单元测试
- `tests/integration/cli_end_to_end.rs` — CLI 端到端集成测试

**Notes**: 空 prompt 启动交互模式，复用 AiCliCommand 基础设施，Provider 注入与任务模式一致。

---

### REQ-010: Claude Code 会话历史集成（Hook-Based）

**Status**: ❌ Deprecated

**Implementation Files**: (已删除)

**Notes**: Claude Code 会话历史集成功能已删除，相关 Hook 机制不再使用。

---

### REQ-011: AI CLI 更新/安装管理

**Status**: 🟢 Done
**SPEC References**: ARCH-008, API-005

**Implementation Files**:
- `src/cli_manager.rs` — AI CLI 检测/安装/更新管理

**Test Files**:
- `tests/unit/status_command.rs` — 状态命令单元测试

**Notes**: 统一管理 Claude/Codex/Gemini CLI 的安装与更新。

---

### REQ-012: 智能 MCP 路由系统

**Status**: 🟢 Done
**SPEC References**: ARCH-012, DATA-012, API-012

**Implementation Files**:
- `src/mcp_routing/decision.rs` — LLM 路由决策引擎
- `src/mcp_routing/index.rs` — 工具索引
- `src/mcp_routing/embedding.rs` — 向量嵌入 (fastembed)
- `src/mcp_routing/registry.rs` — 路由注册表
- `src/mcp_routing/config.rs` — 路由配置 (mcp.json)
- `src/mcp_routing/config_watcher.rs` — 配置热更新监听 (notify)
- `src/mcp_routing/models.rs` — 路由数据模型

**Test Files**:
- `tests/mcp_routing_workflow.rs` — MCP 路由工作流测试
- `tests/real_llm_backend_e2e.rs` — 真实 LLM 后端 E2E 测试
- `tests/dynamic_registry_unit_test.rs` — 动态注册表单元测试
- `tests/integration/dynamic_tool_registry.rs` — 动态工具注册表集成测试

**Notes**: 语义搜索路由，mcp.json 配置，热更新支持。

---

### REQ-013: 动态 JS 编排工具系统

**Status**: 🟢 Done
**SPEC References**: ARCH-013, DATA-012, API-012

**Implementation Files**:
- `src/mcp/js_executor.rs` — JS 工具执行器
- `src/mcp_routing/capability_generator.rs` — 能力描述生成
- `src/mcp_routing/codegen.rs` — 代码生成
- `src/mcp_routing/pool.rs` — Boa 运行时连接池
- `src/mcp_routing/js_orchestrator/engine.rs` — JS 编排引擎
- `src/mcp_routing/js_orchestrator/injector.rs` — 依赖注入器
- `src/mcp_routing/js_orchestrator/validator.rs` — 代码验证器
- `src/mcp_routing/js_orchestrator/workflow_planner.rs` — 工作流规划器
- `src/mcp_routing/js_orchestrator/prompts.rs` — LLM 提示模板
- `src/mcp_routing/js_orchestrator/schema_corrector.rs` — Schema 修正器
- `src/mcp_routing/js_orchestrator/schema_validator.rs` — Schema 验证器

**Test Files**:
- `tests/real_js_execution_test.rs` — 真实 JS 执行测试
- `tests/real_req013_phase1_capability_e2e.rs` — REQ-013 Phase1 能力 E2E 测试
- `tests/real_req013_phase2_dynamic_tool_e2e.rs` — REQ-013 Phase2 动态工具 E2E 测试

**Notes**: Boa 引擎驱动，运行时连接池，LLM 辅助工作流规划。

---

### REQ-014: AI CLI 任务生命周期管理和角色系统

**Status**: 🟢 Done
**SPEC References**: ARCH-014, DATA-004, API-008

**Implementation Files**:
- `src/roles/mod.rs` — 角色系统入口
- `src/roles/builtin.rs` — 内置角色模板 (en/zh-CN)
- `src/supervisor.rs` — 进程监控与执行
- `src/storage.rs` — 任务持久化存储
- `src/registry.rs` — 任务注册表
- `src/unified_registry.rs` — 统一注册表
- `src/registry_factory.rs` — 注册表工厂
- `src/task_record.rs` — 任务记录模型

**Test Files**:
- `tests/unit/roles_tests.rs` — 角色系统单元测试
- `tests/unit/role_param_tests.rs` — 角色参数单元测试
- `tests/task_lifecycle_tests.rs` — 任务生命周期测试

**Notes**: 内置多语言角色模板，完整任务生命周期追踪。

---

### REQ-015: Google Drive OAuth 授权流程

**Status**: ❌ Deprecated
**SPEC References**: ARCH-003, DATA-003, API-003

**Implementation Files**:
- `src/cli_oauth.rs` — OAuth CLI 流程 (disabled)
- `src/sync/oauth_client.rs` — OAuth 客户端 (disabled)
- `src/sync/smart_oauth.rs` — 智能 OAuth (disabled)

**Notes**: Google Drive OAuth public client 不再被 Google 支持，push/pull 命令已禁用 (v0.5.19+)。

---

### REQ-016: MCP 仓库 CLI - 多源聚合搜索与安装

**Status**: 🟢 Done
**SPEC References**: ARCH-015, API-015

**Implementation Files**:
- `src/commands/mcp/registry/mod.rs` — Registry 模块入口
- `src/commands/mcp/registry/aggregator.rs` — 多源聚合器
- `src/commands/mcp/registry/official.rs` — 官方 Registry 源
- `src/commands/mcp/registry/smithery.rs` — Smithery 源
- `src/commands/mcp/registry/install.rs` — 安装逻辑
- `src/commands/mcp/registry/search.rs` — 搜索逻辑
- `src/commands/mcp/registry/info.rs` — 信息查询
- `src/commands/mcp/registry/interactive.rs` — 交互式安装
- `src/commands/mcp/registry/source.rs` — 数据源抽象
- `src/commands/mcp/registry/types.rs` — 类型定义
- `src/commands/mcp/registry/update.rs` — 更新逻辑

**Test Files**:
- `tests/mcp_registry.rs` — MCP Registry 测试
- `tests/real_mcp_registry_cli_e2e.rs` — MCP Registry CLI E2E 测试

**Notes**: 多源聚合搜索 (Official + Smithery)，统一安装流程。

---

### REQ-017: AIW 插件市场系统

**Status**: 🟡 Partial
**SPEC References**: ARCH-017, DATA-017, API-017

**Implementation Files**:
- `src/commands/market/mod.rs` — 插件市场模块入口
- `src/commands/market/cli.rs` — 市场 CLI 命令
- `src/commands/market/cli_marketplace.rs` — 市场 CLI 交互
- `src/commands/market/cli_plugins.rs` — 插件 CLI 管理
- `src/commands/market/cli_utils.rs` — CLI 工具函数
- `src/commands/market/plugin.rs` — 插件模型
- `src/commands/market/plugin_io.rs` — 插件 I/O
- `src/commands/market/installer.rs` — 插件安装器
- `src/commands/market/validator.rs` — 插件验证器
- `src/commands/market/config.rs` — 市场配置
- `src/commands/market/config_utils.rs` — 配置工具
- `src/commands/market/cache.rs` — 缓存管理
- `src/commands/market/filter.rs` — 过滤器
- `src/commands/market/source.rs` — 数据源抽象
- `src/commands/market/github_source.rs` — GitHub 源
- `src/commands/market/local_source.rs` — 本地源
- `src/commands/market/remote_source.rs` — 远程源

**Test Files**:
- `tests/unit/marketplace.rs` — 市场单元测试
- `tests/market_cli.rs` — 市场 CLI 测试
- `tests/marketplace.rs` — 市场集成测试
- `tests/integration/market_cli.rs` — 市场 CLI 集成测试

**Notes**: 基础搜索/安装/管理功能已实现，部分高级功能仍在开发中。

---

### REQ-018: MCP Browse 环境变量快速跳过

**Status**: 🟢 Done
**SPEC References**: ARCH-018

**Implementation Files**:
- `src/commands/mcp/registry/browse.rs` — EnvInputState + Browse 事件处理

**Test Files**:
- `tests/mcp_browse_018_skip_optional.rs` — 快速跳过测试
- `tests/integration/mcp_browse_018_skip_optional.rs` — 快速跳过集成测试
- `tests/e2e/agentic-warden/mcp_browse_complete_workflow.rs` — Browse 完整工作流 E2E 测试

**Notes**: 快速跳过仅在可选变量场景触发，避免破坏必填输入。

---

### REQ-019: MCP Browse 已安装 MCP 服务器查看

**Status**: 🟢 Done
**SPEC References**: ARCH-019, DATA-019

**Implementation Files**:
- `src/commands/mcp/registry/browse.rs` — Browse 界面逻辑
- `src/tui/screens/installed_mcp.rs` — 已安装 MCP 列表界面

**Test Files**:
- `tests/mcp_browse_019_installed_mcps.rs` — 已安装 MCP 测试
- `tests/integration/mcp_browse_019_installed_mcps.rs` — 已安装 MCP 集成测试
- `tests/e2e/agentic-warden/mcp_browse_complete_workflow.rs` — Browse 完整工作流 E2E 测试

**Notes**: 列表加载复用 McpConfigManager，按名称排序保证可预测导航。

---

### REQ-020: MCP Browse 已安装 MCP 环境变量编辑

**Status**: 🟢 Done
**SPEC References**: ARCH-020, DATA-020

**Implementation Files**:
- `src/commands/mcp/registry/browse.rs` — Browse 编辑流程
- `src/tui/screens/installed_mcp.rs` — 已安装 MCP 编辑界面

**Test Files**:
- `tests/mcp_browse_020_edit_env_vars.rs` — 环境变量编辑测试
- `tests/integration/mcp_browse_020_edit_env_vars.rs` — 环境变量编辑集成测试
- `tests/e2e/agentic-warden/mcp_browse_complete_workflow.rs` — Browse 完整工作流 E2E 测试

**Notes**: 预加载原值并跟踪 modified 状态，取消编辑不污染配置。

---

### REQ-021: AI CLI 自动故障切换系统

**Status**: 🟢 Done
**SPEC References**: ARCH-021, DATA-021, API-021

**Implementation Files**:
- `src/auto_mode/mod.rs` — Auto 模式入口
- `src/auto_mode/config.rs` — Auto 模式配置
- `src/auto_mode/judge.rs` — 故障判定与切换逻辑
- `src/commands/auto.rs` — Auto 模式命令

**Test Files**:
- `tests/unit/auto_mode.rs` — Auto 模式单元测试

**Notes**: 自动检测 AI CLI 故障并切换到备选 CLI。

---

### REQ-022: Auto 模式 CLI+Provider 组合轮转

**Status**: 🟢 Done
**SPEC References**: ARCH-021, DATA-022, DATA-023, API-021

**Implementation Files**:
- `src/auto_mode/mod.rs` — Auto 模式入口
- `src/auto_mode/config.rs` — 组合轮转配置
- `src/auto_mode/judge.rs` — 轮转判定逻辑
- `src/commands/auto.rs` — Auto 模式命令

**Test Files**:
- `tests/unit/auto_mode.rs` — Auto 模式单元测试

**Notes**: CLI+Provider 组合轮转，冷却机制防止频繁切换。

---

### REQ-023: Git 仓库检查和 Worktree 管理

**Status**: 🟢 Done
**SPEC References**: ARCH-023

**Implementation Files**:
- `src/worktree.rs` — Git worktree 管理 (git2)

**Notes**: 基于 git2 库实现 worktree 创建/切换/清理。

---

### REQ-024: OpenAI 环境变量配置

**Status**: 🟢 Done
**SPEC References**: ARCH-013, API-013

**Implementation Files**:
- `src/provider/env_mapping.rs` — 环境变量映射规则 (含 OpenAI 映射)
- `src/provider/env_injector.rs` — 环境变量注入
- `src/provider/config.rs` — Provider 配置

**Notes**: OPENAI_ENDPOINT / OPENAI_TOKEN / OPENAI_MODEL 环境变量配置，环境变量优先于配置文件。

---

### REQ-025: Claude CLI Max-Token 补丁系统

**Status**: 🟢 Done

**Implementation Files**:
- `src/patcher/mod.rs` — 补丁系统入口
- `src/patcher/versions.rs` — 通用正则模式 `MAX_CONTEXT_TOKENS_SEARCH_REGEX` + `validate`/`encode_max_context_tokens` + `ClaudeVersion`（版本检测）
- `src/patcher/registry.rs` — `get_max_context_tokens_patches` 生成 max-token patch 模式
- `src/patcher/file.rs` — 文件补丁应用（regex 匹配 + 等长数值替换）
- `src/patcher/runtime.rs` — 内存补丁应用（`apply_max_context_tokens_patch`，整区域 64MB 读入 + regex 扫描，修复跨块漏匹配）
- `src/patcher/types.rs` — `FeatureType::MaxContextTokens` + `UnifiedPatchPattern`（Cow + `use_regex`/`regex_replace_values`）
- `src/patcher/error.rs` — 补丁错误类型
- `src/patcher/platform/mod.rs` — 平台适配入口（`mod native`）
- `src/patcher/platform/unix.rs` — Unix 平台补丁
- `src/patcher/platform/macos.rs` — macOS 平台补丁
- `src/patcher/platform/windows.rs` — Windows 平台补丁
- `src/commands/patch.rs` — Patch CLI 命令（`execute_set_max_tokens`）
- `src/commands/parser.rs` — `PatchAction::SetMaxTokens`
- `src/config.rs` — `PatchConfig`（max_context_tokens / auto_compact_window，默认 500000，持久化到 `~/.aiw/patch.json`）
- `src/supervisor.rs` — `apply_max_context_tokens_patches`（启动时触发）

**Notes**: max-token patch 通过通用正则匹配 Claude CLI 二进制常量块 `var X=200000,Y=200000,...`，把默认上下文窗口（`YOt`）和 autoCompact 阈值（`Pte`）等长替换为可配置值（100000~999999，默认 500000）。变量名无关，跨版本通用（已验证 2.1.195）。替代旧的 firstParty patch（破坏 CC 中转站识别和中国时区识别，已彻底删除）。支持文件补丁（持久化）和内存补丁（运行时），跨平台 (Linux x64/arm64, macOS arm64, Windows x64)。

---

### REQ-026: Claude CLI AntiTelemetry 补丁系统

**Status**: 🟢 Done

**Implementation Files**:
- `src/patcher/mod.rs` — 补丁系统入口（导出 `get_feature_patches`）
- `src/patcher/types.rs` — `FeatureType::AntiTelemetry` + `UnifiedPatchPattern`（Cow + `use_regex=false` 字面量模式）
- `src/patcher/registry.rs` — `get_antitelemetry_patches` 生成 file + memory patch 模式（`/api/event_logging/v2/batch` -> `/api/event_logging/v2/xxxxx`，27 字节等长）
- `src/patcher/file.rs` — 文件补丁应用（字面量匹配 + `replace_pattern` 整段替换）
- `src/patcher/runtime.rs` — `apply_literal_memory_patch`（字面量内存替换，等长校验，命中首个匹配即写入）
- `src/patcher/error.rs` — 补丁错误类型（`PatchError::PatternNotFound`）
- `src/patcher/platform/*.rs` — 平台适配（Unix/macOS/Windows 内存读写）
- `src/commands/patch.rs` — `execute_disable_telemetry` + `execute_apply_patch`（max-token + anti-telemetry 独立应用）+ `execute_patch_status`（anti-telemetry 状态检查）
- `src/commands/parser.rs` — `PatchAction::DisableTelemetry`
- `src/supervisor.rs` — `apply_max_context_tokens_patches`（max-token 后追加 anti-telemetry 内存补丁）+ `apply_antitelemetry_memory_patch_background`（`start_interactive_cli` 后台线程路径）

**Notes**: AntiTelemetry patch 通过字面量替换截断 CC 客户端上报通道：`/api/event_logging/v2/batch` -> `/api/event_logging/v2/xxxxx`（27 字节等长），让上报端点 404 静默失败。阻断 CC v2.1.195 上报机器指纹/设备信息/IP/项目信息的间谍行为。与 max-token patch 独立（一个失败不影响另一个），跨版本稳定（API 路径字面量，非 minified 变量名）。支持文件补丁（持久化）和内存补丁（运行时），启动时自动触发。

---

### REQ-027: Claude CLI AntiSpy 补丁系统

**Status**: 🟢 Done

**Implementation Files**:
- `src/patcher/mod.rs` — 补丁系统入口（导出 `get_feature_patches`）
- `src/patcher/types.rs` — `FeatureType::AntiSpy` + `UnifiedPatchPattern`（Cow + `use_regex=false` 字面量模式）
- `src/patcher/registry.rs` — `get_antispy_patches` 生成 4 个 patch 模式（KIt file+memory, Hsp file+memory）
- `src/patcher/file.rs` — 文件补丁应用（字面量匹配 + `replace_pattern` 整段替换）
- `src/patcher/runtime.rs` — `apply_literal_memory_patch`（字面量内存替换，等长校验，命中首个匹配即写入）
- `src/patcher/error.rs` — 补丁错误类型（`PatchError::PatternNotFound`）
- `src/patcher/platform/*.rs` — 平台适配（Unix/macOS/Windows 内存读写）
- `src/commands/patch.rs` — `execute_disable_spy` + `execute_apply_patch`（max-token + anti-telemetry + anti-spy 独立应用）+ `execute_patch_status`（anti-spy 状态检查）
- `src/commands/parser.rs` — `PatchAction::DisableSpy`
- `src/supervisor.rs` — `apply_max_context_tokens_patches`（anti-telemetry 后追加 anti-spy 内存补丁）+ `apply_antitelemetry_memory_patch_background`（`start_interactive_cli` 后台线程路径，anti-telemetry 后追加 anti-spy）

**Notes**: AntiSpy patch 通过函数级等长字面量替换让 CC 本地识别全失明：(1) `KIt()` 时区识别 `Intl.DateTimeFormat().resolvedOptions().timeZone`（48 字节）-> `"UTC"/*` + 39 个 `.` + `*/`（48 字节注释填充），时区永远返回 UTC，真实时区不泄露，`cnTZ` 永远 false；(2) `Hsp()` 中转站识别 `function Hsp(){if($Sn())return null;let e=Asp()`（47 字节）-> `function Hsp(){return null;         let e=Asp()`（47 字节空格填充），`Hsp()` 永远返回 null，`known`/`labKw`/`cnTZ`/`host` 全 null。不碰 `$Sn()`（保留 firstParty 专属功能）。与 max-token / AntiTelemetry patch 独立（一个失败不影响另一个），跨版本稳定（函数体字面量，非 minified 变量名）。支持文件补丁（持久化）和内存补丁（运行时），启动时自动触发。

---

### REQ-028: Claude CLI AntiPromptBias 补丁系统

**Status**: 🟢 Done

**Implementation Files**:
- `src/patcher/mod.rs` — 补丁系统入口（导出 `get_feature_patches`）
- `src/patcher/types.rs` — `FeatureType::AntiPromptBias` + `UnifiedPatchPattern`（Cow + `use_regex=false` 字面量模式）
- `src/patcher/registry.rs` — `get_antipromptbias_patches` 生成 2 个 patch 模式（file + memory，63 字节等长）
- `src/patcher/file.rs` — 文件补丁应用（字面量匹配 + `replace_pattern` 整段替换）
- `src/patcher/runtime.rs` — `apply_literal_memory_patch`（字面量内存替换，等长校验，命中首个匹配即写入）
- `src/patcher/error.rs` — 补丁错误类型（`PatchError::PatternNotFound`）
- `src/patcher/platform/*.rs` — 平台适配（Unix/macOS/Windows 内存读写）
- `src/commands/patch.rs` — `execute_disable_prompt_bias` + `execute_apply_patch`（max-token + anti-telemetry + anti-spy + anti-prompt-bias 独立应用）+ `execute_patch_status`（anti-prompt-bias 状态检查）
- `src/commands/parser.rs` — `PatchAction::DisablePromptBias`
- `src/supervisor.rs` — `apply_max_context_tokens_patches`（anti-spy 后追加 anti-prompt-bias 内存补丁）+ `apply_antitelemetry_memory_patch_background`（`start_interactive_cli` 后台线程路径，anti-spy 后追加 anti-prompt-bias）

**Notes**: AntiPromptBias patch 通过等长字面量替换消除 CC 给第三方用户注入的 Provider context 提示词偏见：`if(g7())n.push("**Provider context:** This session is not using`（63 字节）-> `if(0   )n.push("**Provider context:** This session is not using`（63 字节空格填充），`if(g7())` 永远 false → Provider context prompt 不注入，模型不感知 provider 差异，行为更一致。只跳过这一条 prompt，不影响其他 firstParty 门控（OAuth/能力/模型选择等照常）。与 max-token / AntiTelemetry / AntiSpy patch 独立（一个失败不影响另一个），跨版本稳定（prompt 字面量，非 minified 变量名）。支持文件补丁（持久化）和内存补丁（运行时），启动时自动触发。

---

## Cross-Cutting Concerns

### Security
| Concern | Related REQs | Implementation |
|---------|-------------|----------------|
| Token/密钥安全 | REQ-002, REQ-024 | `src/provider/env_injector.rs`, 进程隔离 |
| 进程命名空间隔离 | REQ-001 | `src/core/process_tree.rs`, `src/core/shared_map.rs` |
| Max-Token 补丁验证 | REQ-025 | `src/patcher/versions.rs`, `MAX_CONTEXT_TOKENS_SEARCH_REGEX` 通用正则 + `validate_max_context_tokens` 6 位数校验 |

| AntiTelemetry 上报截断 | REQ-026 | `src/patcher/registry.rs`, `get_antitelemetry_patches` 字面量替换 `/api/event_logging/v2/batch` -> `/api/event_logging/v2/xxxxx`（27 字节等长） |
| AntiSpy 本地识别失明 | REQ-027 | `src/patcher/registry.rs`, `get_antispy_patches` 函数级字面量替换 KIt()→UTC（48 字节）+ Hsp()→null（47 字节） |
| AntiPromptBias 提示词偏见消除 | REQ-028 | `src/patcher/registry.rs`, `get_antipromptbias_patches` 字面量替换 if(g7())→if(0   )（63 字节等长，跳过 Provider context prompt） |
### Performance
| Concern | Related REQs | Implementation |
|---------|-------------|----------------|
| 共享内存协调 (< 1ms) | REQ-001, REQ-005 | `src/core/shared_map.rs` |
| 进程树缓存 (5s TTL) | REQ-001 | `src/core/process_tree.rs` |
| Boa 运行时连接池 | REQ-013 | `src/mcp_routing/pool.rs` |
| 配置热更新 | REQ-012 | `src/mcp_routing/config_watcher.rs` |

### Error Handling
| Concern | Related REQs | Implementation |
|---------|-------------|----------------|
| 统一错误类型 | All | `src/error.rs` (thiserror + anyhow) |
| Provider 领域错误 | REQ-002 | `src/provider/error.rs` |
| 补丁错误 | REQ-025 | `src/patcher/error.rs` |
| AntiTelemetry 补丁错误 | REQ-026 | `src/patcher/error.rs`, `PatchError::PatternNotFound`（字面量未找到/replace_pattern 缺失/长度不一致） |
| AntiSpy 补丁错误 | REQ-027 | `src/patcher/error.rs`, `PatchError::PatternNotFound`（字面量未找到/replace_pattern 缺失/长度不一致） |
| AntiPromptBias 补丁错误 | REQ-028 | `src/patcher/error.rs`, `PatchError::PatternNotFound`（字面量未找到/replace_pattern 缺失/长度不一致） |
| 退出码分类 | API | SPEC/04-API-DESIGN.md#Error-Codes |

---

## Coverage Summary

| Status | Count | REQ IDs |
|--------|-------|---------|
| 🟢 Done | 24 | REQ-001, 002, 004, 005, 006, 007, 008, 009, 011, 012, 013, 014, 016, 018, 019, 020, 021, 022, 023, 024, 025, 026, 027, 028 |
| 🟡 Partial | 1 | REQ-017 |
| ❌ Deprecated | 3 | REQ-003, 010, 015 |
| **Total** | **25** | |

**Coverage Rate**: 24/25 active REQs completed (96%), 1 partial

---

## Validation Checklist

- [x] All active requirements have corresponding architecture decisions
- [x] All architecture decisions are implemented in code
- [x] All data structures have validation rules
- [x] All APIs have error handling defined
- [x] All critical paths have test coverage
- [x] REQ-009 fully implemented (interactive mode)
- [x] REQ-025 max-token patcher system (通用正则跨版本，已验证 2.1.195；firstParty patch 已彻底删除)
- [x] REQ-026 anti-telemetry patcher system (字面量截断 event_logging 端点 -> 404，27 字节等长，跨版本稳定)
- [x] REQ-027 anti-spy patcher system (函数级字面量替换 KIt()→UTC + Hsp()→null，48/47 字节等长，跨版本稳定，不碰 $Sn())
- [x] REQ-028 anti-prompt-bias patcher system (字面量替换 if(g7())→if(0   )，63 字节等长，跳过 Provider context prompt，不碰其他 firstParty 门控)
- [x] Deprecated REQs (003, 010, 015) clearly marked with reasons
- [x] No TODO/FIXME/stub in production code
- [x] All test files mapped to corresponding REQs
