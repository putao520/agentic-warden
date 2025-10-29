# agentic-warden

<div align="center">

![agentic-warden logo](https://img.shields.io/badge/agentic--warden-v0.3.0-blue.svg)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

**🤖 通用AI Agent管理器**

支持 Claude、Codex、Gemini 等多种AI Agent的统一调用接口，提供进程管理、配置同步和批量任务执行功能

</div>

## ✨ 特性

- 🎯 **统一Agent接口** - 一套语法调用多个AI Agent
- 🚀 **批量任务执行** - 同时向多个Agent发送相同任务
- 📊 **进程树管理** - 跨平台进程监控和任务追踪
- ☁️ **配置云同步** - Google Drive集成的配置备份同步
- 🔒 **OAuth 2.0认证** - 安全的云端访问
- 📦 **跨平台压缩** - TAR.GZ、ZIP、7Z格式支持
- 🧹 **任务监控** - 自动清理和状态管理

## 🚀 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/your-username/codex-warden.git
cd codex-warden

# 构建项目
cargo build --release

# 添加到PATH (可选)
export PATH=$PATH:$(pwd)/target/release
```

### 基本用法

#### 单个Agent调用

```bash
# 使用Claude写代码
agentic-warden claude "写一个Rust快速排序算法"

# 使用Codex生成代码
agentic-warden codex "生成Python数据可视化脚本"

# 使用Gemini解释概念
agentic-warden gemini "解释什么是微服务架构"
```

#### 批量Agent调用

```bash
# 向所有Agent发送任务
agentic-warden all "review this code and suggest improvements"

# 向指定Agent组合发送任务
agentic-warden "claude|gemini" "比较这两种编程方法的优缺点"
agentic-warden "codex|claude" "为这个项目编写文档"
```

#### 配置同步

```bash
# 推送配置到云端
agentic-warden push

# 从云端拉取配置
agentic-warden pull

# 查看同步状态
agentic-warden status
```

#### 任务管理

```bash
# 监控任务执行状态
agentic-warden wait

# 进入CLI管理界面
agentic-warden
```

## 📖 详细文档

### 支持的AI CLI

| AI助手 | 命令 | 描述 | 固化参数 |
|--------|------|------|----------|
| **Claude** | `claude` | Anthropic Claude代码助手 | `-p --dangerously-skip-permissions` |
| **Codex** | `codex` | OpenAI Codex代码生成工具 | `exec --dangerously-bypass-approvals-and-sandbox` |
| **Gemini** | `gemini` | Google Gemini AI助手 | `--approval-mode yolo` |

### CLI选择器语法

```bash
# 单个CLI
codex-warden claude "task description"

# 全部CLI
codex-warden all "task description"

# 组合CLI (需要引号)
codex-warden "claude|gemini" "task description"
codex-warden "claude|codex|gemini" "task description"
```

### 配置文件

#### 认证配置 (`~/.agentic-warden/auth.json`)

```json
{
  "client_id": "your-google-client-id.apps.googleusercontent.com",
  "client_secret": "your-google-client-secret",
  "access_token": "ya29.a0AaHxxxxxxxxxxx",
  "refresh_token": "ya29.c0Ab-xxxxxxxxxxx",
  "expires_at": "2024-12-31T23:59:59Z"
}
```

#### 同步配置 (`~/.agentic-warden/sync.json`)

```json
{
  "config": {
    "directories": ["~/.claude", "~/.codex", "~/.gemini"],
    "auto_sync_enabled": false,
    "sync_interval_minutes": 60
  },
  "state": {
    "directories": {
      "~/.claude": {
        "hash": "d41d8cd98f00b204e9800998ecf8427e",
        "last_sync": "2024-01-01T12:00:00Z"
      }
    },
    "last_sync": "2024-01-01T12:00:00Z"
  }
}
```

## 🛠️ 开发

### 环境要求

- Rust 1.70+
- 支持的平台：Windows、Linux、macOS

### 开发设置

```bash
# 克隆仓库
git clone https://github.com/your-username/codex-warden.git
cd codex-warden

# 安装依赖
cargo build

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 生成文档
cargo doc --open
```

### 项目结构

```
src/
├── main.rs              # 主程序入口
├── cli_type.rs          # CLI类型解析和多CLI支持
├── supervisor.rs        # CLI进程管理和任务执行
├── registry.rs          # 共享内存任务注册表
├── process_tree.rs      # 跨平台进程树检测
├── wait_mode.rs         # 任务监控和清理
├── sync/                # 配置同步模块
│   ├── mod.rs
│   ├── sync_config_manager.rs
│   ├── google_drive_client.rs
│   ├── directory_hasher.rs
│   ├── config_packer.rs
│   └── compressor.rs
├── platform/            # 平台特定代码
│   ├── mod.rs
│   ├── windows.rs
│   └── unix.rs
└── [其他支持模块...]
```

## 📋 命令参考

### Agent任务执行

```bash
agentic-warden <AGENT_SELECTOR> "<TASK_DESCRIPTION>"
```

**参数说明**:
- `AGENT_SELECTOR`: Agent选择器 (`claude`, `codex`, `gemini`, `all`, `"agent1|agent2"`)
- `TASK_DESCRIPTION`: 任务描述文本

**示例**:
```bash
agentic-warden claude "实现一个二叉搜索树"
agentic-warden all "review this rust code"
agentic-warden "claude|gemini" "explain quantum computing"
```

### 配置同步

```bash
# 推送配置到Google Drive
agentic-warden push [directory...]

# 从Google Drive拉取配置
agentic-warden pull [directory...]

# 查看同步状态
agentic-warden status

# 重置同步状态
agentic-warden reset

# 列出可同步的目录
agentic-warden list
```

### 任务管理

```bash
# 等待任务完成并显示状态
agentic-warden wait

# 启动CLI管理界面(无参数)
agentic-warden
```

## 🔧 配置

### 环境变量

```bash
# 自定义Agent路径
export CLAUDE_BIN="/path/to/claude"
export CODEX_BIN="/path/to/codex"
export GEMINI_BIN="/path/to/gemini"

# 配置目录
export AGENTIC_WARDEN_CONFIG_DIR="/custom/config/dir"
```

### Google Drive设置

1. 在Google Cloud Console创建OAuth 2.0客户端ID
2. 将客户端信息添加到 `~/.agentic-warden/auth.json`
3. 运行 `agentic-warden status` 验证连接

## 🐛 故障排除

### 常见问题

**Q: CLI未找到错误**
```bash
Error: 'claude' not found in PATH
```
A: 设置环境变量或确保CLI在PATH中：
```bash
export CLAUDE_BIN="/path/to/claude"
```

**Q: Google Drive认证失败**
```bash
Error: Invalid credentials
```
A: 检查 `auth.json` 文件或重新生成OAuth令牌

**Q: 共享内存错误**
```bash
Error: Failed to connect to shared memory
```
A: 重启系统或清理共享内存段

### 日志文件

任务日志保存在系统临时目录：
- Windows: `%TEMP%\<pid>.log`
- Linux/macOS: `/tmp/<pid>.log`

### 调试模式

```bash
# 启用调试日志
RUST_LOG=debug agentic-warden claude "test task"
```

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细信息。

### 贡献流程

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Claude](https://claude.ai) - AI代码助手
- [OpenAI Codex](https://openai.com/) - 代码生成AI
- [Google Gemini](https://gemini.google.com/) - 多模态AI助手
- Rust社区 - 提供优秀的系统编程语言和生态系统

## 📞 联系方式

- 项目主页: [https://github.com/your-username/agentic-warden](https://github.com/your-username/agentic-warden)
- 问题反馈: [Issues](https://github.com/your-username/agentic-warden/issues)
- 功能请求: [Feature Requests](https://github.com/your-username/agentic-warden/discussions)

---

<div align="center">

**⭐ 如果这个项目对你有帮助，请给它一个星标！**

Made with ❤️ by the agentic-warden team

</div>