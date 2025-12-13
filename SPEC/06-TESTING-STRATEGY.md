# 测试策略文档

**版本**: v6.0.0
**最后更新**: 2025-11-26
**维护者**: Claude Code

---

## 1. 测试目标与原则

### 1.1 测试目标

本测试策略确保 Agentic-Warden 项目在所有模块上达到以下质量标准：

1. **功能完整性**: 所有REQ需求都有对应的验证测试
2. **代码可靠性**: 核心业务逻辑覆盖率 > 80%
3. **集成稳定性**: 跨模块交互测试覆盖关键路径
4. **用户体验**: TUI和CLI交互流程完整测试
5. **安全性**: OAuth认证和沙箱执行环境安全测试

### 1.2 测试原则

- **SPEC驱动**: 每个REQ-ID必须有至少一个测试用例
- **CI容器化**: 除单元测试外，所有测试必须在CI容器中执行
- **测试分层**: 按照单元 → 集成 → E2E 分层实施
- **真实服务**: 集成测试禁止使用Mock，必须连接真实服务
- **持续验证**: 每次提交都运行全套测试套件

### 1.3 测试铁律 🔒

以下规则**绝对禁止违反**：

1. **禁止 `#[ignore]` 标记**: 所有测试必须在 `cargo test` 时正常运行，禁止使用 `#[ignore]` 跳过测试
2. **禁止 Mock**: 集成/E2E 测试必须连接真实服务（MCP服务器、AI CLI、Ollama等）
3. **禁止过时测试**: 测试代码必须与当前 API 保持同步，发现过时测试立即删除或更新

---

## 2. 测试分层架构

### 2.1 单元测试 (Unit Tests)

**适用范围**: 单个函数、类、模块的独立逻辑测试

**执行环境**: 开发主机直接运行（可使用Mock）

**测试文件位置**: `/tests/unit/`

**测试命名规范**: `TEST-UNIT-{模块}-{序号}`

**性能要求**: 单个测试执行时间 < 100ms

**当前测试文件** (7个文件, 约1500行):
```
tests/unit/
├── capability_detection.rs    # 能力检测测试
├── cli_parser.rs             # CLI解析测试
└── roles_tests.rs            # 角色管理测试
```

**覆盖率目标**:
- 核心业务逻辑: > 80%
- 错误处理路径: 100%
- 边界条件: > 90%

### 2.2 集成测试 (Integration Tests)

**适用范围**: 模块间交互、API接口、数据流转

**执行环境**: CI容器环境，连接**真实服务**（禁止使用Mock）

**测试文件位置**: `/tests/integration/`

**测试命名规范**: `TEST-INT-{模块}-{序号}`

**性能要求**: 单个测试执行时间 < 30秒

**当前测试文件** (9个文件, 约1800行):
```
tests/integration/
├── cli_end_to_end.rs          # CLI端到端测试
├── config_integration.rs      # 配置集成测试
├── dynamic_tool_registry.rs   # 动态工具注册表测试
├── js_orchestrator_tests.rs   # JS编排测试
├── mcp_task_launching.rs      # MCP任务启动测试
├── pwait_command.rs          # PWait命令测试
├── pwait_pid_parameter.rs    # PWait PID参数测试
├── shared_memory_isolation.rs # 共享内存隔离测试
└── tui_navigation.rs         # TUI导航测试
```

**外部依赖要求**:
- MCP服务器 (可通过docker-compose启动)
- Ollama实例 (用于LLM路由决策)

**禁止事项**:
- ❌ 禁止使用单元测试Mock（mockall, mockito等）
- ❌ 禁止在主机直接执行集成测试
- ✅ 必须在CI容器中通过 `docker-compose.ci.yml` 运行

### 2.3 E2E测试 (End-to-End Tests)

**适用范围**: 完整用户场景、跨项目核心业务流程

**执行环境**: 完整CI环境（所有服务的前端+后端+基础设施）

**测试文件位置**: `/tests/e2e/{项目}/`

**测试命名规范**: `TEST-E2E-{项目}-{序号}`

**性能要求**: 单个测试执行时间 < 5分钟

**当前测试文件** (9个文件, 约1300行):
```
tests/e2e/agentic-warden/
├── ai_cli_update_e2e_tests.rs           # AI CLI更新E2E
├── e2e_cli_workflow.rs                  # CLI工作流E2E
├── interactive_mode_e2e_tests.rs        # 交互模式E2E
├── mcp_intelligent_route_claude_code_e2e.rs # MCP智能路由E2E
├── mcp_js_tool_e2e_tests.rs             # MCP JS工具E2E
├── mcp_routing_workflow.rs              # MCP路由工作流E2E
├── process_tree_e2e_tests.rs            # 进程树E2E
├── provider_injection_e2e_tests.rs      # Provider注入E2E
├── scenario_comprehensive.rs            # 综合场景E2E
└── task_lifecycle_tests.rs              # 任务生命周期E2E
```

**测试环境要求**:
- 所有服务通过 `docker-compose.ci.yml` 启动
- 真实Google Drive API (测试账号)
- 真实Ollama服务
- 真实的AI CLI工具 (claude, codex, gemini)

**执行命令**:
```bash
# 启动CI环境
docker-compose -f docker-compose.ci.yml up -d

# 运行E2E测试
docker-compose -f docker-compose.ci.yml run --rm e2e-tester
```

---

## 3. 测试覆盖需求矩阵

### 3.1 需求-测试映射表

| REQ-ID | 功能描述 | 优先级 | 测试类型 | 测试文件 | 覆盖率 |
|--------|---------|--------|---------|----------|--------|
| REQ-001 | AI CLI进程树追踪 | P0 | E2E | process_tree_e2e_tests.rs | ⚠️ 待验证 |
| REQ-002 | 第三方Provider管理 | P0 | 集成/单元 | provider_config.rs, provider_injection_e2e_tests.rs | ⚠️ 待验证 |
| REQ-003 | Google Drive同步 | P1 | 集成/E2E | sync_command.rs (需要恢复) | ⚠️ **未覆盖** |
| REQ-005 | Wait模式跨进程等待 | P2 | E2E | pwait_command.rs, pwait_pid_parameter.rs | ⚠️ 待验证 |
| REQ-006 | AI CLI工具检测与状态管理 | P1 | 集成 | capability_detection.rs | ⚠️ 待验证 |
| REQ-007 | MCP服务器 | P1 | 集成 | mcp_task_launching.rs | ⚠️ 待验证 |
| REQ-011 | AI CLI更新/安装管理 | P1 | E2E | ai_cli_update_e2e_tests.rs | ✅ 已覆盖 |
| REQ-012 | 智能MCP路由系统 | P0 | 集成/E2E | mcp_intelligent_route_claude_code_e2e.rs | ⚠️ 待验证 |
| REQ-013 | 动态JS编排工具系统 (Phase 1: 能力描述) | P0 | E2E | real_req013_phase1_capability_e2e.rs | ✅ 已覆盖 |
| REQ-013 | 动态JS编排工具系统 (Phase 2: 动态工具注册和调用) | P0 | E2E | real_req013_phase2_dynamic_tool_e2e.rs | ✅ 已覆盖 |
| REQ-014 | AI CLI角色系统 | P1 | 单元 | roles_tests.rs | ⚠️ 待验证 |
| REQ-016 | MCP Registry CLI多源聚合 | P1 | 单元/E2E | mcp_registry.rs, real_mcp_registry_cli_e2e.rs | ✅ 已覆盖 |

**覆盖率状态说明**:
- ✅ 已覆盖: 测试文件存在且通过了验证
- ⚠️ 待验证: 测试文件存在但需要检查覆盖完整性
- ❌ 未覆盖: 测试文件缺失
- 🔧 需要修复: 功能变更后测试需要更新

### 3.2 模块1: AI CLI管理系统测试覆盖

```
功能组件              | REQ-ID | 测试类型 | 测试文件               | 状态
---------------------|--------|----------|------------------------|--------
进程树追踪           | 001    | E2E      | process_tree_e2e_tests | ⚠️
Provider管理         | 002    | 集成     | provider_config        | ⚠️
Wait模式             | 005    | E2E      | pwait_command          | ⚠️
工具检测             | 006    | 单元     | capability_detection   | ⚠️
更新管理             | 011    | E2E      | ai_cli_update_e2e      | ✅
角色系统             | 014    | 单元     | roles_tests            | ⚠️
```

**待恢复/补充测试**:
- [ ] `tests/integration/sync_integration.rs` - Google Drive同步集成测试
- [ ] `tests/e2e/agentic-warden/google_drive_sync_e2e.rs` - Google Drive同步E2E测试

### 3.3 模块2: Google Drive同步系统测试覆盖

**注意**: Google Drive同步功能在v6.0.0中恢复，相关测试需要重建。

| 功能组件 | 测试类型 | 建议测试文件 | 当前状态 |
|---------|---------|-------------|---------|
| OAuth认证流程 | 集成 | `tests/integration/oauth_flow.rs` | ❌ **缺失** |
| Push备份 | E2E | `tests/e2e/agentic-warden/sync_push_e2e.rs` | ❌ **缺失** |
| Pull恢复 | E2E | `tests/e2e/agentic-warden/sync_pull_e2e.rs` | ❌ **缺失** |
| List备份 | 集成 | `tests/integration/sync_list.rs` | ❌ **缺失** |
| 错误处理 | 单元 | `tests/unit/sync_errors.rs` | ❌ **缺失** |

**紧急**: Google Drive同步功能恢复后，测试尚未重建，需要在下一个版本(v6.1.0)中补充。

### 3.4 模块3: MCP代理路由系统测试覆盖

```
功能组件              | REQ-ID | 测试类型 | 测试文件                          | 状态
---------------------|--------|----------|-----------------------------------|--------
MCP服务器核心        | 007    | 集成     | mcp_task_launching                | ⚠️
智能路由             | 012    | E2E      | mcp_intelligent_route              | ⚠️
能力描述生成         | 013-P1 | E2E      | real_req013_phase1_capability_e2e  | ✅
动态工具注册         | 013-P1 | 单元     | capability_description_e2e_test    | ✅
动态工具完整调用链   | 013-P2 | E2E      | real_req013_phase2_dynamic_tool_e2e| ✅
JS编排引擎           | 013-P2 | 集成     | js_orchestrator_tests              | ⚠️
JS工具执行           | 013-P2 | E2E      | mcp_js_tool_e2e_tests              | ⚠️
```

**注意**:
- ✅ REQ-013 Phase 1（能力描述生成）已完成真实环境E2E测试覆盖
- ✅ REQ-013 Phase 2（动态工具注册和调用）已完成完整E2E测试覆盖
  - 测试覆盖：基础动态工具流程、JS编排工具、LRU缓存驱逐、工具复用、Query vs Dynamic模式对比
- REQ-012的智能路由功能已移除会话历史依赖，相关测试需要验证是否仍然能通过。

### 3.5 已删除模块测试

**CC会话管理系统** (REQ-010) - 保持删除状态，相关测试已删除：

- ✅ `tests/e2e/agentic-warden/conversation_history_e2e_tests.rs` - 已删除
- ✅ `tests/integration/memory_integration.rs` - 已删除
- ✅ `tests/unit/hook_parser.rs` - 已删除

**验证**: 确认没有残留的REQ-010相关测试代码。

---

## 4. 测试执行策略

### 4.1 本地开发测试

```bash
# 单元测试 (可在主机运行)
cargo test --lib --tests unit

# 快速冒烟测试
cargo test --test unit -- --nocapture

# 特定模块测试
cargo test --test capability_detection
```

### 4.2 CI/CD集成测试

GitHub Actions工作流 (`.github/workflows/ci.yml`):

```yaml
# 单元测试作业 (主机执行, ~2分钟)
unit-tests:
  - cargo test --lib
  - cargo test --tests unit
  - cargo fmt --check
  - cargo clippy -- -D warnings

# 集成测试作业 (容器内执行, ~5-10分钟)
integration-tests:
  - docker-compose -f docker-compose.ci.yml up -d
  - docker-compose -f docker-compose.ci.yml run --rm integration-tester
  - 上传测试报告到artifacts

# E2E测试作业 (容器内执行, ~15-20分钟)
e2e-tests:
  - 依赖: integration-tests (确保集成测试通过)
  - docker-compose -f docker-compose.ci.yml up -d
  - docker-compose -f docker-compose.ci.yml run --rm e2e-tester
  - 上传E2E测试日志
```

### 4.3 手动测试清单

在发布前需要执行的手动测试:

- [ ] **CLI启动测试**
  - `aiw --version` 显示正确版本
  - `aiw --help` 显示完整帮助信息
  - `aiw codex "hello"` 成功启动Codex
  - `aiw claude "hello"` 成功启动Claude

- [ ] **Provider管理测试**
  - `aiw provider` 启动TUI界面
  - 可以添加新的provider配置
  - 可以切换默认provider

- [ ] **Google Drive同步测试** (需要OAuth)
  - `aiw push` 首次运行时提示认证
  - 认证后配置成功备份到Google Drive
  - `aiw list` 显示远程备份列表
  - `aiw pull` 成功恢复配置

- [ ] **MCP服务器管理测试**
  - `aiw mcp list` 显示已配置的MCP服务器
  - `aiw mcp add test "echo test"` 添加测试服务器
  - `aiw mcp enable test` 启用测试服务器

- [ ] **角色系统测试**
  - `aiw roles list` 显示可用角色
  - 角色文件正确加载到任务执行中

---

## 5. 测试质量指标

### 5.1 覆盖率目标

| 模块 | 目标行覆盖率 | 目标分支覆盖率 | 当前状态 |
|-----|-------------|---------------|---------|
| cli_type | 90% | 85% | ⚠️ 待验证 |
| provider | 85% | 80% | ⚠️ 待验证 |
| storage | 90% | 85% | ⚠️ 待验证 |
| mcp_routing | 80% | 75% | ⚠️ 待验证 |
| sync | 85% | 80% | ❌ **待重建** |

### 5.2 测试维护要求

**每周检查项**:
- [ ] 所有测试是否通过
- [ ] 测试覆盖率是否有下降趋势
- [ ] 新增代码是否包含对应测试
- [ ] Flaky测试是否增加

**每月检查项**:
- [ ] 测试策略文档是否更新
- [ ] 测试需求映射表是否完整
- [ ] 废弃测试是否清理
- [ ] 测试执行时间是否符合要求

---

## 6. 测试恢复计划

### 6.1 紧急: Google Drive同步测试 (v6.1.0)

**优先级**: P0 - 阻塞发布

Google Drive同步功能在v6.0.0中恢复，但测试尚未重建。

**需要补充的测试**:

1. **单元测试** (est: 4小时)
   - `tests/unit/sync_auth.rs` - OAuth认证单元测试
   - `tests/unit/sync_errors.rs` - 错误处理测试

2. **集成测试** (est: 8小时)
   - `tests/integration/sync_push.rs` - Push功能集成测试
   - `tests/integration/sync_pull.rs` - Pull功能集成测试
   - `tests/integration/sync_list.rs` - List功能集成测试

3. **E2E测试** (est: 8小时)
   - `tests/e2e/agentic-warden/google_drive_sync_e2e.rs` - 完整同步流程E2E测试

**预计工作量**: 20小时

### 6.2 高优先级: MCP路由测试完善 (v6.1.0)

**优先级**: P1

REQ-012移除会话历史依赖后，需要验证测试完整性。

**需要验证的测试**:
- [ ] `mcp_intelligent_route_claude_code_e2e.rs` - 确保无会话历史依赖
- [ ] `mcp_routing_workflow.rs` - 验证路由逻辑完整性

**预计工作量**: 4小时

### 6.3 中优先级: AI CLI管理测试增强 (v6.2.0)

**优先级**: P2

目前AI CLI管理测试覆盖不完整，需要增强。

**需要补充的测试**:
- [ ] `tests/unit/cli_manager_test.rs` - CLI工具管理单元测试
- [ ] `tests/unit/provider_config_test.rs` - Provider配置单元测试
- [ ] `tests/integration/provider_switch.rs` - Provider切换集成测试

**预计工作量**: 12小时

---

## 7. 测试自动化

### 7.1 GitHub Actions工作流

**.github/workflows/ci.yml** 已配置:

- **unit-tests** job: 单元测试和代码质量检查
- **integration-tests** job: 集成测试 (CI容器)
- **e2e-tests** job: E2E测试 (依赖integration-tests)
- **coverage** job: 覆盖率报告生成

### 7.2 测试运行脚本

`scripts/test_runner.sh` 提供统一测试接口:

```bash
# 运行所有测试
./scripts/test_runner.sh all

# 运行单元测试
./scripts/test_runner.sh unit

# 运行快速测试套件
./scripts/test_runner.sh quick

# Docker模式运行集成测试
./scripts/test_runner.sh --docker integration
```

---

## 8. 附录: 测试需求追踪

### 8.1 需求测试覆盖矩阵 (最新)

**截止v6.0.0版本**

| REQ-ID | 功能描述 | 删除状态 | 测试状态 | 备注 |
|--------|---------|----------|----------|------|
| REQ-001 | AI CLI进程树追踪 | 保留 | ⚠️ 待验证 | |
| REQ-002 | 第三方Provider管理 | 保留 | ⚠️ 待验证 | |
| REQ-003 | Google Drive同步 | **已恢复** | ❌ **缺失** | 需要重建测试 |
| REQ-005 | Wait模式跨进程等待 | 保留 | ⚠️ 待验证 | |
| REQ-006 | AI CLI工具检测与状态管理 | 保留 | ⚠️ 待验证 | |
| REQ-007 | MCP服务器 | 保留 | ⚠️ 待验证 | |
| REQ-008 | 指定供应商模式AI CLI启动 | 保留 | ✅ 已覆盖 | 通过集成测试 |
| REQ-009 | 交互式AI CLI启动 | 保留 | ⚠️ 待验证 | |
| REQ-010 | Claude Code会话历史集成 | **已删除** | - | 保持删除 |
| REQ-011 | AI CLI更新/安装管理 | 保留 | ✅ 已覆盖 | E2E测试覆盖 |
| REQ-012 | 智能MCP路由系统 | 保留 | ⚠️ 待验证 | 移除会话历史依赖 |
| REQ-013 | 动态JS编排工具系统 (Phase 1) | 保留 | ✅ 已覆盖 | 真实环境E2E测试 |
| REQ-013 | 动态JS编排工具系统 (Phase 2) | 保留 | ⚠️ 待验证 | JS编排需验证 |
| REQ-014 | AI CLI角色系统 | 保留 | ⚠️ 待验证 | |
| REQ-016 | MCP Registry CLI多源聚合 | 保留 | ✅ 已覆盖 | 单元测试+E2E测试 |

**总体覆盖率**: ~75% (需要提升到80%+)
**缺失测试**: Google Drive同步相关测试 (P0优先级), REQ-013 Phase 2 JS编排测试
**最新更新**: REQ-016 MCP Registry CLI E2E测试已补充 (2025-12-09)
**注意事项**: CI容器化铁律已实施，所有非单元测试必须通过docker-compose.ci.yml执行

---

**文档维护历史**

| 日期 | 版本 | 作者 | 变更内容 |
|------|------|------|----------|
| 2025-12-09 | v6.0.3 | Claude | 补充 REQ-016 MCP Registry CLI E2E测试 |
| 2025-11-27 | v6.0.2 | Claude | 补充 REQ-013 Phase 1 真实环境E2E测试 |
| 2025-11-26 | v6.0.0 | Claude | 初始版本 - 根据v6.0.0代码库状态创建 |

