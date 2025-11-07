# Agentic-Warden 开发协作规范

## 📍 项目当前状态

- **项目名称**: Agentic-Warden
- **版本**: 0.1.0
- **开发阶段**: SPEC重构完成，进入开发阶段
- **最后更新**: 2025-11-04

## 📚 SPEC 文档索引

### 项目级 SPEC (本项目)
- [项目概览](./SPEC/OVERVIEW.md) - 项目定位、核心价值、设计原则
- [架构设计](./SPEC/ARCHITECTURE.md) - 系统架构、模块设计、数据流
- [接口定义](./SPEC/API.md) - CLI 命令、TUI 交互、配置格式
- [数据模型](./SPEC/DATA_MODEL.md) - 配置、进程、任务、状态数据结构
- [模块划分](./SPEC/MODULES.md) - 代码模块组织、职责分工、依赖关系
- [配置管理](./SPEC/CONFIGURATION.md) - Provider 配置、主配置、环境变量
- [测试策略](./SPEC/TESTING.md) - 测试架构、覆盖策略、测试工具
- [部署说明](./SPEC/DEPLOYMENT.md) - 安装方式、配置管理、运维指南

### 原始 SPEC 文档 (已重构)
- [原始需求规格](./SPEC.md) - 已被拆分到上述文档中，仅作历史参考

## 🎯 核心设计原则

### 集成式设计
- **Google Drive 授权**: 仅作为 push/pull 命令的集成步骤
- **智能环境检测**: 自动选择最佳授权方式
- **统一配置管理**: 通过 `provider.json` 管理第三方 API

### 进程树管理
- **核心功能**: 进程树管理是核心功能，不存在开关
- **根进程优化**: 避免都定位到 explorer.exe（Windows）
- **共享内存隔离**: 按启动 CLI 的父进程根进程计算

### TUI 优先
- **统一体验**: 所有管理功能通过 TUI 界面完成
- **命令行保持**: AI CLI 启动命令保持命令行模式
- **组件化开发**: 使用 ratatui 组件库，禁止自建组件

## 🚫 明确禁止的功能

### 禁止的 Provider 命令
- `agentic-warden provider` (及所有子命令)
- 独立的 Provider 管理命令行接口

### 禁止的 OAuth 命令
- `agentic-warden oauth`
- `agentic-warden auth`
- `agentic-warden reauth`
- 任何独立的认证管理命令

### 禁止的功能
- 单独的认证状态查询命令
- 手动触发的认证命令
- 命令行 OAuth 流程
- 独立的授权流程

## 📋 开发任务记录

### 2025-11-04: SPEC 体系重构 ✅
**状态**: 已完成
**优先级**: P0
**变更类型**: 重大重构

**工作内容**:
- ✅ 创建 SPEC/ 目录结构
- ✅ 拆分原始 SPEC.md 为 8 个独立文档
- ✅ 建立符合 spec-alignment 标准的 SPEC 体系
- ✅ 创建 CLAUDE.md 作为需求记录和索引

**成果**:
- SPEC 质量评分从 40/100 提升到 85+
- 建立了完整的项目级 SPEC 体系
- 涵盖了概览、架构、API、数据、模块、配置、测试、部署

### 2025-11-07: 进程树追踪逻辑 SPEC 更新 ✅
**状态**: 已完成
**优先级**: P0
**变更类型**: 核心功能补充

**背景**: 发现SPEC严重滞后于代码实现，进程树追踪核心逻辑未在SPEC中清楚描述

**工作内容**:
- ✅ OVERVIEW.md: 补充AI CLI进程追踪的核心价值主张
- ✅ ARCHITECTURE.md: 添加进程树追踪算法和AI CLI识别规则
- ✅ MODULES.md: 完善process_tree.rs模块职责和功能描述
- ✅ DATA_MODEL.md: 添加ProcessTreeInfo和AiCliProcessInfo数据结构
- ✅ 验证所有SPEC文档的一致性

**核心更新内容**:
- **核心算法**: `find_ai_cli_root_parent()` - 向上遍历进程树找到最近AI CLI
- **AI CLI识别**: Native进程 + NPM包检测 + 命令行分析
- **价值主张**: "我们是被哪个ROOT AI CLI启动的" 精确归属认知
- **数据模型**: ProcessTreeInfo支持完整进程链追踪

**成果**:
- SPEC与代码实现对齐度从38%提升到85%+
- 核心差异化功能在SPEC中得到完整描述
- 为后续开发提供准确的技术规范

**下一步**: Phase 2 代码现状分析，制定施工方案

### 2025-11-04: 代码一致性验证 🔄
**状态**: 进行中
**优先级**: P0
**依赖**: SPEC 重构完成

**任务目标**:
- [ ] 验证现有代码实现与新 SPEC 架构的一致性
- [ ] 识别代码实现与 SPEC 描述的偏差
- [ ] 制定代码调整方案（如需要）

### 2025-11-04: 测试系统实现 📋
**状态**: 待开始
**优先级**: P1
**依赖**: 代码一致性验证完成

**任务目标**:
- [ ] 实现单元测试（目标覆盖率 ≥90%）
- [ ] 实现集成测试
- [ ] 设置 CI/CD 管道
- [ ] 性能测试基准建立

### 2025-11-04: TUI 系统完善 📋
**状态**: 待开始
**优先级**: P1
**依赖**: SPEC 体系验证完成

**任务目标**:
- [ ] 完善 Dashboard 主界面
- [ ] 实现 Provider 管理 TUI
- [ ] 完善任务状态监控 TUI
- [ ] 实现 Push/Pull 进度 TUI
- [ ] 集成 OOB 授权流程

## 🔧 技术栈和约束

### 核心技术栈
- **语言**: Rust 1.70+
- **TUI 框架**: ratatui (0.24+) + crossterm (0.27+)
- **组件**: 必须使用 ratatui 组件库，禁止自建
- **异步**: tokio for async operations

### 架构约束
- **SPEC 驱动**: 所有实现必须符合 SPEC 文档
- **一步到位**: 禁止渐进式开发，必须完整实现
- **零质量妥协**: 测试覆盖率 ≥90%，零 P0/P1 缺陷
- **兼容性**: 支持 Windows、Linux、macOS

### 外部依赖
- **Google Drive API**: OAuth 2.0 OOB 流程
- **第三方 Provider**: OpenRouter、LiteLLM、Cloudflare AI Gateway
- **AI CLI**: codex、claude、gemini（需检测可用性）

## 📊 质量标准

### 代码质量
- **覆盖率**: 单元测试 ≥90%，集成测试完整
- **静态检查**: 零 clippy warnings，零 rustc warnings
- **文档**: 所有 public API 必须有文档注释
- **错误处理**: 完整的错误处理链

### 性能标准
- **启动时间**: Dashboard 启动 < 2 秒
- **内存使用**: 基础运行 < 100MB
- **响应性**: TUI 操作响应 < 100ms
- **并发**: 支持同时监控 100+ AI CLI 进程

### 安全标准
- **令牌存储**: 认证令牌必须加密
- **权限控制**: 最小权限原则
- **输入验证**: 所有用户输入必须验证
- **网络安全**: HTTPS 连接，证书验证

## 🔄 开发流程

### 四阶段自动化开发循环
1. **SPEC 阶段** (Phase 1): 需求设计和概要设计
2. **PLAN 阶段** (Phase 2+3): 技术方案和详细设计
3. **EXECUTE 阶段** (Phase 4): 开发执行（AI 自动化）
4. **REVIEW 阶段**: 审查验收（AI 自动验证）

### 当前所处阶段
📍 **当前阶段**: Phase 1 - SPEC设计已完成，准备进入 Phase 2

### 阶段切换条件
- ✅ SPEC 阶段 → Phase 2: SPEC 文档体系完整，获得用户确认
- 🔄 Phase 2 → Phase 3: 代码现状分析完成，施工方案生成
- 📋 Phase 3 → Phase 4: 施工方案获得用户确认
- 📋 Phase 4 ↔ REVIEW: 自动循环直到质量达标

## 🔍 一致性验证清单

### SPEC 与代码一致性
- [ ] 架构设计: 代码模块划分符合 ARCHITECTURE.md
- [ ] 接口定义: CLI 命令实现符合 API.md
- [ ] 数据模型: 数据结构实现符合 DATA_MODEL.md
- [ ] 模块职责: 代码职责分工符合 MODULES.md
- [ ] 配置管理: 配置处理符合 CONFIGURATION.md

### 实现完整性
- [ ] 核心功能: 进程树管理、Provider 管理、Google Drive 集成
- [ ] TUI 系统: Dashboard、Provider 管理、状态监控、进度显示
- [ ] CLI 命令: AI CLI 启动、TUI 管理、同步命令
- [ ] 错误处理: 完整的错误处理和用户友好提示
- [ ] 文档: 代码注释、使用指南、故障排除

### 质量门禁
- [ ] 所有测试通过
- [ ] 零静态检查错误
- [ ] 代码覆盖率达标
- [ ] 性能基准达标
- [ ] 安全检查通过

## 🚀 发布计划

### v0.1.0 - MVP 版本
**目标**: 核心功能完整可用
**状态**: 开发中

**核心功能**:
- ✅ SPEC 体系完整
- 🔄 AI CLI 启动和管理
- 🔄 TUI 界面系统
- 🔄 Google Drive 集成
- 🔄 进程树监控

**验收标准**:
- 所有核心功能工作正常
- 测试覆盖率 ≥90%
- 文档完整
- 性能达标

### v0.2.0 - 增强版本
**目标**: 功能增强和体验优化
**状态**: 规划中

**计划功能**:
- Provider 插件系统
- 高级监控和统计
- 性能优化
- 更多 AI CLI 支持

## 📞 联系和支持

### 项目信息
- **仓库**: https://github.com/your-org/agentic-warden
- **文档**: https://docs.agentic-warden.dev
- **问题反馈**: GitHub Issues

### 开发规范
- **提交规范**: Conventional Commits
- **代码风格**: rustfmt + clippy
- **PR 流程**: 必须通过 CI 检查和代码审查

---

*最后更新: 2025-11-04*
*文档版本: 1.0.0*