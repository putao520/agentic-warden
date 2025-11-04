# Agentic-Warden 需求规格说明

## 核心设计原则

### 1. Google OOB授权设计
- **集成式授权**：Google Drive授权仅作为push/pull命令的集成步骤，不提供独立授权功能
- **自动触发**：授权仅在执行push/pull命令时自动检测并触发
- **智能检测**：自动检测环境（桌面/服务器/无头）并选择最佳授权方式
- **并发处理**：同时监听本地回调和处理用户手动输入
- **简洁响应**：授权回调返回简单的HTTP成功响应，Token持久化到 ~/.agentic-warden/auth.json

### 2. 配置管理
- **统一配置格式**：通过 `~/.agentic-warden/provider.json` 集中管理第三方 API 提供商
- **UI驱动管理**：所有Provider管理通过TUI界面完成
- **环境变量注入**：根据-p选择的供应商自动注入环境变量到AI CLI进程
- **多提供商支持**：支持 OpenRouter, LiteLLM, Cloudflare AI Gateway 等第三方服务

### 3. 进程树管理
- **核心功能**：进程树管理是核心功能，不存在开关
- **根进程优化**：在Windows下优化根父进程查找，避免都定位到explorer.exe
- **AI CLI识别**：精确识别NPM AI CLI类型，支持跨平台
- **共享内存隔离**：按启动CLI的父进程根进程计算隔离

### 4. 全局任务扫描
- **优化扫描范围**：实例ID范围1-100，减少扫描次数
- **过滤空实例**：只显示有任务的实例，空实例不显示
- **跨实例访问**：通过ConnectedRegistry安全访问其他实例

### 5. Status命令
- **仅显示实时任务状态**：不同内存共享区域的任务列表分组显示
- **父进程分组**：按父进程进程名和PID分组
- **实时刷新**：任务新增和结束都要UI显示
- **提示词缩写**：长提示词支持...缩写

## 明确禁止的功能

### ❌ 禁止的Provider命令
- `agentic-warden provider`
- `agentic-warden provider list`
- `agentic-warden provider add`
- `agentic-warden provider edit`
- `agentic-warden provider remove`
- `agentic-warden provider show`
- `agentic-warden provider test`
- `agentic-warden provider set-default`

### ❌ 禁止的OAuth命令
- `agentic-warden oauth`
- `agentic-warden oauth-status`
- `agentic-warden auth`
- `agentic-warden reauth`

### ❌ 禁止的功能
- 单独的认证状态查询命令
- 手动触发的认证命令
- 命令行OAuth流程
- 认证进度显示（除了简单的日志）
- 独立的授权流程

## CLI 命令格式

```bash
# AI CLI 启动
agentic-warden codex "prompt"
agentic-warden codex -p openrouter "prompt"
agentic-warden claude --provider litellm "prompt"
agentic-warden gemini -p cloudflare "prompt"

# 多AI启动
agentic-warden codex|claude "同时问两个 AI"
agentic-warden codex|claude|gemini "问三个 AI"
agentic-warden all "问所有已安装的 AI"

# TUI 命令
agentic-warden              # Dashboard
agentic-warden status       # 任务状态
agentic-warden push [dirs...]  # Push进度TUI
agentic-warden pull            # Pull进度TUI
```

**-p 参数规则**：
- 适用于所有 AI CLI 启动命令
- 使用 `Command::env()` 注入环境变量到子进程
- 不污染父进程环境

## provider.json 配置结构

```json
{
  "$schema": "https://agentic-warden.dev/schema/provider.json",
  "providers": {
    "openrouter": {
      "description": "OpenRouter 统一 LLM 网关",
      "compatible_with": ["codex", "claude"],
      "env": {
        "OPENAI_API_KEY": "sk-or-v1-xxx",
        "OPENAI_BASE_URL": "https://openrouter.ai/api/v1"
      }
    },
    "litellm": {
      "description": "LiteLLM 本地代理",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxx",
        "ANTHROPIC_BASE_URL": "http://localhost:4000"
      }
    },
    "official": {
      "description": "官方 API (默认)",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {}
    }
  },
  "default_provider": "official"
}
```

## 统一 TUI 设计

### 1. 设计原则
- **所有交互使用 TUI**：统一使用 ratatui + crossterm 框架，使用组件库组件
- **命令行优先**：AI CLI 启动命令保持命令行模式（直接输出结果）
- **TUI 用于管理和监控**：Provider 管理、任务状态、进度显示等使用 TUI
- **集成式授权**：push/pull 命令自动检测并触发 OOB 流程

### 2. TUI 命令清单

#### 2.1 Dashboard 主界面
```bash
agentic-warden  # 无参数时显示 Dashboard
```

**显示内容**：
- 🤖 AI CLI 状态：安装情况、版本、默认 Provider
- 📊 任务概要：当前运行的任务（简化版，最多显示 5 个）

**交互**：
- `P` - 进入 Provider 管理
- `S` - 进入任务状态详情
- `Q` - 退出

#### 2.2 Provider 管理 TUI
**显示内容**：
- 所有 Provider 列表
- 每个 Provider 的描述、兼容性

**交互**：
- `↑↓` - 选择 Provider
- `A` - 添加新 Provider
- `E` - 编辑选中的 Provider
- `D` - 删除选中的 Provider
- `Enter` - 设置为默认 Provider
- `ESC` - 返回 Dashboard

**编辑界面**：
根据Provider的compatible_with字段显示对应的AI类型，并显示对应的环境变量字段供编辑。

#### 2.3 任务状态 TUI
```bash
agentic-warden status  # 进入任务状态 TUI
```

**显示内容**：
- 所有运行中的任务（详细版）
- 按父进程分组
- 实时刷新（每 2 秒）

**交互**：
- `↑↓` - 选择任务
- `K` - 终止选中的任务
- `R` - 手动刷新
- `ESC/Q` - 返回 Dashboard

#### 2.4 Push/Pull 进度 TUI
```bash
agentic-warden push [dirs...]  # 自动显示进度 TUI
agentic-warden pull            # 自动显示进度 TUI
```

**集成授权流程**：
1. 执行push/pull命令
2. 检查 Google Drive 授权状态
3. 如果未授权，显示授权对话框
4. 用户同意后，启动 OOB 流程（TUI 显示）
5. 授权成功后，自动继续执行 push/pull
6. 显示进度 TUI

### 3. TUI 实现要求

#### 3.1 技术栈
- **框架**: ratatui (0.24+)
- **事件处理**: crossterm (0.27+)
- **组件**: 使用ratatui组件库的现成组件，禁止自建组件

#### 3.2 文件结构
```
src/
├── tui/
│   ├── mod.rs                  # TUI 框架入口
│   ├── app.rs                  # 主应用状态管理
│   ├── screens/
│   │   ├── mod.rs
│   │   ├── dashboard.rs        # Dashboard 主界面
│   │   ├── provider.rs         # Provider 列表
│   │   ├── provider_edit.rs    # Provider 编辑
│   │   ├── status.rs           # 任务状态
│   │   ├── push.rs             # Push 进度
│   │   ├── pull.rs             # Pull 进度
│   └── event.rs                # 事件处理
```

#### 3.3 核心功能
- **屏幕导航**: 使用 `ESC` 返回上一级，`Q` 退出
- **键盘交互**: 上下键选择，Enter 确认，Space 切换
- **自动刷新**: Dashboard 和 Status 每 2 秒自动刷新
- **进度显示**: Push/Pull 实时显示压缩、上传、下载进度
- **集成式授权**: push/pull命令自动触发OOB流程

## 待实现需求

1. **统一 TUI 系统**
   - 实现 Dashboard 主界面
   - 实现 Provider 管理 TUI（列表 + 编辑）
   - 实现 Status TUI（实时任务监控）
   - 实现 Push/Pull 进度 TUI（集成授权）
   - 实现集成式 OOB 授权流程

2. **Provider 功能完善**
   - 支持 `-p` 参数用于所有 AI CLI 命令
   - 支持多 AI 启动语法（`codex|claude|gemini`）
   - 支持 `all` 启动所有 AI

3. **删除所有未经授权的命令**
4. **保持授权完全集成化**
5. **确保status命令只显示任务状态**