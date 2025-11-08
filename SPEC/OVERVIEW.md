# Agentic-Warden 项目概览

## 项目定位

Agentic-Warden 是一个 AI CLI 工具的统一管理和进程监控平台，专为多 AI 环境下的开发工作流而设计。

## 核心价值

### 🎯 统一管理入口
- **多 AI 支持**: 统一管理 codex、claude、gemini 等 AI CLI 工具
- **Provider 管理**: 支持第三方 API 提供商（OpenRouter、LiteLLM、Cloudflare AI Gateway）
- **配置统一**: 通过 `~/.agentic-warden/provider.json` 集中管理配置

### 🚀 智能进程树监控

#### 🎯 核心价值：AI CLI 进程追踪与归属识别
**智能根进程识别**:
- 向上遍历进程树，自动识别启动当前进程的最上层 AI CLI (codex/claude/gemini)
- 避免传统工具将所有进程都归因于 explorer.exe 的问题
- 提供真正的"我们是被哪个 ROOT AI CLI 启动的"认知

**进程隔离机制**:
- 基于AI CLI根进程进行任务分组和共享内存隔离
- 确保不同AI CLI启动的任务互不干扰
- 支持多AI并发工作流的清晰管理

**核心功能特性**:
- **实时监控**: 监控所有 AI CLI 进程状态和任务执行情况
- **智能根进程查找**: 向上追溯进程树，找到启动当前进程的AI CLI根进程
- **进程归属识别**: 明确标识每个任务属于哪个AI CLI会话
- **共享内存隔离**: 按AI CLI根进程进行计算和任务隔离

### 🔌 MCP (Model Context Protocol) 服务器
- **外部集成**: 通过 MCP 协议为外部 AI 助手提供 Agentic-Warden 功能访问
- **进程管理工具**: `monitor_processes`, `get_process_tree`, `terminate_process`
- **Provider 状态工具**: `get_provider_status` 获取配置信息
- **AI CLI 启动工具**: `start_ai_cli` 通过外部方式启动 AI CLI
- **标准传输**: 支持 stdio 传输协议，兼容 Claude Code 等 MCP 客户端

### 📁 Google Drive 集成
- **集成式授权**: Google Drive 授权仅作为 push/pull 命令的集成步骤
- **智能检测**: 自动检测环境（桌面/服务器/无头）并选择最佳授权方式
- **自动触发**: 授权仅在执行 push/pull 命令时自动检测并触发

### 🎨 统一 TUI 体验
- **Dashboard**: 主界面，显示 AI CLI 状态和任务概要
- **Provider 管理**: TUI 界面管理第三方 API 提供商
- **任务监控**: 实时显示任务状态，按父进程分组
- **进度显示**: Push/Pull 操作的实时进度 TUI

## 技术栈

- **语言**: Rust
- **TUI 框架**: ratatui (0.24+) + crossterm (0.27+)
- **MCP 协议**: Model Context Protocol v1.0，支持 stdio 传输
- **进程监控**: 跨平台进程树遍历和 AI CLI 进程识别
- **组件**: 使用 ratatui 组件库的现成组件
- **授权**: Google OAuth 2.0 OOB 流程
- **配置**: JSON 格式配置文件

## 目标用户

1. **多 AI 用户**: 同时使用多个 AI CLI 工具的开发者
2. **团队协作者**: 需要统一管理 AI 配置的团队
3. **DevOps 工程师**: 需要 AI 进程监控和管理的场景
4. **个人开发者**: 简化 AI 工具使用流程的开发者

## 核心设计原则

### 简洁性
- 命令行优先：AI CLI 启动命令保持命令行模式（直接输出结果）
- TUI 用于管理：Provider 管理、任务状态、进度显示等使用 TUI
- 集成式授权：push/pull 命令自动检测并触发 OOB 流程

### 一致性
- 统一 TUI 设计：所有交互使用 TUI，统一使用 ratatui 框架
- 环境变量注入：根据 -p 参数选择的供应商自动注入环境变量到 AI CLI 进程
- 配置格式统一：通过 `provider.json` 集中管理第三方 API 提供商

### 可靠性
- **智能进程树管理**: 核心差异化功能，自动识别AI CLI根进程
- **实时监控**: 任务新增和结束都要实时显示在 UI 中
- **进程归属追踪**: 精确识别每个任务属于哪个AI CLI会话
- **错误处理**: 优雅处理授权失败、网络错误等异常情况

## 项目边界

### ✅ 包含功能
- AI CLI 统一启动和管理
- 第三方 Provider 配置管理
- 进程树监控和任务状态显示
- MCP (Model Context Protocol) 服务器集成
- Google Drive 集成（push/pull）
- 统一 TUI 界面

### ❌ 不包含功能
- 独立的授权管理命令
- AI CLI 的具体功能实现
- 云端服务或数据存储
- 多用户权限管理
- 企业级 SSO 集成

## 版本信息

- **当前版本**: 0.1.0
- **Rust 版本要求**: 1.70+
- **平台支持**: Windows, Linux, macOS