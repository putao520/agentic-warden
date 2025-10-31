# Agentic-Warden 需求规格说明

## 核心设计原则

### 1. OAuth认证设计
- **UI流程驱动**：所有OAuth认证必须通过UI流程自动完成，不提供命令行接口
- **自动触发**：认证仅在需要时自动触发（如sync时发现未认证）
- **智能检测**：自动检测环境（桌面/服务器/无头）并选择最佳认证方式
- **并发处理**：同时监听本地回调和处理用户手动输入
- **简洁响应**：OAuth回调返回简单的HTTP成功响应，不需要复杂页面

### 2. 配置管理
- **统一配置格式**：使用sync.json格式，包含config和state两部分
- **自动认证集成**：ConfigSyncManager集成SmartOAuth，按需自动认证
- **不显示认证状态命令**：认证状态仅作为内部信息，不提供单独的查询命令

### 3. 进程树管理
- **核心功能**：进程树管理是核心功能，不存在开关，必须启用
- **根进程优化**：在Windows下优化根父进程查找，避免都定位到explorer.exe
- **AI CLI识别**：精确识别NPM AI CLI类型，支持跨平台
- **共享内存隔离**：按启动CLI的父进程根进程计算隔离

### 4. 全局任务扫描
- **优化扫描范围**：实例ID范围1-100，减少扫描次数
- **过滤空实例**：只显示有任务的实例，空实例不显示
- **跨实例访问**：通过ConnectedRegistry安全访问其他实例

## 实现的功能模块

### 1. SmartOAuth认证系统 ✅
- 并发监听本地回调端口(8080)和用户手动输入
- 智能环境检测(桌面/服务器/无头)
- 自动选择最佳认证方式
- 简单HTTP响应处理
- Token持久化到 ~/.agentic-warden/auth.json

### 2. ConfigSyncManager ✅
- 集成SmartOAuthAuthenticator
- 按需自动触发认证
- 不提供单独的认证状态查询

### 3. 进程树管理 ✅
- 移除PROCESS_TREE_FEATURE_ENV环境变量
- 核心功能必须启用
- 优化根父进程查找
- NPM AI CLI类型精确识别

### 4. 全局任务扫描 ✅
- 扫描范围1-100
- 过滤空实例
- 跨实例任务聚合

### 5. Status命令
- **仅显示实时任务状态**：不同内存共享区域的任务列表分组显示
- **父进程分组**：按父进程进程名和PID分组
- **实时刷新**：任务新增和结束都要UI显示
- **提示词缩写**：长提示词支持...缩写
- **不显示PUSH/PULL进度**：这些在执行对应任务时自己显示

## 明确禁止的功能

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

## 技术要求

### 1. 代码质量
- 使用 `#[deny(unused_imports)]` 等严格lint规则
- 异步函数使用 `?` 运算符
- 完整的错误处理和日志记录
- 单元测试和集成测试

### 2. 架构设计
- OAuth完全集成在需要认证的功能中
- 不暴露认证细节给最终用户
- 自动化的用户体验

## 第三方 API Provider 支持

### 1. 设计目标
- **统一配置管理**：通过 `~/.agentic-warden/provider.json` 集中管理第三方 API 提供商
- **环境变量注入**：为每个 AI CLI 自动注入对应的环境变量
- **多提供商支持**：支持 OpenRouter, LiteLLM, Cloudflare AI Gateway 等第三方服务
- **CLI 集成**：通过 `-p/--provider` 参数指定使用的提供商

### 2. CLI 命令格式
```bash
# 使用默认 provider (official)
agentic-warden codex "prompt"

# 使用指定 provider
agentic-warden codex -p openrouter "prompt"
agentic-warden claude --provider litellm "prompt"
agentic-warden gemini -p cloudflare "prompt"

# Provider 管理命令
agentic-warden provider list                    # 列出所有 provider
agentic-warden provider add <name>              # 交互式添加 provider
agentic-warden provider edit <name>             # 编辑 provider 配置
agentic-warden provider remove <name>           # 删除 provider
agentic-warden provider show <name>             # 显示配置(隐藏密钥)
agentic-warden provider test <name>             # 测试连接
agentic-warden provider set-default <name>      # 设置默认 provider
```

### 3. provider.json 配置结构
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
        "ANTHROPIC_BASE_URL": "http://localhost:4000",
        "OPENAI_API_KEY": "sk-xxx",
        "OPENAI_BASE_URL": "http://localhost:4000"
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

### 4. 环境变量映射规则

| AI CLI | 支持的环境变量 | 第三方 API 支持 |
|--------|---------------|----------------|
| **codex** | `OPENAI_API_KEY`, `OPENAI_BASE_URL` | ✅ 完全支持 |
| **claude** | `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL` | ✅ 完全支持 |
| **gemini** | `GOOGLE_API_KEY`, `https_proxy` | ⚠️ 通过代理支持 |

### 5. 实现要求

#### 5.1 文件结构
```
src/
├── provider/
│   ├── mod.rs              # 模块导出
│   ├── config.rs           # ProviderConfig 数据结构
│   ├── manager.rs          # ProviderManager (加载/保存/验证)
│   ├── env_injector.rs     # 环境变量注入器
│   └── commands.rs         # provider 子命令实现
```

#### 5.2 核心功能
- **ProviderManager**: 负责加载、保存、验证 provider.json
- **EnvInjector**: 在启动 AI CLI 前注入环境变量到子进程
- **兼容性验证**: 检查 provider 是否支持指定的 AI 类型
- **交互式配置**: 通过 dialoguer 提供友好的配置界面

#### 5.3 安全性
- provider.json 文件权限设为 `0600` (仅用户可读写)
- 显示配置时隐藏 API 密钥(仅显示前4位和后4位)
- 使用 `Command::env()` 注入环境变量到子进程,不污染父进程

#### 5.4 错误处理
- Provider 不存在时提示用户添加
- Provider 不兼容时列出可用的 provider
- 配置文件损坏时提供修复建议

### 6. 用户体验

#### 6.1 首次使用流程
```bash
$ agentic-warden codex -p openrouter "hello"
❌ Error: Provider 'openrouter' not found

💡 Tip: Add a new provider with:
   agentic-warden provider add openrouter
```

#### 6.2 添加 provider
```bash
$ agentic-warden provider add openrouter
📝 Adding new provider: openrouter

Description: OpenRouter unified gateway
Compatible with (comma-separated): codex,claude

Environment variables:
  OPENAI_API_KEY: sk-or-***
  OPENAI_BASE_URL: https://openrouter.ai/api/v1

✅ Provider 'openrouter' added successfully!
```

#### 6.3 使用 provider
```bash
$ agentic-warden codex -p litellm "优化代码"
🔌 Using provider: litellm
🚀 Launching codex...
[正常输出]
```

### 7. 默认配置
首次运行时自动在 `~/.agentic-warden/provider.json` 创建:
```json
{
  "providers": {
    "official": {
      "description": "Official API endpoints",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {}
    }
  },
  "default_provider": "official"
}
```

## 统一 TUI (Terminal User Interface) 设计

### 1. 设计原则
- **所有交互使用 TUI**：统一使用 ratatui + crossterm 框架，不使用零散的 dialoguer 交互
- **命令行优先**：AI CLI 启动命令保持命令行模式（直接输出结果）
- **TUI 用于管理和监控**：Provider 管理、任务状态、进度显示等使用 TUI
- **自动触发认证**：push/pull 等需要认证的功能自动检测并触发 OAuth 流程

### 2. TUI 命令清单

#### 2.1 Dashboard 主界面
```bash
agentic-warden  # 无参数时显示 Dashboard
```

**显示内容**：
- 🤖 AI CLI 状态：安装情况、版本、默认 Provider
- 📊 任务概要：当前运行的任务（简化版，最多显示 5 个）
- 🔐 授权状态：Google Drive 认证状态、Token 过期时间、Provider 数量

**交互**：
- `P` - 进入 Provider 管理
- `S` - 进入任务状态详情
- `I` - 安装/更新 AI CLI
- `Q` - 退出

**界面示例**：
```
┌────────────────────────────────────────────────────────────────┐
│ Agentic-Warden Dashboard                            v0.3.0     │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│ 🤖 AI CLI Status                                               │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ ✅ codex      v1.2.3    [Installed]    Provider: openrouter│ │
│ │ ✅ claude     v2.0.1    [Installed]    Provider: official  │ │
│ │ ❌ gemini     -         [Not installed]                    │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
│ 📊 Running Tasks                                    (2 active) │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ • codex (PID: 12345)  "Writing a function..."   [2m 30s]  │ │
│ │ • claude (PID: 12346) "Code review..."          [45s]     │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
│ 🔐 Authorization Status                                        │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ Google Drive:  ✅ Authenticated (expires in 25 days)       │ │
│ │ Last Sync:     2025-10-30 15:30:00                        │ │
│ │ Providers:     3 configured (1 default)                   │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
│ [P] Providers  [S] Status  [I] Install  [Q] Quit              │
└────────────────────────────────────────────────────────────────┘
```

#### 2.2 Provider 管理 TUI
```bash
agentic-warden provider  # 直接进入 Provider 管理 TUI（不需要子命令）
```

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

**界面示例**：
```
┌────────────────────────────────────────────────────────────────┐
│ Provider Management                                            │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  > official (default)     [Official API]                       │
│    openrouter            [OpenRouter Gateway]                  │
│    litellm               [LiteLLM Proxy]                       │
│                                                                │
│                                                                │
│ [A] Add  [D] Delete  [E] Edit  [Enter] Set Default  [ESC] Back│
└────────────────────────────────────────────────────────────────┘
```

**编辑界面**：
```
┌────────────────────────────────────────────────────────────────┐
│ Edit Provider: openrouter                                      │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Description:  OpenRouter Gateway                              │
│                                                                │
│  Compatible with:  [x] codex  [x] claude  [ ] gemini          │
│                                                                │
│  Environment Variables for codex:                              │
│    > OPENAI_API_KEY     = sk-or-***                           │
│      OPENAI_BASE_URL    = https://openrouter.ai/api/v1        │
│                                                                │
│  Environment Variables for claude:                             │
│      ANTHROPIC_API_KEY  = (not set)                            │
│      ANTHROPIC_BASE_URL = (not set)                            │
│                                                                │
│ [↑↓] Navigate  [Enter] Edit  [Space] Toggle  [Ctrl+S] Save    │
└────────────────────────────────────────────────────────────────┘
```

**环境变量映射规则**：
- **codex** → `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_ORG_ID`
- **claude** → `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`
- **gemini** → `GOOGLE_API_KEY`, `https_proxy`

根据用户勾选的兼容 AI 类型，动态显示对应的环境变量分组。

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

**界面示例**：
```
┌────────────────────────────────────────────────────────────────┐
│ Running Tasks                          [Auto-refresh: 2s]      │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Parent: codex (PID: 12345)  [openrouter]                     │
│    └─ Task: Writing a function...                   [Running] │
│       Started: 2 min ago                                       │
│                                                                │
│  Parent: claude (PID: 12346)  [official]                      │
│    └─ Task: Code review...                          [Running] │
│       Started: 30 sec ago                                      │
│                                                                │
│                                                                │
│ [R] Refresh  [K] Kill Task  [Q] Quit                           │
└────────────────────────────────────────────────────────────────┘
```

#### 2.4 Push/Pull 进度 TUI
```bash
agentic-warden push [dirs...]  # 自动显示进度 TUI
agentic-warden pull            # 自动显示进度 TUI
```

**自动认证流程**：
1. 检查 Google Drive 认证状态
2. 如果未认证，显示认证对话框
3. 用户同意后，启动 OAuth 流程（TUI 显示）
4. 认证成功后，自动继续执行 push/pull
5. 显示进度 TUI

**未认证对话框**：
```
┌────────────────────────────────────────────────────────────────┐
│ Google Drive Authentication Required                           │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ⚠️  You need to authenticate with Google Drive to use        │
│      push/pull features.                                       │
│                                                                │
│  This will:                                                    │
│    • Store credentials in ~/.agentic-warden/auth.json         │
│    • Allow backup/sync of your configuration files            │
│    • Open your browser for Google authentication              │
│                                                                │
│                                                                │
│  Authenticate now?                                             │
│                                                                │
│  [Y] Yes, authenticate  [N] No, cancel                         │
└────────────────────────────────────────────────────────────────┘
```

**OAuth 进行中**：
```
┌────────────────────────────────────────────────────────────────┐
│ Google Drive Authentication                                    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  🌐 Browser should have opened automatically                   │
│                                                                │
│  Authorization URL:                                            │
│  https://accounts.google.com/o/oauth2/auth?...                │
│                                                                │
│  📝 Waiting for authorization...                               │
│                                                                │
│  • You can click the URL above if browser didn't open         │
│  • Or manually enter the authorization code below             │
│                                                                │
│  Authorization Code (optional): ___________________________    │
│                                                                │
│  [ESC] Cancel                                                  │
└────────────────────────────────────────────────────────────────┘
```

**Push 进度**：
```
┌────────────────────────────────────────────────────────────────┐
│ Pushing to Google Drive                                        │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Step 1/3: Compressing files                                  │
│  [████████████████████████] 100%                              │
│  ✅ Created archive: config-backup-20251030.tar.gz (2.3 MB)   │
│                                                                │
│  Step 2/3: Uploading to Google Drive                          │
│  [████████░░░░░░░░░░░░░░░░] 45%                               │
│  Uploaded: 1.0 MB / 2.3 MB                                    │
│  Speed: 500 KB/s   ETA: 3s                                    │
│                                                                │
│  Step 3/3: Verifying...                                       │
│  [░░░░░░░░░░░░░░░░░░░░░░░░] 0%                                │
│                                                                │
│ [ESC] Cancel                                                   │
└────────────────────────────────────────────────────────────────┘
```

### 3. 非 TUI 命令（保持命令行模式）

以下命令直接输出结果到终端，不进入 TUI：

```bash
# AI CLI 启动（单个）
agentic-warden codex "写一个函数"
agentic-warden codex -p openrouter "写一个函数"
agentic-warden claude --provider litellm "优化代码"
agentic-warden gemini "分析数据"

# AI CLI 启动（多个，使用 | 连接）
agentic-warden codex|claude "同时问两个 AI"
agentic-warden codex|claude -p openrouter "使用同一 provider"
agentic-warden codex|claude|gemini "问三个 AI"

# AI CLI 启动（全部）
agentic-warden all "问所有已安装的 AI"
agentic-warden all -p litellm "全部使用 litellm"
```

**-p 参数规则**：
- 适用于所有 AI CLI 启动命令
- 使用 `Command::env()` 注入环境变量到子进程
- 不污染父进程环境

### 4. TUI 实现要求

#### 4.1 技术栈
- **框架**: ratatui (0.24+)
- **事件处理**: crossterm (0.27+)
- **已有依赖**: 项目已包含这些依赖，无需添加

#### 4.2 文件结构
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
│   │   └── oauth.rs            # OAuth 认证界面
│   ├── widgets/
│   │   ├── mod.rs
│   │   ├── input.rs            # 输入框组件
│   │   ├── progress.rs         # 进度条组件
│   │   ├── list.rs             # 列表选择组件
│   │   └── dialog.rs           # 对话框组件
│   └── event.rs                # 事件处理
```

#### 4.3 核心功能
- **屏幕导航**: 使用 `ESC` 返回上一级，`Q` 退出
- **键盘交互**: 上下键选择，Enter 确认，Space 切换
- **自动刷新**: Dashboard 和 Status 每 2 秒自动刷新
- **进度显示**: Push/Pull 实时显示压缩、上传、下载进度
- **OAuth 集成**: 整合现有的 SmartOAuth 流程到 TUI 界面

#### 4.4 环境变量映射
在 Provider 编辑界面，根据用户勾选的 AI 类型动态显示环境变量：

```rust
// src/provider/env_mapping.rs
pub fn get_env_vars_for_ai_type(ai_type: AiType) -> Vec<EnvVarMapping> {
    match ai_type {
        AiType::Codex => vec![
            EnvVarMapping { key: "OPENAI_API_KEY", required: true },
            EnvVarMapping { key: "OPENAI_BASE_URL", required: false },
            EnvVarMapping { key: "OPENAI_ORG_ID", required: false },
        ],
        AiType::Claude => vec![
            EnvVarMapping { key: "ANTHROPIC_API_KEY", required: true },
            EnvVarMapping { key: "ANTHROPIC_BASE_URL", required: false },
        ],
        AiType::Gemini => vec![
            EnvVarMapping { key: "GOOGLE_API_KEY", required: true },
            EnvVarMapping { key: "https_proxy", required: false },
        ],
    }
}
```

### 5. 自动认证流程

**触发时机**：
- 用户执行 `push` 或 `pull` 命令
- 检测到未认证或 token 过期

**流程**：
1. 显示认证对话框（TUI）
2. 用户选择 Yes → 启动 OAuth 流程
3. OAuth 界面（TUI）显示 URL 和等待状态
4. 整合 SmartOAuth 的并发回调/手动输入
5. 认证成功 → 显示成功消息（2 秒）
6. **自动继续执行原始命令**（push/pull）
7. 显示进度 TUI

**用户拒绝或失败**：
- 显示错误消息
- 退出程序

### 6. 迁移计划

**需要替换 dialoguer 的地方**：
1. `src/provider/commands.rs` - Provider 添加/编辑交互 → Provider TUI
2. `src/sync/config_sync_manager.rs` - Google Drive 授权确认 → 认证对话框 TUI
3. `src/sync/smart_oauth.rs` - 手动输入授权码 → OAuth TUI
4. `src/cli_manager.rs` - 选择 AI CLI → 移除（保持命令行）

## 待实现需求

1. **统一 TUI 系统**
   - 实现 Dashboard 主界面
   - 实现 Provider 管理 TUI（列表 + 编辑）
   - 实现 Status TUI（实时任务监控）
   - 实现 Push/Pull 进度 TUI
   - 实现 OAuth TUI（整合 SmartOAuth）
   - 实现自动认证流程
   - 创建可复用 TUI 组件（input, progress, list, dialog）

2. **Provider 功能完善**
   - 支持环境变量按 AI 类型分组显示
   - 支持 `-p` 参数用于所有 AI CLI 命令
   - 支持多 AI 启动语法（`codex|claude|gemini`）
   - 支持 `all` 启动所有 AI

3. **删除所有未经授权的OAuth命令**
4. **保持认证完全自动化和集成化**
5. **确保status命令只显示任务状态**