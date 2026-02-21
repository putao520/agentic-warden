# AIW 2.0 升级计划

> 创建时间: 2026-02-14
> 目标: 架构重构 + 部分向后兼容

---

## 📊 当前问题分析

### 巨型文件 (需拆分)
- [ ] `supervisor.rs` (1227行) - 职责过多
- [ ] `mcp/mod.rs` (1030行) - 功能耦合
- [ ] `error.rs` (986行) - 错误类型分散
- [ ] `cli_manager.rs` (748行)
- [ ] `storage.rs` (726行)

### 模块耦合
- [ ] `mcp/` 与 `mcp_routing/` 职责重叠
- [ ] `registry.rs` 与 `unified_registry.rs` 重复

### 遗留代码
- [ ] `pwait_mode.rs` + `wait_mode.rs` 应合并
- [ ] `sync/` 模块功能不明确

### 编译警告 (优先修复)
- [ ] `codegen.rs:137` - unused variable `output_file`
- [ ] `config.rs:7` - unused import `CliType`
- [ ] `mcp_routing/mod.rs:360` - unused variable `embed`
- [ ] `mcp_routing/mod.rs:12` - unused import `RegistryConfig`
- [ ] `mcp_routing/mod.rs:500` - unused variable `db_path`
- [ ] `render_helpers.rs:384-416` - dead_code `ProgressState`
- [ ] `supervisor.rs:65` - dead_code variant `Capture`
- [ ] `app_state.rs:17` - unused import `Duration`
- [ ] `cli_order.rs:25` - dead_code field `original_order`
- [ ] `signal.rs:67-68` - function_casts_as_integer
- [ ] `config_packer.rs:153` - unused variable `regex_pattern`
- [ ] `config_packer.rs:447` - dead_code `pack_skills_directory`

---

## Phase 1: 基础设施重构 (v2.0-alpha)

### 1.1 统一错误处理 [P0]
- [ ] 设计 `AiwError` 枚举 (使用 thiserror)
- [ ] 合并 `RegistryError` → `AiwError::Registry`
- [ ] 合并 `ConfigError` → `AiwError::Config`
- [ ] 合并 `ProcessError` → `AiwError::Process`
- [ ] 迁移所有 `Result<T, XxxError>` 到 `Result<T, AiwError>`
- [ ] 删除旧错误类型文件

### 1.2 Config 模块重构 [P0]
- [ ] 设计单一 `Config` struct
- [ ] 实现配置热加载 (watch + reload)
- [ ] 支持 `config.toml` 可选格式
- [ ] 合并 `config.rs` + `provider/config.rs` + `mcp_routing/config.rs`

### 1.3 拆分 supervisor.rs [P1]
- [ ] 创建 `process/mod.rs` 模块
- [ ] 提取 `process/spawn.rs` - 进程启动逻辑
- [ ] 提取 `process/monitor.rs` - 进程监控
- [ ] 提取 `process/signal.rs` - 信号处理
- [ ] 提取 `task/mod.rs` - 任务管理
- [ ] 保留 `supervisor.rs` 作为编排层 (<200行)

### 1.4 拆分 storage.rs [P1]
- [ ] 创建 `storage/mod.rs` 模块
- [ ] 提取 `storage/backend.rs` - 存储后端抽象
- [ ] 提取 `storage/task.rs` - 任务存储
- [ ] 提取 `storage/session.rs` - 会话存储

### 1.5 合并 wait 模块 [P1]
- [ ] 合并 `pwait_mode.rs` + `wait_mode.rs` → `process/wait.rs`
- [ ] 删除原文件

---

## Phase 2: MCP 模块合并 (v2.0-beta)

### 2.1 创建 aiw-mcp crate [P0]
- [ ] 创建 `crates/aiw-mcp/Cargo.toml`
- [ ] 迁移 `mcp/` 模块
- [ ] 迁移 `mcp_routing/` 模块
- [ ] 合并重复代码

### 2.2 MCP Server 重构 [P0]
- [ ] 统一 `mcp/server.rs` 实现
- [ ] 提取 `mcp/transport.rs` - 传输层
- [ ] 提取 `mcp/protocol.rs` - 协议处理

### 2.3 Registry 统一 [P1]
- [ ] 合并 `registry.rs` + `unified_registry.rs`
- [ ] 设计 `trait Registry`
- [ ] 实现 `OfficialRegistry`, `SmitheryRegistry`

### 2.4 评估 js_orchestrator [P1]
- [ ] 分析使用场景
- [ ] 决定保留/简化/移除
- [ ] 如保留，提取为独立 crate

### 2.5 Marketplace 模块化 [P2]
- [ ] 创建 `marketplace/mod.rs`
- [ ] 提取 `marketplace/source.rs` - 源管理
- [ ] 提取 `marketplace/install.rs` - 安装逻辑
- [ ] 支持多源配置

---

## Phase 3: CLI 层重构 (v2.0-rc)

### 3.1 命令解析重构 [P0]
- [ ] 引入 `clap` derive 宏
- [ ] 定义 `enum Command` 顶层命令
- [ ] 定义子命令结构体
- [ ] 移除手动解析代码

### 3.2 Router trait 设计 [P1]
```rust
trait CliRouter {
    fn route(&self, args: &Args) -> Result<ExitCode>;
    fn name(&self) -> &str;
}
```
- [ ] 设计 trait 接口
- [ ] 实现 `ClaudeRouter`, `CodexRouter`, `GeminiRouter`
- [ ] 实现动态路由注册

### 3.3 Provider 插件化 [P1]
- [ ] 设计 `trait Provider`
- [ ] 提取 Provider 配置独立
- [ ] 支持运行时加载

### 3.4 Role 系统优化 [P2]
- [ ] 支持 Role 组合 (多个 role 合并)
- [ ] 支持 Role 继承 (base + override)
- [ ] 优化内置 Role 加载机制

---

## Phase 4: 清理与文档 (v2.0-release)

### 4.1 移除废弃代码 [P0]
- [ ] 运行 `cargo clippy -- -D dead_code`
- [ ] 删除所有未使用的函数/结构体
- [ ] 清理注释掉的代码

### 4.2 Workspace 结构 [P0]
- [ ] 创建 workspace Cargo.toml
- [ ] 创建 `crates/aiw-core/`
- [ ] 创建 `crates/aiw-cli/`
- [ ] 创建 `crates/aiw-mcp/`
- [ ] 创建 `crates/aiw-tui/`
- [ ] 迁移主入口到 `crates/aiw/`

### 4.3 API 文档 [P1]
- [ ] 所有 pub 项添加 rustdoc
- [ ] 添加模块级文档
- [ ] 添加示例代码

### 4.4 README 更新 [P1]
- [ ] 更新架构图
- [ ] 更新安装说明
- [ ] 更新用法示例

### 4.5 迁移指南 [P0]
- [ ] 创建 `MIGRATION-2.0.md`
- [ ] 记录废弃命令
- [ ] 记录配置变更
- [ ] 提供迁移脚本

---

## 🔄 兼容性清单

### ✅ 保留命令 (向后兼容)
- `aiw claude/codex/gemini ...` - AI CLI 路由
- `aiw auto ...` - 自动故障转移
- `aiw mcp serve/list/add/remove` - MCP 管理
- `aiw plugin install/remove/list` - 插件管理
- `aiw roles list` - Role 列表
- `aiw config` - 配置管理
- `aiw wait` - 等待任务

### ❌ 废弃命令 (2.0 移除)
- `aiw status --tui` → 合并到 `aiw dashboard`
- `aiw pwait` → 合并到 `aiw wait <pid>`
- `aiw sync` → 移除

### ⚠️ 配置变更
- `~/.aiw/config.json` - 保持兼容
- 新增 `~/.aiw/config.toml` 支持 (可选)

---

## 📅 里程碑

| 版本 | 状态 | 内容 |
|------|------|------|
| 2.0-alpha | ⏳ 待开始 | Phase 1 完成 |
| 2.0-beta | ⏳ 待开始 | Phase 2 完成 |
| 2.0-rc | ⏳ 待开始 | Phase 3 完成 |
| 2.0-release | ⏳ 待开始 | Phase 4 完成 |

---

## 📝 开发笔记

### 优先修复项
1. 先修复编译警告，保持代码健康
2. Phase 1.1 统一错误处理是后续重构的基础
3. Phase 2 MCP 合并前需要先完成 Phase 1

### 风险点
- MCP 模块合并可能影响现有插件
- Router trait 设计需要考虑扩展性
- Workspace 迁移需要一次性完成

---

*最后更新: 2026-02-14*
