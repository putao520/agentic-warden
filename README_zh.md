# AIW - AI CLI 统一网关

[English](README.md)

<div align="center">

![Version](https://img.shields.io/badge/version-0.5.80-blue?style=flat-square)
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
