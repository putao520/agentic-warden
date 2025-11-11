# Agentic-Warden API 接口定义

## CLI 命令接口

### 1. AI CLI 启动命令

#### 1.1 基本语法
```bash
agentic-warden <ai_type> [-p <provider>] "<prompt>"
```

#### 1.2 命令格式
```bash
# 单 AI 启动
agentic-warden codex "prompt"
agentic-warden codex -p openrouter "prompt"
agentic-warden claude --provider litellm "prompt"
agentic-warden gemini -p cloudflare "prompt"

# 多 AI 启动
agentic-warden codex|claude "同时问两个 AI"
agentic-warden codex|claude|gemini "问三个 AI"
agentic-warden all "问所有已安装的 AI"
```

#### 1.3 参数说明
- `<ai_type>`: AI 类型，支持 `codex`, `claude`, `gemini`, `all`
- `-p <provider>`: 可选，指定第三方 Provider
- `--provider <provider>`: 同 `-p`，长格式
- `<prompt>`: 提示词，必须用引号包围

#### 1.4 环境变量注入
根据选择的 Provider 自动注入环境变量：
```rust
fn inject_env_vars(provider_name: &str) -> HashMap<String, String> {
    let provider = config_manager.get_provider(provider_name)?;
    provider.env.clone()
}
```

#### 1.5 多 AI 语法解析
```bash
# 解析规则
codex|claude -> ["codex", "claude"]
codex|claude|gemini -> ["codex", "claude", "gemini"]
all -> 所有已安装的 AI
```

### 2. TUI 管理命令

#### 2.1 Dashboard 命令
```bash
agentic-warden              # 无参数显示 Dashboard
agentic-warden dashboard    # 显式指定
```
**功能**: 显示主界面，包含 AI CLI 状态和任务概要

#### 2.2 Status 命令
```bash
agentic-warden status         # 默认显示文本摘要
agentic-warden status --tui   # 启动TUI界面
```
**功能**:
- **文本模式（默认）**: 显示当前进程隔离区的运行中任务数量
  - 有任务时输出: `running X tasks!`
  - 无任务时输出: `No tasks!`
- **TUI模式（--tui）**: 进入任务状态 TUI，实时显示所有运行中的任务

**示例**:
```bash
$ agentic-warden status
running 3 tasks!

$ agentic-warden status
No tasks!
```

#### 2.3 Push 命令
```bash
agentic-warden push [dirs...]  # 推送指定目录到 Google Drive
agentic-warden push            # 推送当前目录
```
**功能**: 推送文件到 Google Drive，自动检测授权并显示进度 TUI

#### 2.4 Pull 命令
```bash
agentic-warden pull            # 从 Google Drive 拉取所有文件
```
**功能**: 从 Google Drive 拉取文件，自动检测授权并显示进度 TUI

#### 2.5 Wait 命令
```bash
agentic-warden wait [--timeout <duration>] [--verbose]
```
**功能**: 等待当前进程所有任务完成（跨进程共享内存）

**参数**:
- `--timeout <duration>`: 超时时间，如 `12h`, `30m`, `1d`（默认: `12h`）
- `--verbose, -v`: 显示详细进度信息

**示例**:
```bash
$ agentic-warden wait
Waiting for all concurrent AI CLI tasks to complete...
All tasks completed!

$ agentic-warden wait --timeout 1h --verbose
Waiting for all concurrent AI CLI tasks to complete (timeout: 1h)...
Task PID 12345 completed: success
All tasks completed!
```

#### 2.6 PWait 命令
```bash
agentic-warden pwait <PID>
```
**功能**: 等待指定PID进程的共享内存任务完成

**参数**:
- `<PID>`: 必填，要等待的进程PID

**示例**:
```bash
$ agentic-warden pwait 12345
Waiting for 3 tasks from PID 12345 to complete...
Task PID 67890 completed: success
Task PID 67891 completed: success
Task PID 67892 completed: success
All tasks completed!

$ agentic-warden pwait 99999
No tasks found for PID 99999
```

**共享内存命名规则**:
- 每个进程使用独立的共享内存区域，命名格式: `{PID}_task`
- pwait命令连接到指定PID的共享内存区域
- 进程结束时自动清理对应的共享内存

#### 2.7 Provider 管理
```bash
agentic-warden provider    # 直接进入 Provider 管理 TUI 界面
```
**功能**: 直接启动 TUI 并显示 Provider 管理界面（等效于先启动 Dashboard 再按 'P' 键）

**注意**:
- Provider 管理功能完全集成在 TUI 界面中
- 该命令只是一个快捷方式，直接进入指定的 TUI 屏幕
- 用户也可以通过 Dashboard 的快捷键（'P'）进入 Provider 管理界面

## TUI 界面交互接口

### 1. Dashboard 界面

#### 1.1 显示内容
- 🤖 **AI CLI 状态**: 安装情况、版本、默认 Provider
- 📊 **任务概要**: 当前运行的任务（最多显示 5 个）

#### 1.2 键盘交互
| 按键 | 功能 | 说明 |
|------|------|------|
| `P` | 进入 Provider 管理 | 切换到 Provider 列表界面 |
| `S` | 进入任务状态 | 切换到任务状态界面 |
| `Q` | 退出 | 退出程序 |
| `ESC` | 返回/退出 | 同 Q |

### 2. Provider 管理界面

#### 2.1 Provider 列表界面
**显示内容**:
- 所有 Provider 列表
- 每个 Provider 的描述、兼容性、默认状态

**键盘交互**:
| 按键 | 功能 | 说明 |
|------|------|------|
| `↑↓` | 选择 Provider | 上下移动选择光标 |
| `A` | 添加新 Provider | 进入添加界面 |
| `E` | 编辑选中 Provider | 进入编辑界面 |
| `D` | 删除选中 Provider | 删除并确认 |
| `Enter` | 设为默认 | 将选中 Provider 设为默认 |
| `ESC` | 返回 Dashboard | 返回主界面 |

#### 2.2 Provider 编辑界面
**显示内容**:
- Provider 名称（只读）
- 描述文本（可编辑）
- 兼容的 AI 类型（复选框）
- 环境变量列表（键值对编辑）

**交互逻辑**:
```rust
pub struct ProviderEditState {
    pub provider: Provider,
    pub editing_field: EditField,
    pub env_vars: Vec<(String, String)>,
}

pub enum EditField {
    Description,
    CompatibleWith,
    EnvVars(usize), // 索引到 env_vars
}
```

### 3. 任务状态界面

#### 3.1 显示内容
- 所有运行中的任务（详细版）
- 按父进程分组显示
- 实时刷新（每 2 秒）

**任务信息格式**:
```
📁 Explorer.exe (PID: 1234)
├── 🤖 codex: "Write a function..." [Running] 00:01:23
├── 🤖 claude: "Debug this issue..." [Running] 00:00:45
└── 🤖 gemini: "Explain this code..." [Completed] 00:02:10
```

#### 3.2 键盘交互
| 按键 | 功能 | 说明 |
|------|------|------|
| `↑↓` | 选择任务 | 上下移动选择光标 |
| `K` | 终止选中任务 | 发送终止信号 |
| `R` | 手动刷新 | 立即刷新任务列表 |
| `ESC/Q` | 返回 Dashboard | 返回主界面 |

### 4. Push/Pull 进度界面

#### 4.1 自动触发
```bash
# 执行 push 命令时的流程
1. 检查 Google Drive 授权状态
2. 如果未授权 → 显示授权对话框
3. 用户同意 → 启动 Device Flow 认证
4. 授权成功 → 自动开始 push 操作
5. 显示进度 TUI
```

#### 4.2 进度显示格式
```
📤 Pushing to Google Drive
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 85% (127/149 files)

📁 src/
├── ✅ main.rs (1.2KB)
├── ✅ lib.rs (3.4KB)
└── 🔄 processing... tui/mod.rs (2.1KB)

📊 Total: 127 files | ✅ Completed: 108 | 🔄 Processing: 1 | ⏳ Pending: 18
```

#### 4.3 授权对话框
```rust
pub struct AuthDialog {
    pub auth_url: String,
    pub status: AuthStatus,
}

pub enum AuthStatus {
    Waiting,        // 等待用户操作
    CallbackStarted, // 回调服务器启动
    Authorized,     // 授权成功
    Failed(String), // 授权失败
}
```

## 配置文件接口

### 1. Provider JSON 格式
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

### 2. Auth JSON 格式
```json
{
  "access_token": "ya29.a0AfH6SMC...",
  "refresh_token": "1//0g3...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "expiry_time": "2025-11-04T19:30:00Z"
}
```

## 错误处理接口

### 1. CLI 错误响应
```bash
# Provider 不存在
Error: Provider 'invalid-provider' not found
Available providers: openrouter, litellm, official

# 授权失败
Error: Google Drive authorization failed
Please run: agentic-warden push

# AI CLI 未安装
Error: 'codex' is not installed or not in PATH
Please install codex CLI first
```

### 2. TUI 错误对话框
```
┌─────────────────────────────────────┐
│              ❌ Error               │
├─────────────────────────────────────┤
│ Failed to connect to Google Drive   │
│                                     │
│ Network timeout after 30 seconds    │
│                                     │
│         [  Retry  ]  [  Cancel  ]   │
└─────────────────────────────────────┘
```

### 3. 错误代码定义
```rust
#[derive(Debug, thiserror::Error)]
pub enum AgenticWardenError {
    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    #[error("Authorization failed: {0}")]
    AuthFailed(String),

    #[error("AI CLI '{0}' not found in PATH")]
    AiCliNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Process tree error: {0}")]
    ProcessTreeError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## 扩展接口

### 1. Provider 插件接口
```rust
pub trait ProviderPlugin {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn compatible_with(&self) -> Vec<AiType>;
    fn env_vars(&self) -> HashMap<String, String>;
    fn validate(&self) -> Result<(), String>;
}
```

### 2. AI CLI 检测接口
```rust
pub trait AiCliDetector {
    fn detect_ai_cli(&self) -> Vec<DetectedAiCli>;
    fn is_available(&self, ai_type: AiType) -> bool;
    fn get_version(&self, ai_type: AiType) -> Option<String>;
}

pub struct DetectedAiCli {
    pub ai_type: AiType,
    pub path: PathBuf,
    pub version: String,
    pub is_default: bool,
}
```