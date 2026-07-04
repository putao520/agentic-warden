# AIW - AI CLI 统一网关

[English](README.md)

<div align="center">

![Version](https://img.shields.io/badge/version-0.6.0-blue?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

**AI CLI 工具的统一路由器与代理**

</div>

## 什么是 AIW？

AIW 是一个**统一网关**，作为 AI CLI 代理路由器，支持 Provider 切换、角色注入和透明参数转发。

```
┌─────────────────────────────────────────────────────────────┐
│                         AIW 网关                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────┐                                     │
│  │   AI CLI 路由器      │                                     │
│  │                     │                                     │
│  │  aiw claude ...  ───┼───► Claude CLI                     │
│  │  aiw codex ...   ───┼───► Codex CLI                      │
│  │  aiw gemini ...  ───┼───► Gemini CLI                     │
│  │                     │                                     │
│  │  + Provider 切换    │                                     │
│  │  + 角色注入         │                                     │
│  │  + Tool Search 解锁 │                                     │
│  │  + 参数透传         │                                     │
│  │  + 工作目录控制     │                                     │
│  └─────────────────────┘                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 安装

```bash
# 从 crates.io 安装
cargo install aiw

# 验证安装
aiw --version
```

## AI CLI 路由器

### 基本用法

```bash
# 交互模式 - 直接启动 AI CLI
aiw claude    # 以交互模式启动 Claude CLI
aiw codex     # 以交互模式启动 Codex CLI
aiw gemini    # 以交互模式启动 Gemini CLI

# 非交互模式 - 带提示词
aiw claude "解释这段代码"
aiw codex "写测试用例"
aiw gemini "翻译成中文"

# 自动模式：跨 AI CLI 自动故障转移
aiw auto "修复这个 bug"              # 按顺序尝试 CLI，失败时自动切换
aiw auto -p auto "实现这个功能"      # 自动切换 CLI + 自动选择 Provider

# 路由到多个 AI CLI
aiw all "审查这段代码"              # 所有可用的 CLI
aiw "claude|gemini" "比较不同方案"  # 指定 CLI
```

### Provider 切换 (-p)

```bash
# 切换 API provider，无需更改 AI CLI
aiw claude -p openrouter "解释这个"
aiw claude -p glm "解释这个"
aiw claude -p anthropic "解释这个"

# 自动选择兼容的 provider
aiw claude -p auto "解释这个"        # 随机选择兼容的 provider
aiw auto -p auto "实现这个功能"      # 自动 CLI + 自动 provider

# Provider 配置文件：~/.aiw/providers.json
```

### Tool Search 解锁

使用第三方 API Provider（非 Anthropic 官方）时，Claude 的 Tool Search 功能默认被禁用。AIW 通过运行时内存补丁自动解锁此功能。

```bash
# 使用第三方 Provider 时自动启用 Tool Search
aiw claude -p glm "使用网络搜索查找最新的 Rust 新闻"
aiw claude -p openrouter "搜索文档"
```

**工作原理**：AIW 对 Claude 进程应用运行时补丁，将 `if(O8()==="firstParty"&&!JB())` 改为 `if(O8()!=="firstParty"&&!JB())`，为第三方 Provider 启用 Tool Search。

- 支持 Linux/macOS/Windows
- 非破坏性：仅影响运行中的进程，不修改系统文件
- 自动重试，延迟递增（最多 5 次）

### 自动模式（自动故障转移）

```bash
# 自动模式按配置的顺序尝试 CLI+Provider 组合，失败时切换
aiw auto "修复这个 bug"

# 配置 CLI+Provider 执行顺序
aiw config cli-order  # TUI 管理顺序（↑/↓ 移动，r 重置，q 保存）
```

**配置** (`~/.aiw/config.json`)：
```json
{
  "auto_execution_order": [
    {"cli": "codex", "provider": "auto"},
    {"cli": "gemini", "provider": "auto"},
    {"cli": "claude", "provider": "glm"},
    {"cli": "claude", "provider": "local"},
    {"cli": "claude", "provider": "official"}
  ]
}
```

- 同一 CLI 可配置多个 Provider（如 claude+glm → claude+local → claude+official）
- Provider "auto" 表示使用 CLI 的默认 Provider 选择
- 顺序可通过 TUI 或直接编辑配置文件完全自定义

### 角色注入 (-r)

```bash
# 在任务前注入角色提示词
aiw claude -r common "写一个函数"
aiw claude -r security "审查这段代码"
aiw claude -r debugger "修复这个 bug"

# 内置角色 + ~/.aiw/role/*.md 中的自定义角色
aiw roles list
```

### 工作目录 (-C)

```bash
# 在指定目录启动 AI CLI
aiw claude -C /path/to/project "实现功能"
aiw claude -r common -C ~/myproject "修复 bug"
```

### Git Worktree（隔离执行）

AIW 自动创建 git worktree 用于隔离 AI CLI 执行。

```bash
# AIW 自动为 git 仓库创建 worktree
aiw codex -C /path/to/repo "实现功能"

# 完成后，AIW 输出：
# === AIW WORKTREE END ===
# Worktree: /tmp/aiw-worktree-a1b2c3d4
# Branch: main
# Commit: abc123def456
```

AI CLI 在临时 worktree（`/tmp/aiw-worktree-<hash>`）中工作，保持工作目录干净。Worktree 在完成后保留，供手动审查 —— 合并更改或按需删除。

### 透明参数转发

```bash
# 所有未知标志转发给 AI CLI
aiw claude -p glm --model sonnet --debug api "解释这个"
aiw claude -r security --print --output-format json "审查"

# 顺序：aiw 标志 (-r, -p, -C) → AI CLI 标志 → 提示词
```

### 综合示例

```bash
# 包含所有选项的完整示例
aiw claude -r common -p glm -C ~/project --model sonnet "实现 REQ-001"
#          ^^^^^^^^  ^^^^^  ^^^^^^^^^^^  ^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^
#          角色      Provider  工作目录      转发参数       提示词
```

## 任务监控

```bash
# 显示任务状态
aiw status

# 等待所有 AI CLI 任务完成
aiw wait

# 等待指定进程
aiw pwait <PID>
```

## 补丁管理（文件 + 运行时）

AIW 支持文件补丁和内存补丁，对 Claude Code 提供**反间谍/反上报**能力——让 CC 本地环境识别全失明、截断客户端上报给 Anthropic。

```bash
# 查看补丁状态
aiw patch status

# 一键应用全部补丁（推荐）
aiw patch apply --max-context-tokens 500000

# 从备份还原原始二进制
aiw patch restore
```

**补丁类型**：
- **文件补丁**：修改磁盘上的 Claude CLI 二进制（重启后保持，自动备份到 `.aiw-backup`）
- **内存补丁**：文件未修补时运行时自动应用（best-effort）

**支持的安装方式**：
- Native binary (ELF): `~/.local/share/claude/versions/<version>` — **仅支持 GCS native binary**（Bun 打包的 ELF，字节级 patch 目标）
- ❌ npm 安装（`npm install -g @anthropic-ai/claude-code`）**不支持** — 构建产物不同，binary 布局不同，patch regex 不保证命中

**支持的版本**（语义正则 + 稳定字面量，跨版本通用）：

| 版本 | Linux x64 | Linux arm64 |
|---------|-----------|-------------|
| 2.1.195 | ✅ | ✅ |
| 2.1.196 | ✅ | ✅ |
| 2.1.197 | ✅ | ✅ |
| 2.1.198 | ✅ | ✅ |
| 2.1.199 | ✅ | ✅ |
| 2.1.201 | ✅ | ✅ |

> CC 2.1.195+ native binary 仅在 GCS 发布 `linux-x64` + `linux-arm64`。macOS/Windows 自 2.1.195 起不再发布 native binary（GCS 404）。历史版本 2.1.72-74 曾支持 macOS arm64 / Windows x64。

运行 `aiw patch status` 检查版本是否支持。补丁用**语义正则**（通配 minified 变量名 `jUt`/`kre`、`ke`/`Ie`、`dJ`/`dX`、`jJr`/`jXr`）和**稳定字面量**（API 路径、环境变量名），无需维护版本签名数据库即可跨版本工作。

**六层补丁**（反间谍 + 能力解锁）：

| 补丁 | 作用 | 机制 |
|------|------|------|
| **MaxContextTokens** | 解锁默认上下文窗口 + autoCompact 阈值（200000 → 可配置，如 500000） | 正则 `var \w+=200000,\w+=200000[^;]*;`（兼容 4/5/6 元素块） |
| **AntiTelemetry** | 截断 CC 客户端上报（`/api/event_logging/v2/batch` → 404），machineID/userID/设备指纹不发 | 字面量 `batch`→`xxxxx`（27B 等长） |
| **AntiSpy（逃生口短路）** | patch `_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL` 检查让 `fu()` 永远返回 true——一次性关闭 30+ 调用点：中转站身份上报（`custom_base_url` 标记）、归因标头歧视（`cch=00000`）、工具集过滤、ToolSearch 门控、模型覆写门控 | 语义正则 `if(\w+._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0` → `if(1)`（55B 等长） |
| **AntiSpy（时区）** | 时区识别失明——`Intl.DateTimeFormat().resolvedOptions().timeZone` 永远返回 `UTC`，真实时区不泄露 | 字面量 → `"UTC"/*...*/`（48B 等长） |
| **AntiPromptBias** | 消除给第三方用户注入的 Provider context 提示词偏见（`if(dJ())n.push("**Provider context:**...")` → `if(0)`） | 语义正则 `if(\w+())` 通配函数名（63B 等长） |
| **AntiAtis** | 防止 `x-cc-atis` 追踪 header 注入——patch atis 提取函数让它永远返回 `void 0`（逃生口短路 patch 副作用激活 `tMi(firstParty)&&gu()` 条件，patch 提取函数从源头中和） | 语义正则 `function \w+(){let e=\w+()?.atis;...}` → `function zzz(){return void 0}`（80B 等长） |

> **设计说明**：逃生口短路 patch 基于 CC 官方逃生口环境变量 `_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL`（`st()` 函数对 `1`/`true`/`yes`/`on` 解析为 truthy）。patch 检查本身（而非注入环境变量）让它永久生效且覆盖更多调用点。CC v2.1.198 砍掉了被曝光的 `Hsp()` 显性探针（Asia/Shanghai 时区 + base64 主机列表），识别回归 `Cot()`/`fu()` host 比对，由逃生口短路 patch 一并中和。

## 更新

```bash
# 更新 AIW 本身及所有已安装的 AI CLI 工具
aiw update
```

`update` 命令检查并更新：
- **AIW**：通过 `cargo install aiw --force` 更新（如果通过 cargo 安装）
- **Claude CLI**：使用原生 `claude update`（支持 npm 和 cargo 两种安装方式）
- **Codex CLI**：通过 `npm update -g openai-codex` 更新
- **Gemini CLI**：通过 `npm update -g gemini-cli` 更新

**输出示例：**
```
Checking for updates...
✅ AIW 更新成功！
✅ Claude CLI 已更新 (2.1.71 → 2.1.72)
✅ Codex CLI 已是最新版本
⚠️  Gemini CLI 未安装
```

## 配置文件

| 文件 | 用途 |
|------|---------|
| `~/.aiw/config.json` | AIW 全局配置 |
| `~/.aiw/providers.json` | AI Provider 配置 |
| `~/.aiw/role/*.md` | 自定义角色提示词 |

### 全局配置 (~/.aiw/config.json)

```json
{
  "user_roles_dir": "~/.claude/roles",
  "auto_execution_order": [
    {"cli": "codex", "provider": "auto"},
    {"cli": "gemini", "provider": "auto"},
    {"cli": "claude", "provider": "auto"}
  ]
}
```

| 选项 | 类型 | 说明 |
|--------|------|-------------|
| `user_roles_dir` | string | 用户角色的自定义目录（支持 `~` 展开）。如果设置，AIW 将从此目录加载用户角色，而不是 `~/.aiw/role/` |
| `auto_execution_order` | array | 自动模式的 CLI+Provider 组合。每个条目包含 `cli`（codex/gemini/claude）和 `provider`（Provider 名称或 "auto"）。使用 `aiw config cli-order` TUI 管理 |

这允许您在单个位置（如 `~/.claude/roles/`）管理所有角色，并在不同工具之间共享。

## 可用角色

运行 `aiw roles list` 查看所有内置角色。常用角色：

| 角色 | 用途 |
|------|----------|
| `common` | 通用编码（推荐作为基础） |
| `frontend-standards` | 前端开发 |
| `database-standards` | 后端 / 数据库工作 |
| `testing-standards` | 测试代码 |
| `security` | 安全审查 |
| `debugger` | 调试 |
| `devops` | DevOps / 基础设施 |

可用逗号组合角色：`-r common,frontend-standards`

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

[GitHub](https://github.com/putao520/agentic-warden) | [crates.io](https://crates.io/crates/aiw)

---

## 开发者设置

克隆仓库后，启用项目专用的 Git hooks：

```bash
git config core.hooksPath .githooks
```

这确保提交遵循项目规范（例如：更新版本时必须同步更新 README）。
