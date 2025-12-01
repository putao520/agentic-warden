# AIW - AI工作流编排工具

## 功能特性

### 🔧 AI多账户管理
- Claude、OpenAI、Gemini等AI账户统一管理
- 支持OpenRouter、LiteLLM、Cloudflare AI等提供商
- 账户切换和配置管理

### 📁 智能文件同步
- 黑名单机制智能过滤文件
- 支持推送和拉取操作
- 云存储自动同步

### 🎨 现代化终端界面
- 交互式TUI界面
- 实时状态监控
- 可视化操作面板

### 🌐 跨平台支持
- Linux x64 / Windows x64
- 零依赖部署

## 快速开始

### 安装
```bash
npm install -g aiw
```

### 基本使用
```bash
# 显示帮助
aiw --help

# 查看版本
aiw --version

# 启动交互界面
aiw
```

### AI账户管理
```bash
# 列出提供商
aiw provider list

# 添加账户
aiw provider add

# 删除账户
aiw provider remove
```

### 文件同步
```bash
# 推送文件到云端
aiw push

# 从云端拉取文件
aiw pull

# 查看同步状态
aiw status
```

## 许可证

MIT