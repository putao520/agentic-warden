# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2025-11-11

### 🚀 Major Changes
- **SPEC体系重构**: 完整重构项目设计文档体系，建立了符合spec-alignment标准的SPEC文档库
- **MCP功能简化**: 简化MCP服务器实现，移除复杂的rmcp依赖，专注于核心功能
- **测试系统完善**: 修复所有测试失败问题，建立完整的回归测试体系

### ✨ Added
- **完整SPEC体系**: 创建了包含9个核心文档的SPEC设计库
  - `SPEC/OVERVIEW.md` - 项目概览和核心价值
  - `SPEC/ARCHITECTURE.md` - 系统架构和技术决策
  - `SPEC/DATA_MODEL.md` - 数据模型和结构
  - `SPEC/API.md` - 接口定义和命令规范
  - `SPEC/MODULES.md` - 模块划分和职责
  - `SPEC/CONFIGURATION.md` - 配置管理
  - `SPEC/TESTING.md` - 测试策略
  - `SPEC/DEPLOYMENT.md` - 部署说明
  - `SPEC/MCP-INTEGRATION.md` - MCP集成设计

- **进程树追踪核心逻辑**: 完善了AI CLI进程树追踪的核心算法和实现
- **测试清理机制**: 实现了完整的测试隔离和清理机制

### 🔧 Fixed
- **编译错误修复**: 修复了Windows平台的ChildResources编译问题
- **测试失败修复**: 解决了所有回归测试失败问题
  - `test_complete_task_lifecycle` - 修复任务清理逻辑
  - `test_status_command_workflow` - 修复跨进程命名空间问题
  - `test_pwait_waits_for_multiple_real_processes` - 修复测试隔离问题
  - 平台特定错误处理兼容性
- **Doctest修复**: 修复了unified_registry的文档测试

- **内存泄漏修复**: 修复了`get_completed_unread_tasks()`方法不清理已完成任务的问题

### 🗑️ Removed
- **过时文档**: 清理了8个过时和重复的MD文件
  - `SPEC_ALIGNMENT_REVIEW_2025-11-10.md`
  - `SPEC_CODE_ALIGNMENT_REPORT.md`
  - `SPEC_IMPLEMENTATION_GAP_ANALYSIS.md`
  - `SPEC_ALIGNMENT_REVIEW_CORRECTED.md`
  - `TEST_COVERAGE_REPORT.md`
  - `MCP_QUALITY_UPGRADE_PLAN.md`
  - `TESTING.md`
  - `REGISTRY_FACTORY.md`

- **复杂依赖**: 移除了不必要的rmcp库依赖，简化MCP实现

### 📊 Quality
- **测试覆盖率**: 维持136个测试全部通过的状态
- **平台兼容性**: 确保Windows和Linux平台完全兼容
- **代码质量**: 零编译错误，仅保留无害的编译警告

### 🎯 Breaking Changes
- **MCP API变更**: 简化的MCP接口，移除了复杂的JSON-RPC实现
- **文档结构**: SPEC文档体系重构，提供更清晰的设计指导

## [0.4.5] - 2025-11-04

### 🔧 Fixed
- Windows平台进程管理兼容性改进
- 共享内存存储稳定性提升

## [0.4.4] - 2025-10-28

### ✨ Added
- Google Drive OAuth集成
- TUI界面基础框架
- Provider管理系统

## [0.4.0] - 2025-10-15

### 🚀 Major Changes
- **进程树管理**: 核心进程树追踪功能实现
- **任务注册系统**: 跨进程任务状态管理
- **多AI CLI支持**: 支持codex、claude、gemini等AI CLI

---

## Upgrade Guide

### From 0.4.x to 0.5.0

1. **Breaking**: MCP接口已简化，如果您在使用MCP功能，请更新相关代码
2. **Documentation**: 所有设计文档已迁移到SPEC/目录，请参考新的文档结构
3. **Testing**: 测试系统更加稳定，建议重新运行您的测试套件

### Compatibility

- **Rust**: Requires Rust 1.70+
- **Platforms**: Windows, Linux, macOS
- **Dependencies**: No major dependency changes in this release